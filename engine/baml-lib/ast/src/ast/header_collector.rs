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
    pub parent_id: Option<String>,
    /// Next element in control flow (stubbed)
    pub next_id: Option<String>,
    /// What kind of statement this header labels
    pub label_kind: HeaderLabelKind,
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
    /// Explicit mapping of container headers (e.g., if-labeled) to their child scope roots
    pub branch_children: HashMap<String, Vec<String>>, // parent_header_id -> [child_root_header_id]
    /// Mapping of header id -> names of functions called by the expression this header labels
    pub header_calls: HashMap<String, Vec<String>>, // header_id -> [callee_name]
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
    // Mapping for branches collected during traversal
    branch_children: HashMap<String, Vec<String>>, // parent -> [child_root]
    // Parent header (if any) to attach the next new scope's first header to (used for if branches)
    pending_branch_parent: Option<String>,
    // Mapping during collection: header id -> function names called by the labeled expression
    header_fn_calls: HashMap<String, Vec<String>>,
}

#[derive(Debug, Clone)]
struct RawHeader {
    id: String,
    title: String,
    original_level: u8,
    span: Span,
    // origin is tracked during traversal only; we don't need it after building
    label_kind: HeaderLabelKind,
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
            branch_children: HashMap::new(),
            pending_branch_parent: None,
            header_fn_calls: HashMap::new(),
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

    fn add_header(
        &mut self,
        title: String,
        level: u8,
        span: Span,
        is_final_expr: bool,
        label_kind: HeaderLabelKind,
    ) -> String {
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
                // Also record explicit branch child mapping when present
                if let Some(parent) = &self.pending_branch_parent {
                    self.branch_children
                        .entry(parent.clone())
                        .or_default()
                        .push(id.clone());
                }
                // Remember root for same-scope hierarchy
                self.scope_root_header.insert(scope, id.clone());
                self.scope_root_connected.insert(scope, false);
            }
            // Do not create same-scope edges here; nested_edges is for cross-scope relationships

            // Track last final header in scope
            if is_final_expr {
                self.last_final_in_scope.insert(scope, id.clone());
            }

            // Update last header in scope
            self.last_header_in_scope.insert(scope, id);
            // Return the last inserted header id
            return self
                .last_header_in_scope
                .get(&scope)
                .cloned()
                .expect("header id must exist");
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
                    let _ = self.add_header(
                        h.title.clone(),
                        h.level,
                        h.span.clone(),
                        false,
                        HeaderLabelKind::Function,
                    );
                }
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
                let _scope = self.push_scope();
                self.scope_is_top_level.insert(_scope, true);
                for h in &expr_fn.annotations {
                    let _ = self.add_header(
                        h.title.clone(),
                        h.level,
                        h.span.clone(),
                        false,
                        HeaderLabelKind::Function,
                    );
                }
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
            Expression::ExprBlock(block, _span) => {
                // Generic expression block scope
                if self.pending_next_scope_kind.is_none() {
                    self.pending_next_scope_kind = Some(ScopeKind::Generic);
                }
                self.visit_expression_block(block);
            }
            Expression::If(_cond, then_expr, else_expr, _span) => {
                println!("IF expression encountered; preparing THEN and ELSE scopes");
                // Attach the next new scopes (then/else) to the last header in the current scope
                let parent_for_branches = self
                    .current_scope()
                    .and_then(|s| self.last_header_in_scope.get(&s).cloned());

                self.pending_next_scope_kind = Some(ScopeKind::IfThen);
                self.pending_branch_parent = parent_for_branches.clone();
                println!(
                    "-- entering THEN branch (parent: {:?})",
                    parent_for_branches
                );
                self.visit_expression(then_expr);
                println!("-- exited THEN branch");
                self.pending_branch_parent = None;

                if let Some(else_expr) = else_expr {
                    self.pending_next_scope_kind = Some(ScopeKind::IfElse);
                    println!(
                        "-- entering ELSE branch (parent: {:?})",
                        parent_for_branches
                    );
                    self.pending_branch_parent = parent_for_branches;
                    self.visit_expression(else_expr);
                    println!("-- exited ELSE branch");
                    self.pending_branch_parent = None;
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
        println!(
            "ENTER BLOCK scope {:?} with {} stmts",
            _scope,
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
                    let mut stmt_header_ids: Vec<String> = Vec::new();
                    for h in &let_stmt.annotations {
                        // Determine label kind by the expression bound by this let
                        let label_kind = match &let_stmt.expr {
                            Expression::If(_, _, _, _) => HeaderLabelKind::If,
                            _ => HeaderLabelKind::Expression,
                        };
                        // Statement labels live in the current scope
                        let hid = self.add_header(
                            h.title.clone(),
                            h.level,
                            h.span.clone(),
                            false,
                            label_kind,
                        );
                        if !hid.is_empty() {
                            stmt_header_ids.push(hid);
                        }
                    }
                    // Collect top-level calls for the statement expression and attribute to all headers
                    let top_calls = collect_top_level_calls(&let_stmt.expr);
                    if !top_calls.is_empty() {
                        for hid in stmt_header_ids {
                            self.header_fn_calls
                                .entry(hid)
                                .or_default()
                                .extend(top_calls.clone());
                        }
                    }
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
                    for h in &for_stmt.annotations {
                        let _ = self.add_header(
                            h.title.clone(),
                            h.level,
                            h.span.clone(),
                            false,
                            HeaderLabelKind::For,
                        );
                    }
                    // Iterate expression evaluated in current scope
                    self.visit_expression(&for_stmt.iterator);
                    // Now visit the body in its own scope (no prefixed headers)
                    self.pending_next_scope_kind = Some(ScopeKind::ForBody);
                    self.visit_expression_block(&for_stmt.body);
                }
            }
        }

        // Headers that apply to the final expression belong to this scope and come last
        let mut expr_header_ids: Vec<String> = Vec::new();
        for h in &block.expr_headers {
            let label_kind = match block.expr.as_ref() {
                Expression::If(_, _, _, _) => HeaderLabelKind::If,
                _ => HeaderLabelKind::Expression,
            };
            let hid = self.add_header(h.title.clone(), h.level, h.span.clone(), true, label_kind);
            if !hid.is_empty() {
                expr_header_ids.push(hid);
            }
        }

        // Final expr
        self.visit_expression(&block.expr);

        // Attribute top-level calls of the final expression to its headers
        if !expr_header_ids.is_empty() {
            let top_calls = collect_top_level_calls(&block.expr);
            if !top_calls.is_empty() {
                for hid in expr_header_ids {
                    self.header_fn_calls
                        .entry(hid)
                        .or_default()
                        .extend(top_calls.clone());
                }
            }
        }

        // Default header fallback for blocks without any headers
        let kind_for_scope = self
            .scope_kind
            .get(&_scope)
            .copied()
            .unwrap_or(ScopeKind::Generic);
        // Do not inject synthetic headers for if-branches.
        // These create confusing extra nodes like "if statement 1" when an ELSE branch
        // has no explicit headers. We only auto-inject for generic/loop scopes.
        if !matches!(kind_for_scope, ScopeKind::IfThen | ScopeKind::IfElse) {
            let (type_label, default_label_kind) = match kind_for_scope {
                ScopeKind::ForBody => ("for loop", HeaderLabelKind::For),
                ScopeKind::TopLevel => ("block", HeaderLabelKind::None),
                ScopeKind::Generic => ("block", HeaderLabelKind::None),
                // The if branches are handled above with the guard.
                ScopeKind::IfThen | ScopeKind::IfElse => unreachable!(),
            };
            let span_for_default = block.expr.span().clone();
            self.ensure_default_header_current_scope(
                type_label,
                span_for_default,
                None,
                default_label_kind,
            );
        }

        self.pop_scope();
    }

    // Helper removed; for-loop annotations are now added to outer scope

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
        // Expose scope roots
        index.scope_root_header = self.scope_root_header;
        // Expose scope kinds
        index.scope_kind = self.scope_kind;
        // Expose explicit branch mapping
        index.branch_children = self.branch_children;
        // Expose header -> call mapping
        index.header_calls = self.header_fn_calls;
        index
    }
}

#[cfg(test)]
mod tests {
    use internal_baml_diagnostics::SourceFile;

    use super::*;
    use crate::{ast::Span, parser::parse};

    fn collect(src: &str, name: &str) -> HeaderIndex {
        let path = std::path::PathBuf::from(format!("/virtual/{name}.baml"));
        let source = SourceFile::new_allocated(path.clone(), src.into());
        let (ast, diags) = parse(std::path::Path::new("."), &source).unwrap();
        assert!(
            !diags.has_errors(),
            "unexpected parse errors: {}",
            diags.to_pretty_string()
        );
        HeaderCollector::collect(&ast)
    }

    #[test]
    fn header_collector_basic_if() {
        let src = r#"
fn BasicIf() -> string {
    # This is a named statement
    let z = if true {
        # Then Statement
        12
    } else {
        # Else Statement
        13
    };
    z
}
"#;
        let idx = collect(src, "basic_if");
        let titles: Vec<_> = idx.headers.iter().map(|h| h.title.as_str()).collect();
        assert!(titles.contains(&"This is a named statement"));
        assert!(titles.contains(&"Then Statement"));
        assert!(titles.contains(&"Else Statement"));
        let parent = idx
            .headers
            .iter()
            .find(|h| h.title == "This is a named statement")
            .unwrap();
        let kids = idx
            .branch_children
            .get(&parent.id)
            .expect("branch children");
        assert_eq!(kids.len(), 2);
    }

    #[test]
    fn header_collector_nested_scopes_if() {
        let src = r#"
fn NestedScopes() -> string {
    # Top Level
    let x = "hello";
    ## First Section
    let z = if x == "hello" {
        ### Inside If Block
        let y = "world";
        x + " " + y
    } else {
        ### Inside Else Block
        "goodbye"
    };
    z
}
"#;
        let idx = collect(src, "nested_scopes");
        let titles: Vec<_> = idx.headers.iter().map(|h| h.title.as_str()).collect();
        for t in [
            "Top Level",
            "First Section",
            "Inside If Block",
            "Inside Else Block",
        ] {
            assert!(titles.contains(&t), "missing header {t}");
        }
        let parent = idx
            .headers
            .iter()
            .find(|h| h.title == "First Section")
            .unwrap();
        let kids = idx
            .branch_children
            .get(&parent.id)
            .expect("branch children");
        assert_eq!(kids.len(), 2);
    }

    #[test]
    fn header_collector_complex_workflow_like_case() {
        // Reproduces the scenario where a top-level header precedes a let-bound if expression,
        // followed by additional headers/steps after the conditional. We expect:
        // - The post-branch headers (Generate Summary, Create Final Output) to be present
        // - No synthetic "if statement N" headers to be injected for the else branch
        let src = r#"
fn ProcessVideo(transcript: string, title: string?) -> string {
    # Main Processing Pipeline

    let computed_title = if !title {
        ## Generate Title
        GenerateTitle(transcript)
    } else {
        title
    };

    ## Generate Summary
    let summary = SummarizeVideo(transcript);

    ## Create Final Output
    computed_title + ": " + summary
}
"#;
        let idx = collect(src, "complex_workflow_like");
        let titles: Vec<_> = idx.headers.iter().map(|h| h.title.as_str()).collect();
        for t in [
            "Main Processing Pipeline",
            "Generate Title",
            "Generate Summary",
            "Create Final Output",
        ] {
            assert!(
                titles.contains(&t),
                "expected header '{}' to be present; got {:?}",
                t,
                titles
            );
        }

        // Ensure we did not synthesize a fake else header like "if statement 1"
        assert!(
            !titles.iter().any(|t| t.starts_with("if statement")),
            "unexpected synthetic if-statement header present: {:?}",
            titles
        );

        // Branch children of the top narrative header should include the THEN branch only
        // (ELSE has no explicit headers).
        let main = idx
            .headers
            .iter()
            .find(|h| h.title == "Main Processing Pipeline")
            .expect("missing Main Processing Pipeline header");
        if let Some(kids) = idx.branch_children.get(&main.id) {
            assert_eq!(kids.len(), 1, "expected only THEN child scope root");
        }
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
            branch_children: HashMap::new(),
            pending_branch_parent: None,
            header_fn_calls: HashMap::new(),
        }
    }
}

/// Collect the top-level function application names for the given expression.
/// Only captures the outermost call(s) that structurally represent the expression,
/// ignoring nested calls within arguments or sub-expressions.
fn collect_top_level_calls(expr: &Expression) -> Vec<String> {
    match expr {
        // If the expression is a block, the top-level expression is inside it
        Expression::ExprBlock(block, _span) => collect_top_level_calls(&block.expr),
        // For an if-expression, the top-level construct is branching, not a direct call
        Expression::If(_cond, _then_expr, _else_expr, _span) => Vec::new(),
        // For a direct function application, capture its name
        Expression::App(app) => vec![app.name.name().to_string()],
        // For other expressions (values, identifiers, constructors, etc.), no top-level calls
        _ => Vec::new(),
    }
}
