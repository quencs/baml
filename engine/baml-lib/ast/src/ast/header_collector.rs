use std::{collections::HashMap, sync::Arc};

use internal_baml_diagnostics::Span;

use super::{
    Ast, ExprFn, Expression, ExpressionBlock, Field, Header, Stmt, Top, TopId, ValExpId,
    ValueExprBlock, WithName, WithSpan,
};

/// Alias for header identifiers for improved readability
type HeaderId = String;

/// A simple numeric identifier for a logical header scope (any block: function, for-loop body, expr block, etc.)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ScopeId(pub u32);

/// Classification of scope kinds for visualization semantics
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScopeKind {
    TopLevel,
    ForBody,
    IfThen,
    IfElse,
    Generic,
}

/// Classification of what kind of statement a header labels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HeaderLabelKind {
    None,
    Expression,
    If,
    For,
    Function,
}

/// Minimal header representation for rendering and navigation
#[derive(Debug, Clone)]
pub struct RenderableHeader {
    pub id: String,
    pub title: String,
    /// Normalized within its scope so the first header is level 1
    pub level: u8,
    pub span: Span,
    pub scope: ScopeId,
    /// Markdown parent within the same scope
    pub parent_id: Option<HeaderId>,
    /// Next element in control flow (stubbed)
    pub next_id: Option<HeaderId>,
    /// What kind of statement this header labels
    pub label_kind: HeaderLabelKind,
}

/// Collected index of headers with simple querying APIs
#[derive(Debug, Default, Clone)]
pub struct HeaderIndex {
    pub headers: Vec<RenderableHeader>,
    by_scope: HashMap<ScopeId, Vec<usize>>, // header indexes in source order
    /// Cross-scope/navigational edges between headers (e.g., control-flow nesting)
    pub nested_edges: Vec<(HeaderId, HeaderId)>, // (from_id, to_id)
    /// Root header id per scope (first header encountered in the scope)
    pub scope_root_header: HashMap<ScopeId, HeaderId>,
    /// Kind classification per scope
    pub scope_kind: HashMap<ScopeId, ScopeKind>,
    /// Explicit mapping of container headers (e.g., if-labeled) to their child scope roots
    pub branch_children: HashMap<HeaderId, Vec<HeaderId>>, // parent_header_id -> [child_root_header_id]
    /// Mapping of header id -> names of functions called by the expression this header labels
    pub header_calls: HashMap<HeaderId, Vec<String>>, // header_id -> [callee_name]
}

impl HeaderIndex {
    pub fn headers_in_scope(&self, scope: ScopeId) -> Vec<&RenderableHeader> {
        self.by_scope
            .get(&scope)
            .into_iter()
            .flat_map(|idxs| idxs.iter().map(|i| &self.headers[*i]))
            .collect()
    }

    pub fn find_by_id(&self, id: &str) -> Option<&RenderableHeader> {
        self.headers.iter().find(|h| h.id == id)
    }
}

/// Internal collector to walk AST and build a HeaderIndex
#[derive(Debug)]
pub struct HeaderCollector {
    scope_counter: u32,
    scope_stack: Vec<ScopeId>,
    // Raw headers by scope before normalization and parenting
    raw_by_scope: HashMap<ScopeId, Vec<RawHeader>>, // source order
    // Accumulated nested edges (from_id, to_id)
    nested_edges: Vec<(HeaderId, HeaderId)>,
    // Classification per scope
    scope_kind: HashMap<ScopeId, ScopeKind>,
    // Pending classification for the next scope to be pushed
    pending_next_scope_kind: Option<ScopeKind>,
    // Simple counters per type label for auto-generated headers
    auto_counters: HashMap<String, u32>,
    // Mapping during collection: header id -> function names called by the labeled expression
    header_fn_calls: HashMap<HeaderId, Vec<String>>,
}

#[derive(Debug, Clone)]
struct RawHeader {
    id: String,
    title: String,
    original_level: u8,
    span: Span,
    label_kind: HeaderLabelKind,
}

impl HeaderCollector {
    pub fn collect(ast: &Ast) -> HeaderIndex {
        let mut c = Self {
            scope_counter: 0,
            scope_stack: Vec::new(),
            raw_by_scope: HashMap::new(),
            nested_edges: Vec::new(),
            scope_kind: HashMap::new(),
            pending_next_scope_kind: None,
            auto_counters: HashMap::new(),
            header_fn_calls: HashMap::new(),
        };
        c.visit_ast(ast);
        c.build_index()
    }

    fn push_scope(&mut self) -> ScopeId {
        self.scope_counter += 1;
        let id = ScopeId(self.scope_counter);
        self.scope_stack.push(id);
        // Classify this scope when requested
        if let Some(kind) = self.pending_next_scope_kind.take() {
            self.scope_kind.insert(id, kind);
        }
        id
    }

    fn pop_scope(&mut self) {
        self.scope_stack.pop();
    }

    fn current_scope(&self) -> Option<ScopeId> {
        self.scope_stack.last().copied()
    }

    /// Returns the last header id in the current scope (if any).
    fn current_parent_header(&self) -> Option<HeaderId> {
        let scope = self.current_scope()?;
        self.raw_by_scope
            .get(&scope)
            .and_then(|v| v.last().map(|r| r.id.clone()))
    }

    /// Returns the last header id in the immediate parent scope (if any).
    fn parent_scope_last_header(&self) -> Option<HeaderId> {
        if self.scope_stack.len() < 2 {
            return None;
        }
        let parent_scope = self.scope_stack[self.scope_stack.len() - 2];
        self.raw_by_scope
            .get(&parent_scope)
            .and_then(|v| v.last().map(|r| r.id.clone()))
    }

    fn add_header(
        &mut self,
        title: String,
        level: u8,
        span: Span,
        _is_final_expr: bool,
        label_kind: HeaderLabelKind,
    ) -> HeaderId {
        if let Some(scope) = self.current_scope() {
            let entry = self.raw_by_scope.entry(scope).or_default();
            let id: HeaderId = format!(
                "{}:{}-{}:{}:{}",
                title,
                span.file.path(),
                span.start,
                span.end,
                scope.0
            );
            let is_first_in_scope = entry.is_empty();
            println!(
                "ADD HEADER '{}' (level {}) -> scope {:?}",
                title, level, scope
            );
            entry.push(RawHeader {
                id: id.clone(),
                title: title.clone(),
                original_level: level,
                span: span.clone(),
                label_kind,
            });

            // If first header in this scope and we have a parent header in the outer scope, add nested edge
            if is_first_in_scope {
                if let Some(parent_id) = self.parent_scope_last_header() {
                    // Only connect parent -> child for nested scopes, not for top-level scopes
                    let is_top_level =
                        matches!(self.scope_kind.get(&scope), Some(ScopeKind::TopLevel));
                    if !is_top_level {
                        self.nested_edges.push((parent_id.clone(), id.clone()));
                    }
                }
            }

            // Return the last inserted header id
            return self.current_parent_header().expect("header id must exist");
        }
        String::new()
    }

    fn ensure_default_header_current_scope(
        &mut self,
        type_label: &str,
        span: Span,
        override_title: Option<String>,
        label_kind: HeaderLabelKind,
    ) {
        let Some(scope) = self.current_scope() else {
            return;
        };
        let has_any = self
            .raw_by_scope
            .get(&scope)
            .map(|v| !v.is_empty())
            .unwrap_or(false);
        if has_any {
            return;
        }
        let title = if let Some(t) = override_title {
            t
        } else {
            let counter = self
                .auto_counters
                .entry(type_label.to_string())
                .and_modify(|c| *c += 1)
                .or_insert(1);
            format!("{} {}", type_label, *counter)
        };
        self.add_header(title, 1, span, false, label_kind);
    }

    /// Ensure the current (parent) scope has a default container header based on the
    /// pending next scope kind. Skips if the current scope already has headers or if the
    /// next scope kind is an if-branch.
    fn ensure_parent_default_header_for_next_scope(&mut self, span: Span) {
        let Some(scope) = self.current_scope() else {
            return;
        };
        let has_any = self
            .raw_by_scope
            .get(&scope)
            .map(|v| !v.is_empty())
            .unwrap_or(false);
        if has_any {
            return;
        }
        let next_kind = self.pending_next_scope_kind.unwrap_or(ScopeKind::Generic);
        // Do not create default container headers for if branches
        if matches!(next_kind, ScopeKind::IfThen | ScopeKind::IfElse) {
            return;
        }
        let (type_label, label_kind) = match next_kind {
            ScopeKind::ForBody => ("loop", HeaderLabelKind::For),
            ScopeKind::TopLevel | ScopeKind::Generic => ("block", HeaderLabelKind::None),
            ScopeKind::IfThen | ScopeKind::IfElse => unreachable!(),
        };
        self.ensure_default_header_current_scope(type_label, span, None, label_kind);
    }

    /// Add a set of headers (annotations) into the current scope with a fixed
    /// label kind and final-expression flag. Returns the created header ids in
    /// source order.
    fn add_headers_for_annotations(
        &mut self,
        headers: &[Arc<Header>],
        label_kind: HeaderLabelKind,
        is_final_expr: bool,
    ) -> Vec<HeaderId> {
        let mut header_ids: Vec<HeaderId> = Vec::new();
        if headers.is_empty() {
            return header_ids;
        }
        // Only the deepest-level header inherits the expression's label kind.
        // Outer headers act as structural containers and keep a neutral label.
        let deepest_level = headers.iter().map(|h| h.level).max().unwrap_or(1);
        for h in headers {
            let effective_label = if h.level == deepest_level {
                label_kind
            } else {
                HeaderLabelKind::None
            };
            let hid = self.add_header(
                h.title.clone(),
                h.level,
                h.span.clone(),
                is_final_expr,
                effective_label,
            );
            if !hid.is_empty() {
                header_ids.push(hid);
            }
        }
        header_ids
    }

    /// Determine how to label headers for a given expression.
    fn label_kind_for_expr(expr: &Expression) -> HeaderLabelKind {
        match expr {
            Expression::If(_, _, _, _) => HeaderLabelKind::If,
            _ => HeaderLabelKind::Expression,
        }
    }

    /// Attribute top-level call names in `expr` to all `header_ids`.
    fn attribute_calls_to_headers(&mut self, header_ids: &[HeaderId], expr: &Expression) {
        let top_calls = collect_top_level_calls(expr);
        if top_calls.is_empty() {
            return;
        }
        for hid in header_ids {
            self.header_fn_calls
                .entry(hid.clone())
                .or_default()
                .extend(top_calls.clone());
        }
    }

    fn visit_ast(&mut self, ast: &Ast) {
        for top in &ast.tops {
            self.visit_top(top);
        }
    }

    fn visit_top(&mut self, top: &Top) {
        match top {
            Top::Function(block)
            | Top::Client(block)
            | Top::Generator(block)
            | Top::TestCase(block)
            | Top::RetryPolicy(block) => {
                // ValueExprBlock is a root scope
                self.pending_next_scope_kind = Some(ScopeKind::TopLevel);
                let _ = self.push_scope();
                // Block-level headers label this scope
                let _ = self.add_headers_for_annotations(
                    &block.annotations,
                    HeaderLabelKind::Function,
                    false,
                );
                // Default header logic for top-level blocks
                if block.annotations.is_empty() {
                    let type_label = block.get_type();
                    let (override_title, label_kind) = if type_label == "function" {
                        (Some(block.name().to_string()), HeaderLabelKind::Function)
                    } else {
                        (None, HeaderLabelKind::None)
                    };
                    self.ensure_default_header_current_scope(
                        type_label,
                        block.span().clone(),
                        override_title,
                        label_kind,
                    );
                }
                // Visit fields/expressions inside
                for field in &block.fields {
                    self.visit_field_expression(field);
                }
                self.pop_scope();
            }
            Top::ExprFn(expr_fn) => {
                // Use the body ExpressionBlock as the scope for expr fn
                self.pending_next_scope_kind = Some(ScopeKind::TopLevel);
                let _ = self.push_scope();
                let _ = self.add_headers_for_annotations(
                    &expr_fn.annotations,
                    HeaderLabelKind::Function,
                    false,
                );
                if expr_fn.annotations.is_empty() {
                    self.ensure_default_header_current_scope(
                        "function",
                        expr_fn.span.clone(),
                        Some(expr_fn.name.name().to_string()),
                        HeaderLabelKind::Function,
                    );
                }
                self.visit_expression_block(&expr_fn.body);
                self.pop_scope();
            }
            _ => {}
        }
    }

    fn visit_field_expression(&mut self, field: &Field<Expression>) {
        if let Some(expr) = &field.expr {
            self.visit_expression(expr);
        }
    }

    fn visit_expression(&mut self, expr: &Expression) {
        match expr {
            Expression::ExprBlock(block, block_span) => {
                if self.pending_next_scope_kind.is_none() {
                    self.pending_next_scope_kind = Some(ScopeKind::Generic);
                }
                // Ensure parent default header based on pending scope kind
                self.ensure_parent_default_header_for_next_scope(block_span.clone());
                self.visit_expression_block(block);
            }
            Expression::If(_cond, then_expr, else_expr, span) => {
                // If this scope has no headers yet, synthesize a branching container header
                // so THEN/ELSE child scopes render inside a decision within this scope.
                if let Some(scope) = self.current_scope() {
                    let has_any = self
                        .raw_by_scope
                        .get(&scope)
                        .map(|v| !v.is_empty())
                        .unwrap_or(false);
                    if !has_any {
                        self.ensure_default_header_current_scope(
                            "if",
                            span.clone(),
                            None,
                            HeaderLabelKind::If,
                        );
                    }
                }
                println!("IF expression encountered; preparing THEN and ELSE scopes");
                self.pending_next_scope_kind = Some(ScopeKind::IfThen);
                println!("-- entering THEN branch");
                self.visit_expression(then_expr);
                println!("-- exited THEN branch");

                if let Some(else_expr) = else_expr {
                    self.pending_next_scope_kind = Some(ScopeKind::IfElse);
                    println!("-- entering ELSE branch");
                    self.visit_expression(else_expr);
                    println!("-- exited ELSE branch");
                }
            }
            Expression::Lambda(_args, body, _) => {
                self.pending_next_scope_kind = Some(ScopeKind::Generic);
                self.visit_expression_block(body);
            }
            Expression::Array(exprs, _) => {
                for e in exprs {
                    self.visit_expression(e);
                }
            }
            Expression::Map(map, _) => {
                for (k, v) in map {
                    self.visit_expression(k);
                    self.visit_expression(v);
                }
            }
            Expression::ClassConstructor(cons, _) => {
                for f in &cons.fields {
                    match f {
                        super::ClassConstructorField::Named(_, e) => self.visit_expression(e),
                        super::ClassConstructorField::Spread(e) => self.visit_expression(e),
                    }
                }
            }
            Expression::UnaryOperation { expr, .. } => self.visit_expression(expr),
            _ => {}
        }
    }

    fn visit_expression_block(&mut self, block: &ExpressionBlock) {
        let scope_id = self.push_scope();

        // Visit statements first (preserve source order for MD parenting)
        println!(
            "ENTER BLOCK scope {:?} with {} stmts",
            scope_id,
            block.stmts.len()
        );
        for stmt in &block.stmts {
            match stmt {
                Stmt::Let(let_stmt) => {
                    println!(
                        "LET stmt with {} annotations: {:?}",
                        let_stmt.annotations.len(),
                        let_stmt
                            .annotations
                            .iter()
                            .map(|h| h.title.as_str())
                            .collect::<Vec<_>>()
                    );
                    let label_kind = Self::label_kind_for_expr(&let_stmt.expr);
                    let stmt_header_ids =
                        self.add_headers_for_annotations(&let_stmt.annotations, label_kind, false);
                    // Collect top-level calls for the statement expression and attribute to all headers
                    self.attribute_calls_to_headers(&stmt_header_ids, &let_stmt.expr);
                    self.visit_expression(&let_stmt.expr);
                }
                Stmt::ForLoop(for_stmt) => {
                    println!(
                        "FOR stmt with {} annotations: {:?}",
                        for_stmt.annotations.len(),
                        for_stmt
                            .annotations
                            .iter()
                            .map(|h| h.title.as_str())
                            .collect::<Vec<_>>()
                    );
                    // Record for-loop annotation headers in the current (outer) scope
                    let _ = self.add_headers_for_annotations(
                        &for_stmt.annotations,
                        HeaderLabelKind::For,
                        false,
                    );
                    // Iterate expression evaluated in current scope
                    self.visit_expression(&for_stmt.iterator);
                    // Ensure a parent container header exists for the upcoming loop body.
                    // If there are no explicit loop annotations, insert a synthetic
                    // top-level (level 1) "loop N" header to act as the loop container,
                    // except when the loop body is a simple if-branch container; in that case,
                    // avoid creating an extra synthetic header to prevent redundant containers.
                    if for_stmt.annotations.is_empty() {
                        let body_expr = &for_stmt.body.expr;
                        let body_is_simple_if_container =
                            matches!(body_expr.as_deref(), Some(Expression::If(_, _, _, _)));
                        if !body_is_simple_if_container {
                            let counter = self
                                .auto_counters
                                .entry("loop".to_string())
                                .and_modify(|c| *c += 1)
                                .or_insert(1);
                            let title = format!("loop {}", *counter);
                            let _ = self.add_header(
                                title,
                                1, // ensure sibling at top level within the scope
                                for_stmt.span.clone(),
                                false,
                                HeaderLabelKind::For,
                            );
                        }
                    }
                    // Mark next scope as loop body and visit it
                    self.pending_next_scope_kind = Some(ScopeKind::ForBody);
                    // Now visit the body in its own scope (no prefixed headers)
                    self.visit_expression_block(&for_stmt.body);
                }
                Stmt::Expression(expr) => {
                    // Plain expression statements – just visit the expression
                    self.visit_expression(expr);
                }
                Stmt::Assign(assign_stmt) => {
                    // Visit the RHS expression so calls/headers inside are discovered
                    self.visit_expression(&assign_stmt.expr);
                }
                Stmt::AssignOp(assign_op_stmt) => {
                    // Visit the RHS expression so calls/headers inside are discovered
                    self.visit_expression(&assign_op_stmt.expr);
                }
            }
        }

        // Headers that apply to the final expression belong to this scope and come last
        let label_kind = block
            .expr
            .as_deref()
            .map(Self::label_kind_for_expr)
            .unwrap_or(HeaderLabelKind::Expression);
        let expr_header_ids =
            self.add_headers_for_annotations(&block.expr_headers, label_kind, true);

        // Final expr
        if let Some(expr) = &block.expr {
            self.visit_expression(expr);
        }

        // Attribute top-level calls of the final expression to its headers
        if let Some(expr) = &block.expr {
            self.attribute_calls_to_headers(&expr_header_ids, expr);
        }

        // Default header fallback for blocks without any headers
        let kind_for_scope = self
            .scope_kind
            .get(&scope_id)
            .copied()
            .unwrap_or(ScopeKind::Generic);
        // Do not inject synthetic headers for if-branches.
        // These create confusing extra nodes like "if statement 1" when an ELSE branch
        // has no explicit headers. We only auto-inject for generic/loop scopes.
        if !matches!(kind_for_scope, ScopeKind::IfThen | ScopeKind::IfElse) {
            let maybe_defaults: Option<(&str, HeaderLabelKind)> = match kind_for_scope {
                // Do not inject synthetic headers for loop bodies; the loop container is represented
                // by the outer header annotations (e.g., "Content Loop").
                ScopeKind::ForBody => None,
                ScopeKind::TopLevel => Some(("block", HeaderLabelKind::None)),
                ScopeKind::Generic => Some(("block", HeaderLabelKind::None)),
                ScopeKind::IfThen | ScopeKind::IfElse => unreachable!(),
            };
            if let Some((type_label, default_label_kind)) = maybe_defaults {
                // Pick a reasonable span for the default header
                let span_for_default = if let Some(expr) = &block.expr {
                    expr.span().clone()
                } else if let Some(first_stmt) = block.stmts.first() {
                    first_stmt.span().clone()
                } else {
                    // If there is nothing, skip creating a default header
                    // as we do not have a meaningful span.
                    self.pop_scope();
                    return;
                };
                self.ensure_default_header_current_scope(
                    type_label,
                    span_for_default,
                    None,
                    default_label_kind,
                );
            }
        }

        self.pop_scope();
    }

    fn build_index(self) -> HeaderIndex {
        let mut index = HeaderIndex {
            headers: Vec::new(),
            by_scope: HashMap::new(),
            nested_edges: Vec::new(),
            scope_root_header: HashMap::new(),
            scope_kind: HashMap::new(),
            branch_children: HashMap::new(),
            header_calls: HashMap::new(),
        };

        // Do not inject implicit headers; only render actual source headers

        // Build normalized headers and parent relationships per scope
        for (scope, raw_list) in &self.raw_by_scope {
            if raw_list.is_empty() {
                continue;
            }
            let min_level = raw_list.iter().map(|r| r.original_level).min().unwrap_or(1);

            // Stack of (header_id, level)
            let mut stack: Vec<(String, u8)> = Vec::new();

            for raw in raw_list {
                let norm_level = raw
                    .original_level
                    .saturating_sub(min_level)
                    .saturating_add(1);
                let id = raw.id.clone();

                // Find markdown parent within scope
                let parent_id = loop {
                    if let Some((parent, plevel)) = stack.last() {
                        if *plevel < norm_level {
                            break Some(parent.clone());
                        } else {
                            stack.pop();
                        }
                    } else {
                        break None;
                    }
                };

                let header = RenderableHeader {
                    id: id.clone(),
                    title: raw.title.clone(),
                    level: norm_level,
                    span: raw.span.clone(),
                    scope: *scope,
                    parent_id,
                    next_id: None, // stub
                    label_kind: raw.label_kind,
                };

                let idx = index.headers.len();
                index.headers.push(header);
                index.by_scope.entry(*scope).or_default().push(idx);

                stack.push((id, norm_level));
            }
        }

        // Carry over nested edges collected during traversal
        index.nested_edges = self.nested_edges;
        // Compute scope roots as the first header per scope
        for (scope, idxs) in &index.by_scope {
            if let Some(first_idx) = idxs.first().copied() {
                let root_id = index.headers[first_idx].id.clone();
                index.scope_root_header.insert(*scope, root_id);
            }
        }
        // Expose scope kinds
        index.scope_kind = self.scope_kind;
        // Compute branch children mapping from nested_edges filtered by child being scope root
        let id_to_scope: HashMap<String, ScopeId> = index
            .headers
            .iter()
            .map(|h| (h.id.clone(), h.scope))
            .collect();
        let mut branch_pairs: Vec<(String, String)> = Vec::new();
        for (from, to) in &index.nested_edges {
            if let Some(scope) = id_to_scope.get(to) {
                if let Some(root_id) = index.scope_root_header.get(scope) {
                    if root_id == to {
                        branch_pairs.push((from.clone(), to.clone()));
                    }
                }
            }
        }
        for (from, child_root) in branch_pairs {
            index
                .branch_children
                .entry(from)
                .or_default()
                .push(child_root);
        }
        // Expose header -> call mapping
        index.header_calls = self.header_fn_calls;
        index
    }
}

/// Collect the top-level function application names for the given expression.
/// Only captures the outermost call(s) that structurally represent the expression,
/// ignoring nested calls within arguments or sub-expressions.
fn collect_top_level_calls(expr: &Expression) -> Vec<String> {
    match expr {
        // If the expression is a block, the top-level expression is inside it
        Expression::ExprBlock(block, _span) => {
            if let Some(inner) = &block.expr {
                collect_top_level_calls(inner)
            } else {
                Vec::new()
            }
        }
        // For an if-expression, the top-level construct is branching, not a direct call
        Expression::If(_cond, _then_expr, _else_expr, _span) => Vec::new(),
        // For a direct function application, capture its name
        Expression::App(app) => vec![app.name.name().to_string()],
        // For other expressions (values, identifiers, constructors, etc.), no top-level calls
        _ => Vec::new(),
    }
}
