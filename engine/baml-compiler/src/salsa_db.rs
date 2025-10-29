//! Salsa database for incremental BAML compilation.
//!
//! This module provides a Salsa-based incremental compilation infrastructure for BAML.
//! Currently implemented as a "big blob" query that wraps the entire compilation pipeline,
//! with the plan to incrementalize individual phases later.
//!
//! ## Architecture
//!
//! The compilation pipeline:
//! ```text
//! SourceFileSet (input)
//!   ↓
//! compile_baml_to_bytecode (query)
//!   ├─ Parse sources → AST
//!   ├─ Lower AST → HIR
//!   ├─ Typecheck HIR → THIR
//!   └─ Generate THIR → Bytecode
//!   ↓
//! CompilationResult (output)
//! ```
//!
//! ## Future Incrementalization
//!
//! The plan is to split this into separate queries:
//! - `parse_sources(db, sources) -> ParserDatabase`
//! - `lower_to_hir(db, parser_db) -> Hir`
//! - `typecheck_to_thir(db, hir) -> THir`
//! - `generate_bytecode(db, thir) -> BamlVmProgram`
//!
//! This will allow Salsa to cache intermediate results and only recompute
//! affected phases when source files change.

use std::{
    collections::{BTreeMap, HashMap},
    path::PathBuf,
    sync::Arc,
};

use baml_vm::{indexable::Pool, BamlVmProgram};
use internal_baml_ast as ast;
use internal_baml_diagnostics::{Diagnostics, SourceFile};
use internal_baml_parser_database::ParserDatabase;

use crate::{codegen, hir, thir};

/// Result of compilation - always produces output with optional diagnostics.
///
/// In an incremental compiler, compilation never "fails" in the traditional sense.
/// Instead, it always produces *some* output (which may be partial/incomplete),
/// along with diagnostics that describe any issues encountered.
///
/// This design allows the compiler to provide IDE features (autocomplete, hover, etc.)
/// even when the code has errors.
///
/// The program is wrapped in Arc to make the type implement PartialEq efficiently
/// (by comparing pointers) and to avoid expensive clones.
#[derive(Clone, Debug)]
pub struct CompilationResult {
    /// The compiled program (may be partial/incomplete if diagnostics contains errors)
    pub program: Arc<BamlVmProgram>,
    /// Diagnostics collected during compilation (errors, warnings, etc.)
    pub diagnostics: Diagnostics,
}

impl PartialEq for CompilationResult {
    fn eq(&self, other: &Self) -> bool {
        // Compare programs by pointer equality for efficiency
        // We don't compare diagnostics because they don't implement PartialEq
        // and for Salsa's purposes, the program is the key output
        Arc::ptr_eq(&self.program, &other.program)
    }
}

impl CompilationResult {
    /// Check if compilation had any errors
    pub fn has_errors(&self) -> bool {
        self.diagnostics.has_errors()
    }

    /// Get the program, consuming the result
    pub fn into_program(self) -> BamlVmProgram {
        Arc::unwrap_or_clone(self.program)
    }

    /// Get a reference to the program
    pub fn program(&self) -> &BamlVmProgram {
        &self.program
    }

    /// Get a reference to the diagnostics
    pub fn diagnostics(&self) -> &Diagnostics {
        &self.diagnostics
    }
}

/// Input: A collection of BAML source files.
///
/// This is a Salsa "input" type - when its contents change, all downstream
/// queries that depend on it will be invalidated and recomputed.
///
/// # Structure
///
/// We use a `BTreeMap` instead of `HashMap` for:
/// - Deterministic ordering (helpful for testing and debugging)
/// - Consistent iteration order
///
/// Files are stored as `Arc<str>` to make cloning cheap, which is required
/// by Salsa's query system.
#[salsa::input]
pub struct SourceFileSet {
    /// Map from file path to file contents.
    pub files: BTreeMap<PathBuf, Arc<str>>,
}

/// The "big blob" query: Compile all BAML sources to VM bytecode.
///
/// This function wraps the entire BAML compilation pipeline in a single Salsa query:
/// 1. Parse source files into AST
/// 2. Lower AST to HIR (High-level IR)
/// 3. Typecheck HIR to produce THIR (Typed HIR)
/// 4. Generate bytecode from THIR
///
/// Currently, all intermediate steps happen inside this single query, so we don't
/// get incremental benefits from Salsa yet. The plan is to refactor this into
/// separate queries for each phase, allowing Salsa to cache and reuse intermediate
/// results.
///
/// # Return Value
///
/// Always returns a `CompilationResult` containing:
/// - A program (potentially partial if there were errors)
/// - Diagnostics (errors, warnings, etc.)
///
/// The compiler will do its best to produce a valid program even in the presence
/// of errors, allowing IDE features to work with partially valid code.
///
/// # Example
///
/// ```rust,ignore
/// use baml_compiler::{CompilerDatabase, SourceFileSet, compile_baml_to_bytecode};
/// use std::collections::BTreeMap;
/// use std::sync::Arc;
/// use std::path::PathBuf;
///
/// let db = CompilerDatabase::default();
///
/// let mut files = BTreeMap::new();
/// files.insert(
///     PathBuf::from("main.baml"),
///     Arc::from("class Foo { x int }".into())
/// );
///
/// let sources = SourceFileSet::new(&db, files);
/// let result = compile_baml_to_bytecode(&db, sources);
///
/// // Check for errors
/// if result.has_errors() {
///     eprintln!("Compilation errors: {}", result.diagnostics().to_pretty_string());
/// }
///
/// // Use the program (even if there were errors)
/// let program = result.program();
/// ```
#[salsa::tracked]
pub fn compile_baml_to_bytecode(
    db: &dyn salsa::Database,
    sources: SourceFileSet,
) -> CompilationResult {
    // Initialize diagnostics collector
    let mut diag = Diagnostics::new("compilation".into());

    // Step 1: Build ParserDatabase from source files
    let mut parser_db = ParserDatabase::new();

    // Parse each source file into AST and add to parser database
    for (path, contents) in sources.files(db).iter() {
        let source_file = SourceFile::new_allocated(path.clone(), contents.clone());

        match ast::parse(source_file.path_buf(), &source_file) {
            Ok((ast, parse_diag)) => {
                parser_db.add_ast(ast);
                // Merge diagnostics
                for error in parse_diag.errors() {
                    diag.push_error(error.clone());
                }
                for warning in parse_diag.warnings() {
                    diag.push_warning(warning.clone());
                }
            }
            Err(e) => {
                // Merge diagnostics from error
                for error in e.errors() {
                    diag.push_error(error.clone());
                }
                for warning in e.warnings() {
                    diag.push_warning(warning.clone());
                }
            }
        }
    }

    // Validate the parser database (name resolution, type resolution, attributes)
    // Continue even if validation fails - we still want to produce output
    let _ = parser_db.validate(&mut diag);

    // Finalize dependency resolution and cycle detection
    parser_db.finalize(&mut diag);

    // Step 2: Lower AST → HIR
    // This performs desugaring (for loops → while loops, etc.)
    // Continue even if there are errors - we can still lower what we have
    let hir = hir::Hir::from_ast(&parser_db.ast);

    // Step 3: Typecheck HIR → THIR
    // This adds type information to expressions
    // Typechecking will add errors to diag but won't stop processing
    let thir = thir::typecheck::typecheck(&hir, &mut diag);

    // Step 4: Generate bytecode from THIR
    // Even if there were type errors, we try to generate bytecode from what we have
    // This allows partial programs to be used for IDE features
    let program = match codegen::compile_thir_to_bytecode(&thir) {
        Ok(program) => program,
        Err(e) => {
            // If bytecode generation fails catastrophically, we still need to return *something*
            // Create an empty/minimal program and record the error in diagnostics
            let error_msg = format!("Failed to generate bytecode: {}", e);
            let error_source =
                SourceFile::new_allocated(PathBuf::from("<bytecode>"), Arc::from(""));
            diag.push_error(
                internal_baml_diagnostics::DatamodelError::new_validation_error(
                    &error_msg,
                    internal_baml_diagnostics::Span::empty(error_source),
                ),
            );
            // Return an empty program as fallback
            BamlVmProgram {
                objects: Pool::new(),
                globals: Pool::new(),
                resolved_function_names: HashMap::new(),
                resolved_class_names: HashMap::new(),
                resolved_enums_names: HashMap::new(),
            }
        }
    };

    CompilationResult {
        program: Arc::new(program),
        diagnostics: diag,
    }
}

/// Concrete database implementation.
///
/// This is the actual database type that users will instantiate to perform
/// BAML compilation. It contains the Salsa storage.
///
/// # Example
///
/// ```rust,ignore
/// let db = CompilerDatabase::default();
/// // Use db with Salsa queries...
/// ```
#[salsa::db]
#[derive(Default, Clone)]
pub struct CompilerDatabase {
    storage: salsa::Storage<Self>,
}

impl salsa::Database for CompilerDatabase {}
