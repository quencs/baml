use std::collections::HashMap;

use internal_baml_diagnostics::Span;

use super::{
    Ast, ExprFn, Expression, ExpressionBlock, Field, Header, Stmt, Top, TopId, ValExpId,
    ValueExprBlock, WithName, WithSpan,
};

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
    pub parent_id: Option<String>,
    /// Next element in control flow (stubbed)
    pub next_id: Option<String>,
}

/// Collected index of headers with simple querying APIs
#[derive(Debug, Default, Clone)]
pub struct HeaderIndex {
    pub headers: Vec<RenderableHeader>,
    by_scope: HashMap<ScopeId, Vec<usize>>, // header indexes in source order
    /// Cross-scope/navigational edges between headers (e.g., control-flow nesting)
    pub nested_edges: Vec<(String, String)>, // (from_id, to_id)
    /// Root header id per scope (first header encountered in the scope)
    pub scope_root_header: HashMap<ScopeId, String>,
    /// Kind classification per scope
    pub scope_kind: HashMap<ScopeId, ScopeKind>,
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
    // Track last header produced in each scope (by id)
    last_header_in_scope: HashMap<ScopeId, String>,
    // When entering a new scope, we record the parent header id from the enclosing scope
    // so that the first header in the new scope can be connected via a nested edge.
    pending_parent_for_scope: HashMap<ScopeId, Option<String>>,
    // Accumulated nested edges (from_id, to_id)
    nested_edges: Vec<(String, String)>,
    // Root header of a scope (first header encountered in that scope)
    scope_root_header: HashMap<ScopeId, String>,
    // Whether we already connected root -> first child within the same scope
    scope_root_connected: HashMap<ScopeId, bool>,
    // Track last header that was marked as final expression header within the scope
    last_final_in_scope: HashMap<ScopeId, String>,
    // Whether a scope is a top-level (direct child of Top)
    scope_is_top_level: HashMap<ScopeId, bool>,
    // Classification per scope
    scope_kind: HashMap<ScopeId, ScopeKind>,
    // Pending classification for the next scope to be pushed
    pending_next_scope_kind: Option<ScopeKind>,
    // Simple counters per type label for auto-generated headers
    auto_counters: HashMap<String, u32>,
}

#[derive(Debug, Clone)]
struct RawHeader {
    id: String,
    title: String,
    original_level: u8,
    span: Span,
    // origin is tracked during traversal only; we don't need it after building
}

impl HeaderCollector {
    pub fn collect(ast: &Ast) -> HeaderIndex {
        let mut c = Self {
            scope_counter: 0,
            scope_stack: Vec::new(),
            raw_by_scope: HashMap::new(),
            last_header_in_scope: HashMap::new(),
            pending_parent_for_scope: HashMap::new(),
            nested_edges: Vec::new(),
            scope_root_header: HashMap::new(),
            scope_root_connected: HashMap::new(),
            last_final_in_scope: HashMap::new(),
            scope_is_top_level: HashMap::new(),
            scope_kind: HashMap::new(),
            pending_next_scope_kind: None,
            auto_counters: HashMap::new(),
        };
        c.visit_ast(ast);
        c.build_index()
    }

    fn push_scope(&mut self) -> ScopeId {
        // Determine potential parent header from the current scope before pushing
        let parent_header_id = self
            .current_scope()
            .and_then(|parent_scope| self.last_header_in_scope.get(&parent_scope).cloned());

        self.scope_counter += 1;
        let id = ScopeId(self.scope_counter);
        self.scope_stack.push(id);
        // Record pending parent for this new scope
        self.pending_parent_for_scope.insert(id, parent_header_id);
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

    fn add_header(&mut self, title: String, level: u8, span: Span, is_final_expr: bool) {
        if let Some(scope) = self.current_scope() {
            let entry = self.raw_by_scope.entry(scope).or_default();
            let id = format!(
                "{}:{}-{}:{}:{}",
                title,
                span.file.path(),
                span.start,
                span.end,
                scope.0
            );
            let is_first_in_scope = entry.is_empty();
            entry.push(RawHeader {
                id: id.clone(),
                title: title.clone(),
                original_level: level,
                span: span.clone(),
            });

            // If first header in this scope and we have a pending parent, add nested edge
            if is_first_in_scope {
                if let Some(Some(parent_id)) = self.pending_parent_for_scope.get(&scope) {
                    // Only connect parent -> child for nested scopes, not for top-level scopes
                    let is_top = self
                        .scope_is_top_level
                        .get(&scope)
                        .copied()
                        .unwrap_or(false);
                    if !is_top {
                        self.nested_edges.push((parent_id.clone(), id.clone()));
                    }
                }
                // Remember root for same-scope hierarchy
                self.scope_root_header.insert(scope, id.clone());
                self.scope_root_connected.insert(scope, false);
            }
            // If this is the first non-root header, connect root -> this within same scope
            if !is_first_in_scope {
                if let Some(root) = self.scope_root_header.get(&scope) {
                    let already = self
                        .scope_root_connected
                        .get(&scope)
                        .copied()
                        .unwrap_or(false);
                    if !already && *root != id {
                        // For top-level scope we DO want root -> next nested edge; for inner scopes we skip
                        let is_top = self
                            .scope_is_top_level
                            .get(&scope)
                            .copied()
                            .unwrap_or(false);
                        if is_top {
                            self.nested_edges.push((root.clone(), id.clone()));
                        }
                        self.scope_root_connected.insert(scope, true);
                    }
                }
            }

            // Track last final header in scope, and propagate to nearest top-level ancestor
            if is_final_expr {
                self.last_final_in_scope.insert(scope, id.clone());
                // Also set last final for nearest top-level ancestor so we can draw
                // a nested edge root(top-level) -> final header, matching reference
                if let Some(&ancestor_scope) = self
                    .scope_stack
                    .iter()
                    .rev()
                    .find(|s| self.scope_is_top_level.get(*s).copied().unwrap_or(false))
                {
                    self.last_final_in_scope.insert(ancestor_scope, id.clone());
                }
            }

            // Update last header in scope
            self.last_header_in_scope.insert(scope, id);
        }
    }

    fn ensure_default_header_current_scope(
        &mut self,
        type_label: &str,
        span: Span,
        override_title: Option<String>,
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
        self.add_header(title, 1, span, false);
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
                let _scope = self.push_scope();
                self.scope_is_top_level.insert(_scope, true);
                // Block-level headers label this scope
                for h in &block.annotations {
                    // Function-level headers live in this scope
                    self.add_header(h.title.clone(), h.level, h.span.clone(), false);
                }
                // Default header logic for top-level blocks
                if block.annotations.is_empty() {
                    let type_label = block.get_type();
                    let override_title = if type_label == "function" {
                        Some(block.name().to_string())
                    } else {
                        None
                    };
                    self.ensure_default_header_current_scope(
                        type_label,
                        block.span().clone(),
                        override_title,
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
                let _scope = self.push_scope();
                self.scope_is_top_level.insert(_scope, true);
                for h in &expr_fn.annotations {
                    self.add_header(h.title.clone(), h.level, h.span.clone(), false);
                }
                if expr_fn.annotations.is_empty() {
                    self.ensure_default_header_current_scope(
                        "function",
                        expr_fn.span.clone(),
                        Some(expr_fn.name.name().to_string()),
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
            Expression::ExprBlock(block, _span) => {
                // Generic expression block scope
                if self.pending_next_scope_kind.is_none() {
                    self.pending_next_scope_kind = Some(ScopeKind::Generic);
                }
                self.visit_expression_block(block);
            }
            Expression::If(_cond, then_expr, else_expr, span) => {
                // Ensure a container header in the current (outer) scope for this if
                let label = "if statement";
                let counter = self
                    .auto_counters
                    .entry(label.to_string())
                    .and_modify(|c| *c += 1)
                    .or_insert(1);
                let title = format!("{} {}", label, *counter);
                self.add_header(title, 1, span.clone(), false);
                self.pending_next_scope_kind = Some(ScopeKind::IfThen);
                self.visit_expression(then_expr);
                if let Some(else_expr) = else_expr {
                    self.pending_next_scope_kind = Some(ScopeKind::IfElse);
                    self.visit_expression(else_expr);
                }
            }
            Expression::Lambda(_args, body, _) => {
                // Treat lambda body like a generic scope, but do not auto-inject headers
                self.pending_next_scope_kind = Some(ScopeKind::Generic);
                // Lambda's body span isn't tracked directly; skip span assignment here
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
            Expression::Not(e, _) => self.visit_expression(e),
            _ => {}
        }
    }

    fn visit_expression_block(&mut self, block: &ExpressionBlock) {
        let _scope = self.push_scope();

        // Visit statements first (preserve source order for MD parenting)
        for stmt in &block.stmts {
            match stmt {
                Stmt::Let(let_stmt) => {
                    for h in &let_stmt.annotations {
                        // Statement labels live in the current scope
                        self.add_header(h.title.clone(), h.level, h.span.clone(), false);
                    }
                    self.visit_expression(&let_stmt.expr);
                }
                Stmt::ForLoop(for_stmt) => {
                    // Record for-loop annotation headers in the current (outer) scope
                    for h in &for_stmt.annotations {
                        self.add_header(h.title.clone(), h.level, h.span.clone(), false);
                    }
                    // Iterate expression evaluated in current scope
                    self.visit_expression(&for_stmt.iterator);
                    // Ensure a default for-loop header in the outer scope if none was provided
                    if for_stmt.annotations.is_empty() {
                        let label = "for loop";
                        let counter = self
                            .auto_counters
                            .entry(label.to_string())
                            .and_modify(|c| *c += 1)
                            .or_insert(1);
                        // Use level 2 so it nests under the current root header (e.g., function)
                        let title = format!("{} {}", label, *counter);
                        self.add_header(title, 2, for_stmt.span.clone(), false);
                    }
                    // Now visit the body in its own scope (no prefixed headers)
                    self.pending_next_scope_kind = Some(ScopeKind::ForBody);
                    self.visit_expression_block(&for_stmt.body);
                }
            }
        }

        // Headers that apply to the final expression belong to this scope and come last
        for h in &block.expr_headers {
            self.add_header(h.title.clone(), h.level, h.span.clone(), true);
        }

        // Final expr
        self.visit_expression(&block.expr);

        // After visiting this block, if no headers were added for this scope, inject defaults
        let kind_for_scope = self
            .scope_kind
            .get(&_scope)
            .copied()
            .unwrap_or(ScopeKind::Generic);
        let type_label = match kind_for_scope {
            ScopeKind::ForBody => "for loop",
            ScopeKind::IfThen | ScopeKind::IfElse => "if statement",
            ScopeKind::TopLevel => "block",
            ScopeKind::Generic => "block",
        };
        let span_for_default = block.expr.span().clone();
        self.ensure_default_header_current_scope(type_label, span_for_default, None);

        self.pop_scope();
    }

    // Helper removed; for-loop annotations are now added to outer scope

    fn build_index(mut self) -> HeaderIndex {
        let mut index = HeaderIndex {
            headers: Vec::new(),
            by_scope: HashMap::new(),
            nested_edges: Vec::new(),
            scope_root_header: HashMap::new(),
            scope_kind: HashMap::new(),
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
                };

                let idx = index.headers.len();
                index.headers.push(header);
                index.by_scope.entry(*scope).or_default().push(idx);

                stack.push((id, norm_level));
            }
        }

        // Add one extra edge root -> last final header only for top-level scopes,
        // avoiding self-loops and duplicates (the same edge may have been created when
        // the child scope's first header connected to the parent's last header).
        for (scope, root_id) in &self.scope_root_header {
            let is_top = self.scope_is_top_level.get(scope).copied().unwrap_or(false);
            if !is_top {
                continue;
            }
            if let Some(final_id) = self.last_final_in_scope.get(scope) {
                if final_id != root_id {
                    let exists = self
                        .nested_edges
                        .iter()
                        .any(|(a, b)| a == root_id && b == final_id);
                    if !exists {
                        self.nested_edges.push((root_id.clone(), final_id.clone()));
                    }
                }
            }
        }
        // Carry over nested edges collected during traversal
        index.nested_edges = self.nested_edges;
        // Expose scope roots
        index.scope_root_header = self.scope_root_header;
        // Expose scope kinds
        index.scope_kind = self.scope_kind;
        index
    }
}

impl Default for HeaderCollector {
    fn default() -> Self {
        Self {
            scope_counter: 0,
            scope_stack: Vec::new(),
            raw_by_scope: HashMap::new(),
            last_header_in_scope: HashMap::new(),
            pending_parent_for_scope: HashMap::new(),
            nested_edges: Vec::new(),
            scope_root_header: HashMap::new(),
            scope_root_connected: HashMap::new(),
            last_final_in_scope: HashMap::new(),
            scope_is_top_level: HashMap::new(),
            scope_kind: HashMap::new(),
            pending_next_scope_kind: None,
            auto_counters: HashMap::new(),
        }
    }
}
