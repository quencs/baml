use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

use super::{ControlFlowVisualization, Node, NodeType};

/// Convert a `ControlFlowVisualization` into a Mermaid flowchart definition.
pub fn to_mermaid(viz: &ControlFlowVisualization) -> String {
    const ROOT_PARENT_KEY: &str = "__root__";

    let mut lines = Vec::new();
    lines.push("flowchart TD".to_string());

    let mut node_entries: Vec<(String, &Node)> = viz
        .nodes
        .values()
        .map(|node| (node.id.encode(), node))
        .collect();
    node_entries.sort_by(|a, b| a.0.cmp(&b.0));

    let mut id_lookup: HashMap<String, String> = HashMap::new();
    for (encoded_id, _) in &node_entries {
        let mermaid_id = sanitize_node_id(encoded_id);
        id_lookup.insert(encoded_id.clone(), mermaid_id);
    }

    let mut children_map: HashMap<String, Vec<(String, &Node)>> = HashMap::new();
    for (encoded_id, node) in &node_entries {
        let parent_key = node
            .parent_node_id
            .as_ref()
            .map(|id| id.encode())
            .unwrap_or_else(|| ROOT_PARENT_KEY.to_string());

        children_map
            .entry(parent_key)
            .or_default()
            .push((encoded_id.clone(), *node));
    }

    for list in children_map.values_mut() {
        list.sort_by(|a, b| a.0.cmp(&b.0));
    }

    render_subgraphs(ROOT_PARENT_KEY, 4, &children_map, &id_lookup, &mut lines);

    let mut edges: Vec<(String, String)> = viz
        .edges_by_src
        .iter()
        .flat_map(|(src, list)| {
            let src_id = src.encode();
            list.iter()
                .map(move |edge| (src_id.clone(), edge.dst.encode()))
        })
        .collect();
    edges.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));

    for (src, dst) in edges {
        let src_id = id_lookup
            .get(&src)
            .cloned()
            .unwrap_or_else(|| sanitize_node_id(&src));
        let dst_id = id_lookup
            .get(&dst)
            .cloned()
            .unwrap_or_else(|| sanitize_node_id(&dst));

        let edge_render = render_edge(&src_id, &dst_id);
        lines.push(format!("    {}", edge_render));
    }

    lines.join("\n")
}

fn render_subgraphs(
    parent_key: &str,
    indent: usize,
    children_map: &HashMap<String, Vec<(String, &Node)>>,
    id_lookup: &HashMap<String, String>,
    lines: &mut Vec<String>,
) {
    let Some(children) = children_map.get(parent_key) else {
        return;
    };

    for (encoded_id, node) in children {
        render_node_with_children(encoded_id, node, indent, children_map, id_lookup, lines);
    }
}

fn render_node_with_children(
    encoded_id: &str,
    node: &Node,
    indent: usize,
    children_map: &HashMap<String, Vec<(String, &Node)>>,
    id_lookup: &HashMap<String, String>,
    lines: &mut Vec<String>,
) {
    let indent_str = " ".repeat(indent);
    let mermaid_id = id_lookup
        .get(encoded_id)
        .cloned()
        .unwrap_or_else(|| sanitize_node_id(encoded_id));

    let rendered_label = node_label(node);
    let escaped_label = escape_mermaid_text(&rendered_label);
    let definition = render_node(&mermaid_id, node, &escaped_label);

    if let Some(children) = children_map.get(encoded_id) {
        let subgraph_id = format!("cluster_{}", mermaid_id);
        lines.push(format!(
            "{}subgraph {}[\"{}\"]",
            indent_str, subgraph_id, escaped_label
        ));
        lines.push(format!("{}    {}", indent_str, definition));
        render_subgraphs(encoded_id, indent + 4, children_map, id_lookup, lines);
        lines.push(format!("{}end", indent_str));
    } else {
        lines.push(format!("{}{}", indent_str, definition));
    }
}

fn node_label(node: &Node) -> String {
    if node.label.trim().is_empty() {
        fallback_label(&node.node_type)
    } else {
        node.label.clone()
    }
}

fn fallback_label(node_type: &NodeType) -> String {
    match node_type {
        NodeType::FunctionRoot => "function".to_string(),
        NodeType::HeaderContextEnter => "header".to_string(),
        NodeType::BranchGroup => "branch".to_string(),
        NodeType::BranchArm => "arm".to_string(),
        NodeType::Loop => "loop".to_string(),
        NodeType::OtherScope => "scope".to_string(),
    }
}

fn render_node(id: &str, node: &Node, label: &str) -> String {
    match node.node_type {
        NodeType::FunctionRoot => format!("{id}((\"{label}\"))"),
        NodeType::BranchGroup => format!("{id}{{\"{label}\"}}"),
        NodeType::BranchArm => format!("{id}[\"{label}\"]"),
        NodeType::Loop => format!("{id}([\"{label}\"])"),
        NodeType::HeaderContextEnter | NodeType::OtherScope => format!("{id}[\"{label}\"]"),
    }
}

fn render_edge(src: &str, dst: &str) -> String {
    format!("{src} --> {dst}")
}

fn sanitize_node_id(encoded: &str) -> String {
    let mut sanitized = String::with_capacity(encoded.len() + 8);
    sanitized.push('N');
    for ch in encoded.chars() {
        if ch.is_ascii_alphanumeric() {
            sanitized.push(ch);
        } else {
            sanitized.push('_');
        }
    }

    let mut hasher = DefaultHasher::new();
    encoded.hash(&mut hasher);
    let suffix = hasher.finish() & 0xffff;
    sanitized.push('_');
    sanitized.push_str(&format!("{:04x}", suffix));
    sanitized
}

fn escape_mermaid_text(input: &str) -> String {
    let mut escaped = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#39;"),
            '[' => escaped.push_str("&#91;"),
            ']' => escaped.push_str("&#93;"),
            '{' => escaped.push_str("&#123;"),
            '}' => escaped.push_str("&#125;"),
            '|' => escaped.push_str("&#124;"),
            '\\' => escaped.push_str("&#92;"),
            '\n' | '\r' => escaped.push_str("<br/>"),
            _ => escaped.push(ch),
        }
    }
    escaped
}
