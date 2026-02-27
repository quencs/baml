#![allow(clippy::print_stdout, clippy::print_stderr)]

use std::{path::PathBuf, sync::Arc};

use anyhow::{Context, Result, anyhow};
use baml_db::{baml_compiler_diagnostics::Severity, baml_compiler_emit};
use baml_project::ProjectDatabase;
use baml_workspace::discover_baml_files;
use bex_engine::{BexEngine, FunctionCallContextBuilder};
use clap::Args;
use sys_native::{CallId, SysOpsExt};

#[derive(Args, Clone, Debug)]
pub struct RunArgs {
    /// Name of the function to run (must take no arguments)
    pub function_name: String,

    /// Path to the baml_src directory
    #[arg(long, default_value = ".")]
    pub from: PathBuf,
}

impl RunArgs {
    pub fn run(&self) -> Result<crate::ExitCode> {
        let from = std::fs::canonicalize(&self.from)
            .with_context(|| format!("Could not resolve baml_src path: {}", self.from.display()))?;

        // Set up the compiler database and load all .baml files.
        let mut db = ProjectDatabase::new();
        let project = db.set_project_root(&from);
        let baml_files = discover_baml_files(&from);
        if baml_files.is_empty() {
            eprintln!("No .baml files found in {}", from.display());
            return Ok(crate::ExitCode::Other);
        }

        for file_path in &baml_files {
            let content = std::fs::read_to_string(file_path)
                .with_context(|| format!("Failed to read {}", file_path.display()))?;
            db.add_or_update_file(file_path, &content);
        }

        // Check for diagnostic errors before compiling.
        let source_files = db.get_source_files();
        let diagnostics = baml_project::collect_diagnostics(&db, project, &source_files);
        let errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .collect();
        if !errors.is_empty() {
            eprintln!("Compilation errors found ({}):", errors.len());
            for diag in &errors {
                eprintln!("  error: {}", diag.message);
            }
            return Ok(crate::ExitCode::Other);
        }

        // Compile to bytecode (no test cases needed).
        let compile_options = baml_compiler_emit::CompileOptions {
            emit_test_cases: false,
        };
        let bytecode = baml_compiler_emit::generate_project_bytecode(&db, &compile_options)
            .map_err(|e| anyhow!("Compilation failed: {e:?}"))?;

        // Create the engine with native (tokio-based) sys ops.
        let engine = BexEngine::new(bytecode, Arc::new(sys_native::SysOps::native()), None)
            .map_err(|e| anyhow!("Failed to create engine: {e:?}"))?;

        // Validate the function exists and takes no arguments.
        let params = engine
            .function_params(&self.function_name)
            .map_err(|e| anyhow!("Function '{}' not found: {e:?}", self.function_name))?;

        if !params.is_empty() {
            let param_names: Vec<&str> = params.iter().map(|(name, _)| *name).collect();
            eprintln!(
                "Function '{}' takes {} argument(s): {}",
                self.function_name,
                params.len(),
                param_names.join(", ")
            );
            eprintln!("Only zero-argument functions can be run directly.");
            return Ok(crate::ExitCode::Other);
        }

        // Create a tokio runtime and execute the function.
        let rt = tokio::runtime::Runtime::new().context("Failed to create tokio runtime")?;

        match rt.block_on(engine.call_function(
            &self.function_name,
            vec![],
            FunctionCallContextBuilder::new(CallId::next()).build(),
        )) {
            Ok(result) => {
                println!("{result:?}");
                Ok(crate::ExitCode::Success)
            }
            Err(e) => {
                eprintln!("Error: {e:?}");
                Ok(crate::ExitCode::Other)
            }
        }
    }
}
