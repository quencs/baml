use std::collections::{BTreeMap, BTreeSet, HashMap};

use super::{Ast, HeaderCollector, HeaderIndex, RenderableHeader, ScopeId, WithSpan};

/// A Mermaid flowchart (LR) generator focused on headers and control-flow-like structure.
///
/// It renders nested connections as Mermaid subgraphs: the header that owns a nested scope
/// becomes a subgraph container (titled with the header text) and the nested scope headers
/// are rendered inside the container. Sibling elements are connected linearly with `-->`.
/// Connections never cross container boundaries; containers themselves are the units that
/// connect to other elements.
#[derive(Debug, Default)]
pub struct BamlVisDiagramGenerator {
    content: Vec<String>,
    // Counter for generating stable unique ids for nodes and subgraphs
    id_counter: u32,
    // Cache header.id -> node id for simple header nodes
    header_node_ids: HashMap<String, String>,
    // Cache header.id -> subgraph id for container headers
    header_subgraph_ids: HashMap<String, String>,
}

impl BamlVisDiagramGenerator {
    pub fn new() -> Self {
        Self {
            content: vec!["flowchart LR".to_string()],
            id_counter: 0,
            header_node_ids: HashMap::new(),
            header_subgraph_ids: HashMap::new(),
        }
    }

    /// Generate a Mermaid flowchart (LR) showing headers as linear steps and
    /// nested scopes as subgraphs.
    pub fn generate_headers_flowchart(ast: &Ast) -> String {
        let index = HeaderCollector::collect(ast);
        let mut g = Self::new();
        g.render(&index);
        g.content.join("\n")
    }

    /// Back-compat API used by the example. `use_fancy` toggles optional cosmetic styling.
    pub fn generate_with_styling(ast: &Ast, use_fancy: bool) -> String {
        let out = Self::generate_headers_flowchart(ast);
        if use_fancy {
            // Prepend a few cosmetic styles supported by Mermaid flowcharts
            let mut styled: Vec<String> = Vec::new();
            styled.push("%% fancy styling".to_string());
            styled.push(
                "%% note: flowchart LR does not support rich HTML labels reliably".to_string(),
            );
            styled.push("%% simple link style".to_string());
            styled.push("%% linkStyle default stroke:#666,stroke-width:2px".to_string());
            styled.push(String::new());
            // Insert after the first line (the 'flowchart LR' declaration)
            if let Some(pos) = out.find('\n') {
                let (first, rest) = out.split_at(pos + 1);
                let mut merged = String::new();
                merged.push_str(first);
                merged.push_str(&styled.join("\n"));
                merged.push('\n');
                merged.push_str(rest);
                return merged;
            } else {
                styled.insert(0, out);
                return styled.join("\n");
            }
        }
        out
    }

    fn render(&mut self, index: &HeaderIndex) {
        // Map header.id -> &RenderableHeader for quick lookup
        let mut by_id: HashMap<&str, &RenderableHeader> = HashMap::new();
        for h in &index.headers {
            by_id.insert(&h.id, h);
        }

        // Build set of headers that are targets of nested edges (child scope roots)
        let mut nested_targets: BTreeSet<&str> = BTreeSet::new();
        // Map of container header id -> child scope root header ids (preserve stable order)
        let mut container_to_children: HashMap<&str, Vec<&str>> = HashMap::new();
        for (from, to) in &index.nested_edges {
            if let (Some(f), Some(t)) = (by_id.get(from.as_str()), by_id.get(to.as_str())) {
                // Only consider cross-scope nested edges for containerization
                if f.scope != t.scope {
                    nested_targets.insert(t.id.as_str());
                    container_to_children
                        .entry(f.id.as_str())
                        .or_default()
                        .push(t.id.as_str());
                }
            }
        }

        // Determine top-level scopes: those scope roots that are not nested targets
        // We will render each top-level scope's sequence independently
        // and do not link across scopes to avoid crossing conceptual boundaries.
        // Order top-level scopes by their root header source position for stability.
        let mut top_scopes_ordered: BTreeMap<(String, u32, u32), ScopeId> = BTreeMap::new();
        for (_scope, root_id) in &index.scope_root_header {
            if !nested_targets.contains(root_id.as_str()) {
                if let Some(root) = index.find_by_id(root_id) {
                    top_scopes_ordered.insert(
                        (
                            root.span.file.path().to_string(),
                            root.span.start as u32,
                            root.span.end as u32,
                        ),
                        root.scope,
                    );
                }
            }
        }

        for (_k, scope) in top_scopes_ordered {
            self.render_scope_sequence(index, scope, &by_id, &container_to_children);
        }
    }

    /// Render a scope sequence (top-level or nested) as a set of nodes/subgraphs and
    /// connect them linearly. For nested scopes this is called inside a subgraph.
    fn render_scope_sequence(
        &mut self,
        index: &HeaderIndex,
        scope: ScopeId,
        by_id: &HashMap<&str, &RenderableHeader>,
        container_to_children: &HashMap<&str, Vec<&str>>,
    ) {
        // Collect all headers in this scope in source order so markdown children
        // are included inside their container sequences.
        let mut items: Vec<&RenderableHeader> = index.headers_in_scope(scope);

        // Materialize items into visual ids (node or subgraph ids)
        let mut visual_ids: Vec<String> = Vec::new();
        for header in items.drain(..) {
            let id = if container_to_children.contains_key(header.id.as_str()) {
                // Render a subgraph container for this header, then render its nested child scope(s) inside
                self.ensure_subgraph(header);
                // Render combined child scopes inside the subgraph, linearly
                if let Some(children) = container_to_children.get(header.id.as_str()) {
                    // Sort children by source position for stable order
                    let mut children_sorted: Vec<&RenderableHeader> = children
                        .iter()
                        .filter_map(|cid| by_id.get(*cid).copied())
                        .collect();
                    children_sorted.sort_by_key(|h| (h.span.file.path().to_string(), h.span.start));

                    // Enter subgraph: indent and set a vertical direction for readability
                    let subgraph_id = self
                        .header_subgraph_ids
                        .get(header.id.as_str())
                        .cloned()
                        .expect("subgraph id must exist");
                    self.content.push(format!(
                        "  subgraph {}[\"{}\"]",
                        subgraph_id,
                        escape_label(&header.title)
                    ));
                    self.content.push("    direction TB".to_string());

                    // Inside the container, render each child scope sequence
                    let mut representative_ids: Vec<String> = Vec::new();
                    for child_root in children_sorted {
                        let child_scope = child_root.scope;
                        // Render full child scope contents (may include nested subgraphs)
                        self.render_scope_sequence(
                            index,
                            child_scope,
                            by_id,
                            container_to_children,
                        );
                        // Use the child root as representative for inter-child linear connection
                        let rep_id = if container_to_children.contains_key(child_root.id.as_str()) {
                            // child is a container; connect via its subgraph id
                            self.ensure_subgraph(child_root)
                        } else {
                            // simple node representative
                            self.ensure_node(child_root)
                        };
                        representative_ids.push(rep_id);
                    }

                    // Connect child containers/nodes linearly
                    self.connect_sequence(&representative_ids, 4);

                    // Close subgraph
                    self.content.push("  end".to_string());
                    subgraph_id
                } else {
                    // No children recorded; still emit empty subgraph
                    let subgraph_id = self
                        .header_subgraph_ids
                        .get(header.id.as_str())
                        .cloned()
                        .expect("subgraph id must exist");
                    self.content.push(format!(
                        "  subgraph {}[\"{}\"]",
                        subgraph_id,
                        escape_label(&header.title)
                    ));
                    self.content.push("    direction TB".to_string());
                    self.content.push("  end".to_string());
                    subgraph_id
                }
            } else {
                // Simple header node
                self.ensure_node(header)
            };

            visual_ids.push(id);
        }

        // Connect the items linearly with top-level indentation
        self.connect_sequence(&visual_ids, 0);
    }

    /// Ensure a simple node exists for the header and return its id.
    fn ensure_node(&mut self, header: &RenderableHeader) -> String {
        if let Some(id) = self.header_node_ids.get(&header.id) {
            return id.clone();
        }
        let id = self.next_id("n");
        self.header_node_ids.insert(header.id.clone(), id.clone());
        self.content
            .push(format!("  {}[\"{}\"]", id, escape_label(&header.title)));
        id
    }

    /// Ensure a subgraph id exists for the container header and return its id.
    fn ensure_subgraph(&mut self, header: &RenderableHeader) -> String {
        if let Some(id) = self.header_subgraph_ids.get(&header.id) {
            return id.clone();
        }
        let id = self.next_id("sg");
        self.header_subgraph_ids
            .insert(header.id.clone(), id.clone());
        id
    }

    /// Connect a sequence of ids linearly with `-->` at given indent level (spaces).
    fn connect_sequence(&mut self, ids: &[String], indent_spaces: usize) {
        if ids.len() < 2 {
            return;
        }
        let indent = " ".repeat(indent_spaces);
        for pair in ids.windows(2) {
            let a = &pair[0];
            let b = &pair[1];
            self.content.push(format!("{}{} --> {}", indent, a, b));
        }
    }

    fn next_id(&mut self, prefix: &str) -> String {
        let id = format!("{}{}", prefix, self.id_counter);
        self.id_counter += 1;
        id
    }
}

#[inline]
fn escape_label(s: &str) -> String {
    s.replace('"', "&quot;")
}
