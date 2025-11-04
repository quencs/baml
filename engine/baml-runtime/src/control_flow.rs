use std::{collections::HashMap, fmt, str::FromStr};

use anyhow::{anyhow, Result};
use baml_compiler::hir;
use internal_baml_core::ast::Span;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct NodeId {
    function: String,
    segments: Vec<PathSegment>,
}

impl NodeId {
    fn new(function: &str, segments: &[PathSegment]) -> Self {
        Self {
            function: function.to_string(),
            segments: segments.to_vec(),
        }
    }

    pub fn parent(&self) -> Option<Self> {
        if self.segments.is_empty() {
            return None;
        }

        let mut segments = self.segments.clone();
        segments.pop();
        Some(NodeId::new(&self.function, &segments))
    }

    pub fn encode(&self) -> String {
        encode_segments(&self.function, &self.segments)
    }
}

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.encode())
    }
}

impl FromStr for NodeId {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        decode_segments(s)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum PathSegment {
    Statement { ordinal: u32 },
    Header { slug: String, ordinal: u16 },
    Scope { kind: ScopeKind, ordinal: u16 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ScopeKind {
    FunctionRoot,
    Block,
    LoopBody,
    BranchArm,
}

impl ScopeKind {
    fn as_str(&self) -> &'static str {
        match self {
            ScopeKind::FunctionRoot => "func",
            ScopeKind::Block => "block",
            ScopeKind::LoopBody => "loop",
            ScopeKind::BranchArm => "arm",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Node {
    pub id: NodeId,
    pub parent_node_id: Option<NodeId>,
    pub label: String,
    pub span: Span,
    pub node_type: NodeType,
}

impl Node {
    fn new(id: NodeId, label: impl Into<String>, span: Span, node_type: NodeType) -> Self {
        let parent_node_id = id.parent();
        Self {
            id,
            parent_node_id,
            label: label.into(),
            span,
            node_type,
        }
    }

    fn root(id: NodeId, span: Span, label: impl Into<String>) -> Self {
        Self {
            parent_node_id: None,
            id,
            label: label.into(),
            span,
            node_type: NodeType::FunctionRoot,
        }
    }
}

#[derive(Clone, Debug)]
pub enum NodeType {
    FunctionRoot,
    ExprBlock,
    Branch,
    Loop,
    Llm { client: String },
    ImpliedByNewScope,
    ImpliedByStatement,
}

#[derive(Clone, Debug)]
pub struct Edge {
    pub src: NodeId,
    pub dst: NodeId,
    pub label: String,
}

#[derive(Clone, Debug, Default)]
pub struct ControlFlowVisualization {
    pub nodes: HashMap<NodeId, Node>,
    pub edges_by_src: HashMap<NodeId, Vec<Edge>>,
}

#[derive(Default)]
struct ControlFlowVizBuilder {
    nodes: HashMap<NodeId, Node>,
    edges: Vec<Edge>,
}

impl ControlFlowVizBuilder {
    fn add_node(&mut self, node: Node) {
        self.nodes.insert(node.id.clone(), node);
    }

    fn add_edge(&mut self, src: NodeId, dst: NodeId, label: String) {
        self.edges.push(Edge { src, dst, label });
    }

    fn finish(self) -> ControlFlowVisualization {
        let mut edges_by_src: HashMap<NodeId, Vec<Edge>> = HashMap::new();
        for edge in self.edges {
            edges_by_src.entry(edge.src.clone()).or_default().push(edge);
        }

        ControlFlowVisualization {
            nodes: self.nodes,
            edges_by_src,
        }
    }
}

struct NodePathCursor {
    function: String,
    segments: Vec<PathSegment>,
}

impl NodePathCursor {
    fn new(function: &str) -> Self {
        Self {
            function: function.to_string(),
            segments: Vec::new(),
        }
    }

    fn push_scope(&mut self, kind: ScopeKind, ordinal: u16) {
        self.segments.push(PathSegment::Scope { kind, ordinal });
    }

    fn push_statement(&mut self, ordinal: u32) -> NodeId {
        self.segments.push(PathSegment::Statement { ordinal });
        NodeId::new(&self.function, &self.segments)
    }

    fn push_header(&mut self, slug: &str, ordinal: u16) -> NodeId {
        self.segments.push(PathSegment::Header {
            slug: slug.to_string(),
            ordinal,
        });
        NodeId::new(&self.function, &self.segments)
    }

    fn current_id(&self) -> NodeId {
        NodeId::new(&self.function, &self.segments)
    }

    fn pop(&mut self) {
        self.segments.pop();
    }

    fn pop_scope(&mut self, kind: ScopeKind) {
        if matches!(self.segments.last(), Some(PathSegment::Scope { kind: k, .. }) if *k == kind) {
            self.segments.pop();
        }
    }
}

struct ScopeFrame {
    kind: ScopeKind,
    next_statement_ordinal: u32,
    next_child_scope: u16,
}

impl ScopeFrame {
    fn new(kind: ScopeKind) -> Self {
        Self {
            kind,
            next_statement_ordinal: 0,
            next_child_scope: 0,
        }
    }

    fn bump_child_scope(&mut self) -> u16 {
        let current = self.next_child_scope;
        self.next_child_scope += 1;
        current
    }
}

pub enum HirFunctionRef<'hir> {
    Expr(&'hir hir::ExprFunction),
    Llm(&'hir hir::LlmFunction),
}

pub fn build_from_hir(hir: &hir::Hir, function_name: &str) -> Result<ControlFlowVisualization> {
    if let Some(func) = hir.expr_functions.iter().find(|f| f.name == function_name) {
        return Ok(build_function_graph(HirFunctionRef::Expr(func)));
    }

    if let Some(func) = hir.llm_functions.iter().find(|f| f.name == function_name) {
        return Ok(build_function_graph(HirFunctionRef::Llm(func)));
    }

    Err(anyhow!(
        "function `{}` not found while building control-flow visualization",
        function_name
    ))
}

pub fn build_function_graph(function: HirFunctionRef<'_>) -> ControlFlowVisualization {
    match function {
        HirFunctionRef::Expr(func) => build_expr_function_graph(func),
        HirFunctionRef::Llm(func) => build_llm_function_graph(func),
    }
}

fn build_expr_function_graph(func: &hir::ExprFunction) -> ControlFlowVisualization {
    let mut ctx = HirTraversalContext::new(func.name.as_str(), func.span.clone());
    ctx.visit_block(&func.body);
    ctx.finish()
}

fn build_llm_function_graph(func: &hir::LlmFunction) -> ControlFlowVisualization {
    let mut builder = ControlFlowVizBuilder::default();
    let mut cursor = NodePathCursor::new(&func.name);
    cursor.push_scope(ScopeKind::FunctionRoot, 0);
    let root_id = cursor.current_id();
    builder.add_node(Node::root(root_id.clone(), func.span.clone(), &func.name));

    let stmt_id = {
        cursor.push_statement(0);
        let id = cursor.current_id();
        cursor.pop();
        id
    };

    let llm_node = Node::new(
        stmt_id.clone(),
        format!("LLM client: {}", func.client),
        func.span.clone(),
        NodeType::Llm {
            client: func.client.clone(),
        },
    );

    builder.add_node(llm_node);
    builder.add_edge(root_id, stmt_id, String::new());

    builder.finish()
}

struct HirTraversalContext {
    graph: ControlFlowVizBuilder,
    cursor: NodePathCursor,
    scope_stack: Vec<ScopeFrame>,
    flow_frontier: Vec<NodeId>,
}

impl HirTraversalContext {
    fn new(function_name: &str, span: Span) -> Self {
        let mut cursor = NodePathCursor::new(function_name);
        cursor.push_scope(ScopeKind::FunctionRoot, 0);
        let mut graph = ControlFlowVizBuilder::default();
        let root_id = cursor.current_id();
        graph.add_node(Node::root(root_id.clone(), span, function_name.to_string()));

        Self {
            graph,
            cursor,
            scope_stack: vec![ScopeFrame::new(ScopeKind::FunctionRoot)],
            flow_frontier: vec![root_id],
        }
    }

    fn finish(self) -> ControlFlowVisualization {
        self.graph.finish()
    }

    fn visit_block(&mut self, block: &hir::Block) {
        let is_root = matches!(
            self.scope_stack.last().map(|f| f.kind),
            Some(ScopeKind::FunctionRoot)
        );
        if !is_root {
            let ordinal = self
                .scope_stack
                .last_mut()
                .map(|frame| frame.bump_child_scope())
                .unwrap_or(0);
            self.cursor.push_scope(ScopeKind::Block, ordinal);
            let scope_id = self.cursor.current_id();
            let node = Node::new(
                scope_id.clone(),
                "scope",
                derive_block_span(block),
                NodeType::ImpliedByNewScope,
            );
            self.emit_node(node, None);
            self.scope_stack.push(ScopeFrame::new(ScopeKind::Block));
        } else {
            self.scope_stack
                .push(ScopeFrame::new(ScopeKind::FunctionRoot));
        }

        for (idx, stmt) in block.statements.iter().enumerate() {
            if let Some(frame) = self.scope_stack.last_mut() {
                frame.next_statement_ordinal = idx as u32;
            }
            self.visit_statement(stmt);
        }

        if let Some(expr) = &block.trailing_expr {
            if let Some(frame) = self.scope_stack.last_mut() {
                frame.next_statement_ordinal = block.statements.len() as u32;
            }
            self.visit_expression(expr);
        }

        self.scope_stack.pop();

        if !is_root {
            self.cursor.pop_scope(ScopeKind::Block);
        }
    }

    fn visit_statement(&mut self, stmt: &hir::Statement) {
        match stmt {
            hir::Statement::AnnotatedStatement { headers, statement } => {
                if !headers.is_empty() {
                    self.emit_header_nodes(headers);
                }
                if let Some(inner) = statement.as_deref() {
                    self.visit_statement(inner);
                }
            }
            hir::Statement::Let { span, .. }
            | hir::Statement::Assign { span, .. }
            | hir::Statement::AssignOp { span, .. }
            | hir::Statement::Declare { span, .. }
            | hir::Statement::DeclareAndAssign { span, .. }
            | hir::Statement::WatchOptions { span, .. }
            | hir::Statement::WatchNotify { span, .. }
            | hir::Statement::Semicolon { span, .. }
            | hir::Statement::Assert { span, .. }
            | hir::Statement::Break(span)
            | hir::Statement::Continue(span) => {
                self.emit_implied_node(span.clone(), NodeType::ImpliedByStatement);
            }
            hir::Statement::Expression { expr, .. } => self.visit_expression(expr),
            hir::Statement::Return { expr, span } => {
                self.visit_expression(expr);
                self.emit_terminal(span.clone(), "return");
            }
            hir::Statement::While { block, span, .. } => {
                self.visit_loop(block, span.clone(), LoopFlavor::While);
            }
            hir::Statement::ForLoop { block, span, .. } => {
                self.visit_loop(block, span.clone(), LoopFlavor::For);
            }
            hir::Statement::CForLoop { block, .. } => {
                let span = derive_block_span(block);
                self.visit_loop(block, span, LoopFlavor::CFor);
            }
        }
    }

    fn visit_expression(&mut self, expr: &hir::Expression) {
        match expr {
            hir::Expression::If {
                if_branch,
                else_branch,
                span,
                ..
            } => self.visit_if(if_branch, else_branch.as_deref(), span.clone()),
            hir::Expression::Block(block, _) => self.visit_block(block),
            _ => {
                if let Some(span) = expression_primary_span(expr) {
                    self.emit_implied_node(span, NodeType::ExprBlock);
                } else {
                    self.emit_implied_node(Span::fake(), NodeType::ExprBlock);
                }
            }
        }
    }

    fn emit_header_nodes(&mut self, headers: &[String]) {
        for (idx, header) in headers.iter().enumerate() {
            let slug = slugify(header);
            let node_id = self.cursor.push_header(&slug, idx as u16);
            let node = Node::new(
                node_id.clone(),
                header.clone(),
                Span::fake(),
                NodeType::ExprBlock,
            );
            self.emit_node(node, None);
            self.cursor.pop();
        }
    }

    fn emit_implied_node(&mut self, span: Span, node_type: NodeType) {
        let ordinal = self
            .scope_stack
            .last()
            .map(|frame| frame.next_statement_ordinal)
            .unwrap_or_default();
        let node_id = self.cursor.push_statement(ordinal);
        let label = describe_node_type(&node_type).to_string();
        let node = Node::new(node_id.clone(), label, span, node_type);
        self.emit_node(node, None);
        self.cursor.pop();
    }

    fn emit_node(&mut self, node: Node, edge_label: Option<String>) {
        let node_id = node.id.clone();
        let label = edge_label.unwrap_or_default();
        for predecessor in self.flow_frontier.clone() {
            self.graph
                .add_edge(predecessor, node_id.clone(), label.clone());
        }
        self.graph.add_node(node);
        self.flow_frontier.clear();
        self.flow_frontier.push(node_id);
    }

    fn emit_terminal(&mut self, span: Span, label: &str) {
        let ordinal = self
            .scope_stack
            .last()
            .map(|frame| frame.next_statement_ordinal)
            .unwrap_or_default();
        let node_id = self.cursor.push_statement(ordinal);
        let node = Node::new(node_id.clone(), label, span, NodeType::ImpliedByStatement);
        self.emit_node(node, None);
        self.cursor.pop();
        self.flow_frontier.clear();
    }

    fn visit_if(
        &mut self,
        then_expr: &hir::Expression,
        else_expr: Option<&hir::Expression>,
        span: Span,
    ) {
        let ordinal = self
            .scope_stack
            .last()
            .map(|frame| frame.next_statement_ordinal)
            .unwrap_or_default();
        let branch_id = self.cursor.push_statement(ordinal);
        let node = Node::new(branch_id.clone(), "if", span, NodeType::Branch);
        self.emit_node(node, None);
        self.cursor.pop();

        self.flow_frontier = vec![branch_id.clone()];
        let then_exit = self.walk_branch_arm(then_expr);

        let else_exit = else_expr.map(|expr| {
            self.flow_frontier = vec![branch_id.clone()];
            self.walk_branch_arm(expr)
        });

        self.flow_frontier = merge_branch_exits(branch_id, then_exit, else_exit);
    }

    fn walk_branch_arm(&mut self, expr: &hir::Expression) -> Vec<NodeId> {
        let ordinal = self
            .scope_stack
            .last_mut()
            .map(|frame| frame.bump_child_scope())
            .unwrap_or(0);
        self.cursor.push_scope(ScopeKind::BranchArm, ordinal);
        self.scope_stack.push(ScopeFrame::new(ScopeKind::BranchArm));

        match expr {
            hir::Expression::Block(block, _) => self.visit_block(block),
            _ => self.visit_expression(expr),
        }

        let exits = self.flow_frontier.clone();

        self.scope_stack.pop();
        self.cursor.pop_scope(ScopeKind::BranchArm);

        exits
    }

    fn visit_loop(&mut self, body: &hir::Block, span: Span, flavor: LoopFlavor) {
        let ordinal = self
            .scope_stack
            .last()
            .map(|frame| frame.next_statement_ordinal)
            .unwrap_or_default();
        let loop_id = self.cursor.push_statement(ordinal);
        let node = Node::new(loop_id.clone(), flavor.label(), span, NodeType::Loop);
        self.emit_node(node, None);
        self.cursor.pop();

        self.flow_frontier = vec![loop_id.clone()];

        let scope_ord = self
            .scope_stack
            .last_mut()
            .map(|frame| frame.bump_child_scope())
            .unwrap_or(0);
        self.cursor.push_scope(ScopeKind::LoopBody, scope_ord);
        self.scope_stack.push(ScopeFrame::new(ScopeKind::LoopBody));

        self.visit_block(body);

        let exits = self.flow_frontier.clone();
        for exit in exits {
            self.graph
                .add_edge(exit, loop_id.clone(), "repeat".to_string());
        }

        self.scope_stack.pop();
        self.cursor.pop_scope(ScopeKind::LoopBody);

        self.flow_frontier = vec![loop_id];
    }
}

enum LoopFlavor {
    While,
    For,
    CFor,
}

impl LoopFlavor {
    fn label(&self) -> &'static str {
        match self {
            LoopFlavor::While => "while",
            LoopFlavor::For => "for",
            LoopFlavor::CFor => "cfor",
        }
    }
}

fn merge_branch_exits(
    branch_id: NodeId,
    mut then_exit: Vec<NodeId>,
    else_exit: Option<Vec<NodeId>>,
) -> Vec<NodeId> {
    if let Some(mut exit) = else_exit {
        if exit.is_empty() {
            exit.push(branch_id.clone());
        }
        then_exit.extend(exit);
        then_exit
    } else {
        if then_exit.is_empty() {
            then_exit.push(branch_id);
        }
        then_exit
    }
}

fn derive_block_span(block: &hir::Block) -> Span {
    if let Some(span) = block
        .statements
        .iter()
        .filter_map(statement_primary_span)
        .next()
    {
        return span;
    }

    block
        .trailing_expr
        .as_ref()
        .and_then(|expr| expression_primary_span(expr))
        .unwrap_or_else(Span::fake)
}

fn statement_primary_span(stmt: &hir::Statement) -> Option<Span> {
    match stmt {
        hir::Statement::Let { span, .. }
        | hir::Statement::Assign { span, .. }
        | hir::Statement::AssignOp { span, .. }
        | hir::Statement::Declare { span, .. }
        | hir::Statement::DeclareAndAssign { span, .. }
        | hir::Statement::WatchOptions { span, .. }
        | hir::Statement::WatchNotify { span, .. }
        | hir::Statement::Semicolon { span, .. }
        | hir::Statement::Assert { span, .. }
        | hir::Statement::Break(span)
        | hir::Statement::Continue(span)
        | hir::Statement::Return { span, .. }
        | hir::Statement::While { span, .. }
        | hir::Statement::ForLoop { span, .. } => Some(span.clone()),
        hir::Statement::Expression { span, .. } => Some(span.clone()),
        hir::Statement::AnnotatedStatement { statement, .. } => {
            statement.as_deref().and_then(statement_primary_span)
        }
        hir::Statement::CForLoop { .. } => None,
    }
}

fn expression_primary_span(expr: &hir::Expression) -> Option<Span> {
    match expr {
        hir::Expression::ArrayAccess { span, .. }
        | hir::Expression::FieldAccess { span, .. }
        | hir::Expression::MethodCall { span, .. }
        | hir::Expression::BoolValue(_, span)
        | hir::Expression::NumericValue(_, span)
        | hir::Expression::Identifier(_, span)
        | hir::Expression::StringValue(_, span)
        | hir::Expression::RawStringValue(_, span)
        | hir::Expression::Array(_, span)
        | hir::Expression::Map(_, span)
        | hir::Expression::JinjaExpressionValue(_, span)
        | hir::Expression::Call { span, .. }
        | hir::Expression::ClassConstructor(_, span)
        | hir::Expression::BinaryOperation { span, .. }
        | hir::Expression::UnaryOperation { span, .. }
        | hir::Expression::Paren(_, span)
        | hir::Expression::If { span, .. }
        | hir::Expression::Block(_, span) => Some(span.clone()),
    }
}

fn slugify(header: &str) -> String {
    let mut slug = String::with_capacity(header.len());
    let mut last_dash = false;
    for ch in header.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
            last_dash = false;
        } else if !last_dash {
            slug.push('-');
            last_dash = true;
        }
    }
    slug.trim_matches('-').to_string()
}

fn describe_node_type(node_type: &NodeType) -> &str {
    match node_type {
        NodeType::FunctionRoot => "function",
        NodeType::ExprBlock => "expr",
        NodeType::Branch => "branch",
        NodeType::Loop => "loop",
        NodeType::Llm { .. } => "llm",
        NodeType::ImpliedByNewScope => "scope",
        NodeType::ImpliedByStatement => "stmt",
    }
}

fn encode_segments(function: &str, segments: &[PathSegment]) -> String {
    let mut encoded = String::from(function);
    for segment in segments {
        encoded.push('|');
        match segment {
            PathSegment::Statement { ordinal } => {
                encoded.push_str("s:");
                encoded.push_str(&ordinal.to_string());
            }
            PathSegment::Header { slug, ordinal } => {
                encoded.push_str("h:");
                encoded.push_str(slug);
                encoded.push(':');
                encoded.push_str(&ordinal.to_string());
            }
            PathSegment::Scope { kind, ordinal } => {
                encoded.push_str("c:");
                encoded.push_str(kind.as_str());
                encoded.push(':');
                encoded.push_str(&ordinal.to_string());
            }
        }
    }
    encoded
}

fn decode_segments(encoded: &str) -> Result<NodeId> {
    let mut parts = encoded.split('|');
    let function = parts.next().ok_or_else(|| anyhow!("invalid NodeId"))?;
    let mut segments = Vec::new();
    for part in parts {
        let mut tokens = part.split(':');
        let tag = tokens.next().unwrap_or("");
        match tag {
            "s" => {
                let ordinal: u32 = tokens
                    .next()
                    .ok_or_else(|| anyhow!("missing statement ordinal"))?
                    .parse()?;
                segments.push(PathSegment::Statement { ordinal });
            }
            "h" => {
                let slug = tokens
                    .next()
                    .ok_or_else(|| anyhow!("missing header slug"))?
                    .to_string();
                let ordinal: u16 = tokens
                    .next()
                    .ok_or_else(|| anyhow!("missing header ordinal"))?
                    .parse()?;
                segments.push(PathSegment::Header { slug, ordinal });
            }
            "c" => {
                let kind = tokens.next().ok_or_else(|| anyhow!("missing scope kind"))?;
                let ordinal: u16 = tokens
                    .next()
                    .ok_or_else(|| anyhow!("missing scope ordinal"))?
                    .parse()?;
                let scope_kind = match kind {
                    "func" => ScopeKind::FunctionRoot,
                    "block" => ScopeKind::Block,
                    "loop" => ScopeKind::LoopBody,
                    "arm" => ScopeKind::BranchArm,
                    _ => return Err(anyhow!("unknown scope kind")),
                };
                segments.push(PathSegment::Scope {
                    kind: scope_kind,
                    ordinal,
                });
            }
            _ => return Err(anyhow!("unknown segment")),
        }
    }
    Ok(NodeId::new(function, &segments))
}

#[cfg(test)]
mod tests {
    use super::*;
    use baml_types::ir_type::{type_meta, TypeIR, TypeValue};

    fn simple_function() -> hir::ExprFunction {
        let span = Span::fake();
        let let_stmt = hir::Statement::Let {
            name: "x".into(),
            value: hir::Expression::NumericValue("1".into(), span.clone()),
            annotated_type: None,
            watch: None,
            span: span.clone(),
        };

        let return_expr = hir::Expression::Identifier("x".into(), span.clone());
        let return_stmt = hir::Statement::Return {
            expr: return_expr,
            span: span.clone(),
        };

        hir::ExprFunction {
            name: "Simple".into(),
            parameters: Vec::new(),
            return_type: TypeIR::Primitive(TypeValue::Int, type_meta::IR::default()),
            body: hir::Block {
                statements: vec![let_stmt, return_stmt],
                trailing_expr: None,
            },
            span,
        }
    }

    #[test]
    fn builds_simple_expr_function() {
        let mut hir = hir::Hir::empty();
        hir.expr_functions.push(simple_function());

        let viz = build_from_hir(&hir, "Simple").expect("graph should build");

        // Expect root + let + return expression + terminal
        assert!(viz
            .nodes
            .values()
            .any(|node| matches!(node.node_type, NodeType::FunctionRoot)));
        assert!(viz
            .nodes
            .values()
            .any(|node| matches!(node.node_type, NodeType::ImpliedByStatement)));

        // Root should have at least one outgoing edge
        let root = viz
            .nodes
            .values()
            .find(|node| matches!(node.node_type, NodeType::FunctionRoot))
            .expect("root node");
        assert!(viz
            .edges_by_src
            .get(&root.id)
            .map(|edges| !edges.is_empty())
            .unwrap_or(false));
    }

    #[test]
    fn missing_function_errors() {
        let hir = hir::Hir::empty();
        let err = build_from_hir(&hir, "DoesNotExist").unwrap_err();
        assert!(format!("{err}").contains("DoesNotExist"));
    }
}
