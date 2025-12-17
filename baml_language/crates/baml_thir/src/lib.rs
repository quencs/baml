//! Typed High-level Intermediate Representation.
//!
//! Provides type checking and inference for BAML.
//!
//! # Architecture
//!
//! The THIR layer performs bidirectional type checking:
//! - **Inference (synthesize)**: Compute the type of an expression from its structure
//! - **Checking**: Verify an expression has an expected type
//!
//! This follows patterns from rust-analyzer and ruff for incremental type checking.

use std::collections::HashMap;

use baml_base::{Name, SourceFile, Span};
use baml_diagnostics::compiler_error::TypeError;
use baml_hir::{
    ExprBody, ExprId, FunctionBody, FunctionSignature, Pattern, StmtId, project_class_fields,
};
use baml_workspace::Project;

mod lower;
pub mod pretty;
mod types;

pub use lower::lower_type_ref;
pub use pretty::{expr_to_string, render_body_tree, render_function_tree};
pub use types::*;

// ============================================================================
// Path Resolution
// ============================================================================

/// Resolved path categories after name resolution.
///
/// When we encounter a multi-segment path like `user.name.length` or `Status.Active`,
/// we need to determine what it actually refers to. This enum captures the different
/// possibilities.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolvedPath {
    /// Local variable: `user` or `user.name.length`
    /// The name is the local variable. Field access types are in `path_segment_types`.
    Local { name: Name },

    /// Enum variant: `Status.Active`
    /// First segment is the enum type, second is the variant name.
    EnumVariant { enum_name: Name, variant_name: Name },

    /// Module item: `baml.HttpMethod.Get`
    /// The path navigates through modules to reach an item.
    ModuleItem {
        module_path: Vec<Name>,
        item_name: Name,
    },

    /// Function reference: `MyFunction`
    /// A single-segment path that resolves to a function.
    Function { name: Name },

    /// Method call on a type: `image.from_url`
    /// Used when the receiver is a type with associated methods.
    Method {
        receiver_type: Name,
        method_name: Name,
    },

    /// Unknown/unresolved path
    Unknown,
}

//
// ──────────────────────────────────────────────────────────── DATABASE ─────
//

/// Database trait for THIR queries.
///
/// This trait extends `baml_hir::Db` and provides access to all THIR-related
/// Salsa queries, including type inference and the initial typing context.
#[salsa::db]
pub trait Db: baml_hir::Db {}

// ============================================================================
// Typing Context Construction
// ============================================================================

/// Build typing context from a list of source files.
///
/// This maps function names to their arrow types, e.g.:
/// `Foo` -> `(int) -> int` for `function Foo(x: int) -> int`
///
/// This is used as the starting scope when type-checking function bodies,
/// allowing function calls to be properly typed.
///
/// Note: This is not a Salsa query because it returns `Ty<'db>` which contains
/// lifetime-parameterized data. Callers should cache the result if needed.
pub fn build_typing_context_from_files<'db>(
    db: &'db dyn Db,
    files: &[SourceFile],
) -> HashMap<Name, Ty<'db>> {
    let mut context = HashMap::new();

    for file in files {
        let items_struct = baml_hir::file_items(db, *file);
        let items = items_struct.items(db);

        for item in items {
            if let baml_hir::ItemId::Function(func_loc) = item {
                let signature = baml_hir::function_signature(db, *func_loc);

                // Build the arrow type: (param_types) -> return_type
                let param_types: Vec<Ty<'db>> = signature
                    .params
                    .iter()
                    .map(|p| lower_type_ref(db, &p.type_ref))
                    .collect();

                let return_type = lower_type_ref(db, &signature.return_type);

                let func_type = Ty::Function {
                    params: param_types,
                    ret: Box::new(return_type),
                };

                context.insert(signature.name.clone(), func_type);
            }
        }
    }

    context
}

/// Build class fields map from source files.
///
/// This maps class names to their field types, e.g.:
/// `Baz` -> { `name` -> `String` }
///
/// Used for field access type checking.
///
/// This function lowers HIR `TypeRefs` to THIR `Ty`s. It iterates through
/// the provided files and uses the Salsa-tracked `baml_hir::class_fields` query
/// for each class, providing better incrementality than the old implementation.
///
/// Note: Once `baml_workspace::project_files` is implemented, this can be
/// replaced with a simpler version that uses `baml_hir::project_class_fields`.
pub fn build_class_fields_from_files(
    db: &dyn Db,
    project: Project,
) -> HashMap<Name, HashMap<Name, Ty<'_>>> {
    let class_fields = project_class_fields(db, project);

    class_fields
        .classes(db)
        .iter()
        .map(|(class_name, class_fields)| {
            (
                class_name.clone(),
                class_fields
                    .iter()
                    .map(|(field_name, field_type)| {
                        (field_name.clone(), lower_type_ref(db, field_type))
                    })
                    .collect(),
            )
        })
        .collect()
}

/// Build class fields map for a project using Salsa queries.
///
/// This maps class names to their field types, e.g.:
/// `Baz` -> { `name` -> `String` }
///
/// Used for field access type checking.
///
/// This function uses the Salsa-tracked `baml_hir::project_class_fields` query
/// for maximum incrementality, then lowers HIR `TypeRefs` to THIR `Ty`s.
///
/// This is the preferred API - it properly uses the Salsa query system.
///
/// TODO: How do we make this incremental/cached? It seems like the
/// `ClassId` and `EnumId` inside `Ty`, which are salsa references, make it
/// impossible to track `Ty`.
pub fn lower_project_class_fields(
    db: &dyn Db,
    root: baml_workspace::Project,
) -> HashMap<Name, HashMap<Name, Ty<'_>>> {
    let hir_fields = baml_hir::project_class_fields(db, root);

    hir_fields
        .classes(db)
        .iter()
        .map(|(class_name, fields)| {
            let lowered_fields = fields
                .iter()
                .map(|(field_name, type_ref)| (field_name.clone(), lower_type_ref(db, type_ref)))
                .collect();
            (class_name.clone(), lowered_fields)
        })
        .collect()
}

// ============================================================================
// Type Inference Results
// ============================================================================

/// Result of type inference for a function.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InferenceResult<'db> {
    /// Inferred return type of the function.
    pub return_type: Ty<'db>,
    /// Types of parameters.
    pub param_types: HashMap<Name, Ty<'db>>,
    /// Types inferred for each expression.
    pub expr_types: HashMap<ExprId, Ty<'db>>,
    /// For multi-segment path expressions, the type of each segment.
    /// For `o.inner.value` where `o: Outer`, stores `[Outer, Inner, int]`.
    /// Used by codegen to look up field indices at each step.
    pub path_segment_types: HashMap<ExprId, Vec<Ty<'db>>>,
    /// Type checking errors.
    pub errors: Vec<TypeError<Ty<'db>>>,
}

// ============================================================================
// Type Context
// ============================================================================

/// Context for type inference, tracking scopes and accumulated results.
pub struct TypeContext<'db> {
    db: &'db dyn Db,
    /// Stack of variable scopes (innermost last).
    scopes: Vec<HashMap<Name, Ty<'db>>>,
    /// Class field types: `class_name` -> (`field_name` -> `field_type`)
    class_fields: HashMap<Name, HashMap<Name, Ty<'db>>>,
    /// Inferred types for expressions.
    expr_types: HashMap<ExprId, Ty<'db>>,
    /// For multi-segment paths, the type of each segment.
    path_segment_types: HashMap<ExprId, Vec<Ty<'db>>>,
    /// Accumulated type errors.
    errors: Vec<TypeError<Ty<'db>>>,
}

impl<'db> TypeContext<'db> {
    /// Create a new type context with an initial scope of global bindings.
    ///
    /// The initial scope typically contains top-level function types, allowing
    /// function calls to be properly typed. Pass an empty `HashMap` for no globals.
    pub fn new(db: &'db dyn Db, globals: HashMap<Name, Ty<'db>>) -> Self {
        TypeContext {
            db,
            scopes: vec![globals],
            class_fields: HashMap::new(),
            expr_types: HashMap::new(),
            path_segment_types: HashMap::new(),
            errors: Vec::new(),
        }
    }

    /// Create a new type context with global bindings and class field information.
    pub fn with_class_fields(
        db: &'db dyn Db,
        globals: HashMap<Name, Ty<'db>>,
        class_fields: HashMap<Name, HashMap<Name, Ty<'db>>>,
    ) -> Self {
        TypeContext {
            db,
            scopes: vec![globals],
            class_fields,
            expr_types: HashMap::new(),
            path_segment_types: HashMap::new(),
            errors: Vec::new(),
        }
    }

    /// Look up a field in a class.
    pub fn lookup_class_field(&self, class_name: &Name, field_name: &Name) -> Option<&Ty<'db>> {
        self.class_fields
            .get(class_name)
            .and_then(|fields| fields.get(field_name))
    }

    /// Push a new scope.
    pub fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    /// Pop the current scope.
    pub fn pop_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    /// Define a variable in the current scope.
    pub fn define(&mut self, name: Name, ty: Ty<'db>) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name, ty);
        }
    }

    /// Look up a variable in the scope chain.
    pub fn lookup(&self, name: &Name) -> Option<&Ty<'db>> {
        for scope in self.scopes.iter().rev() {
            if let Some(ty) = scope.get(name) {
                return Some(ty);
            }
        }
        None
    }

    /// Record the type of an expression.
    pub fn set_expr_type(&mut self, expr: ExprId, ty: Ty<'db>) {
        self.expr_types.insert(expr, ty);
    }

    /// Get the type of an expression.
    #[allow(dead_code)]
    pub fn get_expr_type(&self, expr: ExprId) -> Option<&Ty<'db>> {
        self.expr_types.get(&expr)
    }

    /// Add a type error.
    pub fn push_error(&mut self, error: TypeError<Ty<'db>>) {
        self.errors.push(error);
    }

    /// Get the database reference.
    pub fn db(&self) -> &'db dyn Db {
        self.db
    }

    /// Resolve a path to determine what it refers to.
    ///
    /// This is the core path resolution logic that determines whether a path like
    /// `user.name` is a local variable with field access, an enum variant, a module
    /// item, etc.
    ///
    /// # Resolution Order
    /// 1. Check if the first segment is a local variable -> Local with field accesses
    /// 2. Check if it's a function name -> Function
    /// 3. Check if first segment is a class name (for enum variants)
    /// 4. Check if it's a module path (TODO: not yet implemented)
    /// 5. Unknown
    pub fn resolve_path(&self, segments: &[Name]) -> ResolvedPath {
        if segments.is_empty() {
            return ResolvedPath::Unknown;
        }

        let first = &segments[0];

        // Check if first segment is a local variable
        if self.lookup(first).is_some() {
            return ResolvedPath::Local {
                name: first.clone(),
            };
        }

        // For single-segment paths, check if it's a function
        if segments.len() == 1 {
            // Check globals (which include functions)
            if self.scopes.first().and_then(|s| s.get(first)).is_some() {
                return ResolvedPath::Function {
                    name: first.clone(),
                };
            }
        }

        // For two-segment paths, check if it could be an enum variant
        // TODO: This needs access to the type registry to check if `first` is an enum
        if segments.len() == 2 {
            // For now, we can't distinguish enum variants without more context
            // This will be populated when we have the Module infrastructure
        }

        // TODO: Check module paths when Module type is fully integrated

        // Unknown path
        ResolvedPath::Unknown
    }
}

// ============================================================================
// Type Inference
// ============================================================================

/// Infer types for a function body.
///
/// This is the main entry point for type inference. It takes a pre-lowered
/// function body and infers types for all expressions.
///
/// The `globals` parameter provides types for top-level functions, allowing
/// function calls to be properly typed. Pass `None` if no global context is needed.
///
/// Note: In a full implementation, this would be a Salsa tracked function.
/// For now, it's a regular function that takes the body directly.
pub fn infer_function_body<'db>(
    db: &'db dyn Db,
    body: &FunctionBody,
    param_types: HashMap<Name, Ty<'db>>,
    expected_return: &Ty<'db>,
    globals: Option<HashMap<Name, Ty<'db>>>,
    class_fields: Option<HashMap<Name, HashMap<Name, Ty<'db>>>>,
) -> InferenceResult<'db> {
    let mut ctx = TypeContext::with_class_fields(
        db,
        globals.unwrap_or_default(),
        class_fields.unwrap_or_default(),
    );

    // Add parameters to the current scope (on top of globals)
    for (name, ty) in &param_types {
        ctx.define(name.clone(), ty.clone());
    }

    // Type check the body
    let return_type = match body {
        FunctionBody::Expr(expr_body) => {
            if let Some(root_expr) = expr_body.root_expr {
                infer_expr(&mut ctx, root_expr, expr_body)
            } else {
                Ty::Void
            }
        }
        FunctionBody::Llm(_) => {
            // LLM functions return their declared return type
            expected_return.clone()
        }
        FunctionBody::Missing => Ty::Unknown,
    };

    // Check return type matches (if we have span info, we'd report errors here)
    if !return_type.is_subtype_of(expected_return)
        && !return_type.is_unknown()
        && !expected_return.is_unknown()
    {
        // We'd need the span of the function body for this error
        // For now, we skip this check
    }

    InferenceResult {
        return_type,
        param_types,
        expr_types: ctx.expr_types,
        path_segment_types: ctx.path_segment_types,
        errors: ctx.errors,
    }
}

/// Infer types for a function given its signature and body.
///
/// This is the entry point for type inference from the test suite.
/// It takes pre-fetched signature and body data, allowing the caller (`baml_db`)
/// to handle the Salsa queries for fetching this data.
///
/// The `globals` parameter provides types for top-level functions, allowing
/// function calls to be properly typed. Pass `None` if no global context is needed.
pub fn infer_function<'db>(
    db: &'db dyn Db,
    signature: &FunctionSignature,
    body: &FunctionBody,
    globals: Option<HashMap<Name, Ty<'db>>>,
    class_fields: Option<HashMap<Name, HashMap<Name, Ty<'db>>>>,
) -> InferenceResult<'db> {
    // Convert parameter TypeRefs to Tys
    let param_types: HashMap<Name, Ty<'db>> = signature
        .params
        .iter()
        .map(|param| {
            let ty = lower_type_ref(db, &param.type_ref);
            (param.name.clone(), ty)
        })
        .collect();

    // Convert return type
    let expected_return = lower_type_ref(db, &signature.return_type);

    // Delegate to the body inference function
    infer_function_body(
        db,
        body,
        param_types,
        &expected_return,
        globals,
        class_fields,
    )
}

/// Infer the type of an expression (synthesize mode).
fn infer_expr<'db>(ctx: &mut TypeContext<'db>, expr_id: ExprId, body: &ExprBody) -> Ty<'db> {
    use baml_hir::Expr;

    let expr = &body.exprs[expr_id];

    // Create a placeholder span for errors (ideally we'd track spans in ExprBody)
    let span = Span::new(
        baml_base::FileId::new(0),
        text_size::TextRange::empty(0.into()),
    );

    let ty = match expr {
        Expr::Literal(lit) => infer_literal(lit),

        Expr::Path(segments) => {
            if segments.is_empty() {
                Ty::Unknown
            } else if segments.len() == 1 {
                // Single segment: simple variable lookup
                let name = &segments[0];
                if let Some(ty) = ctx.lookup(name) {
                    ty.clone()
                } else {
                    ctx.push_error(TypeError::UnknownVariable {
                        name: name.to_string(),
                        span,
                    });
                    Ty::Unknown
                }
            } else {
                // Multi-segment path: first segment is variable, rest are field accesses
                // TODO: Add proper resolution for enum variants and module paths
                let first = &segments[0];
                let mut ty = if let Some(t) = ctx.lookup(first) {
                    t.clone()
                } else {
                    ctx.push_error(TypeError::UnknownVariable {
                        name: first.to_string(),
                        span,
                    });
                    return Ty::Unknown;
                };

                // Record segment types for codegen (first segment type, then each field access result)
                let mut segment_types = vec![ty.clone()];

                // Apply field accesses for remaining segments
                for field in &segments[1..] {
                    ty = infer_field_access(ctx, &ty, field, span);
                    segment_types.push(ty.clone());
                }

                // Store segment types for this path expression
                ctx.path_segment_types.insert(expr_id, segment_types);

                ty
            }
        }

        Expr::Binary { lhs, op, rhs } => {
            let lhs_ty = infer_expr(ctx, *lhs, body);
            let rhs_ty = infer_expr(ctx, *rhs, body);
            infer_binary_op(ctx, *op, &lhs_ty, &rhs_ty, span)
        }

        Expr::Unary { op, expr: inner } => {
            let inner_ty = infer_expr(ctx, *inner, body);
            infer_unary_op(ctx, *op, &inner_ty, span)
        }

        Expr::Call { callee, args } => {
            // Check if this is a method call (callee is a FieldAccess or multi-segment Path)
            // If so, we need to pass the receiver as the first argument
            let (callee_ty, effective_args) = match &body.exprs[*callee] {
                Expr::FieldAccess { base, field: _ } => {
                    // Method call: receiver.method(args) -> Type.method(receiver, args)
                    // This handles complex expressions like `f().method()` or `arr[0].method()`
                    let receiver_ty = infer_expr(ctx, *base, body);
                    let callee_ty = infer_expr(ctx, *callee, body);

                    // Build effective args: [receiver_type, ...explicit_args]
                    let mut effective_args = vec![receiver_ty];
                    for arg in args {
                        effective_args.push(infer_expr(ctx, *arg, body));
                    }
                    (callee_ty, effective_args)
                }
                Expr::Path(segments) if segments.len() >= 2 => {
                    // Method call via Path: `receiver.method(args)`
                    // For multi-segment paths like `baz.Greeting()`, the first segment(s)
                    // form the receiver and the last segment is the method name.
                    //
                    // We infer the receiver type from all segments except the last,
                    // then look up the method on that type.
                    let receiver_segments = &segments[..segments.len() - 1];

                    // Infer receiver type (could be single var or nested field access)
                    let receiver_ty = if receiver_segments.len() == 1 {
                        // Simple receiver: `baz.method()`
                        ctx.lookup(&receiver_segments[0])
                            .cloned()
                            .unwrap_or(Ty::Unknown)
                    } else {
                        // Nested receiver: `obj.field.method()`
                        let first = &receiver_segments[0];
                        let mut ty = ctx.lookup(first).cloned().unwrap_or(Ty::Unknown);
                        for field in &receiver_segments[1..] {
                            ty = infer_field_access(ctx, &ty, field, span);
                        }
                        ty
                    };

                    let callee_ty = infer_expr(ctx, *callee, body);

                    // Build effective args: [receiver_type, ...explicit_args]
                    let mut effective_args = vec![receiver_ty];
                    for arg in args {
                        effective_args.push(infer_expr(ctx, *arg, body));
                    }
                    (callee_ty, effective_args)
                }
                _ => {
                    // Regular function call (single-segment Path or other expression)
                    let callee_ty = infer_expr(ctx, *callee, body);
                    let arg_types: Vec<Ty<'db>> =
                        args.iter().map(|arg| infer_expr(ctx, *arg, body)).collect();
                    (callee_ty, arg_types)
                }
            };

            // If the callee is a function type, check arguments and return the return type
            match &callee_ty {
                Ty::Function { params, ret } => {
                    // Check argument count
                    if effective_args.len() != params.len() {
                        ctx.push_error(TypeError::ArgumentCountMismatch {
                            expected: params.len(),
                            found: effective_args.len(),
                            span,
                        });
                    }

                    // Check argument types
                    for (arg_ty, param_ty) in effective_args.iter().zip(params.iter()) {
                        if !arg_ty.is_subtype_of(param_ty) {
                            ctx.push_error(TypeError::TypeMismatch {
                                expected: param_ty.clone(),
                                found: arg_ty.clone(),
                                span, // Ideally we'd have the span of each arg
                            });
                        }
                    }

                    // Return the function's return type
                    (**ret).clone()
                }
                Ty::Unknown => Ty::Unknown,
                _ => {
                    ctx.push_error(TypeError::NotCallable {
                        ty: callee_ty,
                        span,
                    });
                    Ty::Unknown
                }
            }
        }

        Expr::FieldAccess { base, field } => {
            let base_ty = infer_expr(ctx, *base, body);
            infer_field_access(ctx, &base_ty, field, span)
        }

        Expr::Index { base, index } => {
            let base_ty = infer_expr(ctx, *base, body);
            let index_ty = infer_expr(ctx, *index, body);
            infer_index_access(ctx, &base_ty, &index_ty, span)
        }

        Expr::Array { elements } => {
            if elements.is_empty() {
                Ty::List(Box::new(Ty::Unknown))
            } else {
                // Infer element type from first element
                let elem_ty = infer_expr(ctx, elements[0], body);
                // Check all elements have compatible types
                for &elem in &elements[1..] {
                    let other_ty = infer_expr(ctx, elem, body);
                    if !other_ty.is_subtype_of(&elem_ty) {
                        ctx.push_error(TypeError::TypeMismatch {
                            expected: elem_ty.clone(),
                            found: other_ty,
                            span,
                        });
                    }
                }
                Ty::List(Box::new(elem_ty))
            }
        }

        Expr::Object { type_name, fields } => {
            // Infer field types
            for (_, value_expr) in fields {
                infer_expr(ctx, *value_expr, body);
            }
            // Return the named type if type_name is provided
            if let Some(name) = type_name {
                Ty::Named(name.clone())
            } else {
                // Anonymous object - return Unknown for now
                Ty::Unknown
            }
        }

        Expr::Block { stmts, tail_expr } => {
            ctx.push_scope();

            // Type check statements
            for &stmt_id in stmts {
                check_stmt(ctx, stmt_id, body);
            }

            // Type of block is type of tail expression
            let result = if let Some(tail) = tail_expr {
                infer_expr(ctx, *tail, body)
            } else {
                Ty::Void
            };

            ctx.pop_scope();
            result
        }

        Expr::If {
            condition,
            then_branch,
            else_branch,
        } => {
            // Condition must be bool
            let cond_ty = infer_expr(ctx, *condition, body);
            if !cond_ty.is_subtype_of(&Ty::Bool) {
                ctx.push_error(TypeError::TypeMismatch {
                    expected: Ty::Bool,
                    found: cond_ty,
                    span,
                });
            }

            // Infer branch types
            let then_ty = infer_expr(ctx, *then_branch, body);
            let else_ty = if let Some(else_expr) = else_branch {
                infer_expr(ctx, *else_expr, body)
            } else {
                Ty::Void
            };

            // Result is union of branches (simplified)
            if then_ty == else_ty {
                then_ty
            } else if else_branch.is_none() {
                // if without else returns optional
                Ty::Union(vec![then_ty, Ty::Null])
            } else {
                Ty::Union(vec![then_ty, else_ty])
            }
        }

        // Match expressions synthesize a type.
        // TODO: we should support bidirectional type checking
        Expr::Match { scrutinee, arms } => {
            let scrutinee_ty = infer_expr(ctx, *scrutinee, body);

            if arms.is_empty() {
                // Empty match is non-exhaustive (unless scrutinee is uninhabited).
                // An uninhabited type has no possible values, so an empty match is
                // actually exhaustive—there are no cases to handle.
                // See `Ty::is_uninhabited()` for the full definition and rationale.
                if !scrutinee_ty.is_uninhabited() {
                    ctx.push_error(TypeError::NonExhaustiveMatch {
                        scrutinee_type: scrutinee_ty.clone(),
                        missing_cases: vec!["all cases".to_string()],
                        span,
                    });
                }
                Ty::Unknown
            } else {
                // Perform exhaustiveness checking and unreachable arm detection
                check_match_exhaustiveness(ctx, &scrutinee_ty, arms, body, span);

                // Collect result types from all arms
                let arm_types: Vec<Ty> = arms
                    .iter()
                    .map(|arm| {
                        // Push a scope for the arm's pattern bindings
                        ctx.push_scope();

                        // Extract pattern and determine the narrowed type
                        let pattern = &body.patterns[arm.pattern];
                        let (binding_name, narrowed_ty) =
                            extract_pattern_binding(ctx, pattern, &scrutinee_ty, body);

                        // Bind the pattern variable with the narrowed type
                        if let Some(name) = binding_name {
                            ctx.define(name, narrowed_ty);
                        }

                        // Type-check the guard (if present)
                        if let Some(guard) = arm.guard {
                            let guard_ty = infer_expr(ctx, guard, body);
                            if !guard_ty.is_subtype_of(&Ty::Bool) && !guard_ty.is_unknown() {
                                ctx.push_error(TypeError::TypeMismatch {
                                    expected: Ty::Bool,
                                    found: guard_ty,
                                    span,
                                });
                            }
                        }

                        // Type-check the arm body
                        let body_ty = infer_expr(ctx, arm.body, body);

                        ctx.pop_scope();
                        body_ty
                    })
                    .collect();

                // If all arms have the same type, use that; otherwise union
                if arm_types.iter().all(|t| t == &arm_types[0]) {
                    arm_types.into_iter().next().unwrap_or(Ty::Unknown)
                } else {
                    Ty::Union(arm_types)
                }
            }
        }

        Expr::Missing => Ty::Unknown,
    };

    ctx.set_expr_type(expr_id, ty.clone());
    ty
}

/// Extract binding name and narrowed type from a match pattern.
///
/// Returns `(Some(name), narrowed_type)` for binding patterns, or `(None, scrutinee_type)` for
/// patterns that don't introduce bindings (literals, enum variants, unions).
///
/// # Type Narrowing Rules
/// - `name: Type` binds `name` with type `Type` (from the type annotation)
/// - `name` (without type) binds `name` with the scrutinee type (catch-all)
/// - `_` is a special case of binding that's semantically discarded later
/// - Literals, enum variants, and union patterns don't introduce bindings
fn extract_pattern_binding<'db>(
    ctx: &TypeContext<'db>,
    pattern: &Pattern,
    scrutinee_ty: &Ty<'db>,
    _body: &ExprBody,
) -> (Option<Name>, Ty<'db>) {
    match pattern {
        // Typed binding: `s: Success` -> s has type Success
        Pattern::TypedBinding { name, ty } => {
            let narrowed_ty = lower_type_ref(ctx.db(), ty);
            (Some(name.clone()), narrowed_ty)
        }

        // Simple binding: `x` or `_` -> binds with scrutinee type (catch-all)
        Pattern::Binding(name) => {
            // `_` is semantically discarded but still creates a binding during type checking
            // The "discard" behavior is handled in codegen, not here
            (Some(name.clone()), scrutinee_ty.clone())
        }

        // Literal patterns don't introduce bindings
        Pattern::Literal(_) => (None, scrutinee_ty.clone()),

        // Enum variant patterns don't introduce bindings
        // (they match by value equality, not type)
        Pattern::EnumVariant { .. } => (None, scrutinee_ty.clone()),

        // Union patterns don't introduce bindings
        // (they're unions of literals or enum variants)
        Pattern::Union(_) => (None, scrutinee_ty.clone()),
    }
}

// ============================================================================
// Match Exhaustiveness and Unreachability Checking
// ============================================================================

/// Represents the coverage of a match pattern.
///
/// Coverage is used to track which cases have been handled by match arms.
/// This enables both exhaustiveness checking (ensuring all cases are covered)
/// and unreachable arm detection (detecting arms that can never match).
#[derive(Debug, Clone, PartialEq, Eq)]
enum PatternCoverage<'db> {
    /// Covers a specific named type (class, enum, or primitive).
    Type(Ty<'db>),
    /// Covers all remaining cases (catch-all binding like `_` or `other`).
    CatchAll,
    /// Covers nothing (patterns with guards don't guarantee coverage).
    Guarded,
    /// Covers a specific literal value.
    Literal(baml_hir::Literal),
    /// Covers multiple patterns (union of coverage).
    Union(Vec<PatternCoverage<'db>>),
}

/// Extract the coverage of a pattern (what cases it handles).
///
/// # Arguments
/// * `pattern` - The pattern to analyze
/// * `has_guard` - Whether this arm has a guard expression
/// * `body` - The expression body (for accessing nested patterns)
/// * `db` - Database reference for type lowering
///
/// # Coverage Rules (per BEP-002)
/// - Guards do NOT contribute to exhaustiveness (always return `Guarded`)
/// - Untyped bindings (`x`, `_`) are catch-alls covering all remaining cases
/// - Typed bindings (`s: Success`) cover that specific type
/// - Literals cover nothing for type exhaustiveness (they're value-level)
/// - Enum variants cover nothing for type exhaustiveness (they're value-level)
fn extract_pattern_coverage<'db>(
    pattern: &Pattern,
    has_guard: bool,
    body: &ExprBody,
    db: &'db dyn Db,
) -> PatternCoverage<'db> {
    // Guards prevent a pattern from contributing to exhaustiveness
    if has_guard {
        return PatternCoverage::Guarded;
    }

    match pattern {
        // Untyped binding (including `_`) is a catch-all
        Pattern::Binding(_) => PatternCoverage::CatchAll,

        // Typed binding covers the specified type
        Pattern::TypedBinding { ty, .. } => {
            let lowered_ty = lower_type_ref(db, ty);
            PatternCoverage::Type(lowered_ty)
        }

        // Literals cover a specific value (not type-level coverage)
        Pattern::Literal(lit) => PatternCoverage::Literal(lit.clone()),

        // Enum variants are value-level, not type-level coverage
        Pattern::EnumVariant { .. } => PatternCoverage::Literal(baml_hir::Literal::Null), // Placeholder

        // Union patterns combine coverage of sub-patterns
        Pattern::Union(sub_pats) => {
            let coverages: Vec<_> = sub_pats
                .iter()
                .map(|pat_id| {
                    let sub_pattern = &body.patterns[*pat_id];
                    extract_pattern_coverage(sub_pattern, false, body, db)
                })
                .collect();
            PatternCoverage::Union(coverages)
        }
    }
}

/// Expand a type into its constituent cases for exhaustiveness checking.
///
/// For union types, this returns each member of the union.
/// For other types, returns a single-element set containing the type itself.
fn expand_type_to_cases<'db>(ty: &Ty<'db>) -> Vec<Ty<'db>> {
    match ty {
        Ty::Union(members) => {
            // Flatten nested unions and collect all cases
            members.iter().flat_map(expand_type_to_cases).collect()
        }
        Ty::Optional(inner) => {
            // Optional<T> is equivalent to T | null
            let mut cases = expand_type_to_cases(inner);
            cases.push(Ty::Null);
            cases
        }
        _ => vec![ty.clone()],
    }
}

/// Check if two types overlap (have common values).
///
/// This is used for exhaustiveness checking to determine if a pattern
/// covering one type could potentially match values of another type.
fn types_overlap<'db>(ty1: &Ty<'db>, ty2: &Ty<'db>) -> bool {
    // Same type always overlaps
    if ty1 == ty2 {
        return true;
    }

    // Handle union types
    match (ty1, ty2) {
        (Ty::Union(members1), _) => members1.iter().any(|m| types_overlap(m, ty2)),
        (_, Ty::Union(members2)) => members2.iter().any(|m| types_overlap(ty1, m)),
        (Ty::Named(n1), Ty::Named(n2)) => n1 == n2,
        _ => false,
    }
}

/// Check match exhaustiveness and detect unreachable arms.
///
/// This function implements the exhaustiveness checking rules from BEP-002:
/// 1. All cases must be covered explicitly or via catch-all
/// 2. Guards do NOT contribute to exhaustiveness
/// 3. Catch-all (`_` or untyped binding) covers remaining cases
/// 4. Arms after a catch-all are unreachable
///
/// # Errors
/// - `TypeError::NonExhaustiveMatch` if not all cases are covered
/// - `TypeError::UnreachableArm` if an arm can never match
fn check_match_exhaustiveness<'db>(
    ctx: &mut TypeContext<'db>,
    scrutinee_ty: &Ty<'db>,
    arms: &[baml_hir::MatchArm],
    body: &ExprBody,
    span: Span,
) {
    // Skip exhaustiveness checking for unknown/error types
    if scrutinee_ty.is_unknown() || scrutinee_ty.is_error() {
        return;
    }

    // Expand the scrutinee type into its constituent cases
    let all_cases = expand_type_to_cases(scrutinee_ty);

    // Track which cases have been covered (without guards)
    let mut covered_cases: Vec<Ty<'db>> = Vec::new();
    let mut has_catch_all = false;

    for (_arm_idx, arm) in arms.iter().enumerate() {
        let pattern = &body.patterns[arm.pattern];
        let has_guard = arm.guard.is_some();
        let coverage = extract_pattern_coverage(pattern, has_guard, body, ctx.db());

        // Check for unreachable arm
        // An arm is unreachable if:
        // 1. A previous catch-all (without guard) already covered all cases
        // 2. All types this pattern could match are already covered
        if has_catch_all {
            // After a catch-all, all subsequent arms are unreachable
            // We need a span for this arm, but we don't have one in MatchArm
            // For now, use the match expression span
            ctx.push_error(TypeError::UnreachableArm { span });
            continue;
        }

        // Check if this arm is unreachable due to type coverage
        if !has_guard {
            match &coverage {
                PatternCoverage::Type(arm_ty) => {
                    // Check if all cases this arm could match are already covered
                    let arm_cases = expand_type_to_cases(arm_ty);
                    let all_arm_cases_covered = arm_cases.iter().all(|case| {
                        covered_cases.iter().any(|covered| {
                            case == covered
                                || case.is_subtype_of(covered)
                                || types_overlap(case, covered)
                        })
                    });

                    if all_arm_cases_covered && !arm_cases.is_empty() {
                        ctx.push_error(TypeError::UnreachableArm { span });
                    }
                }
                PatternCoverage::CatchAll => {
                    // This arm is a catch-all - mark it
                    has_catch_all = true;
                }
                _ => {}
            }
        }

        // Update covered cases (only for non-guarded patterns)
        if !has_guard {
            match coverage {
                PatternCoverage::CatchAll => {
                    // Catch-all covers everything
                    covered_cases = all_cases.clone();
                }
                PatternCoverage::Type(ty) => {
                    // Add the covered type(s)
                    for case in expand_type_to_cases(&ty) {
                        if !covered_cases.contains(&case) {
                            covered_cases.push(case);
                        }
                    }
                }
                PatternCoverage::Union(coverages) => {
                    // Add all types from the union
                    for cov in coverages {
                        if let PatternCoverage::Type(ty) = cov {
                            for case in expand_type_to_cases(&ty) {
                                if !covered_cases.contains(&case) {
                                    covered_cases.push(case);
                                }
                            }
                        }
                    }
                }
                PatternCoverage::Literal(_) | PatternCoverage::Guarded => {
                    // Literals and guarded patterns don't contribute to type-level coverage
                }
            }
        }
    }

    // Check exhaustiveness: all cases must be covered
    if !has_catch_all {
        let missing_cases: Vec<String> = all_cases
            .iter()
            .filter(|case| {
                !covered_cases.iter().any(|covered| {
                    *case == covered || case.is_subtype_of(covered) || types_overlap(case, covered)
                })
            })
            .map(|ty| ty.to_string())
            .collect();

        if !missing_cases.is_empty() {
            ctx.push_error(TypeError::NonExhaustiveMatch {
                scrutinee_type: scrutinee_ty.clone(),
                missing_cases,
                span,
            });
        }
    }
}

/// Infer the type of a literal.
fn infer_literal(lit: &baml_hir::Literal) -> Ty<'static> {
    match lit {
        baml_hir::Literal::Int(_) => Ty::Int,
        baml_hir::Literal::Float(_) => Ty::Float,
        baml_hir::Literal::String(_) => Ty::String,
        baml_hir::Literal::Bool(_) => Ty::Bool,
        baml_hir::Literal::Null => Ty::Null,
    }
}

/// Infer the result type of a binary operation.
fn infer_binary_op<'db>(
    ctx: &mut TypeContext<'db>,
    op: baml_hir::BinaryOp,
    lhs: &Ty<'db>,
    rhs: &Ty<'db>,
    span: Span,
) -> Ty<'db> {
    use baml_hir::BinaryOp::{
        Add, And, BitAnd, BitOr, BitXor, Div, Eq, Ge, Gt, Le, Lt, Mod, Mul, Ne, Or, Shl, Shr, Sub,
    };

    match op {
        // Arithmetic operations (and string concatenation for Add)
        Add => match (lhs, rhs) {
            (Ty::Int, Ty::Int) => Ty::Int,
            (Ty::Float, Ty::Float) => Ty::Float,
            (Ty::Int, Ty::Float) => Ty::Float,
            (Ty::Float, Ty::Int) => Ty::Float,
            // String concatenation
            (Ty::String, Ty::String) => Ty::String,
            _ => {
                ctx.push_error(TypeError::InvalidBinaryOp {
                    op: format!("{op:?}"),
                    lhs: lhs.clone(),
                    rhs: rhs.clone(),
                    span,
                });
                Ty::Error
            }
        },
        Sub | Mul | Div | Mod => match (lhs, rhs) {
            (Ty::Int, Ty::Int) => Ty::Int,
            (Ty::Float, Ty::Float) => Ty::Float,
            (Ty::Int, Ty::Float) => Ty::Float,
            (Ty::Float, Ty::Int) => Ty::Float,
            _ => {
                ctx.push_error(TypeError::InvalidBinaryOp {
                    op: format!("{op:?}"),
                    lhs: lhs.clone(),
                    rhs: rhs.clone(),
                    span,
                });
                Ty::Error
            }
        },

        // Comparison operations
        Eq | Ne => Ty::Bool,

        Lt | Le | Gt | Ge => {
            if (lhs.is_subtype_of(&Ty::Int) || lhs.is_subtype_of(&Ty::Float))
                && (rhs.is_subtype_of(&Ty::Int) || rhs.is_subtype_of(&Ty::Float))
            {
                Ty::Bool
            } else {
                ctx.push_error(TypeError::InvalidBinaryOp {
                    op: format!("{op:?}"),
                    lhs: lhs.clone(),
                    rhs: rhs.clone(),
                    span,
                });
                Ty::Error
            }
        }

        // Logical operations
        And | Or => {
            if lhs.is_subtype_of(&Ty::Bool) && rhs.is_subtype_of(&Ty::Bool) {
                Ty::Bool
            } else {
                ctx.push_error(TypeError::InvalidBinaryOp {
                    op: format!("{op:?}"),
                    lhs: lhs.clone(),
                    rhs: rhs.clone(),
                    span,
                });
                Ty::Error
            }
        }

        // Bitwise operations
        BitAnd | BitOr | BitXor | Shl | Shr => {
            if lhs.is_subtype_of(&Ty::Int) && rhs.is_subtype_of(&Ty::Int) {
                Ty::Int
            } else {
                ctx.push_error(TypeError::InvalidBinaryOp {
                    op: format!("{op:?}"),
                    lhs: lhs.clone(),
                    rhs: rhs.clone(),
                    span,
                });
                Ty::Error
            }
        }
    }
}

/// Infer the result type of a unary operation.
fn infer_unary_op<'db>(
    ctx: &mut TypeContext<'db>,
    op: baml_hir::UnaryOp,
    operand: &Ty<'db>,
    span: Span,
) -> Ty<'db> {
    use baml_hir::UnaryOp::{Neg, Not};

    match op {
        Not => {
            if operand.is_subtype_of(&Ty::Bool) {
                Ty::Bool
            } else {
                ctx.push_error(TypeError::InvalidUnaryOp {
                    op: "!".to_string(),
                    operand: operand.clone(),
                    span,
                });
                Ty::Error
            }
        }
        Neg => {
            if operand.is_subtype_of(&Ty::Int) {
                Ty::Int
            } else if operand.is_subtype_of(&Ty::Float) {
                Ty::Float
            } else {
                ctx.push_error(TypeError::InvalidUnaryOp {
                    op: "-".to_string(),
                    operand: operand.clone(),
                    span,
                });
                Ty::Error
            }
        }
    }
}

/// Infer the type of a field access.
///
/// For class types, this handles both field access and method access.
/// Methods are desugared to top-level functions with simple names (not namespaced),
/// so we look them up directly in the global context.
fn infer_field_access<'db>(
    ctx: &mut TypeContext<'db>,
    base: &Ty<'db>,
    field: &Name,
    span: Span,
) -> Ty<'db> {
    let found_field = match base {
        // Ty::Named(class_name) => {
        //     // Try to look up as a method (methods are top-level functions with simple names)
        //     if let Some(method_ty) = ctx.lookup(field) {
        //         return method_ty.clone();
        //     }

        //     // Try to look up as a field in the class
        //     if let Some(field_ty) = ctx.lookup_class_field(class_name, field) {
        //         return field_ty.clone();
        //     }

        //     // Field/method not found
        //     Some(Ty::Unknown)
        // }
        // Ty::Named(class_name) => ctx.lookup_class_field(class_name, field).cloned(),
        Ty::Named(class_name) => ctx
            .lookup(field)
            .or(ctx.lookup_class_field(class_name, field))
            .cloned(),
        Ty::Class(class_id) => {
            let class_fields_data = baml_hir::class_fields(ctx.db(), *class_id);
            let fields = class_fields_data.fields(ctx.db());
            fields
                .iter()
                .find(|(name, _)| name == field)
                .map(|(_, type_ref)| lower_type_ref(ctx.db(), type_ref))
        }
        Ty::Unknown => None,
        _ => None,
    };

    found_field.unwrap_or_else(|| {
        ctx.push_error(TypeError::NoSuchField {
            ty: base.clone(),
            field: field.to_string(),
            span,
        });
        Ty::Unknown
    })
}

/// Infer the type of an index access.
fn infer_index_access<'db>(
    ctx: &mut TypeContext<'db>,
    base: &Ty<'db>,
    index: &Ty<'db>,
    span: Span,
) -> Ty<'db> {
    match base {
        Ty::List(elem) => {
            // Index must be int
            if !index.is_subtype_of(&Ty::Int) {
                ctx.push_error(TypeError::TypeMismatch {
                    expected: Ty::Int,
                    found: index.clone(),
                    span,
                });
            }
            (**elem).clone()
        }
        Ty::Map { key, value } => {
            // Index must match key type
            if !index.is_subtype_of(key) {
                ctx.push_error(TypeError::TypeMismatch {
                    expected: (**key).clone(),
                    found: index.clone(),
                    span,
                });
            }
            (**value).clone()
        }
        Ty::String => {
            // String indexing returns a character (string of length 1)
            if !index.is_subtype_of(&Ty::Int) {
                ctx.push_error(TypeError::TypeMismatch {
                    expected: Ty::Int,
                    found: index.clone(),
                    span,
                });
            }
            Ty::String
        }
        Ty::Unknown => Ty::Unknown,
        _ => {
            ctx.push_error(TypeError::NotIndexable {
                ty: base.clone(),
                span,
            });
            Ty::Unknown
        }
    }
}

/// Type check a statement.
fn check_stmt(ctx: &mut TypeContext<'_>, stmt_id: StmtId, body: &ExprBody) {
    use baml_hir::Stmt;

    let stmt = &body.stmts[stmt_id];

    match stmt {
        Stmt::Let {
            pattern,
            type_annotation,
            initializer,
        } => {
            let ty = if let Some(init) = initializer {
                let init_ty = infer_expr(ctx, *init, body);

                // If there's a type annotation, check it matches
                if let Some(annot) = type_annotation {
                    let annot_ty = lower_type_ref(ctx.db(), annot);
                    if !init_ty.is_subtype_of(&annot_ty) {
                        let span = Span::new(
                            baml_base::FileId::new(0),
                            text_size::TextRange::empty(0.into()),
                        );
                        ctx.push_error(TypeError::TypeMismatch {
                            expected: annot_ty.clone(),
                            found: init_ty,
                            span,
                        });
                    }
                    annot_ty
                } else {
                    init_ty
                }
            } else if let Some(annot) = type_annotation {
                lower_type_ref(ctx.db(), annot)
            } else {
                Ty::Unknown
            };

            // Extract variable name from pattern
            let pat = &body.patterns[*pattern];
            match pat {
                Pattern::Binding(name) => {
                    ctx.define(name.clone(), ty);
                }
                Pattern::TypedBinding { name, ty: _ } => {
                    // TODO: Check declared type matches inferred type
                    ctx.define(name.clone(), ty);
                }
                Pattern::Literal(_) | Pattern::EnumVariant { .. } | Pattern::Union(_) => {
                    // Literals/enum variants/unions don't introduce bindings in let statements
                    // This would be a semantic error, but we'll handle it elsewhere
                }
            }
        }

        Stmt::Expr(expr) => {
            infer_expr(ctx, *expr, body);
        }

        Stmt::Return(expr) => {
            if let Some(e) = expr {
                infer_expr(ctx, *e, body);
            }
            // TODO: Check return type matches function signature
        }

        Stmt::While {
            condition,
            body: while_body,
        } => {
            let cond_ty = infer_expr(ctx, *condition, body);
            if !cond_ty.is_subtype_of(&Ty::Bool) {
                let span = Span::new(
                    baml_base::FileId::new(0),
                    text_size::TextRange::empty(0.into()),
                );
                ctx.push_error(TypeError::TypeMismatch {
                    expected: Ty::Bool,
                    found: cond_ty,
                    span,
                });
            }
            infer_expr(ctx, *while_body, body);
        }

        Stmt::ForIn {
            pattern,
            iterator,
            body: for_body,
        } => {
            let iter_ty = infer_expr(ctx, *iterator, body);

            // Extract element type from iterator
            let elem_ty = match &iter_ty {
                Ty::List(elem) => (**elem).clone(),
                _ => Ty::Unknown,
            };

            ctx.push_scope();

            // Bind the loop variable
            let pat = &body.patterns[*pattern];
            match pat {
                Pattern::Binding(name) => {
                    ctx.define(name.clone(), elem_ty);
                }
                Pattern::TypedBinding { name, ty: _ } => {
                    // TODO: Check declared type matches element type
                    ctx.define(name.clone(), elem_ty);
                }
                Pattern::Literal(_) | Pattern::EnumVariant { .. } | Pattern::Union(_) => {
                    // These patterns don't make sense in for-in loops
                }
            }

            infer_expr(ctx, *for_body, body);
            ctx.pop_scope();
        }

        Stmt::ForCStyle {
            initializer,
            condition,
            update,
            body: for_body,
        } => {
            ctx.push_scope();

            if let Some(init_stmt) = initializer {
                check_stmt(ctx, *init_stmt, body);
            }

            if let Some(cond) = condition {
                let cond_ty = infer_expr(ctx, *cond, body);
                if !cond_ty.is_subtype_of(&Ty::Bool) {
                    let span = Span::new(
                        baml_base::FileId::new(0),
                        text_size::TextRange::empty(0.into()),
                    );
                    ctx.push_error(TypeError::TypeMismatch {
                        expected: Ty::Bool,
                        found: cond_ty,
                        span,
                    });
                }
            }

            if let Some(upd) = update {
                check_stmt(ctx, *upd, body);
            }

            infer_expr(ctx, *for_body, body);
            ctx.pop_scope();
        }

        Stmt::Break | Stmt::Continue => {
            // These are control flow statements with no expressions to type-check.
            // Loop context validation could be added here in the future.
        }

        Stmt::Assign { target, value } => {
            // Type-check both the target and value expressions
            infer_expr(ctx, *target, body);
            infer_expr(ctx, *value, body);
            // TODO: Check that target is assignable (variable or field access)
            // TODO: Check that value type is compatible with target type
        }

        Stmt::AssignOp {
            target,
            op: _,
            value,
        } => {
            // Type-check both the target and value expressions
            infer_expr(ctx, *target, body);
            infer_expr(ctx, *value, body);
            // TODO: Check that target is assignable
            // TODO: Check that the operation is valid for the types
        }

        Stmt::Missing => {}
    }
}
