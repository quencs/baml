//! Per-scope type inference builder.
//!
//! `TypeInferenceBuilder` is the mutable accumulator during a single scope's
//! type inference run. It walks the `ExprBody` arena for expressions belonging
//! to the scope being analyzed, recording inferred types in `expressions`.
//!
//! Implements bidirectional type checking:
//! - `infer_expr` (synthesis): compute type bottom-up
//! - `check_expr` (checking): verify against expected type top-down
//! - `check_stmt`: type-check a statement
//!
//! Key invariant: when encountering a lambda expression body, the builder does
//! NOT recurse into it — lambda bodies are separate scopes with their own
//! `infer_scope_types` Salsa query.

use std::collections::HashMap;

use baml_base::Name;
use baml_compiler2_ast::{Expr, ExprBody, ExprId, PatId, Stmt, StmtId};
use baml_compiler2_hir::{contributions::Definition, package::PackageItems, scope::ScopeId};
use rustc_hash::FxHashMap;

use crate::{
    infer_context::{InferContext, RelatedLocation, TirTypeError, TypeCheckDiagnostics},
    ty::{Freshness, PrimitiveType, Ty},
};

/// Format an f64 as a string suitable for a float literal.
/// Returns `None` for non-finite values (inf, NaN).
fn format_float(v: f64) -> Option<String> {
    if !v.is_finite() {
        return None;
    }
    let s = format!("{v}");
    // Ensure the string always has a decimal point so it reads as float.
    if s.contains('.') {
        Some(s)
    } else {
        Some(format!("{v}.0"))
    }
}

/// Per-scope inference builder.
///
/// Created at the start of `infer_scope_types`, discarded when done.
/// Modeled after Ty's `TypeInferenceBuilder`.
pub struct TypeInferenceBuilder<'db> {
    /// Diagnostic sink.
    context: InferContext<'db>,
    /// Expression types being built up.
    expressions: FxHashMap<ExprId, Ty>,
    /// Binding types: the type a variable is bound to (may differ from the
    /// initializer expression type due to widening or annotation).
    bindings: FxHashMap<PatId, Ty>,
    /// Package items for cross-file name resolution.
    package_items: &'db PackageItems<'db>,
    /// The scope being analyzed (kept for future use).
    #[allow(dead_code)]
    scope: ScopeId<'db>,
    /// Declared return type for the function (used to check return statements).
    declared_return_ty: Option<Ty>,
    /// Local variable bindings: name → inferred type.
    locals: FxHashMap<Name, Ty>,
    /// Resolved type alias map: alias qualified name → expanded Ty.
    /// Used by the normalizer for structural subtype checking.
    aliases: HashMap<crate::ty::QualifiedTypeName, Ty>,
}

impl<'db> TypeInferenceBuilder<'db> {
    pub fn new(
        context: InferContext<'db>,
        package_items: &'db PackageItems<'db>,
        scope: ScopeId<'db>,
        aliases: HashMap<crate::ty::QualifiedTypeName, Ty>,
    ) -> Self {
        Self {
            context,
            expressions: FxHashMap::default(),
            bindings: FxHashMap::default(),
            package_items,
            scope,
            declared_return_ty: None,
            locals: FxHashMap::default(),
            aliases,
        }
    }

    /// Finish building and return the accumulated results.
    pub fn finish(
        self,
    ) -> (
        FxHashMap<ExprId, Ty>,
        FxHashMap<PatId, Ty>,
        TypeCheckDiagnostics<'db>,
    ) {
        let diagnostics = self.context.finish();
        (self.expressions, self.bindings, diagnostics)
    }

    /// Set the declared return type (for return statement checking).
    pub fn set_return_type(&mut self, ty: Ty) {
        self.declared_return_ty = Some(ty);
    }

    /// Report a type error at a raw source span (for type annotations).
    pub fn report_at_span(&self, error: TirTypeError, span: text_size::TextRange) {
        self.context.report_at_span(error, span);
    }

    /// Add a local variable binding (e.g. function parameters).
    pub fn add_local(&mut self, name: Name, ty: Ty) {
        self.locals.insert(name, ty);
    }

    /// Record the type of an expression.
    pub fn record_expr_type(&mut self, expr_id: ExprId, ty: Ty) {
        self.expressions.insert(expr_id, ty);
    }

    // ── Bidirectional Type Checking ─────────────────────────────────────────

    /// Synthesis mode: compute the type of an expression bottom-up.
    pub fn infer_expr(&mut self, expr_id: ExprId, body: &ExprBody) -> Ty {
        let expr = &body.exprs[expr_id];
        let ty = match expr {
            Expr::Literal(lit) => self.infer_literal(lit),
            Expr::Null => Ty::Primitive(PrimitiveType::Null),
            Expr::Path(segments) => self.infer_path(segments.as_slice(), body, expr_id),
            Expr::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.infer_expr(*condition, body);
                let then_ty = self.infer_expr(*then_branch, body);
                if let Some(else_id) = else_branch {
                    let else_ty = self.infer_expr(*else_id, body);
                    self.join_types(&then_ty, &else_ty)
                } else {
                    Ty::Void
                }
            }
            Expr::Call { callee, args } => {
                // Check for container mutation method (e.g. x.push(val))
                // before general callee inference, since resolve_member
                // doesn't know about built-in List/Map methods.
                if let Some(result_ty) =
                    self.try_container_method_call(expr_id, *callee, args, body)
                {
                    result_ty
                } else {
                    let callee_ty = self.infer_expr(*callee, body);
                    for arg in args {
                        self.infer_expr(*arg, body);
                    }
                    match &callee_ty {
                        Ty::Function { params, ret } => {
                            if params.len() != args.len() {
                                self.context.report_simple(
                                    TirTypeError::ArgumentCountMismatch {
                                        expected: params.len(),
                                        got: args.len(),
                                    },
                                    expr_id,
                                );
                            }
                            *ret.clone()
                        }
                        Ty::Unknown | Ty::Error => Ty::Unknown,
                        _ => {
                            self.context.report_simple(
                                TirTypeError::NotCallable {
                                    ty: callee_ty.clone(),
                                },
                                expr_id,
                            );
                            Ty::Unknown
                        }
                    }
                }
            }
            Expr::Block { stmts, tail_expr } => {
                let mut diverged_at: Option<(usize, StmtId)> = None;
                for (i, stmt_id) in stmts.iter().enumerate() {
                    if self.check_stmt(*stmt_id, body) {
                        diverged_at = Some((i, *stmt_id));
                        break;
                    }
                }
                if let Some((div_idx, div_stmt)) = diverged_at {
                    let remaining =
                        stmts.len() - div_idx - 1 + if tail_expr.is_some() { 1 } else { 0 };
                    if remaining > 0 {
                        self.context.report_at_stmt(
                            crate::infer_context::TirTypeError::DeadCode {
                                after: div_stmt,
                                unreachable_count: remaining,
                            },
                            div_stmt,
                        );
                    }
                    Ty::Never
                } else {
                    tail_expr
                        .map(|e| self.infer_expr(e, body))
                        .unwrap_or(Ty::Void)
                }
            }
            Expr::FieldAccess { base, field } => {
                let base_ty = self.infer_expr(*base, body);
                self.resolve_member(&base_ty, field, expr_id)
            }
            Expr::Array { elements } => {
                let elem_types: Vec<Ty> =
                    elements.iter().map(|e| self.infer_expr(*e, body)).collect();
                let elem_ty = self.join_all(&elem_types);
                Ty::List(Box::new(elem_ty))
            }
            Expr::Map { entries } => {
                let mut key_types = Vec::new();
                let mut val_types = Vec::new();
                for (k, v) in entries {
                    key_types.push(self.infer_expr(*k, body));
                    val_types.push(self.infer_expr(*v, body));
                }
                let key_ty = self.join_all(&key_types);
                let val_ty = self.join_all(&val_types);
                Ty::Map(Box::new(key_ty), Box::new(val_ty))
            }
            Expr::Binary { op, lhs, rhs } => {
                let lhs_ty = self.infer_expr(*lhs, body);
                let rhs_ty = self.infer_expr(*rhs, body);
                self.infer_binary_op(op, &lhs_ty, &rhs_ty, expr_id)
            }
            Expr::Unary { op, expr } => {
                let operand_ty = self.infer_expr(*expr, body);
                self.infer_unary_op(op, &operand_ty, expr_id)
            }
            Expr::Match {
                scrutinee, arms, ..
            } => {
                self.infer_expr(*scrutinee, body);
                let arm_types: Vec<Ty> = arms
                    .iter()
                    .map(|arm_id| {
                        let arm = &body.match_arms[*arm_id];
                        self.infer_expr(arm.body, body)
                    })
                    .collect();
                self.join_all(&arm_types)
            }
            Expr::Object {
                type_name, fields, ..
            } => {
                for (_, expr_id) in fields {
                    self.infer_expr(*expr_id, body);
                }
                type_name
                    .as_ref()
                    .and_then(|n| {
                        self.package_items.lookup_type(&[n.clone()]).map(|def| {
                            Ty::Class(crate::lower_type_expr::qualify_def(
                                self.context.db(),
                                def,
                                n,
                            ))
                        })
                    })
                    .unwrap_or(Ty::Unknown)
            }
            Expr::Index { base, index } => {
                let base_ty = self.infer_expr(*base, body);
                self.infer_expr(*index, body);
                match base_ty {
                    Ty::List(elem_ty) | Ty::EvolvingList(elem_ty) => *elem_ty,
                    Ty::Map(_, val_ty) | Ty::EvolvingMap(_, val_ty) => *val_ty,
                    Ty::Unknown | Ty::Error => Ty::Unknown,
                    _ => {
                        self.context.report_simple(
                            TirTypeError::NotIndexable {
                                ty: base_ty.clone(),
                            },
                            expr_id,
                        );
                        Ty::Unknown
                    }
                }
            }
            Expr::Missing => Ty::Unknown,
        };
        self.record_expr_type(expr_id, ty.clone());
        ty
    }

    /// Checking mode: verify an expression against an expected type.
    pub fn check_expr(&mut self, expr_id: ExprId, body: &ExprBody, expected: &Ty) -> Ty {
        let expr = &body.exprs[expr_id];
        match expr {
            // Block: check the tail expression against expected type
            Expr::Block { stmts, tail_expr } => {
                let mut diverged_at: Option<(usize, StmtId)> = None;
                for (i, stmt_id) in stmts.iter().enumerate() {
                    if self.check_stmt(*stmt_id, body) {
                        diverged_at = Some((i, *stmt_id));
                        break;
                    }
                }
                let ty = if let Some((div_idx, div_stmt)) = diverged_at {
                    let remaining =
                        stmts.len() - div_idx - 1 + if tail_expr.is_some() { 1 } else { 0 };
                    if remaining > 0 {
                        self.context.report_at_stmt(
                            crate::infer_context::TirTypeError::DeadCode {
                                after: div_stmt,
                                unreachable_count: remaining,
                            },
                            div_stmt,
                        );
                    }
                    Ty::Never
                } else if let Some(tail) = tail_expr {
                    self.check_expr(*tail, body, expected)
                } else if !matches!(expected, Ty::Unknown | Ty::Void) {
                    // No tail expression, no divergence — block falls through
                    // without producing a value. Report missing return.
                    self.context.report_simple(
                        TirTypeError::MissingReturn {
                            expected: expected.clone(),
                        },
                        expr_id,
                    );
                    expected.clone()
                } else {
                    Ty::Void
                };
                self.record_expr_type(expr_id, ty.clone());
                ty
            }
            // If: check both branches against expected type
            Expr::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.infer_expr(*condition, body);
                let then_ty = self.check_expr(*then_branch, body, expected);
                let ty = if let Some(else_id) = else_branch {
                    let else_ty = self.check_expr(*else_id, body, expected);
                    self.join_types(&then_ty, &else_ty)
                } else {
                    if !matches!(expected, Ty::Void | Ty::Unknown) {
                        self.context
                            .report_simple(TirTypeError::VoidUsedAsValue, expr_id);
                    }
                    Ty::Void
                };
                self.record_expr_type(expr_id, ty.clone());
                ty
            }
            Expr::Array { elements } => {
                let elem_ty = match expected {
                    Ty::List(e) | Ty::EvolvingList(e) => Some(e),
                    _ => None,
                };
                if let Some(elem_ty) = elem_ty {
                    for e in elements {
                        self.check_expr(*e, body, elem_ty);
                    }
                    let ty = expected.clone();
                    self.record_expr_type(expr_id, ty.clone());
                    ty
                } else {
                    self.infer_expr(expr_id, body)
                }
            }
            // Object: if expected is Class(name), check fields
            Expr::Object { fields, .. } => {
                if let Ty::Class(_) = expected {
                    for (_field_name, field_expr) in fields {
                        self.infer_expr(*field_expr, body);
                    }
                    let ty = expected.clone();
                    self.record_expr_type(expr_id, ty.clone());
                    ty
                } else {
                    self.infer_expr(expr_id, body)
                }
            }
            Expr::Map { entries } => {
                let kv = match expected {
                    Ty::Map(k, v) | Ty::EvolvingMap(k, v) => Some((k, v)),
                    _ => None,
                };
                if let Some((key_ty, val_ty)) = kv {
                    for (k, v) in entries {
                        self.check_expr(*k, body, key_ty);
                        self.check_expr(*v, body, val_ty);
                    }
                    let ty = expected.clone();
                    self.record_expr_type(expr_id, ty.clone());
                    ty
                } else {
                    self.infer_expr(expr_id, body)
                }
            }
            // Literal checked against a literal type: compare values directly.
            // On match, strip freshness → Regular. On mismatch, fall through
            // to the default infer-then-check path which will report the error.
            Expr::Literal(lit) if matches!(expected, Ty::Literal(..)) => {
                use crate::ty::Freshness;
                let expected_lit = match expected {
                    Ty::Literal(v, _) => v,
                    _ => unreachable!(),
                };
                if lit == expected_lit {
                    let ty = Ty::Literal(expected_lit.clone(), Freshness::Regular);
                    self.record_expr_type(expr_id, ty.clone());
                    ty
                } else {
                    // Value doesn't match — infer (produces fresh literal) and
                    // let the subtype check report the error.
                    let inferred = self.infer_expr(expr_id, body);
                    if !self.is_subtype(&inferred, expected) {
                        self.context.report(
                            TirTypeError::TypeMismatch {
                                expected: expected.clone(),
                                got: inferred.clone(),
                            },
                            expr_id,
                            Vec::new(),
                        );
                    }
                    inferred
                }
            }
            // All other expressions: infer then subtype-check
            _ => {
                let inferred = self.infer_expr(expr_id, body);
                if matches!(inferred, Ty::Void) && !matches!(expected, Ty::Void | Ty::Unknown) {
                    self.context
                        .report_simple(TirTypeError::VoidUsedAsValue, expr_id);
                } else if !self.is_subtype(&inferred, expected) {
                    self.context.report(
                        TirTypeError::TypeMismatch {
                            expected: expected.clone(),
                            got: inferred.clone(),
                        },
                        expr_id,
                        Vec::new(),
                    );
                }
                inferred
            }
        }
    }

    /// Type-check a statement. Returns `true` if the statement diverges
    /// (i.e. control flow never reaches the next statement).
    pub fn check_stmt(&mut self, stmt_id: StmtId, body: &ExprBody) -> bool {
        let stmt = &body.stmts[stmt_id];
        match stmt {
            Stmt::Expr(expr_id) => {
                let ty = self.infer_expr(*expr_id, body);
                matches!(ty, Ty::Never)
            }
            Stmt::Let {
                pattern,
                initializer,
                type_annotation,
                ..
            } => {
                let init_ty = if let Some(init) = initializer {
                    if let Some(ann_idx) = type_annotation {
                        let mut diags = Vec::new();
                        let ann_ty = crate::lower_type_expr::lower_type_expr(
                            self.context.db(),
                            &body.type_annotations[*ann_idx],
                            self.package_items,
                            &mut diags,
                        );
                        for diag in diags {
                            self.context.report_simple(diag, *init);
                        }
                        let ty = self.check_expr(*init, body, &ann_ty);
                        if matches!(ty, Ty::Void) {
                            self.context
                                .report_simple(TirTypeError::VoidUsedAsValue, *init);
                        }
                        Some(ty)
                    } else {
                        let ty = self.infer_expr(*init, body);
                        if matches!(ty, Ty::Void) {
                            self.context
                                .report_simple(TirTypeError::VoidUsedAsValue, *init);
                        }
                        Some(ty.widen_fresh().make_evolving())
                    }
                } else {
                    None
                };
                // Track local variable binding for name resolution
                if let Some(ty) = init_ty {
                    self.bindings.insert(*pattern, ty.clone());
                    let pat = &body.patterns[*pattern];
                    match pat {
                        baml_compiler2_ast::Pattern::Binding(name) => {
                            self.locals.insert(name.clone(), ty);
                        }
                        baml_compiler2_ast::Pattern::TypedBinding { name, .. } => {
                            self.locals.insert(name.clone(), ty);
                        }
                        _ => {}
                    }
                }
                false
            }
            Stmt::Return(expr) => {
                if let Some(e) = expr {
                    if let Some(ret_ty) = &self.declared_return_ty {
                        let ret_ty = ret_ty.clone();
                        self.check_expr(*e, body, &ret_ty);
                    } else {
                        self.infer_expr(*e, body);
                    }
                }
                true // return always diverges
            }
            Stmt::While {
                condition,
                body: while_body,
                ..
            } => {
                self.infer_expr(*condition, body);
                self.infer_expr(*while_body, body);
                false
            }
            Stmt::Assign { target, value } => {
                // Check for container index mutation: x[i] = val
                if self.try_index_assign_mutation(*target, *value, body) {
                    return false;
                }
                self.infer_expr(*target, body);
                self.infer_expr(*value, body);
                false
            }
            Stmt::AssignOp { target, op, value } => {
                let target_ty = self.infer_expr(*target, body);
                let value_ty = self.infer_expr(*value, body);
                let binary_op = Self::assign_op_to_binary_op(op);
                let result_ty = self.infer_binary_op(&binary_op, &target_ty, &value_ty, *target);
                // Re-record the value expression with the result type so the
                // display shows the operation result, not the raw RHS literal.
                self.record_expr_type(*value, result_ty);
                false
            }
            Stmt::Assert { condition } => {
                self.infer_expr(*condition, body);
                false
            }
            Stmt::Break | Stmt::Continue => true, // break/continue diverge
            Stmt::Missing | Stmt::HeaderComment { .. } => false,
        }
    }

    // ── Helper methods ────────────────────────────────────────────────────────

    /// Extract the short name from a qualified type name for package item lookups.
    fn unqualify(qn: &crate::ty::QualifiedTypeName) -> Name {
        qn.name.clone()
    }

    fn infer_literal(&self, lit: &baml_base::Literal) -> Ty {
        Ty::Literal(lit.clone(), Freshness::Fresh)
    }

    fn infer_path(&mut self, segments: &[Name], _body: &ExprBody, expr_id: ExprId) -> Ty {
        // After AST lowering, Path is always single-segment (a bare identifier).
        // Multi-segment paths like Color.Red or x.field are desugared to FieldAccess chains.
        debug_assert!(
            segments.len() == 1,
            "multi-segment Path should have been desugared to FieldAccess: {:?}",
            segments
        );
        if segments.len() == 1 {
            let name = &segments[0];
            let ty = self.infer_single_name(name);
            if matches!(ty, Ty::Unknown)
                && !self.locals.contains_key(name)
                && self.package_items.lookup_value(&[name.clone()]).is_none()
                && self.package_items.lookup_type(&[name.clone()]).is_none()
            {
                self.context
                    .report_simple(TirTypeError::UnresolvedName { name: name.clone() }, expr_id);
            }
            ty
        } else {
            Ty::Unknown
        }
    }

    /// Resolve a single name to its type.
    ///
    /// Checks local variables first, then value namespace (functions), then
    /// type namespace (classes, enums).
    fn infer_single_name(&self, name: &Name) -> Ty {
        if let Some(ty) = self.locals.get(name) {
            return match ty {
                Ty::EvolvingList(inner) => Ty::List(inner.clone()),
                Ty::EvolvingMap(k, v) => Ty::Map(k.clone(), v.clone()),
                other => other.clone(),
            };
        }
        // Check value namespace (functions, template strings)
        if let Some(def) = self.package_items.lookup_value(&[name.clone()]) {
            match def {
                Definition::Function(func_loc) => {
                    // Get function signature to build the function type
                    let db = self.context.db();
                    let sig = baml_compiler2_hir::signature::function_signature(db, func_loc);
                    let mut diags = Vec::new();
                    let ty = Ty::Function {
                        params: sig
                            .params
                            .iter()
                            .map(|(n, te)| {
                                (
                                    Some(n.clone()),
                                    crate::lower_type_expr::lower_type_expr(
                                        db,
                                        te,
                                        self.package_items,
                                        &mut diags,
                                    ),
                                )
                            })
                            .collect(),
                        ret: Box::new(
                            sig.return_type
                                .as_ref()
                                .map(|te| {
                                    crate::lower_type_expr::lower_type_expr(
                                        db,
                                        te,
                                        self.package_items,
                                        &mut diags,
                                    )
                                })
                                .unwrap_or(Ty::Unknown),
                        ),
                    };
                    // Note: diags from referenced function signatures are not
                    // reported here — they'll be reported at the definition site.
                    ty
                }
                _ => Ty::Unknown,
            }
        } else if let Some(def) = self.package_items.lookup_type(&[name.clone()]) {
            let db = self.context.db();
            match def {
                Definition::Class(_) => {
                    Ty::Class(crate::lower_type_expr::qualify_def(db, def, name))
                }
                Definition::Enum(_) => Ty::Enum(crate::lower_type_expr::qualify_def(db, def, name)),
                Definition::TypeAlias(_) => {
                    Ty::TypeAlias(crate::lower_type_expr::qualify_def(db, def, name))
                }
                _ => Ty::Unknown,
            }
        } else {
            Ty::Unknown
        }
    }

    /// Resolve a member access on a known base type.
    ///
    /// For class types, checks data fields. For enum types, validates variants.
    /// Emits `UnresolvedMember` diagnostics when the base type is known but
    /// the member doesn't exist.
    pub fn resolve_member(&mut self, base_ty: &Ty, member: &Name, at: ExprId) -> Ty {
        match base_ty {
            Ty::Class(class_name) => {
                // Check class fields
                let class_fields = self.lookup_class_fields(class_name);
                if let Some(field_ty) = class_fields.get(member) {
                    return field_ty.clone();
                }

                // Check class methods via the item tree (methods are stored
                // directly on the Class entry, not in the package namespace).
                if let Some(ty) = self.lookup_class_method(class_name, member) {
                    return ty;
                }

                // Known class but member not found — error
                let class_def = self
                    .package_items
                    .lookup_type(&[Self::unqualify(class_name)]);
                let related = class_def
                    .map(|def| vec![(RelatedLocation::Item(def), "class defined here")])
                    .unwrap_or_default();
                self.context.report(
                    TirTypeError::UnresolvedMember {
                        base_type: base_ty.clone(),
                        member: member.clone(),
                    },
                    at,
                    related,
                );
                Ty::Unknown
            }
            Ty::Enum(enum_name) => {
                // Validate that the variant exists
                let variants = self.lookup_enum_variants(enum_name);
                if variants.contains(member) {
                    return Ty::EnumVariant(enum_name.clone(), member.clone());
                }

                // Known enum but variant not found — error
                let enum_def = self
                    .package_items
                    .lookup_type(&[Self::unqualify(enum_name)]);
                let related = enum_def
                    .map(|def| vec![(RelatedLocation::Item(def), "enum defined here")])
                    .unwrap_or_default();
                self.context.report(
                    TirTypeError::UnresolvedMember {
                        base_type: base_ty.clone(),
                        member: member.clone(),
                    },
                    at,
                    related,
                );
                Ty::Unknown
            }
            Ty::Unknown => {
                // Base type unknown — can't resolve member, but don't emit error
                // (the base type error was already reported upstream)
                Ty::Unknown
            }
            _ => {
                // Other types (primitives, lists, maps, etc.) — no members
                self.context.report_simple(
                    TirTypeError::UnresolvedMember {
                        base_type: base_ty.clone(),
                        member: member.clone(),
                    },
                    at,
                );
                Ty::Unknown
            }
        }
    }

    /// Look up class fields from the package items (via item tree).
    ///
    /// Returns a map of field name → resolved field type.
    fn lookup_class_fields(
        &self,
        class_name: &crate::ty::QualifiedTypeName,
    ) -> FxHashMap<Name, Ty> {
        let mut result = FxHashMap::default();
        let short = Self::unqualify(class_name);
        if let Some(def) = self.package_items.lookup_type(&[short]) {
            if let Definition::Class(class_loc) = def {
                let file = class_loc.file(self.context.db());
                let item_tree = baml_compiler2_hir::file_item_tree(self.context.db(), file);
                let class_data = &item_tree[class_loc.id(self.context.db())];
                for field in &class_data.fields {
                    let mut diags = Vec::new();
                    let field_ty = field
                        .type_expr
                        .as_ref()
                        .map(|te| {
                            let ty = crate::lower_type_expr::lower_type_expr(
                                self.context.db(),
                                &te.expr,
                                self.package_items,
                                &mut diags,
                            );
                            for diag in diags.drain(..) {
                                self.context.report_at_span(diag, te.span);
                            }
                            ty
                        })
                        .unwrap_or(Ty::Unknown);
                    result.insert(field.name.clone(), field_ty);
                }
            }
        }
        result
    }

    /// Look up a class method by name from the item tree.
    ///
    /// Methods are stored on the `Class` entry directly (not in the package
    /// namespace), so we resolve the class, iterate its method IDs, and match
    /// by name.
    fn lookup_class_method(
        &self,
        class_name: &crate::ty::QualifiedTypeName,
        method_name: &Name,
    ) -> Option<Ty> {
        let short = Self::unqualify(class_name);
        let def = self.package_items.lookup_type(&[short])?;
        let Definition::Class(class_loc) = def else {
            return None;
        };
        let db = self.context.db();
        let file = class_loc.file(db);
        let item_tree = baml_compiler2_hir::file_item_tree(db, file);
        let class_data = &item_tree[class_loc.id(db)];

        for &method_id in &class_data.methods {
            let method_data = &item_tree[method_id];
            if method_data.name == *method_name {
                let func_loc = baml_compiler2_hir::loc::FunctionLoc::new(db, file, method_id);
                let sig = baml_compiler2_hir::signature::function_signature(db, func_loc);
                let mut diags = Vec::new();
                let ty = Ty::Function {
                    params: sig
                        .params
                        .iter()
                        .map(|(n, te)| {
                            (
                                Some(n.clone()),
                                crate::lower_type_expr::lower_type_expr(
                                    db,
                                    te,
                                    self.package_items,
                                    &mut diags,
                                ),
                            )
                        })
                        .collect(),
                    ret: Box::new(
                        sig.return_type
                            .as_ref()
                            .map(|te| {
                                crate::lower_type_expr::lower_type_expr(
                                    db,
                                    te,
                                    self.package_items,
                                    &mut diags,
                                )
                            })
                            .unwrap_or(Ty::Unknown),
                    ),
                };
                // Note: diags from method signatures are reported at definition site.
                return Some(ty);
            }
        }
        None
    }

    /// Look up enum variants from the package items (via item tree).
    fn lookup_enum_variants(&self, enum_name: &crate::ty::QualifiedTypeName) -> Vec<Name> {
        let short = Self::unqualify(enum_name);
        if let Some(def) = self.package_items.lookup_type(&[short]) {
            if let Definition::Enum(enum_loc) = def {
                let file = enum_loc.file(self.context.db());
                let item_tree = baml_compiler2_hir::file_item_tree(self.context.db(), file);
                let enum_data = &item_tree[enum_loc.id(self.context.db())];
                return enum_data.variants.iter().map(|v| v.name.clone()).collect();
            }
        }
        Vec::new()
    }

    // ── Evolving Container Mutations ─────────────────────────────────────────

    /// Extract the local variable name from an expression, if it's a simple
    /// single-segment path that refers to a known local.
    fn expr_local_name(&self, expr_id: ExprId, body: &ExprBody) -> Option<Name> {
        match &body.exprs[expr_id] {
            Expr::Path(segments) if segments.len() == 1 => {
                let name = &segments[0];
                if self.locals.contains_key(name) {
                    Some(name.clone())
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Try to handle a container mutation method call: x.push(val) / x.append(val).
    ///
    /// If the callee is `base.push(arg)` or `base.append(arg)` where base is a
    /// local with type `List(T)`:
    /// - If `T == Never`: first establishment → update local to `List(arg_ty)`
    /// - If `arg_ty <: T`: ok
    /// - Otherwise: type error
    ///
    /// Returns `Some(return_ty)` if handled, `None` to fall through to general case.
    fn try_container_method_call(
        &mut self,
        call_expr_id: ExprId,
        callee_id: ExprId,
        args: &[ExprId],
        body: &ExprBody,
    ) -> Option<Ty> {
        // After AST lowering, method calls are always FieldAccess:
        //   x.push(val) → Call { callee: FieldAccess { base: Path(["x"]), field: "push" }, ... }
        let (base_id, local_name, method_name) = match &body.exprs[callee_id] {
            Expr::FieldAccess { base, field } => {
                let name = self.expr_local_name(*base, body)?;
                (*base, name, field.clone())
            }
            _ => return None,
        };

        let local_ty = self.locals.get(&local_name)?.clone();

        match method_name.as_str() {
            "push" | "append" if args.len() == 1 => {
                let (elem_ty, is_evolving) = match &local_ty {
                    Ty::EvolvingList(elem) => (elem, true),
                    Ty::List(elem) => (elem, false),
                    _ => return None,
                };

                let arg_ty = self.infer_expr(args[0], body);
                let widened_arg = arg_ty.clone().widen_fresh();

                if matches!(**elem_ty, Ty::Never) {
                    let new_ty = if is_evolving {
                        Ty::EvolvingList(Box::new(widened_arg))
                    } else {
                        Ty::List(Box::new(widened_arg))
                    };
                    self.locals.insert(local_name, new_ty);
                } else if !self.is_subtype(&widened_arg, elem_ty) {
                    self.context.report(
                        TirTypeError::TypeMismatch {
                            expected: *elem_ty.clone(),
                            got: widened_arg,
                        },
                        args[0],
                        Vec::new(),
                    );
                }

                self.record_expr_type(base_id, local_ty);
                self.record_expr_type(callee_id, Ty::Unknown);
                let result = Ty::Primitive(PrimitiveType::Null);
                self.record_expr_type(call_expr_id, result.clone());
                Some(result)
            }
            _ => None,
        }
    }

    /// Try to handle index assignment mutation: x[i] = val on List or Map locals.
    ///
    /// For `List(Never)`: first element establishes element type.
    /// For `Map(Never, Never)`: first entry establishes key and value types.
    ///
    /// Returns `true` if handled, `false` to fall through to general case.
    fn try_index_assign_mutation(
        &mut self,
        target_id: ExprId,
        value_id: ExprId,
        body: &ExprBody,
    ) -> bool {
        let (base_id, index_id) = match &body.exprs[target_id] {
            Expr::Index { base, index } => (*base, *index),
            _ => return false,
        };

        let local_name = match self.expr_local_name(base_id, body) {
            Some(n) => n,
            None => return false,
        };
        let local_ty = match self.locals.get(&local_name) {
            Some(ty) => ty.clone(),
            None => return false,
        };

        match &local_ty {
            Ty::List(elem_ty) | Ty::EvolvingList(elem_ty) => {
                let is_evolving = matches!(local_ty, Ty::EvolvingList(_));
                let index_ty = self.infer_expr(index_id, body);
                let val_ty = self.infer_expr(value_id, body);
                let widened_val = val_ty.clone().widen_fresh();

                if matches!(**elem_ty, Ty::Never) {
                    let new_ty = if is_evolving {
                        Ty::EvolvingList(Box::new(widened_val.clone()))
                    } else {
                        Ty::List(Box::new(widened_val.clone()))
                    };
                    self.locals.insert(local_name, new_ty);
                } else if !self.is_subtype(&widened_val, elem_ty) {
                    self.context.report(
                        TirTypeError::TypeMismatch {
                            expected: *elem_ty.clone(),
                            got: widened_val.clone(),
                        },
                        value_id,
                        Vec::new(),
                    );
                }

                self.record_expr_type(base_id, local_ty);
                self.record_expr_type(index_id, index_ty);
                self.record_expr_type(target_id, widened_val);
                self.record_expr_type(value_id, val_ty);
                true
            }
            Ty::Map(key_ty, val_ty) | Ty::EvolvingMap(key_ty, val_ty) => {
                let is_evolving = matches!(local_ty, Ty::EvolvingMap(_, _));
                let index_ty = self.infer_expr(index_id, body);
                let value_ty = self.infer_expr(value_id, body);
                let widened_key = index_ty.clone().widen_fresh();
                let widened_val = value_ty.clone().widen_fresh();

                if matches!(**key_ty, Ty::Never) && matches!(**val_ty, Ty::Never) {
                    let new_ty = if is_evolving {
                        Ty::EvolvingMap(
                            Box::new(widened_key.clone()),
                            Box::new(widened_val.clone()),
                        )
                    } else {
                        Ty::Map(Box::new(widened_key.clone()), Box::new(widened_val.clone()))
                    };
                    self.locals.insert(local_name, new_ty);
                } else {
                    if !self.is_subtype(&widened_key, key_ty) {
                        self.context.report(
                            TirTypeError::TypeMismatch {
                                expected: *key_ty.clone(),
                                got: widened_key.clone(),
                            },
                            index_id,
                            Vec::new(),
                        );
                    }
                    if !self.is_subtype(&widened_val, val_ty) {
                        self.context.report(
                            TirTypeError::TypeMismatch {
                                expected: *val_ty.clone(),
                                got: widened_val.clone(),
                            },
                            value_id,
                            Vec::new(),
                        );
                    }
                }

                self.record_expr_type(base_id, local_ty);
                self.record_expr_type(index_id, index_ty);
                self.record_expr_type(target_id, widened_val);
                self.record_expr_type(value_id, value_ty);
                true
            }
            _ => false,
        }
    }

    fn join_types(&self, a: &Ty, b: &Ty) -> Ty {
        if matches!(a, Ty::Never) {
            return b.clone();
        }
        if matches!(b, Ty::Never) {
            return a.clone();
        }
        if matches!(a, Ty::Void) || matches!(b, Ty::Void) {
            return Ty::Void;
        }
        if a == b {
            return a.clone();
        }
        // Same literal value, different freshness → normalize to Regular
        if let (Ty::Literal(lit_a, _), Ty::Literal(lit_b, _)) = (a, b) {
            if lit_a == lit_b {
                return Ty::Literal(lit_a.clone(), crate::ty::Freshness::Regular);
            }
        }
        // Build a flat union, deduplicating members
        let mut members = Vec::new();
        let mut push = |ty: &Ty| {
            if let Ty::Union(inner) = ty {
                for m in inner {
                    if !members.contains(m) {
                        members.push(m.clone());
                    }
                }
            } else if !members.contains(ty) {
                members.push(ty.clone());
            }
        };
        push(a);
        push(b);
        if members.len() == 1 {
            members.into_iter().next().unwrap()
        } else {
            Ty::Union(members)
        }
    }

    fn join_all(&self, types: &[Ty]) -> Ty {
        if types.is_empty() {
            return Ty::Never;
        }
        types
            .iter()
            .skip(1)
            .fold(types[0].clone(), |acc, t| self.join_types(&acc, t))
    }

    /// Subtype check — delegates to the normalizer which resolves type aliases
    /// and performs equirecursive structural subtyping.
    fn is_subtype(&self, sub: &Ty, sup: &Ty) -> bool {
        crate::normalize::is_subtype_of(sub, sup, &self.aliases)
    }

    fn infer_binary_op(
        &mut self,
        op: &baml_compiler2_ast::BinaryOp,
        lhs: &Ty,
        rhs: &Ty,
        at: ExprId,
    ) -> Ty {
        // Try constant folding on two literals first.
        if let Some(folded) = Self::try_fold_binary(op, lhs, rhs) {
            return folded;
        }
        use baml_compiler2_ast::BinaryOp;
        match op {
            // Comparison / equality → bool
            BinaryOp::Eq
            | BinaryOp::Ne
            | BinaryOp::Lt
            | BinaryOp::Le
            | BinaryOp::Gt
            | BinaryOp::Ge
            | BinaryOp::Instanceof => Ty::Primitive(PrimitiveType::Bool),

            // Logical → bool
            BinaryOp::And | BinaryOp::Or => Ty::Primitive(PrimitiveType::Bool),

            // Arithmetic: result type depends on operands
            BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Mod => {
                let result = Self::infer_arithmetic(op, lhs, rhs);
                if matches!(result, Ty::Unknown)
                    && !matches!(lhs, Ty::Unknown | Ty::Error)
                    && !matches!(rhs, Ty::Unknown | Ty::Error)
                {
                    self.context.report_simple(
                        TirTypeError::InvalidBinaryOp {
                            op: op.clone(),
                            lhs: lhs.clone(),
                            rhs: rhs.clone(),
                        },
                        at,
                    );
                }
                result
            }

            // Bitwise → int
            BinaryOp::BitAnd
            | BinaryOp::BitOr
            | BinaryOp::BitXor
            | BinaryOp::Shl
            | BinaryOp::Shr => Ty::Primitive(PrimitiveType::Int),
        }
    }

    /// Determine the result type of an arithmetic operation (non-literal fallback).
    ///
    /// String concatenation is only valid for `Add`; other arithmetic ops on
    /// strings are invalid and return `Unknown` (triggering an error upstream).
    fn infer_arithmetic(op: &baml_compiler2_ast::BinaryOp, lhs: &Ty, rhs: &Ty) -> Ty {
        let base_ty = |ty: &Ty| -> Option<PrimitiveType> {
            match ty {
                Ty::Primitive(p) => Some(p.clone()),
                Ty::Literal(lit, _) => Some(PrimitiveType::from_literal(lit)),
                _ => None,
            }
        };
        match (base_ty(lhs), base_ty(rhs)) {
            (Some(PrimitiveType::Float), _) | (_, Some(PrimitiveType::Float)) => {
                Ty::Primitive(PrimitiveType::Float)
            }
            (Some(PrimitiveType::Int), Some(PrimitiveType::Int)) => {
                Ty::Primitive(PrimitiveType::Int)
            }
            (Some(PrimitiveType::String), _) | (_, Some(PrimitiveType::String)) => {
                // String concatenation only for Add
                if matches!(op, baml_compiler2_ast::BinaryOp::Add) {
                    Ty::Primitive(PrimitiveType::String)
                } else {
                    Ty::Unknown
                }
            }
            _ => Ty::Unknown,
        }
    }

    fn infer_unary_op(&mut self, op: &baml_compiler2_ast::UnaryOp, operand: &Ty, at: ExprId) -> Ty {
        // Try constant folding on a literal first.
        if let Some(folded) = Self::try_fold_unary(op, operand) {
            return folded;
        }
        match op {
            baml_compiler2_ast::UnaryOp::Not => Ty::Primitive(PrimitiveType::Bool),
            baml_compiler2_ast::UnaryOp::Neg => match operand {
                Ty::Primitive(PrimitiveType::Int) => Ty::Primitive(PrimitiveType::Int),
                Ty::Primitive(PrimitiveType::Float) => Ty::Primitive(PrimitiveType::Float),
                Ty::Unknown | Ty::Error => Ty::Unknown,
                _ => {
                    self.context.report_simple(
                        TirTypeError::InvalidUnaryOp {
                            op: op.clone(),
                            operand: operand.clone(),
                        },
                        at,
                    );
                    Ty::Unknown
                }
            },
        }
    }

    // ── Constant Folding ─────────────────────────────────────────────────────

    fn merge_freshness(a: crate::ty::Freshness, b: crate::ty::Freshness) -> crate::ty::Freshness {
        use crate::ty::Freshness;
        match (a, b) {
            (Freshness::Regular, Freshness::Regular) => Freshness::Regular,
            _ => Freshness::Fresh,
        }
    }

    /// Try to fold a unary operation on a literal into a new literal.
    fn try_fold_unary(op: &baml_compiler2_ast::UnaryOp, operand: &Ty) -> Option<Ty> {
        use crate::ty::LiteralValue;
        let (lit, f) = match operand {
            Ty::Literal(lit, f) => (lit, *f),
            _ => return None,
        };
        match op {
            baml_compiler2_ast::UnaryOp::Neg => match lit {
                LiteralValue::Int(n) => Some(Ty::Literal(LiteralValue::Int(n.checked_neg()?), f)),
                LiteralValue::Float(s) => {
                    let v: f64 = s.parse().ok()?;
                    Some(Ty::Literal(LiteralValue::Float(format_float(-v)?), f))
                }
                _ => None,
            },
            baml_compiler2_ast::UnaryOp::Not => match lit {
                LiteralValue::Bool(b) => Some(Ty::Literal(LiteralValue::Bool(!b), f)),
                _ => None,
            },
        }
    }

    /// Try to fold a binary operation on two literals into a new literal.
    fn try_fold_binary(op: &baml_compiler2_ast::BinaryOp, lhs: &Ty, rhs: &Ty) -> Option<Ty> {
        use baml_compiler2_ast::BinaryOp;

        use crate::ty::LiteralValue;

        let (lhs_lit, lhs_f) = match lhs {
            Ty::Literal(lit, f) => (lit, *f),
            _ => return None,
        };
        let (rhs_lit, rhs_f) = match rhs {
            Ty::Literal(lit, f) => (lit, *f),
            _ => return None,
        };
        let f = Self::merge_freshness(lhs_f, rhs_f);

        // Int × Int
        if let (LiteralValue::Int(a), LiteralValue::Int(b)) = (lhs_lit, rhs_lit) {
            let (a, b) = (*a, *b);
            return match op {
                BinaryOp::Add => Some(Ty::Literal(LiteralValue::Int(a.checked_add(b)?), f)),
                BinaryOp::Sub => Some(Ty::Literal(LiteralValue::Int(a.checked_sub(b)?), f)),
                BinaryOp::Mul => Some(Ty::Literal(LiteralValue::Int(a.checked_mul(b)?), f)),
                BinaryOp::Div => Some(Ty::Literal(LiteralValue::Int(a.checked_div(b)?), f)),
                BinaryOp::Mod => Some(Ty::Literal(LiteralValue::Int(a.checked_rem(b)?), f)),
                BinaryOp::BitAnd => Some(Ty::Literal(LiteralValue::Int(a & b), f)),
                BinaryOp::BitOr => Some(Ty::Literal(LiteralValue::Int(a | b), f)),
                BinaryOp::BitXor => Some(Ty::Literal(LiteralValue::Int(a ^ b), f)),
                BinaryOp::Shl => {
                    let shift = u32::try_from(b).ok()?;
                    Some(Ty::Literal(LiteralValue::Int(a.checked_shl(shift)?), f))
                }
                BinaryOp::Shr => {
                    let shift = u32::try_from(b).ok()?;
                    Some(Ty::Literal(LiteralValue::Int(a.checked_shr(shift)?), f))
                }
                BinaryOp::Eq => Some(Ty::Literal(LiteralValue::Bool(a == b), f)),
                BinaryOp::Ne => Some(Ty::Literal(LiteralValue::Bool(a != b), f)),
                BinaryOp::Lt => Some(Ty::Literal(LiteralValue::Bool(a < b), f)),
                BinaryOp::Le => Some(Ty::Literal(LiteralValue::Bool(a <= b), f)),
                BinaryOp::Gt => Some(Ty::Literal(LiteralValue::Bool(a > b), f)),
                BinaryOp::Ge => Some(Ty::Literal(LiteralValue::Bool(a >= b), f)),
                _ => None,
            };
        }

        // Float × Float
        if let (LiteralValue::Float(a_s), LiteralValue::Float(b_s)) = (lhs_lit, rhs_lit) {
            let a: f64 = a_s.parse().ok()?;
            let b: f64 = b_s.parse().ok()?;
            return match op {
                BinaryOp::Add => Some(Ty::Literal(LiteralValue::Float(format_float(a + b)?), f)),
                BinaryOp::Sub => Some(Ty::Literal(LiteralValue::Float(format_float(a - b)?), f)),
                BinaryOp::Mul => Some(Ty::Literal(LiteralValue::Float(format_float(a * b)?), f)),
                BinaryOp::Div if b != 0.0 => {
                    Some(Ty::Literal(LiteralValue::Float(format_float(a / b)?), f))
                }
                BinaryOp::Mod if b != 0.0 => {
                    Some(Ty::Literal(LiteralValue::Float(format_float(a % b)?), f))
                }
                BinaryOp::Eq => Some(Ty::Literal(LiteralValue::Bool(a == b), f)),
                BinaryOp::Ne => Some(Ty::Literal(LiteralValue::Bool(a != b), f)),
                BinaryOp::Lt => Some(Ty::Literal(LiteralValue::Bool(a < b), f)),
                BinaryOp::Le => Some(Ty::Literal(LiteralValue::Bool(a <= b), f)),
                BinaryOp::Gt => Some(Ty::Literal(LiteralValue::Bool(a > b), f)),
                BinaryOp::Ge => Some(Ty::Literal(LiteralValue::Bool(a >= b), f)),
                _ => None,
            };
        }

        // Bool × Bool
        if let (LiteralValue::Bool(a), LiteralValue::Bool(b)) = (lhs_lit, rhs_lit) {
            let (a, b) = (*a, *b);
            return match op {
                BinaryOp::And => Some(Ty::Literal(LiteralValue::Bool(a && b), f)),
                BinaryOp::Or => Some(Ty::Literal(LiteralValue::Bool(a || b), f)),
                BinaryOp::Eq => Some(Ty::Literal(LiteralValue::Bool(a == b), f)),
                BinaryOp::Ne => Some(Ty::Literal(LiteralValue::Bool(a != b), f)),
                _ => None,
            };
        }

        // String × String
        if let (LiteralValue::String(a), LiteralValue::String(b)) = (lhs_lit, rhs_lit) {
            return match op {
                BinaryOp::Add => Some(Ty::Literal(LiteralValue::String(format!("{a}{b}")), f)),
                BinaryOp::Eq => Some(Ty::Literal(LiteralValue::Bool(a == b), f)),
                BinaryOp::Ne => Some(Ty::Literal(LiteralValue::Bool(a != b), f)),
                BinaryOp::Lt => Some(Ty::Literal(LiteralValue::Bool(a < b), f)),
                BinaryOp::Le => Some(Ty::Literal(LiteralValue::Bool(a <= b), f)),
                BinaryOp::Gt => Some(Ty::Literal(LiteralValue::Bool(a > b), f)),
                BinaryOp::Ge => Some(Ty::Literal(LiteralValue::Bool(a >= b), f)),
                _ => None,
            };
        }

        None
    }

    fn assign_op_to_binary_op(op: &baml_compiler2_ast::AssignOp) -> baml_compiler2_ast::BinaryOp {
        use baml_compiler2_ast::{AssignOp, BinaryOp};
        match op {
            AssignOp::Add => BinaryOp::Add,
            AssignOp::Sub => BinaryOp::Sub,
            AssignOp::Mul => BinaryOp::Mul,
            AssignOp::Div => BinaryOp::Div,
            AssignOp::Mod => BinaryOp::Mod,
            AssignOp::BitAnd => BinaryOp::BitAnd,
            AssignOp::BitOr => BinaryOp::BitOr,
            AssignOp::BitXor => BinaryOp::BitXor,
            AssignOp::Shl => BinaryOp::Shl,
            AssignOp::Shr => BinaryOp::Shr,
        }
    }
}
