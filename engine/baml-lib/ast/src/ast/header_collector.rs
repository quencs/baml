use std::{collections::HashMap, sync::Arc};

use internal_baml_diagnostics::Span;

use super::{
    Ast, ExprFn, Expression, ExpressionBlock, Field, Header, Stmt, Top, ValueExprBlock, WithName,
    WithSpan,
};

/// Reference to an AST node for tracking structural hierarchy
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ASTNodeRef {
    Function(String),
    ExprFunction(String),
    LetStatement(String),
    ForLoopStatement(String),
    ExpressionBlock,
    IfExpression,
    LambdaExpression,
}

/// Represents the contextual location where a header appears in the AST
/// Uses existing AST enums to avoid duplication
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ASTContext {
    /// Header at the top-level (function, class, enum, etc.)
    TopLevel(String), // The type from Top::get_type()
    /// Header within a statement
    Statement, // Uses Stmt enum internally
    /// Header within an expression block (applies to final expression)
    ExpressionBlockFinal,
}

/// Represents a header with its full contextual information
#[derive(Debug, Clone)]
pub struct ContextualHeader {
    /// The original header data
    pub header: Arc<Header>,
    /// The AST context where this header appears
    pub ast_context: ASTContext,
    /// The AST node that contains this header (for AST hierarchy)
    pub ast_parent: Option<ASTNodeRef>,
    /// AST children (other AST nodes contained within this header's scope)
    pub ast_children: Vec<ASTNodeRef>,
    /// The path through the AST to reach this header
    pub ast_path: Vec<String>,
    /// Header-level parent (based on markdown hierarchy within the same block)
    pub header_parent: Option<Arc<ContextualHeader>>,
    /// Header-level children (based on markdown hierarchy within the same block)
    pub header_children: Vec<Arc<ContextualHeader>>,
    /// Span information for the header
    pub span: Span,
    /// The scope ID for grouping headers that should have markdown hierarchy together
    pub scope_id: String,
}

/// A tree structure representing the hierarchical relationships between headers
#[derive(Debug, Clone)]
pub struct HeaderTree {
    /// Root headers (those with no parent in the hierarchy)
    pub roots: Vec<Arc<ContextualHeader>>,
    /// All headers indexed by their unique identifier
    pub headers_by_id: HashMap<String, Arc<ContextualHeader>>,
    /// Headers grouped by their context type
    pub headers_by_context: HashMap<ASTContext, Vec<Arc<ContextualHeader>>>,
    /// Headers in the order they appear in the source
    pub headers_in_order: Vec<Arc<ContextualHeader>>,
}

/// Configuration for header collection behavior
#[derive(Debug, Clone)]
pub struct HeaderCollectorConfig {
    /// Whether to preserve hierarchy relationships based on header levels
    pub preserve_hierarchy: bool,
    /// Whether to include context information
    pub include_context: bool,
    /// Whether to track AST paths
    pub track_ast_paths: bool,
}

impl Default for HeaderCollectorConfig {
    fn default() -> Self {
        Self {
            preserve_hierarchy: true,
            include_context: true,
            track_ast_paths: true,
        }
    }
}

/// The main header collector that walks the AST and extracts headers
#[derive(Debug)]
pub struct HeaderCollector {
    /// Configuration for collection behavior
    config: HeaderCollectorConfig,
    /// Current AST path being traversed
    current_path: Vec<String>,
    /// Current context stack
    context_stack: Vec<ASTContext>,
    /// Current AST node stack for tracking AST hierarchy
    ast_node_stack: Vec<ASTNodeRef>,
    /// Scope ID counter for generating unique scope identifiers
    scope_counter: usize,
    /// Stack of scope IDs corresponding to ExpressionBlocks that can contain headers
    scope_stack: Vec<String>,
    /// Collected headers
    collected_headers: Vec<Arc<ContextualHeader>>,
}

impl HeaderCollector {
    /// Create a new header collector with default configuration
    pub fn new() -> Self {
        Self::new_with_config(HeaderCollectorConfig::default())
    }

    /// Create a new header collector with custom configuration
    pub fn new_with_config(config: HeaderCollectorConfig) -> Self {
        Self {
            config,
            current_path: Vec::new(),
            context_stack: Vec::new(),
            ast_node_stack: Vec::new(),
            scope_counter: 0,
            scope_stack: Vec::new(),
            collected_headers: Vec::new(),
        }
    }

    /// Collect all headers from an AST into a structured tree
    pub fn collect_headers(ast: &Ast) -> HeaderTree {
        Self::collect_headers_with_config(ast, HeaderCollectorConfig::default())
    }

    /// Collect all headers from an AST with custom configuration
    pub fn collect_headers_with_config(ast: &Ast, config: HeaderCollectorConfig) -> HeaderTree {
        let mut collector = Self::new_with_config(config);
        collector.visit_ast(ast);
        collector.build_header_tree()
    }

    /// Add a path component to the current AST path
    fn push_path(&mut self, component: String) {
        if self.config.track_ast_paths {
            self.current_path.push(component);
        }
    }

    /// Remove the last path component
    fn pop_path(&mut self) {
        if self.config.track_ast_paths {
            self.current_path.pop();
        }
    }

    /// Push a new context onto the stack
    fn push_context(&mut self, context: ASTContext) {
        if self.config.include_context {
            self.context_stack.push(context);
        }
    }

    /// Push a new AST node onto the stack
    fn push_ast_node(&mut self, node_ref: ASTNodeRef) {
        self.ast_node_stack.push(node_ref);
    }

    /// Pop the last AST node from the stack
    fn pop_ast_node(&mut self) {
        self.ast_node_stack.pop();
    }

    /// Enter a new scope (called when visiting ExpressionBlocks that can contain headers)
    fn enter_scope(&mut self) {
        self.scope_counter += 1;
        let scope_id = format!("scope_{}", self.scope_counter);
        self.scope_stack.push(scope_id);
    }

    /// Exit the current scope
    fn exit_scope(&mut self) {
        self.scope_stack.pop();
    }

    /// Get the current scope ID for header grouping
    fn current_scope_id(&self) -> String {
        self.scope_stack
            .last()
            .cloned()
            .unwrap_or_else(|| "root_scope".to_string())
    }

    /// Pop the last context from the stack
    fn pop_context(&mut self) {
        if self.config.include_context {
            self.context_stack.pop();
        }
    }

    /// Collect headers from a vector of header Arc references
    fn collect_headers_from_vec(&mut self, headers: &[Arc<Header>], context: ASTContext) {
        for header in headers {
            self.collect_single_header(header.clone(), context.clone());
        }
    }

    /// Collect a single header and add it to the collection
    fn collect_single_header(&mut self, header: Arc<Header>, context: ASTContext) {
        let ast_parent = self.ast_node_stack.last().cloned();

        // Debug print to see what headers are being collected (disabled)
        // println!("COLLECTOR: Found header '{}' (Level: {}) at path: {} | Context: {:?} | Parent: {:?}",
        //          header.title, header.level, self.current_path.join(" -> "), context, ast_parent);

        // Get the current scope ID for grouping headers that should have markdown hierarchy together
        let scope_id = self.current_scope_id();

        let contextual_header = Arc::new(ContextualHeader {
            header: header.clone(),
            ast_context: context,
            ast_parent,
            ast_children: Vec::new(), // Will be populated during tree building
            ast_path: self.current_path.clone(),
            header_parent: None, // Will be set when building the header hierarchy
            header_children: Vec::new(),
            span: header.span.clone(),
            scope_id,
        });

        self.collected_headers.push(contextual_header);
    }

    /// Build the final header tree from collected headers
    fn build_header_tree(self) -> HeaderTree {
        let mut headers_by_id = HashMap::new();
        let mut headers_by_context: HashMap<ASTContext, Vec<Arc<ContextualHeader>>> =
            HashMap::new();
        let mut headers_in_order = Vec::new();

        // First pass: index all headers and group by context
        for header in &self.collected_headers {
            // Create unique ID using span position and scope to avoid collisions
            // when headers have the same title/level in different AST scopes
            let id = format!(
                "{}_{}_{}_{}_{}_{}",
                header.ast_path.len(),
                header.header.level,
                header.header.title,
                header.span.start,
                header.span.end,
                header.scope_id
            );
            headers_by_id.insert(id, header.clone());

            headers_by_context
                .entry(header.ast_context.clone())
                .or_default()
                .push(header.clone());

            headers_in_order.push(header.clone());
        }

        // Second pass: build both hierarchy relationships if enabled
        let roots = if self.config.preserve_hierarchy {
            // Return headers that are direct children of functions as roots
            // TODO: Implement proper header-level hierarchy with RefCell
            self.collected_headers
                .iter()
                .filter(|h| {
                    // Headers are roots if they're direct children of functions
                    matches!(
                        h.ast_parent,
                        Some(ASTNodeRef::Function(_) | ASTNodeRef::ExprFunction(_))
                    )
                })
                .cloned()
                .collect()
        } else {
            // If not preserving hierarchy, all headers are roots
            self.collected_headers
        };

        HeaderTree {
            roots,
            headers_by_id,
            headers_by_context,
            headers_in_order,
        }
    }

    /// Build header-level hierarchy based on markdown-style levels within blocks
    #[allow(dead_code)]
    fn build_header_level_hierarchy(
        &self,
        headers: &[Arc<ContextualHeader>],
    ) -> Vec<Arc<ContextualHeader>> {
        // Group headers by their AST context to build hierarchy within each block
        let mut headers_by_block: HashMap<String, Vec<Arc<ContextualHeader>>> = HashMap::new();

        for header in headers {
            let block_key = header.ast_path.join("/");
            headers_by_block
                .entry(block_key)
                .or_default()
                .push(header.clone());
        }

        let mut result_headers = Vec::new();

        // Build header hierarchy within each block
        for (_, block_headers) in headers_by_block {
            let mut sorted_headers = block_headers;
            // Sort by position in the source (using span start)
            sorted_headers.sort_by_key(|h| h.span.start);

            let mut stack: Vec<Arc<ContextualHeader>> = Vec::new();

            for header in sorted_headers {
                let current_level = header.header.level;

                // Pop from stack until we find a header with a lower level (potential parent)
                while let Some(last) = stack.last() {
                    if last.header.level < current_level {
                        break;
                    }
                    stack.pop();
                }

                // Create a new header with updated header hierarchy relationships
                let mut new_header = header.as_ref().clone();

                if let Some(parent) = stack.last() {
                    new_header.header_parent = Some(parent.clone());
                    // Note: In a full implementation, would need to update parent's children
                }

                let new_header_arc = Arc::new(new_header);
                stack.push(new_header_arc.clone());
                result_headers.push(new_header_arc);
            }
        }

        result_headers
    }

    /// Build AST hierarchy based on structural containment
    #[allow(dead_code)]
    fn build_ast_hierarchy(
        &self,
        headers: Vec<Arc<ContextualHeader>>,
    ) -> Vec<Arc<ContextualHeader>> {
        // Build a proper tree structure based on AST parent relationships
        let mut headers_by_parent: HashMap<Option<ASTNodeRef>, Vec<Arc<ContextualHeader>>> =
            HashMap::new();

        // Group headers by their AST parent
        for header in headers {
            headers_by_parent
                .entry(header.ast_parent.clone())
                .or_default()
                .push(header);
        }

        // Return headers that have no AST parent (top-level headers)
        headers_by_parent.remove(&None).unwrap_or_default()
    }

    /// Visit the root AST node
    fn visit_ast(&mut self, ast: &Ast) {
        self.push_path("ast".to_string());

        for (idx, top) in ast.tops.iter().enumerate() {
            self.push_path(format!("top[{}]", idx));
            self.visit_top(top);
            self.pop_path();
        }

        self.pop_path();
    }

    /// Visit a top-level AST node
    fn visit_top(&mut self, top: &Top) {
        match top {
            Top::Function(value_expr) => {
                self.push_path(format!("function[{}]", value_expr.name()));
                let context = ASTContext::TopLevel("function".to_string());
                let node_ref = ASTNodeRef::Function(value_expr.name().to_string());
                self.push_context(context.clone());
                self.push_ast_node(node_ref);
                self.visit_value_expression_block(value_expr);
                self.pop_context();
                self.pop_ast_node();
                self.pop_path();
            }
            Top::ExprFn(expr_fn) => {
                self.push_path(format!("expr_fn[{}]", expr_fn.name.name()));
                let context = ASTContext::TopLevel("expr_function".to_string());
                let node_ref = ASTNodeRef::ExprFunction(expr_fn.name.name().to_string());
                self.push_context(context.clone());
                self.push_ast_node(node_ref);
                self.visit_expr_fn(expr_fn);
                self.pop_context();
                self.pop_ast_node();
                self.pop_path();
            }
            Top::Client(value_expr)
            | Top::Generator(value_expr)
            | Top::TestCase(value_expr)
            | Top::RetryPolicy(value_expr) => {
                self.push_path(format!("value_expr[{}]", value_expr.name()));
                self.visit_value_expression_block(value_expr);
                self.pop_path();
            }
            _ => {
                // Other top-level nodes don't typically have headers
            }
        }
    }

    /// Visit a value expression block (function, client, etc.)
    fn visit_value_expression_block(&mut self, value_expr: &ValueExprBlock) {
        // Collect annotations (headers) at the function level
        if !value_expr.annotations.is_empty() {
            let context = ASTContext::TopLevel("function".to_string());
            self.collect_headers_from_vec(&value_expr.annotations, context);
        }

        // Visit fields (expressions)
        for (idx, field) in value_expr.fields.iter().enumerate() {
            self.push_path(format!("field[{}]", idx));
            self.visit_field_expression(field);
            self.pop_path();
        }
    }

    /// Visit an expression function
    fn visit_expr_fn(&mut self, expr_fn: &ExprFn) {
        // Collect annotations (headers) at the function level
        if !expr_fn.annotations.is_empty() {
            let context = ASTContext::TopLevel("expr_function".to_string());
            self.collect_headers_from_vec(&expr_fn.annotations, context);
        }

        // Visit the body expression block
        self.push_path("body".to_string());
        self.visit_expression_block(&expr_fn.body);
        self.pop_path();
    }

    /// Visit a field with Expression
    fn visit_field_expression(&mut self, field: &Field<Expression>) {
        // Visit the expression if present
        if let Some(expr) = &field.expr {
            self.push_path(format!("expr[{}]", field.name.name()));
            self.visit_expression(expr);
            self.pop_path();
        }
    }

    /// Visit an expression
    fn visit_expression(&mut self, expr: &Expression) {
        match expr {
            Expression::ExprBlock(block, _) => {
                self.push_path("expr_block".to_string());
                self.visit_expression_block(block);
                self.pop_path();
            }
            Expression::If(_cond, then_expr, else_expr, _) => {
                self.push_path("if_then".to_string());
                self.visit_expression(then_expr);
                self.pop_path();

                if let Some(else_expr) = else_expr {
                    self.push_path("if_else".to_string());
                    self.visit_expression(else_expr);
                    self.pop_path();
                }
            }
            Expression::Lambda(_args, body, _) => {
                self.push_path("lambda_body".to_string());
                self.visit_expression_block(body);
                self.pop_path();
            }
            Expression::Array(exprs, _) => {
                for (idx, expr) in exprs.iter().enumerate() {
                    self.push_path(format!("array[{}]", idx));
                    self.visit_expression(expr);
                    self.pop_path();
                }
            }
            Expression::Map(map, _) => {
                for (idx, (key_expr, value_expr)) in map.iter().enumerate() {
                    self.push_path(format!("map_key[{}]", idx));
                    self.visit_expression(key_expr);
                    self.pop_path();

                    self.push_path(format!("map_value[{}]", idx));
                    self.visit_expression(value_expr);
                    self.pop_path();
                }
            }
            Expression::ClassConstructor(constructor, _) => {
                for (idx, field) in constructor.fields.iter().enumerate() {
                    self.push_path(format!("constructor_field[{}]", idx));
                    match field {
                        super::ClassConstructorField::Named(_name, expr) => {
                            self.visit_expression(expr);
                        }
                        super::ClassConstructorField::Spread(expr) => {
                            self.visit_expression(expr);
                        }
                    }
                    self.pop_path();
                }
            }
            Expression::Not(expr, _) => {
                self.push_path("not_expr".to_string());
                self.visit_expression(expr);
                self.pop_path();
            }
            _ => {
                // Other expressions (primitives, identifiers, etc.) don't contain headers
            }
        }
    }

    /// Visit an expression block
    fn visit_expression_block(&mut self, block: &ExpressionBlock) {
        let context = ASTContext::Statement;
        let node_ref = ASTNodeRef::ExpressionBlock;
        self.push_context(context.clone());
        self.push_ast_node(node_ref);

        // Enter a new scope for header grouping - this is key!
        // All headers within this ExpressionBlock should be grouped for markdown hierarchy
        self.enter_scope();

        // Visit statements
        for (idx, stmt) in block.stmts.iter().enumerate() {
            self.push_path(format!("stmt[{}]", idx));
            self.visit_stmt(stmt);
            self.pop_path();
        }

        // Visit headers that apply to the final expression
        if !block.expr_headers.is_empty() {
            let final_expr_context = ASTContext::ExpressionBlockFinal;
            self.collect_headers_from_vec(&block.expr_headers, final_expr_context);
        }

        // Visit the final expression
        self.push_path("final_expr".to_string());
        self.visit_expression(&block.expr);
        self.pop_path();

        // Exit the scope
        self.exit_scope();
        self.pop_context();
        self.pop_ast_node();
    }

    /// Visit a statement
    fn visit_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Let(let_stmt) => {
                let context = ASTContext::Statement;
                let node_ref = ASTNodeRef::LetStatement(let_stmt.identifier.name().to_string());
                self.push_context(context.clone());
                self.push_ast_node(node_ref);

                // Collect headers for the let statement
                if !let_stmt.annotations.is_empty() {
                    self.collect_headers_from_vec(&let_stmt.annotations, context);
                }

                // Visit the expression
                self.push_path("let_expr".to_string());
                self.visit_expression(&let_stmt.expr);
                self.pop_path();

                self.pop_context();
                self.pop_ast_node();
            }
            Stmt::ForLoop(for_stmt) => {
                let context = ASTContext::Statement;
                let node_ref = ASTNodeRef::ForLoopStatement(for_stmt.identifier.name().to_string());
                self.push_context(context.clone());
                self.push_ast_node(node_ref);

                // Collect headers for the for loop
                if !for_stmt.annotations.is_empty() {
                    self.collect_headers_from_vec(&for_stmt.annotations, context);
                }

                // Visit the iterable expression
                self.push_path("for_iterator".to_string());
                self.visit_expression(&for_stmt.iterator);
                self.pop_path();

                // Visit the body
                self.push_path("for_body".to_string());
                self.visit_expression_block(&for_stmt.body);
                self.pop_path();

                self.pop_context();
                self.pop_ast_node();
            }
        }
    }
}

impl Default for HeaderCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl HeaderTree {
    /// Get all headers in a flat list
    pub fn all_headers(&self) -> Vec<Arc<ContextualHeader>> {
        self.headers_in_order.clone()
    }

    /// Get headers by their context type
    pub fn headers_in_context(&self, context: &ASTContext) -> Vec<Arc<ContextualHeader>> {
        self.headers_by_context
            .get(context)
            .cloned()
            .unwrap_or_default()
    }

    /// Get root headers (those with no parent)
    pub fn root_headers(&self) -> &[Arc<ContextualHeader>] {
        &self.roots
    }

    /// Find a header by its title
    pub fn find_header_by_title(&self, title: &str) -> Option<Arc<ContextualHeader>> {
        self.headers_in_order
            .iter()
            .find(|h| h.header.title == title)
            .cloned()
    }

    /// Get headers within functions (both regular and expression functions)
    pub fn headers_in_functions(&self) -> Vec<Arc<ContextualHeader>> {
        let function_context = ASTContext::TopLevel("function".to_string());
        let expr_function_context = ASTContext::TopLevel("expr_function".to_string());

        let mut result = self.headers_in_context(&function_context);
        result.extend(self.headers_in_context(&expr_function_context));
        result
    }

    /// Get headers that apply to final expressions in blocks
    pub fn headers_for_final_expressions(&self) -> Vec<Arc<ContextualHeader>> {
        let context = ASTContext::ExpressionBlockFinal;
        self.headers_in_context(&context)
    }

    /// Generate a textual representation of the header tree
    pub fn to_tree_string(&self) -> String {
        let mut result = String::new();
        result.push_str("Header Tree:\n");

        for root in &self.roots {
            self.append_header_to_string(&mut result, root, 0);
        }

        if result == "Header Tree:\n" {
            result.push_str("  (no headers found)\n");
        }

        result
    }

    /// Helper method to recursively append headers to string representation
    fn append_header_to_string(
        &self,
        result: &mut String,
        header: &ContextualHeader,
        indent_level: usize,
    ) {
        let indent = "  ".repeat(indent_level);
        result.push_str(&format!(
            "{}├─ {} (Level: {}, Context: {:?})\n",
            indent, header.header.title, header.header.level, header.ast_context
        ));

        for child in &header.header_children {
            self.append_header_to_string(result, child, indent_level + 1);
        }
    }
}

impl ContextualHeader {
    /// Get the header title
    pub fn title(&self) -> &str {
        &self.header.title
    }

    /// Get the header level
    pub fn level(&self) -> u8 {
        self.header.level
    }

    /// Check if this header has a header-level parent
    pub fn has_header_parent(&self) -> bool {
        self.header_parent.is_some()
    }

    /// Check if this header has header-level children
    pub fn has_header_children(&self) -> bool {
        !self.header_children.is_empty()
    }

    /// Check if this header has an AST parent
    pub fn has_ast_parent(&self) -> bool {
        self.ast_parent.is_some()
    }

    /// Check if this header has AST children
    pub fn has_ast_children(&self) -> bool {
        !self.ast_children.is_empty()
    }

    /// Get the full AST path as a string
    pub fn ast_path_string(&self) -> String {
        self.ast_path.join(" -> ")
    }
}
