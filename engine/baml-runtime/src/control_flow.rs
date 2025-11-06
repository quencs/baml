use std::{fmt, io::Write, str::FromStr};

use anyhow::{anyhow, Result};
use baml_compiler::hir;
use indexmap::IndexMap;
use internal_baml_core::ast::Span;
use pretty::RcDoc;

pub mod mermaid;

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
enum PathSegment {
    FunctionRoot { ordinal: u16 },
    Header { slug: String, ordinal: u16 },
    BranchGroup { slug: String, ordinal: u16 },
    BranchArm { slug: String, ordinal: u16 },
    Loop { slug: String, ordinal: u16 },
    OtherScope { slug: String, ordinal: u16 },
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
    HeaderContextEnter,
    BranchGroup,
    BranchArm,
    Loop,
    OtherScope,
}

#[derive(Clone, Debug)]
pub struct Edge {
    pub src: NodeId,
    pub dst: NodeId,
}

#[derive(Clone, Debug, Default)]
pub struct ControlFlowVisualization {
    pub nodes: IndexMap<NodeId, Node>,
    pub edges_by_src: IndexMap<NodeId, Vec<Edge>>,
}

#[derive(Default)]
struct ControlFlowVizBuilder {
    nodes: IndexMap<NodeId, Node>,
    edges: Vec<Edge>,
}

impl ControlFlowVizBuilder {
    fn add_node(&mut self, node: Node) {
        self.nodes.insert(node.id.clone(), node);
    }

    fn add_edge(&mut self, src: NodeId, dst: NodeId) {
        self.edges.push(Edge { src, dst });
    }

    fn finish(self) -> ControlFlowVisualization {
        let mut edges_by_src: IndexMap<NodeId, Vec<Edge>> = IndexMap::new();
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

    fn push_root(&mut self, ordinal: u16) -> NodeId {
        self.segments.push(PathSegment::FunctionRoot { ordinal });
        NodeId::new(&self.function, &self.segments)
    }

    fn push_header(&mut self, slug: &str, ordinal: u16) -> NodeId {
        self.segments.push(PathSegment::Header {
            slug: slug.to_string(),
            ordinal,
        });
        NodeId::new(&self.function, &self.segments)
    }

    fn push_branch_group(&mut self, slug: &str, ordinal: u16) -> NodeId {
        self.segments.push(PathSegment::BranchGroup {
            slug: slug.to_string(),
            ordinal,
        });
        NodeId::new(&self.function, &self.segments)
    }

    fn push_branch_arm(&mut self, slug: &str, ordinal: u16) -> NodeId {
        self.segments.push(PathSegment::BranchArm {
            slug: slug.to_string(),
            ordinal,
        });
        NodeId::new(&self.function, &self.segments)
    }

    fn push_loop(&mut self, slug: &str, ordinal: u16) -> NodeId {
        self.segments.push(PathSegment::Loop {
            slug: slug.to_string(),
            ordinal,
        });
        NodeId::new(&self.function, &self.segments)
    }

    fn push_other_scope(&mut self, slug: &str, ordinal: u16) -> NodeId {
        self.segments.push(PathSegment::OtherScope {
            slug: slug.to_string(),
            ordinal,
        });
        NodeId::new(&self.function, &self.segments)
    }

    fn pop(&mut self) {
        self.segments.pop();
    }
}

#[derive(Clone, Default)]
struct FrameCounters {
    header: u16,
    branch_group: u16,
    branch_arm: u16,
    loop_node: u16,
    other_scope: u16,
}

enum CounterKind {
    Header,
    BranchGroup,
    BranchArm,
    Loop,
    OtherScope,
}

impl FrameCounters {
    fn next(&mut self, kind: CounterKind) -> u16 {
        match kind {
            CounterKind::Header => {
                let current = self.header;
                self.header += 1;
                current
            }
            CounterKind::BranchGroup => {
                let current = self.branch_group;
                self.branch_group += 1;
                current
            }
            CounterKind::BranchArm => {
                let current = self.branch_arm;
                self.branch_arm += 1;
                current
            }
            CounterKind::Loop => {
                let current = self.loop_node;
                self.loop_node += 1;
                current
            }
            CounterKind::OtherScope => {
                let current = self.other_scope;
                self.other_scope += 1;
                current
            }
        }
    }
}

enum FrameEntry {
    FunctionRoot,
    Header { level: u8 },
    BranchGroup,
    BranchArm,
    Loop,
    OtherScope,
}

struct Frame {
    entry: FrameEntry,
    node_id: NodeId,
    counters: FrameCounters,
    last_linear_child: Option<NodeId>,
}

impl Frame {
    fn new(entry: FrameEntry, node_id: NodeId) -> Self {
        Self {
            entry,
            node_id,
            counters: FrameCounters::default(),
            last_linear_child: None,
        }
    }

    fn next_ordinal(&mut self, kind: CounterKind) -> u16 {
        self.counters.next(kind)
    }

    fn header_level(&self) -> Option<u8> {
        match self.entry {
            FrameEntry::Header { level } => Some(level),
            _ => None,
        }
    }
}

impl FrameEntry {
    fn children_are_linear(&self) -> bool {
        !matches!(self, FrameEntry::BranchGroup)
    }
}

struct BlockHandling {
    wrap: bool,
    label: Option<String>,
}

impl BlockHandling {
    fn inline() -> Self {
        Self {
            wrap: false,
            label: None,
        }
    }

    fn wrap(label: Option<String>) -> Self {
        Self { wrap: true, label }
    }
}

enum LoopFlavor<'a> {
    While {
        condition: &'a hir::Expression,
    },
    For {
        identifier: &'a str,
        iterator: &'a hir::Expression,
    },
    CFor {
        condition: Option<&'a hir::Expression>,
    },
}

impl<'a> LoopFlavor<'a> {
    fn label(&self) -> String {
        match self {
            LoopFlavor::While { condition } => {
                format!("while ({})", render_expression(condition))
            }
            LoopFlavor::For {
                identifier,
                iterator,
            } => format!("for ({} in {})", identifier, render_expression(iterator)),
            LoopFlavor::CFor { condition } => {
                if let Some(expr) = condition {
                    format!("for ({})", render_expression(expr))
                } else {
                    "for (...)".to_string()
                }
            }
        }
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
    ctx.visit_function_body(&func.body);
    ctx.finish()
}

fn build_llm_function_graph(func: &hir::LlmFunction) -> ControlFlowVisualization {
    let mut builder = ControlFlowVizBuilder::default();
    let mut cursor = NodePathCursor::new(&func.name);
    let root_id = cursor.push_root(0);
    builder.add_node(Node::root(root_id.clone(), func.span.clone(), &func.name));

    let slug = slug_or_default("llm", "llm");
    let loop_id = cursor.push_other_scope(&slug, 0);
    let node = Node::new(
        loop_id.clone(),
        format!("LLM client: {}", func.client),
        func.span.clone(),
        NodeType::OtherScope,
    );
    builder.add_node(node);
    builder.add_edge(root_id, loop_id);

    builder.finish()
}

struct HirTraversalContext {
    graph: ControlFlowVizBuilder,
    cursor: NodePathCursor,
    frames: Vec<Frame>,
}

impl HirTraversalContext {
    fn new(function_name: &str, span: Span) -> Self {
        let mut cursor = NodePathCursor::new(function_name);
        let root_id = cursor.push_root(0);
        let mut graph = ControlFlowVizBuilder::default();
        graph.add_node(Node::root(root_id.clone(), span, function_name.to_string()));

        Self {
            graph,
            cursor,
            frames: vec![Frame::new(FrameEntry::FunctionRoot, root_id)],
        }
    }

    fn finish(self) -> ControlFlowVisualization {
        self.graph.finish()
    }

    fn current_parent_index(&self) -> usize {
        self.frames
            .len()
            .checked_sub(1)
            .expect("frame stack always contains root")
    }

    fn visit_function_body(&mut self, block: &hir::Block) {
        let depth = self.frames.len();
        for stmt in &block.statements {
            self.visit_statement(stmt);
        }
        if let Some(expr) = &block.trailing_expr {
            self.visit_expression(expr, BlockHandling::inline());
        }
        self.pop_frames_to(depth);
    }

    fn visit_block_inline(&mut self, block: &hir::Block) {
        let depth = self.frames.len();
        for stmt in &block.statements {
            self.visit_statement(stmt);
        }
        if let Some(expr) = &block.trailing_expr {
            self.visit_expression(expr, BlockHandling::inline());
        }
        self.pop_frames_to(depth);
    }

    fn emit_other_scope(&mut self, block: &hir::Block, span: Span, label: Option<String>) {
        let parent_depth = self.frames.len();
        let ordinal = {
            let frame = self
                .frames
                .last_mut()
                .expect("frame stack should not be empty");
            frame.next_ordinal(CounterKind::OtherScope)
        };
        let label_ref = label.as_deref().unwrap_or("");
        let slug_base = slugify(label_ref);
        let slug = if slug_base.is_empty() {
            format!("other-scope-{}", ordinal)
        } else {
            slug_base
        };
        let node_id = self.cursor.push_other_scope(&slug, ordinal);
        let node_label = label.unwrap_or_else(|| "".to_string());
        let node = Node::new(node_id.clone(), node_label, span, NodeType::OtherScope);
        self.graph.add_node(node);
        let parent_index = self.current_parent_index();
        self.register_child_with_parent(parent_index, &node_id);
        self.frames
            .push(Frame::new(FrameEntry::OtherScope, node_id.clone()));
        self.visit_block_inline(block);
        self.pop_frames_to(parent_depth);
    }

    fn visit_statement(&mut self, stmt: &hir::Statement) {
        match stmt {
            hir::Statement::HeaderContextEnter(header) => self.enter_header(header),
            hir::Statement::Let { name, value, .. } => {
                let is_block = matches!(value, hir::Expression::Block(_, _));
                let behavior = if is_block {
                    BlockHandling::wrap(Some(format!("let {name} = {{ ... }}")))
                } else {
                    BlockHandling::inline()
                };
                self.visit_expression(value, behavior);
            }
            hir::Statement::Assign { value, .. }
            | hir::Statement::AssignOp { value, .. }
            | hir::Statement::DeclareAndAssign { value, .. } => {
                let behavior = if matches!(value, hir::Expression::Block(_, _)) {
                    BlockHandling::wrap(None)
                } else {
                    BlockHandling::inline()
                };
                self.visit_expression(value, behavior);
            }
            hir::Statement::Expression { expr, .. } | hir::Statement::Semicolon { expr, .. } => {
                let behavior = if matches!(expr, hir::Expression::Block(_, _)) {
                    BlockHandling::wrap(None)
                } else {
                    BlockHandling::inline()
                };
                self.visit_expression(expr, behavior);
            }
            hir::Statement::Assert { condition, .. } => {
                self.visit_expression(condition, BlockHandling::inline());
            }
            hir::Statement::Return { expr, .. } => {
                self.visit_expression(expr, BlockHandling::inline());
            }
            hir::Statement::While {
                condition,
                block,
                span,
            } => {
                self.visit_loop(LoopFlavor::While { condition }, block, span.clone());
            }
            hir::Statement::ForLoop {
                identifier,
                iterator,
                block,
                span,
            } => {
                self.visit_loop(
                    LoopFlavor::For {
                        identifier,
                        iterator,
                    },
                    block,
                    span.clone(),
                );
            }
            hir::Statement::CForLoop {
                condition, block, ..
            } => {
                let span = derive_block_span(block);
                let condition_ref = condition.as_ref();
                self.visit_loop(
                    LoopFlavor::CFor {
                        condition: condition_ref,
                    },
                    block,
                    span,
                );
            }
            hir::Statement::Declare { .. }
            | hir::Statement::Break(_)
            | hir::Statement::Continue(_)
            | hir::Statement::WatchOptions { .. }
            | hir::Statement::WatchNotify { .. } => {}
        }
    }

    fn visit_expression(&mut self, expr: &hir::Expression, block_behavior: BlockHandling) {
        match expr {
            hir::Expression::If {
                condition,
                if_branch,
                else_branch,
                span,
                ..
            } => self.visit_if(condition, if_branch, else_branch.as_deref(), span.clone()),
            hir::Expression::Block(block, span) => {
                if block_behavior.wrap {
                    self.emit_other_scope(block, span.clone(), block_behavior.label);
                } else {
                    self.visit_block_inline(block);
                }
            }
            _ => {}
        }
    }

    fn visit_if(
        &mut self,
        condition: &hir::Expression,
        then_expr: &hir::Expression,
        else_expr: Option<&hir::Expression>,
        span: Span,
    ) {
        let parent_depth = self.frames.len();
        let ordinal = {
            let frame = self
                .frames
                .last_mut()
                .expect("frame stack should not be empty");
            frame.next_ordinal(CounterKind::BranchGroup)
        };
        let label = format!("if ({})", render_expression(condition));
        let slug = {
            let slug_base = slugify(&label);
            if slug_base.is_empty() {
                format!("if-{}", ordinal)
            } else {
                slug_base
            }
        };
        let node_id = self.cursor.push_branch_group(&slug, ordinal);
        let node = Node::new(node_id.clone(), label, span, NodeType::BranchGroup);
        self.graph.add_node(node);
        let parent_index = self.current_parent_index();
        self.register_child_with_parent(parent_index, &node_id);
        self.frames
            .push(Frame::new(FrameEntry::BranchGroup, node_id.clone()));

        let arm_label = format!("if ({})", render_expression(condition));
        self.visit_branch_arm(arm_label, then_expr, expression_span(then_expr));

        let mut current_else = else_expr;
        while let Some(expr) = current_else {
            match expr {
                hir::Expression::If {
                    condition: else_condition,
                    if_branch,
                    else_branch,
                    ..
                } => {
                    let label = format!("else if ({})", render_expression(else_condition));
                    self.visit_branch_arm(label, if_branch, expression_span(if_branch));
                    current_else = else_branch.as_deref();
                }
                _ => {
                    self.visit_branch_arm("else".to_string(), expr, expression_span(expr));
                    current_else = None;
                }
            }
        }

        self.pop_frames_to(parent_depth);
    }

    fn visit_branch_arm(&mut self, label: String, expr: &hir::Expression, span: Span) {
        let parent_depth = self.frames.len();
        let ordinal = {
            let frame = self
                .frames
                .last_mut()
                .expect("branch group frame must exist");
            frame.next_ordinal(CounterKind::BranchArm)
        };
        let slug_base = slugify(&label);
        let slug = if slug_base.is_empty() {
            format!("branch-arm-{}", ordinal)
        } else {
            slug_base
        };
        let node_id = self.cursor.push_branch_arm(&slug, ordinal);
        let node = Node::new(node_id.clone(), label, span, NodeType::BranchArm);
        self.graph.add_node(node);
        let parent_index = self.current_parent_index();
        self.register_child_with_parent(parent_index, &node_id);
        self.frames
            .push(Frame::new(FrameEntry::BranchArm, node_id.clone()));
        self.visit_expression(expr, BlockHandling::inline());
        self.pop_frames_to(parent_depth);
    }

    fn visit_loop(&mut self, flavor: LoopFlavor<'_>, block: &hir::Block, span: Span) {
        let parent_depth = self.frames.len();
        let ordinal = {
            let frame = self
                .frames
                .last_mut()
                .expect("frame stack should not be empty");
            frame.next_ordinal(CounterKind::Loop)
        };
        let label = flavor.label();
        let slug_base = slugify(&label);
        let slug = if slug_base.is_empty() {
            format!("loop-{}", ordinal)
        } else {
            slug_base
        };
        let node_id = self.cursor.push_loop(&slug, ordinal);
        let node = Node::new(node_id.clone(), label, span, NodeType::Loop);
        self.graph.add_node(node);
        let parent_index = self.current_parent_index();
        self.register_child_with_parent(parent_index, &node_id);
        self.frames
            .push(Frame::new(FrameEntry::Loop, node_id.clone()));
        self.visit_block_inline(block);
        self.pop_frames_to(parent_depth);
    }

    fn enter_header(&mut self, header: &hir::HeaderContext) {
        let level = header.level.max(1);
        self.pop_headers_to_level(level - 1);

        let ordinal = {
            let frame = self
                .frames
                .last_mut()
                .expect("frame stack should not be empty");
            frame.next_ordinal(CounterKind::Header)
        };

        let mut slug = slugify(&header.title);
        if slug.is_empty() {
            slug = format!("header-{}", ordinal);
        }

        let node_id = self.cursor.push_header(&slug, ordinal);
        let node = Node::new(
            node_id.clone(),
            header.title.clone(),
            header.span.clone(),
            NodeType::HeaderContextEnter,
        );
        self.graph.add_node(node);

        let parent_index = self.current_parent_index();
        self.register_child_with_parent(parent_index, &node_id);
        self.frames
            .push(Frame::new(FrameEntry::Header { level }, node_id));
    }

    fn register_child_with_parent(&mut self, parent_index: usize, node_id: &NodeId) {
        let parent_entry = self.frames[parent_index].entry.children_are_linear();
        if !parent_entry {
            return;
        }
        if let Some(prev) = self.frames[parent_index].last_linear_child.clone() {
            self.graph.add_edge(prev, node_id.clone());
        }
        self.frames[parent_index].last_linear_child = Some(node_id.clone());
    }

    fn pop_headers_to_level(&mut self, desired_level: u8) {
        while let Some(frame) = self.frames.last() {
            match frame.entry {
                FrameEntry::Header { level } if level > desired_level => {
                    self.frames.pop();
                    self.cursor.pop();
                }
                _ => break,
            }
        }
    }

    fn pop_frames_to(&mut self, len: usize) {
        while self.frames.len() > len {
            self.frames.pop();
            self.cursor.pop();
        }
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
        | hir::Statement::ForLoop { span, .. }
        | hir::Statement::Expression { span, .. } => Some(span.clone()),
        hir::Statement::HeaderContextEnter(header) => Some(header.span.clone()),
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

fn expression_span(expr: &hir::Expression) -> Span {
    expression_primary_span(expr).unwrap_or_else(Span::fake)
}

fn render_expression(expr: &hir::Expression) -> String {
    collapse_whitespace(&doc_to_string(expr.to_doc()))
}

fn doc_to_string(doc: RcDoc<'_, ()>) -> String {
    let mut buffer = Vec::new();
    let _ = doc.render(80, &mut buffer);
    String::from_utf8(buffer).unwrap_or_default()
}

fn collapse_whitespace(input: &str) -> String {
    let mut parts = input.split_whitespace();
    let mut result = String::new();
    if let Some(first) = parts.next() {
        result.push_str(first);
        for part in parts {
            result.push(' ');
            result.push_str(part);
        }
    }
    result
}

fn slugify(input: &str) -> String {
    let mut slug = String::with_capacity(input.len());
    let mut last_dash = false;
    for ch in input.chars() {
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

fn slug_or_default(label: &str, default: &str) -> String {
    let candidate = slugify(label);
    if candidate.is_empty() {
        default.to_string()
    } else {
        candidate
    }
}

pub fn describe_node_type(node_type: &NodeType) -> &'static str {
    match node_type {
        NodeType::FunctionRoot => "function",
        NodeType::HeaderContextEnter => "header",
        NodeType::BranchGroup => "branch-group",
        NodeType::BranchArm => "branch-arm",
        NodeType::Loop => "loop",
        NodeType::OtherScope => "other-scope",
    }
}

fn encode_segments(function: &str, segments: &[PathSegment]) -> String {
    let mut encoded = String::from(function);
    for segment in segments {
        encoded.push('|');
        match segment {
            PathSegment::FunctionRoot { ordinal } => {
                encoded.push_str("root:");
                encoded.push_str(&ordinal.to_string());
            }
            PathSegment::Header { slug, ordinal } => {
                encoded.push_str("hdr:");
                encoded.push_str(slug);
                encoded.push(':');
                encoded.push_str(&ordinal.to_string());
            }
            PathSegment::BranchGroup { slug, ordinal } => {
                encoded.push_str("bg:");
                encoded.push_str(slug);
                encoded.push(':');
                encoded.push_str(&ordinal.to_string());
            }
            PathSegment::BranchArm { slug, ordinal } => {
                encoded.push_str("arm:");
                encoded.push_str(slug);
                encoded.push(':');
                encoded.push_str(&ordinal.to_string());
            }
            PathSegment::Loop { slug, ordinal } => {
                encoded.push_str("loop:");
                encoded.push_str(slug);
                encoded.push(':');
                encoded.push_str(&ordinal.to_string());
            }
            PathSegment::OtherScope { slug, ordinal } => {
                encoded.push_str("scope:");
                encoded.push_str(slug);
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
            "root" => {
                let ordinal: u16 = tokens
                    .next()
                    .ok_or_else(|| anyhow!("missing root ordinal"))?
                    .parse()?;
                segments.push(PathSegment::FunctionRoot { ordinal });
            }
            "hdr" => {
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
            "bg" => {
                let slug = tokens
                    .next()
                    .ok_or_else(|| anyhow!("missing branch-group slug"))?
                    .to_string();
                let ordinal: u16 = tokens
                    .next()
                    .ok_or_else(|| anyhow!("missing branch-group ordinal"))?
                    .parse()?;
                segments.push(PathSegment::BranchGroup { slug, ordinal });
            }
            "arm" => {
                let slug = tokens
                    .next()
                    .ok_or_else(|| anyhow!("missing branch-arm slug"))?
                    .to_string();
                let ordinal: u16 = tokens
                    .next()
                    .ok_or_else(|| anyhow!("missing branch-arm ordinal"))?
                    .parse()?;
                segments.push(PathSegment::BranchArm { slug, ordinal });
            }
            "loop" => {
                let slug = tokens
                    .next()
                    .ok_or_else(|| anyhow!("missing loop slug"))?
                    .to_string();
                let ordinal: u16 = tokens
                    .next()
                    .ok_or_else(|| anyhow!("missing loop ordinal"))?
                    .parse()?;
                segments.push(PathSegment::Loop { slug, ordinal });
            }
            "scope" => {
                let slug = tokens
                    .next()
                    .ok_or_else(|| anyhow!("missing scope slug"))?
                    .to_string();
                let ordinal: u16 = tokens
                    .next()
                    .ok_or_else(|| anyhow!("missing scope ordinal"))?
                    .parse()?;
                segments.push(PathSegment::OtherScope { slug, ordinal });
            }
            _ => return Err(anyhow!("unknown segment")),
        }
    }
    Ok(NodeId::new(function, &segments))
}

#[cfg(test)]
mod tests;
