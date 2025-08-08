use std::collections::{BTreeMap, BTreeSet, BTreeSet as _, HashMap, HashSet};

use super::{
    header_collector::ScopeKind, Ast, HeaderCollector, HeaderIndex, RenderableHeader, ScopeId,
    WithSpan,
};

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
    // Track scopes we've already rendered to avoid duplicate emissions
    visited_scopes: HashSet<ScopeId>,
    // Track subgraphs we've already emitted (by subgraph id like sg0, sg1, ...)
    emitted_subgraphs: HashSet<String>,
    // Track linear edges we've already emitted to avoid duplicates (from_id, to_id)
    emitted_edges: BTreeSet<(String, String)>,
    // For branching nodes (e.g., if), map the node rep id -> list of branch terminal rep ids
    branch_endpoints: HashMap<String, Vec<String>>,
}

impl BamlVisDiagramGenerator {
    pub fn new() -> Self {
        Self {
            content: vec!["flowchart LR".to_string()],
            id_counter: 0,
            header_node_ids: HashMap::new(),
            header_subgraph_ids: HashMap::new(),
            visited_scopes: HashSet::new(),
            emitted_subgraphs: HashSet::new(),
            emitted_edges: BTreeSet::new(),
            branch_endpoints: HashMap::new(),
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
            styled.push(
                "classDef loopContainer fill:#e0f7fa,stroke:#006064,stroke-width:2px,color:#000"
                    .to_string(),
            );
            styled.push(
                "classDef decisionNode fill:#fff3e0,stroke:#ef6c00,stroke-width:2px,color:#000"
                    .to_string(),
            );
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
                // Only consider cross-scope nested edges where the target is the ROOT of its scope.
                // This avoids treating auxiliary edges like root->final as separate children.
                if f.scope != t.scope {
                    if let Some(root_id_for_child_scope) = index.scope_root_header.get(&t.scope) {
                        if root_id_for_child_scope == &t.id {
                            nested_targets.insert(t.id.as_str());
                            container_to_children
                                .entry(f.id.as_str())
                                .or_default()
                                .push(t.id.as_str());
                        }
                    }
                }
            }
        }

        // Build markdown hierarchy: parent header id -> direct markdown child headers (same scope)
        let mut markdown_children: HashMap<&str, Vec<&RenderableHeader>> = HashMap::new();
        let mut has_markdown_parent: HashSet<&str> = HashSet::new();
        for h in &index.headers {
            if let Some(pid) = &h.parent_id {
                if let Some(parent) = by_id.get(pid.as_str()) {
                    // Sanity: ensure same scope
                    if parent.scope == h.scope {
                        markdown_children
                            .entry(parent.id.as_str())
                            .or_default()
                            .push(h);
                        has_markdown_parent.insert(h.id.as_str());
                    }
                }
            }
        }
        // Sort markdown children for each parent by source position
        for children in markdown_children.values_mut() {
            children.sort_by_key(|h| (h.span.file.path().to_string(), h.span.start));
        }

        // Determine top-level scopes as those scope roots that are not nested targets
        // Render all scopes (including nested) starting from top-level ones, and allow
        // nested scopes to be drawn inside the parent's markdown container when possible.
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
            self.render_scope_sequence(
                index,
                scope,
                &by_id,
                &container_to_children,
                &markdown_children,
                &has_markdown_parent,
            );
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
        markdown_children: &HashMap<&str, Vec<&RenderableHeader>>,
        has_markdown_parent: &HashSet<&str>,
    ) {
        // Prevent duplicate rendering of the same scope (can be reached via multiple paths)
        if !self.visited_scopes.insert(scope) {
            return;
        }
        // Collect root headers in this scope (those without markdown parents)
        let mut items: Vec<&RenderableHeader> = index
            .headers_in_scope(scope)
            .into_iter()
            .filter(|h| !has_markdown_parent.contains(h.id.as_str()))
            .collect();
        // Sort by source order for stability
        items.sort_by_key(|h| (h.span.file.path().to_string(), h.span.start));

        // Materialize items into visual ids (node or subgraph ids)
        let mut visual_ids: Vec<String> = Vec::new();
        for header in items.drain(..) {
            let id = self.render_header_with_hierarchy(
                index,
                header,
                by_id,
                container_to_children,
                markdown_children,
                has_markdown_parent,
            );
            visual_ids.push(id);
        }

        // Connect the items linearly with awareness of branching endpoints
        if visual_ids.len() >= 2 {
            for i in 0..visual_ids.len() - 1 {
                let cur = &visual_ids[i];
                let next = &visual_ids[i + 1];
                if let Some(branch_ends) = self.branch_endpoints.get(cur).cloned() {
                    for end_id in branch_ends {
                        self.connect_sequence(&vec![end_id.clone(), next.clone()], 0);
                    }
                } else {
                    self.connect_sequence(&vec![cur.clone(), next.clone()], 0);
                }
            }
        }
    }

    /// Render a header as either a simple node or a subgraph container that combines
    /// markdown hierarchy (same-scope) and nested child scopes (cross-scope) inside.
    /// Returns the representative id (node id or subgraph id).
    fn render_header_with_hierarchy(
        &mut self,
        index: &HeaderIndex,
        header: &RenderableHeader,
        by_id: &HashMap<&str, &RenderableHeader>,
        container_to_children: &HashMap<&str, Vec<&str>>,
        markdown_children: &HashMap<&str, Vec<&RenderableHeader>>,
        has_markdown_parent: &HashSet<&str>,
    ) -> String {
        let has_md_children = markdown_children
            .get(header.id.as_str())
            .map(|v| !v.is_empty())
            .unwrap_or(false);
        let nested_children: Vec<&str> = container_to_children
            .get(header.id.as_str())
            .cloned()
            .unwrap_or_default();
        let has_nested_children = !nested_children.is_empty();
        let is_branching = nested_children.len() > 1;

        if !has_md_children && !has_nested_children {
            return self.ensure_node_styled(index, header);
        }

        // Special case: branching header (e.g., if statement). Render as a node that fans out
        if is_branching {
            let parent_node_id = self.ensure_decision_node(index, header);

            let mut branch_last_ids: Vec<String> = Vec::new();
            for child_id in nested_children {
                if let Some(child_root) = by_id.get(child_id) {
                    // Render the child branch scope
                    let child_scope = child_root.scope;
                    self.render_scope_sequence(
                        index,
                        child_scope,
                        by_id,
                        container_to_children,
                        markdown_children,
                        has_markdown_parent,
                    );
                    // Determine representative start id for this branch
                    let start_rep_id = if container_to_children.contains_key(child_root.id.as_str())
                        || markdown_children
                            .get(child_root.id.as_str())
                            .map(|v| !v.is_empty())
                            .unwrap_or(false)
                    {
                        self.ensure_subgraph(child_root)
                    } else {
                        self.ensure_node_styled(index, child_root)
                    };
                    // Connect parent to branch start
                    self.connect_sequence(&vec![parent_node_id.clone(), start_rep_id], 0);

                    // Compute last representative in this branch scope
                    let scope_headers = index.headers_in_scope(child_scope);
                    if let Some(last_hdr) = scope_headers.last() {
                        let end_rep_id = if container_to_children.contains_key(last_hdr.id.as_str())
                            || markdown_children
                                .get(last_hdr.id.as_str())
                                .map(|v| !v.is_empty())
                                .unwrap_or(false)
                        {
                            self.ensure_subgraph(last_hdr)
                        } else {
                            self.ensure_node_styled(index, last_hdr)
                        };
                        branch_last_ids.push(end_rep_id);
                    }
                }
            }
            if !branch_last_ids.is_empty() {
                self.branch_endpoints
                    .insert(parent_node_id.clone(), branch_last_ids);
            }
            return parent_node_id;
        }

        // Ensure subgraph id exists
        self.ensure_subgraph(header);
        let subgraph_id = self
            .header_subgraph_ids
            .get(header.id.as_str())
            .cloned()
            .expect("subgraph id must exist");

        // Render the subgraph once
        let should_render_subgraph = self.emitted_subgraphs.insert(subgraph_id.clone());
        if should_render_subgraph {
            self.content.push(format!(
                "  subgraph {}[\"{}\"]",
                subgraph_id,
                escape_label(&header.title)
            ));
            // Subgraphs rendered in LR direction as requested
            self.content.push("    direction LR".to_string());

            // 1) Render markdown children and connect them linearly to reflect markdown hierarchy
            let mut md_rep_ids: Vec<(String, String)> = Vec::new();
            if let Some(md_children) = markdown_children.get(header.id.as_str()) {
                for ch in md_children.iter() {
                    let rep_id = self.render_header_with_hierarchy(
                        index,
                        ch,
                        by_id,
                        container_to_children,
                        markdown_children,
                        has_markdown_parent,
                    );
                    let pos_key = format!("{}:{:010}", ch.span.file.path(), ch.span.start);
                    md_rep_ids.push((pos_key, rep_id));
                }
            }
            md_rep_ids.sort_by(|a, b| a.0.cmp(&b.0));
            let md_ids_only: Vec<String> = md_rep_ids.into_iter().map(|(_, id)| id).collect();
            // Connect markdown children linearly, but if a child is a branching node,
            // connect each branch endpoint to the next markdown sibling
            if md_ids_only.len() >= 2 {
                for i in 0..md_ids_only.len() - 1 {
                    let cur = &md_ids_only[i];
                    let next = &md_ids_only[i + 1];
                    if let Some(branch_ends) = self.branch_endpoints.get(cur).cloned() {
                        for end_id in branch_ends {
                            self.connect_sequence(&vec![end_id.clone(), next.clone()], 4);
                        }
                    } else {
                        self.connect_sequence(&vec![cur.clone(), next.clone()], 4);
                    }
                }
            }

            // 2) Render nested child scopes (e.g., branches) but DO NOT connect them to avoid cross-branch arrows
            if let Some(nested_child_ids) = container_to_children.get(header.id.as_str()) {
                let mut nested_children_headers: Vec<&RenderableHeader> = nested_child_ids
                    .iter()
                    .filter_map(|cid| by_id.get(*cid).copied())
                    .collect();
                nested_children_headers
                    .sort_by_key(|h| (h.span.file.path().to_string(), h.span.start));

                for child_root in nested_children_headers {
                    let child_scope = child_root.scope;
                    self.render_scope_sequence(
                        index,
                        child_scope,
                        by_id,
                        container_to_children,
                        markdown_children,
                        has_markdown_parent,
                    );
                    // Ensure representative is emitted, but don't connect across branches
                    if container_to_children.contains_key(child_root.id.as_str())
                        || markdown_children
                            .get(child_root.id.as_str())
                            .map(|v| !v.is_empty())
                            .unwrap_or(false)
                    {
                        let _ = self.ensure_subgraph(child_root);
                    } else {
                        let _ = self.ensure_node_styled(index, child_root);
                    }
                }
            }

            // Close subgraph
            self.content.push("  end".to_string());
        }

        subgraph_id
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

    /// Ensure a simple node with class based on scope kind
    fn ensure_node_styled(&mut self, index: &HeaderIndex, header: &RenderableHeader) -> String {
        let id = self.ensure_node(header);
        if let Some(kind) = index.scope_kind.get(&header.scope) {
            match kind {
                ScopeKind::ForBody => {
                    self.content.push(format!("  class {} loopContainer;", id));
                }
                _ => {}
            }
        }
        id
    }

    /// Ensure a decision (rhombus) node for branching headers
    fn ensure_decision_node(&mut self, _index: &HeaderIndex, header: &RenderableHeader) -> String {
        if let Some(id) = self.header_node_ids.get(&header.id) {
            return id.clone();
        }
        let id = self.next_id("n");
        self.header_node_ids.insert(header.id.clone(), id.clone());
        // Rhombus node: use {label} in Mermaid
        self.content
            .push(format!("  {}{{\"{}\"}}", id, escape_label(&header.title)));
        // Apply decisionNode class
        self.content.push(format!("  class {} decisionNode;", id));
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
            // Avoid emitting duplicate edges
            if self.emitted_edges.insert((a.clone(), b.clone())) {
                self.content.push(format!("{}{} --> {}", indent, a, b));
            }
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
