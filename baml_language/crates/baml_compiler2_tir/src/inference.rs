//! Per-scope type inference — Salsa entry queries.
//!
//! `infer_scope_types(db, ScopeId)` is the main query: it returns
//! `ScopeInference`, which maps `ExprId → Ty` for a single scope.
//!
//! Lambda/closure bodies are separate scopes with their own `infer_scope_types`
//! invocation — editing a lambda body only re-runs that scope's query, not
//! the enclosing function's.
//!
//! Per-item queries (`resolve_class_fields`, `resolve_type_alias`) provide
//! Salsa-cached structural type resolution for class fields and type aliases.

use std::{collections::HashMap, sync::Arc};

use baml_base::Name;
use baml_compiler2_ast::{ExprId, PatId};
use baml_compiler2_hir::{
    body::FunctionBody,
    contributions::Definition,
    loc::{ClassLoc, TypeAliasLoc},
    package::{PackageId, PackageItems, package_items},
    scope::{ScopeId, ScopeKind},
};
use rustc_hash::FxHashMap;

use crate::{
    builder::TypeInferenceBuilder,
    infer_context::{InferContext, TypeCheckDiagnostics},
    ty::Ty,
};

// ── Per-Scope Inference Result ─────────────────────────────────────────────

/// Per-scope type inference result.
///
/// Each scope (function body, lambda, class method, block) gets its own
/// `ScopeInference` cached independently by Salsa. This is the Ty-style
/// decomposed approach — NOT a monolithic per-function struct.
///
/// Modeled after Ty's `ScopeInference<'db>` (`infer.rs:557-563`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopeInference<'db> {
    /// Type of every expression within this scope (NOT nested child scopes).
    expressions: FxHashMap<ExprId, Ty>,
    /// Binding types: the type a variable is bound to after widening/annotation.
    /// May differ from the initializer expression type (e.g. `let x = 1` has
    /// expression type `Literal(1, Fresh)` but binding type `int`).
    bindings: FxHashMap<PatId, Ty>,
    /// Diagnostics and other rare data. Heap-allocated only when non-empty.
    extra: Option<Box<ScopeInferenceExtra<'db>>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopeInferenceExtra<'db> {
    pub diagnostics: TypeCheckDiagnostics<'db>,
}

// Safety: `ScopeInference<'db>` contains `ExprId` (arena indices) and `Ty`
// (which contains `Name`, a Salsa-interned type). The `FxHashMap` doesn't
// implement `salsa::Update` automatically; we provide the impl manually.
#[allow(unsafe_code)]
unsafe impl<'db> salsa::Update for ScopeInference<'db> {
    unsafe fn maybe_update(old_pointer: *mut Self, new_value: Self) -> bool {
        #[allow(unsafe_code)]
        let old = unsafe { &*old_pointer };
        if old == &new_value {
            false
        } else {
            #[allow(unsafe_code)]
            unsafe {
                std::ptr::drop_in_place(old_pointer);
                std::ptr::write(old_pointer, new_value);
            }
            true
        }
    }
}

impl<'db> ScopeInference<'db> {
    /// Look up the type of an expression in this scope.
    pub fn expression_type(&self, expr_id: ExprId) -> Option<&Ty> {
        self.expressions.get(&expr_id)
    }

    /// Look up the binding type for a pattern (the type the variable is bound to,
    /// which may differ from the initializer expression type due to widening).
    pub fn binding_type(&self, pat_id: PatId) -> Option<&Ty> {
        self.bindings.get(&pat_id)
    }

    /// Get diagnostics for this scope (empty slice if none).
    pub fn diagnostics(&self) -> &TypeCheckDiagnostics<'db> {
        self.extra
            .as_ref()
            .map(|e| &e.diagnostics)
            .unwrap_or_else(|| {
                // Use a static empty diagnostics — safe since TypeCheckDiagnostics
                // with no diagnostics is logically equivalent to the default.
                static EMPTY: std::sync::OnceLock<TypeCheckDiagnostics<'static>> =
                    std::sync::OnceLock::new();
                // SAFETY: we return a reference with lifetime tied to 'db.
                // The static EMPTY has no 'db-tied data (empty Vec).
                #[allow(unsafe_code)]
                unsafe {
                    let empty = EMPTY.get_or_init(TypeCheckDiagnostics::default);
                    // Extend the lifetime — safe because the data is empty and 'static.
                    &*(empty as *const TypeCheckDiagnostics<'static>
                        as *const TypeCheckDiagnostics<'db>)
                }
            })
    }
}

// ── Main Salsa Query: Per-Scope Inference ───────────────────────────────────

/// Per-scope type inference — the primary Salsa query for type checking.
///
/// Returns expression types for a single scope. Lambda/closure bodies are
/// separate scopes with their own query invocation.
///
/// Keyed by `ScopeId<'db>` (tracked: `File + FileScopeId`), so Salsa caches
/// independently per scope. Editing lambda A does NOT invalidate the enclosing
/// function's `ScopeInference`.
#[salsa::tracked(returns(ref))]
pub fn infer_scope_types<'db>(
    db: &'db dyn crate::Db,
    scope_id: ScopeId<'db>,
) -> ScopeInference<'db> {
    let file = scope_id.file(db);
    let file_scope = scope_id.file_scope_id(db);
    let index = baml_compiler2_hir::file_semantic_index(db, file);
    let scope = &index.scopes[file_scope.index() as usize];

    // Get package items for cross-file resolution
    let pkg_info = baml_compiler2_hir::file_package::file_package(db, file);
    let pkg_id = PackageId::new(db, pkg_info.package.clone());
    let pkg_items = package_items(db, pkg_id);

    let aliases = collect_type_aliases(db, pkg_items);
    let context = InferContext::new(db, scope_id);
    let mut builder = TypeInferenceBuilder::new(context, pkg_items, scope_id, aliases);

    // Dispatch based on scope kind
    match &scope.kind {
        ScopeKind::Function => {
            // Find the function by matching scope range against item_tree functions.
            // This works for both top-level functions AND class methods.
            let item_tree = baml_compiler2_hir::file_item_tree(db, file);
            let mut found = false;
            for (local_id, func_data) in &item_tree.functions {
                if func_data.span == scope.range {
                    let func_loc = baml_compiler2_hir::loc::FunctionLoc::new(db, file, *local_id);
                    let body = baml_compiler2_hir::body::function_body(db, func_loc);
                    let sig = baml_compiler2_hir::signature::function_signature(db, func_loc);

                    if let FunctionBody::Expr(expr_body) = body.as_ref() {
                        // Get declared return type
                        let return_ty = sig
                            .return_type
                            .as_ref()
                            .map(|te| crate::lower_type_expr::lower_type_expr(te, pkg_items))
                            .unwrap_or(Ty::Unknown);

                        // Set declared return type for return statement checking
                        builder.set_return_type(return_ty.clone());

                        // Determine enclosing class name for `self` parameter resolution
                        let enclosing_class_name: Option<Name> =
                            scope.parent.and_then(|parent_idx| {
                                let parent = &index.scopes[parent_idx.index() as usize];
                                if matches!(parent.kind, ScopeKind::Class) {
                                    parent.name.clone()
                                } else {
                                    None
                                }
                            });

                        // Add parameter bindings as locals
                        for (param_name, param_te) in &sig.params {
                            let param_ty = if param_name.as_str() == "self"
                                && matches!(param_te, baml_compiler2_ast::TypeExpr::Unknown)
                            {
                                // `self` parameter with no type annotation — infer from enclosing class
                                enclosing_class_name
                                    .as_ref()
                                    .map(|cn| Ty::Class(cn.clone()))
                                    .unwrap_or(Ty::Unknown)
                            } else {
                                crate::lower_type_expr::lower_type_expr(param_te, pkg_items)
                            };
                            builder.add_local(param_name.clone(), param_ty);
                        }

                        // Check root expression against declared return type
                        if let Some(root_expr) = expr_body.root_expr {
                            builder.check_expr(root_expr, expr_body, &return_ty);
                        }
                    }
                    found = true;
                    break;
                }
            }
            let _ = found;
        }
        ScopeKind::Lambda => {
            // Lambda bodies are handled when the enclosing function walks its ExprBody.
            // When the builder encounters a lambda, it stops — the lambda scope
            // gets its own infer_scope_types invocation later.
            // For now, lambda scope inference is a placeholder.
        }
        ScopeKind::Class => {
            // Class scope: no expressions to type-check.
            // Fields are resolved by resolve_class_fields.
            // Methods are child Function scopes with their own infer_scope_types.
        }
        _ => {
            // Project, Package, Namespace, File, Enum, TypeAlias, Block, Item:
            // typically no expressions to infer at these scope levels.
        }
    }

    let (expressions, bindings, diagnostics) = builder.finish();

    let extra = if diagnostics.is_empty() {
        None
    } else {
        Some(Box::new(ScopeInferenceExtra { diagnostics }))
    };

    ScopeInference {
        expressions,
        bindings,
        extra,
    }
}

// ── Type Alias Collection ────────────────────────────────────────────────────

/// Build a map of alias name → resolved Ty from all type aliases in the package.
fn collect_type_aliases<'db>(
    db: &'db dyn crate::Db,
    pkg_items: &PackageItems<'db>,
) -> HashMap<Name, Ty> {
    let mut aliases = HashMap::new();
    for ns in pkg_items.namespaces.values() {
        for (name, def) in &ns.types {
            if let Definition::TypeAlias(loc) = def {
                let resolved = resolve_type_alias(db, *loc);
                aliases.insert(name.clone(), resolved.ty.clone());
            }
        }
    }
    aliases
}

// ── Per-Item Queries ────────────────────────────────────────────────────────

/// Resolved class fields — `TypeExpr` resolved to `Ty` for each field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedClassFields {
    pub fields: Vec<(Name, Ty)>,
}

// Safety: `ResolvedClassFields` contains `Ty` (which has `Name`, a Salsa
// interned type). Manual `Update` impl uses `PartialEq` for early-cutoff.
#[allow(unsafe_code)]
unsafe impl salsa::Update for ResolvedClassFields {
    unsafe fn maybe_update(old_pointer: *mut Self, new_value: Self) -> bool {
        #[allow(unsafe_code)]
        let old = unsafe { &*old_pointer };
        if old == &new_value {
            false
        } else {
            #[allow(unsafe_code)]
            unsafe {
                std::ptr::drop_in_place(old_pointer);
                std::ptr::write(old_pointer, new_value);
            }
            true
        }
    }
}

/// Resolved type alias body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedTypeAlias {
    pub ty: Ty,
}

#[allow(unsafe_code)]
unsafe impl salsa::Update for ResolvedTypeAlias {
    unsafe fn maybe_update(old_pointer: *mut Self, new_value: Self) -> bool {
        #[allow(unsafe_code)]
        let old = unsafe { &*old_pointer };
        if old == &new_value {
            false
        } else {
            #[allow(unsafe_code)]
            unsafe {
                std::ptr::drop_in_place(old_pointer);
                std::ptr::write(old_pointer, new_value);
            }
            true
        }
    }
}

/// Salsa query: resolved class fields for a specific class.
///
/// Cached per `ClassLoc` — re-runs only when the class definition changes.
#[salsa::tracked(returns(ref))]
pub fn resolve_class_fields<'db>(
    db: &'db dyn crate::Db,
    class_loc: ClassLoc<'db>,
) -> Arc<ResolvedClassFields> {
    let file = class_loc.file(db);
    let item_tree = baml_compiler2_hir::file_item_tree(db, file);
    let pkg_info = baml_compiler2_hir::file_package::file_package(db, file);
    let pkg_id = PackageId::new(db, pkg_info.package.clone());
    let pkg_items = package_items(db, pkg_id);

    let class_data = &item_tree[class_loc.id(db)];
    let fields = class_data
        .fields
        .iter()
        .map(|f| {
            let ty = f
                .type_expr
                .as_ref()
                .map(|te| crate::lower_type_expr::lower_type_expr(&te.expr, pkg_items))
                .unwrap_or(Ty::Unknown);
            (f.name.clone(), ty)
        })
        .collect();

    Arc::new(ResolvedClassFields { fields })
}

/// Salsa query: resolved type alias body.
///
/// Cached per `TypeAliasLoc` — re-runs only when the alias definition changes.
#[salsa::tracked(returns(ref))]
pub fn resolve_type_alias<'db>(
    db: &'db dyn crate::Db,
    alias_loc: TypeAliasLoc<'db>,
) -> Arc<ResolvedTypeAlias> {
    let file = alias_loc.file(db);
    let item_tree = baml_compiler2_hir::file_item_tree(db, file);
    let pkg_info = baml_compiler2_hir::file_package::file_package(db, file);
    let pkg_id = PackageId::new(db, pkg_info.package.clone());
    let pkg_items = package_items(db, pkg_id);

    let alias_data = &item_tree[alias_loc.id(db)];
    let ty = alias_data
        .type_expr
        .as_ref()
        .map(|te| crate::lower_type_expr::lower_type_expr(&te.expr, pkg_items))
        .unwrap_or(Ty::Unknown);

    Arc::new(ResolvedTypeAlias { ty })
}

// ── Rendered Diagnostics ─────────────────────────────────────────────────────

/// Render all diagnostics for a single scope, resolving arena IDs to source
/// `TextRange` via the function body's `AstSourceMap`.
///
/// This is NOT a Salsa query — it's a convenience function that combines the
/// cached `infer_scope_types` result with the `function_body_source_map` to
/// produce display-ready diagnostics.
pub fn render_scope_diagnostics<'db>(
    db: &'db dyn crate::Db,
    scope_id: ScopeId<'db>,
) -> Vec<crate::infer_context::RenderedTirDiagnostic> {
    let inference = infer_scope_types(db, scope_id);
    let diags = inference.diagnostics();
    if diags.is_empty() {
        return Vec::new();
    }

    // Find the source map by matching scope range against item_tree functions.
    let file = scope_id.file(db);
    let file_scope = scope_id.file_scope_id(db);
    let index = baml_compiler2_hir::file_semantic_index(db, file);
    let scope = &index.scopes[file_scope.index() as usize];
    let item_tree = baml_compiler2_hir::file_item_tree(db, file);

    let source_map = item_tree
        .functions
        .iter()
        .find(|(_, f)| f.span == scope.range)
        .and_then(|(local_id, _)| {
            let func_loc = baml_compiler2_hir::loc::FunctionLoc::new(db, file, *local_id);
            baml_compiler2_hir::body::function_body_source_map(db, func_loc)
        });

    diags
        .diagnostics
        .iter()
        .map(|d| d.render(source_map.as_ref()))
        .collect()
}

// ── File-Level Diagnostic Collection ────────────────────────────────────────

/// Collect all type-check diagnostics for a file by iterating all scopes.
///
/// Modeled after Ty's `check_types` (`types.rs:127-168`).
pub fn collect_file_diagnostics<'db>(
    db: &'db dyn crate::Db,
    file: baml_base::SourceFile,
) -> TypeCheckDiagnostics<'db> {
    let index = baml_compiler2_hir::file_semantic_index(db, file);
    let mut all_diagnostics = TypeCheckDiagnostics::default();

    for scope_id in &index.scope_ids {
        let scope_result = infer_scope_types(db, *scope_id);
        all_diagnostics.extend(scope_result.diagnostics());
    }

    all_diagnostics
}
