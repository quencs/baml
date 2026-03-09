//! Pre-Processed Intermediate Representation (PPIR) for compiler2.
//!
//! Sits between the AST and HIR. Responsible for:
//! 1. Stream annotation capture from AST items (type-level via `PpirTy::from_type_expr`,
//!    field-level via `build_ppir_field`)
//! 2. Cross-file name classification (`PpirNames`)
//! 3. Stream type expansion (`stream_expand` on `PpirTy`)
//! 4. `@sap.*` attribute synthesis (`sap_starts_as`, `sap_in_progress_never`)
//! 5. Synthetic AST item generation (`synthesize_stream_items`)
//!
//! PPIR runs before HIR so that `stream_*` types are regular BAML types —
//! they go through the same HIR resolution, TIR type-checking, and codegen
//! as user-defined types. This is a deliberate departure from engine/, where
//! stream types were a dual type system with special semantics.
//!
//! **No union simplification in PPIR.** Synthesized field types may contain
//! redundant unions (e.g., `null | null | string`, `never | int`). This is
//! deliberate — union simplification (flattening, dedup, never-removal,
//! literal subsumption) is deferred to TIR, which already has the machinery
//! for type normalization. Keeping PPIR output unsimplified makes the
//! expansion logic simpler and easier to reason about.

pub mod desugar;
pub mod normalize;
pub mod synthesize;
pub mod ty;

pub use desugar::{
    PpirDesugaredClass, PpirDesugaredField, PpirDesugaredTypeAlias, PpirStreamStartsAs,
    build_ppir_field, default_sap_starts_as, desugar_field, stream_expand,
};
pub use ty::{PpirField, PpirTy, PpirTypeAttrs};

use baml_base::{Name, SourceFile};
use baml_compiler2_ast as ast;
use baml_workspace::Project;
use rustc_hash::{FxHashMap, FxHashSet};

//
// ──────────────────────────────────────────────────────── NAMES ─────
//

/// Cross-file name classification for stream expansion.
///
/// Tells `stream_expand` which names are classes/type-aliases (get `stream_` prefix)
/// vs. enums (stay unchanged). Built from AST items before desugaring.
///
/// `class_names` and `enum_names` map type name → list of block-level `@@stream.*` attribute names.
#[derive(Debug, Clone, Default)]
pub struct PpirNames {
    /// Class names → their `@@stream.*` block attributes.
    pub class_names: FxHashMap<Name, Vec<Name>>,
    /// Enum names → their `@@stream.*` block attributes.
    pub enum_names: FxHashMap<Name, Vec<Name>>,
    /// Type alias names (no block attrs tracked).
    pub type_alias_names: FxHashSet<Name>,
}

//
// ──────────────────────────────────────────────────────── DATABASE ─────
//

/// Database trait for PPIR queries.
///
/// Extends `baml_workspace::Db` — NOT `baml_compiler2_hir::Db`.
/// PPIR sits below HIR in the dependency chain.
#[salsa::db]
pub trait Db: baml_workspace::Db {}

//
// ───────────────────────────────────────────────────── TRACKED STRUCTS ─────
//

/// Per-file result of PPIR desugaring.
/// Contains desugared data for classes and type aliases.
#[salsa::tracked]
pub struct PpirDesugaredItems<'db> {
    #[tracked]
    #[returns(ref)]
    pub classes: Vec<PpirDesugaredClass>,
    #[tracked]
    #[returns(ref)]
    pub type_aliases: Vec<PpirDesugaredTypeAlias>,
}

/// Per-file synthetic AST items produced by PPIR expansion.
/// Contains `stream_*` class and type alias definitions ready to merge into HIR.
#[salsa::tracked]
pub struct PpirExpansionItems<'db> {
    #[tracked]
    #[returns(ref)]
    pub items: Vec<ast::Item>,
}

//
// ────────────────────────────────────────────────────────── SALSA QUERIES ─────
//

/// Collect name sets across all files by walking AST items.
///
/// Reads from `lower_file(syntax_tree(file))` — does NOT depend on HIR.
/// Returns a plain `PpirNames` struct (not Salsa-tracked) since the
/// desugar module operates without db access.
pub fn collect_ppir_names(db: &dyn Db, project: Project) -> PpirNames {
    let mut class_names: FxHashMap<Name, Vec<Name>> = FxHashMap::default();
    let mut enum_names: FxHashMap<Name, Vec<Name>> = FxHashMap::default();
    let mut type_alias_names = FxHashSet::default();

    for file in project.files(db) {
        // Skip builtin/generated files
        if file
            .path(db)
            .to_str()
            .is_some_and(|p| p.starts_with("<builtin>/") || p.starts_with("<generated:"))
        {
            continue;
        }

        let cst = baml_compiler_parser::syntax_tree(db, *file);
        let (items, _) = ast::lower_file(&cst);

        for item in &items {
            match item {
                ast::Item::Class(c) => {
                    let stream_attrs: Vec<Name> = c
                        .attributes
                        .iter()
                        .filter(|a| a.name.starts_with("stream."))
                        .map(|a| a.name.clone())
                        .collect();
                    class_names.insert(c.name.clone(), stream_attrs);
                }
                ast::Item::Enum(e) => {
                    let stream_attrs: Vec<Name> = e
                        .attributes
                        .iter()
                        .filter(|a| a.name.starts_with("stream."))
                        .map(|a| a.name.clone())
                        .collect();
                    enum_names.insert(e.name.clone(), stream_attrs);
                }
                ast::Item::TypeAlias(a) => {
                    type_alias_names.insert(a.name.clone());
                }
                _ => {}
            }
        }
    }

    PpirNames {
        class_names,
        enum_names,
        type_alias_names,
    }
}

/// Compute desugared stream data for a single file.
///
/// For each class: runs `stream_expand` per field to compute `stream_type`,
/// synthesizes `@sap.*` attributes (`sap_in_progress_never`, `sap_starts_as`).
/// For each type alias: runs `stream_expand` on the alias body.
#[salsa::tracked]
pub fn ppir_desugared_items(db: &dyn Db, file: SourceFile) -> PpirDesugaredItems<'_> {
    let file_path = file.path(db);
    if file_path
        .to_str()
        .is_some_and(|p| p.starts_with("<builtin>/") || p.starts_with("<generated:"))
    {
        return PpirDesugaredItems::new(db, Vec::new(), Vec::new());
    }

    let cst = baml_compiler_parser::syntax_tree(db, file);
    let (items, _) = ast::lower_file(&cst);
    let project = db.project();
    let names = collect_ppir_names(db, project);

    let mut desugared_classes = Vec::new();
    let mut desugared_aliases = Vec::new();
    let mut seen_class_names = FxHashSet::default();
    let mut seen_alias_names = FxHashSet::default();

    for item in &items {
        match item {
            ast::Item::Class(c) => {
                if c.name.starts_with("stream_") {
                    continue;
                }
                if !seen_class_names.insert(c.name.clone()) {
                    continue;
                }

                let ppir_fields: Vec<PpirField> = c.fields.iter().map(build_ppir_field).collect();
                let desugared_fields: Vec<PpirDesugaredField> = ppir_fields
                    .iter()
                    .map(|pf| desugar_field(pf, &names))
                    .collect();

                desugared_classes.push(PpirDesugaredClass {
                    name: c.name.clone(),
                    fields: desugared_fields,
                });
            }
            ast::Item::TypeAlias(a) => {
                if a.name.starts_with("stream_") {
                    continue;
                }
                if !seen_alias_names.insert(a.name.clone()) {
                    continue;
                }

                let ty = a
                    .type_expr
                    .as_ref()
                    .map(|s| PpirTy::from_type_expr(&s.expr, &[]))
                    .unwrap_or(PpirTy::Unknown {
                        attrs: PpirTypeAttrs::default(),
                    });

                let expanded_body = stream_expand(&ty, &names);

                desugared_aliases.push(PpirDesugaredTypeAlias {
                    name: a.name.clone(),
                    expanded_body,
                });
            }
            _ => {}
        }
    }

    PpirDesugaredItems::new(db, desugared_classes, desugared_aliases)
}

/// Compute synthetic AST items for a single file's stream_* definitions.
///
/// This is the main entry point called by HIR.
#[salsa::tracked]
pub fn ppir_expansion_items(db: &dyn Db, file: SourceFile) -> PpirExpansionItems<'_> {
    let desugared = ppir_desugared_items(db, file);
    if desugared.classes(db).is_empty() && desugared.type_aliases(db).is_empty() {
        return PpirExpansionItems::new(db, Vec::new());
    }

    // Collect original class attributes for @@stream.done detection and passthrough
    let cst = baml_compiler_parser::syntax_tree(db, file);
    let (items, _) = ast::lower_file(&cst);
    let original_class_attrs: FxHashMap<Name, Vec<ast::RawAttribute>> = items
        .iter()
        .filter_map(|item| match item {
            ast::Item::Class(c) => Some((c.name.clone(), c.attributes.clone())),
            _ => None,
        })
        .collect();

    let synthetic =
        synthesize::synthesize_stream_items(desugared.classes(db), desugared.type_aliases(db), &original_class_attrs);
    PpirExpansionItems::new(db, synthetic)
}

