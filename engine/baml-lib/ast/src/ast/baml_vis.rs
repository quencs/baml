use std::collections::{BTreeSet, HashMap, HashSet};

/// Config: maximum number of direct children (markdown + nested) a non-branching
/// container may have to be flattened into a linear sequence instead of a subgraph.
///
/// For example, with `1` (default), containers with 0 or 1 child are flattened.
/// Top-level scopes are never flattened if they have any children, regardless of this value.
const MAX_CHILDREN_TO_FLATTEN: usize = 0;
// Toggle: render function call nodes (e.g., SummarizeVideo, CreatePR) alongside headers.
const SHOW_CALL_NODES: bool = false;

use internal_baml_diagnostics::SerializedSpan;
use serde_json;

use super::{
    header_collector::HeaderLabelKind, Ast, HeaderCollector, HeaderIndex, RenderableHeader,
    ScopeId, WithSpan,
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
    // Cache for helper call nodes to avoid duplicates. Keyed by (container_id, callee_name)
    call_node_ids: HashMap<(String, String), String>,
    // Track which node ids have had a style class emitted to avoid duplicate class lines
    styled_node_ids: HashSet<String>,
    // Cache representative id for a header id to avoid recomputation
    header_rep_cache: HashMap<String, String>,
    // Map from visual id (e.g. n0, n1) to serialized span for click callbacks
    rep_id_to_span: HashMap<String, SerializedSpan>,
}

impl BamlVisDiagramGenerator {
    pub fn new() -> Self {
        Self {
            content: vec![
                "---".to_string(),
                "config:".to_string(),
                "  layout: elk".to_string(),
                "---".to_string(),
                "".to_string(),
                "flowchart LR".to_string(),
            ],
            id_counter: 0,
            header_node_ids: HashMap::new(),
            header_subgraph_ids: HashMap::new(),
            visited_scopes: HashSet::new(),
            emitted_subgraphs: HashSet::new(),
            emitted_edges: BTreeSet::new(),
            branch_endpoints: HashMap::new(),
            call_node_ids: HashMap::new(),
            styled_node_ids: HashSet::new(),
            header_rep_cache: HashMap::new(),
            rep_id_to_span: HashMap::new(),
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
            // Prepend styles (Mermaid 10 shapes). Use processes shape for loop containers.
            let mut styled: Vec<String> = Vec::new();
            styled.push(
                "classDef loopContainer shape:processes,fill:#e0f7fa,stroke:#006064,stroke-width:2px,color:#000"
                    .to_string(),
            );
            styled.push(
                "classDef decisionNode fill:#fff3e0,stroke:#ef6c00,stroke-width:2px,color:#000"
                    .to_string(),
            );
            styled.push("".to_string());
            if SHOW_CALL_NODES {
                styled.push(
                    "classDef callNode fill:#fffde7,stroke:#f9a825,stroke-width:2px,color:#000"
                        .to_string(),
                );
            }
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

        // Build child scope roots from Hid edges directly
        let mut nested_targets: BTreeSet<&str> = BTreeSet::new();
        let mut container_to_children: HashMap<&str, Vec<&str>> = HashMap::new();
        for (p_hid, c_hid) in index.nested_edges_hid_iter() {
            if let (Some(parent_hdr), Some(child_hdr)) =
                (index.get_by_hid(*p_hid), index.get_by_hid(*c_hid))
            {
                if parent_hdr.scope != child_hdr.scope {
                    if let Some(root_hdr) = index.headers_in_scope_iter(child_hdr.scope).next() {
                        if root_hdr.id == child_hdr.id {
                            nested_targets.insert(child_hdr.id.as_str());
                            container_to_children
                                .entry(parent_hdr.id.as_str())
                                .or_default()
                                .push(child_hdr.id.as_str());
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
            children.sort_by(|a, b| {
                a.span
                    .file
                    .path_buf()
                    .cmp(b.span.file.path_buf())
                    .then(a.span.start.cmp(&b.span.start))
            });
        }

        // Determine top-level scopes: those whose root is not a nested target
        // Collect scopes present
        let mut top_scopes: Vec<(&std::path::Path, usize, usize, ScopeId)> = Vec::new();
        let mut seen_scopes: HashSet<ScopeId> = HashSet::new();
        for h in &index.headers {
            if seen_scopes.insert(h.scope) {
                if let Some(root) = index.headers_in_scope_iter(h.scope).next() {
                    if !nested_targets.contains(root.id.as_str()) {
                        let p = root.span.file.path_buf().as_path();
                        top_scopes.push((p, root.span.start, root.span.end, root.scope));
                    }
                }
            }
        }
        top_scopes.sort_by(|a, b| a.0.cmp(b.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));

        for (_p, _s, _e, scope) in top_scopes {
            self.render_scope_sequence(
                index,
                scope,
                &by_id,
                &container_to_children,
                &markdown_children,
                &has_markdown_parent,
            );
        }

        // After rendering all nodes and edges, append a JSON mapping comment and click callbacks
        if !self.rep_id_to_span.is_empty() {
            if let Ok(json) = serde_json::to_string(&self.rep_id_to_span) {
                // Mermaid ignores %% comments; frontend can parse this
                self.content.push(format!("%%__BAML_SPANMAP__={}", json));
            }
            // Emit click callbacks for each representative id
            // Use a shared global callback name expected by the frontend: bamlMermaidNodeClick
            for rep_id in self.rep_id_to_span.keys() {
                self.content.push(format!(
                    "  click {} call bamlMermaidNodeClick() \"Go to source\"",
                    rep_id
                ));
            }
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
            .headers_in_scope_iter(scope)
            .filter(|h| !has_markdown_parent.contains(h.id.as_str()))
            .collect();
        // Sort by source order for stability
        items.sort_by(|a, b| {
            a.span
                .file
                .path_buf()
                .cmp(b.span.file.path_buf())
                .then(a.span.start.cmp(&b.span.start))
        });

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
                        self.connect_pair(&end_id, next, 0);
                    }
                } else {
                    self.connect_pair(cur, next, 0);
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
        if let Some(rep) = self.header_rep_cache.get(header.id.as_str()) {
            return rep.clone();
        }
        let has_md_children = markdown_children
            .get(header.id.as_str())
            .map(|v| !v.is_empty())
            .unwrap_or(false);
        let nested_children: Vec<&str> = container_to_children
            .get(header.id.as_str())
            .cloned()
            .unwrap_or_default();
        let has_nested_children = !nested_children.is_empty();
        let is_if_label = header.label_kind == HeaderLabelKind::If;
        let is_branching = is_if_label && nested_children.len() >= 1;
        let _is_top_level_scope = false; // simplified: no special top-level handling

        if !has_md_children && !has_nested_children {
            // Simple leaf node: container context (if any) will handle call rendering
            let rep = if is_if_label {
                self.ensure_decision_node(index, header)
            } else {
                self.ensure_node_styled(index, header)
            };
            self.register_rep_span(&rep, &header.span);
            self.header_rep_cache.insert(header.id.clone(), rep.clone());
            return rep;
        }

        // Flatten rule: if a non-branching container has only one (or zero) child,
        // avoid creating a subgraph and render as a simple linear sequence instead.
        let md_children_count = markdown_children
            .get(header.id.as_str())
            .map(|v| v.len())
            .unwrap_or(0);
        let nested_children_count = nested_children.len();

        // If we have exactly one nested child, but that child scope contains multiple headers,
        // treat it as multi-child to avoid collapsing (e.g., for-loop bodies).
        let mut single_nested_child_has_multiple_items = false;
        if nested_children_count == 1 {
            if let Some(child_id) = nested_children.get(0) {
                if let Some(child_root) = by_id.get(*child_id) {
                    let child_scope = child_root.scope;
                    let num_in_child_scope = index.headers_in_scope_iter(child_scope).count();
                    if num_in_child_scope > 1 {
                        single_nested_child_has_multiple_items = true;
                    }
                }
            }
        }

        let total_children = md_children_count + nested_children_count;
        let should_flatten = !is_branching
            && total_children <= MAX_CHILDREN_TO_FLATTEN
            && !(nested_children_count == 1 && single_nested_child_has_multiple_items);

        if should_flatten {
            // Render the parent header as a simple node (or decision if label indicates)
            let parent_rep_id = if is_if_label {
                self.ensure_decision_node(index, header)
            } else {
                self.ensure_node_styled(index, header)
            };
            self.register_rep_span(&parent_rep_id, &header.span);
            // Render calls associated with this header at the current scope level
            self.render_calls_for_header(index, header, None, &parent_rep_id, 0);

            // If there's exactly one child, render it and connect parent -> child,
            // and carry the child's endpoints outward so the next sibling connects after it.
            if md_children_count == 1 {
                if let Some(md_children) = markdown_children.get(header.id.as_str()) {
                    if let Some(&child) = md_children.get(0) {
                        let child_rep_id = self.render_header_with_hierarchy(
                            index,
                            child,
                            by_id,
                            container_to_children,
                            markdown_children,
                            has_markdown_parent,
                        );
                        self.connect_pair(&parent_rep_id, &child_rep_id, 0);
                        let endpoints = self
                            .branch_endpoints
                            .get(&child_rep_id)
                            .cloned()
                            .unwrap_or_else(|| vec![child_rep_id.clone()]);
                        self.branch_endpoints
                            .insert(parent_rep_id.clone(), endpoints);
                    }
                }
            } else if nested_children_count == 1 {
                if let Some(child_id) = nested_children.get(0) {
                    if let Some(child_root) = by_id.get(*child_id) {
                        let child_scope = child_root.scope;
                        // Ensure the nested child scope is rendered
                        self.render_scope_sequence(
                            index,
                            child_scope,
                            by_id,
                            container_to_children,
                            markdown_children,
                            has_markdown_parent,
                        );

                        // Determine representative for the child start to connect from parent
                        let start_rep_id = self.render_header_with_hierarchy(
                            index,
                            child_root,
                            by_id,
                            container_to_children,
                            markdown_children,
                            has_markdown_parent,
                        );
                        self.connect_pair(&parent_rep_id, &start_rep_id, 0);

                        // Render calls for the child at the current scope level anchored to its rep id
                        self.render_calls_for_header(index, child_root, None, &start_rep_id, 0);

                        // Determine the terminal representative in the child scope
                        // Determine the terminal representative in the child scope
                        let scope_headers: Vec<&RenderableHeader> =
                            index.headers_in_scope_iter(child_scope).collect();
                        if let Some(last_hdr) = scope_headers.last() {
                            // Use the child's own endpoints if any; otherwise, its rep id
                            let last_rep = self.render_header_with_hierarchy(
                                index,
                                last_hdr,
                                by_id,
                                container_to_children,
                                markdown_children,
                                has_markdown_parent,
                            );
                            let endpoints = self
                                .branch_endpoints
                                .get(&last_rep)
                                .cloned()
                                .unwrap_or_else(|| vec![last_rep.clone()]);
                            self.branch_endpoints
                                .insert(parent_rep_id.clone(), endpoints);
                        } else {
                            self.branch_endpoints
                                .insert(parent_rep_id.clone(), vec![start_rep_id.clone()]);
                        }
                    }
                }
            }

            self.header_rep_cache
                .insert(header.id.clone(), parent_rep_id.clone());
            return parent_rep_id;
        }

        // Special case: branching header (e.g., if statement). Render as a container
        // subgraph that contains a decision node, the branch scopes, and any markdown
        // children following the branch. Branch endpoints are either connected to the
        // first markdown child (if present) or carried outward to connect to the next
        // sibling in the parent sequence.
        if is_branching {
            // Prepare a subgraph to represent the branching header as a container
            self.ensure_subgraph(header);
            let subgraph_id = self
                .header_subgraph_ids
                .get(header.id.as_str())
                .cloned()
                .expect("subgraph id must exist");

            // Decision node representing the branching header (defined outside to be in scope for return)
            let decision_id = self.ensure_decision_node(index, header);
            self.register_rep_span(&decision_id, &header.span);

            let should_render_subgraph = self.emitted_subgraphs.insert(subgraph_id.clone());
            if should_render_subgraph {
                // Open container
                self.content.push(format!(
                    "  subgraph {}[\"{}\"]",
                    subgraph_id,
                    escape_label(&header.title)
                ));
                self.content.push("    direction LR".to_string());
                // Render calls attached to this header (if any) within the container
                self.render_calls_for_header(index, header, Some(&subgraph_id), &decision_id, 4);

                // Render branch child scopes and connect from decision node to branch starts
                let mut branch_last_ids: Vec<String> = Vec::new();
                for child_id in nested_children {
                    if let Some(child_root) = by_id.get(child_id) {
                        let child_scope = child_root.scope;
                        self.render_scope_sequence(
                            index,
                            child_scope,
                            by_id,
                            container_to_children,
                            markdown_children,
                            has_markdown_parent,
                        );
                        // Determine representative start id for this branch (avoid subgraph ids)
                        let start_rep_id = self.render_header_with_hierarchy(
                            index,
                            child_root,
                            by_id,
                            container_to_children,
                            markdown_children,
                            has_markdown_parent,
                        );
                        // Connect decision node -> branch start
                        self.connect_pair(&decision_id, &start_rep_id, 4);

                        // Render calls for the branch root header inside this container
                        self.render_calls_for_header(
                            index,
                            child_root,
                            Some(&subgraph_id),
                            &start_rep_id,
                            4,
                        );

                        // Compute outward endpoints for this branch scope: from last header's rep id
                        let scope_headers: Vec<&RenderableHeader> =
                            index.headers_in_scope_iter(child_scope).collect();
                        if let Some(last_hdr) = scope_headers.last() {
                            let last_rep = self.render_header_with_hierarchy(
                                index,
                                last_hdr,
                                by_id,
                                container_to_children,
                                markdown_children,
                                has_markdown_parent,
                            );
                            let endpoints = self
                                .branch_endpoints
                                .get(&last_rep)
                                .cloned()
                                .unwrap_or_else(|| vec![last_rep.clone()]);
                            branch_last_ids.extend(endpoints);
                        }
                    }
                }

                // Now render markdown children for this header and connect them linearly
                // Track both rep ids and headers to avoid re-emitting style when rendering call nodes
                let mut md_rep_and_headers: Vec<(
                    (&std::path::Path, usize),
                    String,
                    &RenderableHeader,
                )> = Vec::new();
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
                        let pos_key = (ch.span.file.path_buf().as_path(), ch.span.start);
                        md_rep_and_headers.push((pos_key, rep_id, *ch));
                    }
                }
                md_rep_and_headers.sort_by(|a, b| a.0.cmp(&b.0));
                let md_ids_only: Vec<String> = md_rep_and_headers
                    .iter()
                    .map(|(_, id, _)| id.clone())
                    .collect();

                if !md_ids_only.is_empty() {
                    // Connect branch endpoints to the first markdown child, or the decision to it if no branches
                    let first_md = md_ids_only[0].clone();
                    if !branch_last_ids.is_empty() {
                        for end_id in branch_last_ids.iter().cloned() {
                            self.connect_pair(&end_id, &first_md, 4);
                        }
                    } else {
                        self.connect_pair(&decision_id, &first_md, 4);
                    }
                    // Connect markdown children linearly
                    if md_ids_only.len() >= 2 {
                        for i in 0..md_ids_only.len() - 1 {
                            let cur = &md_ids_only[i];
                            let next = &md_ids_only[i + 1];
                            if let Some(branch_ends) = self.branch_endpoints.get(cur).cloned() {
                                for end_id in branch_ends {
                                    self.connect_pair(&end_id, next, 4);
                                }
                            } else {
                                self.connect_pair(cur, next, 4);
                            }
                        }
                    }
                    // Propagate container endpoints outward via decision node
                    let last_md = md_ids_only.last().cloned().unwrap();
                    let outward = self
                        .branch_endpoints
                        .get(&last_md)
                        .cloned()
                        .unwrap_or_else(|| vec![last_md.clone()]);
                    self.branch_endpoints.insert(decision_id.clone(), outward);
                } else {
                    // No markdown children; carry branch endpoints outward via the container rep id
                    if !branch_last_ids.is_empty() {
                        self.branch_endpoints
                            .insert(decision_id.clone(), branch_last_ids);
                    }
                }

                // Render call targets for markdown children using their existing rep ids
                for (_pos, rep_id, ch) in md_rep_and_headers.iter() {
                    self.render_calls_for_header(index, ch, Some(&subgraph_id), rep_id, 4);
                }

                // Close container
                self.content.push("  end".to_string());
            }

            self.header_rep_cache
                .insert(header.id.clone(), decision_id.clone());
            return decision_id;
        }

        // Ensure subgraph id exists for non-branching containers (markdown hierarchy or single nested child)
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
            let mut md_rep_and_headers: Vec<(
                (&std::path::Path, usize),
                String,
                &RenderableHeader,
            )> = Vec::new();
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
                    let pos_key = (ch.span.file.path_buf().as_path(), ch.span.start);
                    md_rep_and_headers.push((pos_key, rep_id, *ch));
                }
            }
            md_rep_and_headers.sort_by(|a, b| a.0.cmp(&b.0));
            let md_ids_only: Vec<String> = md_rep_and_headers
                .iter()
                .map(|(_, id, _)| id.clone())
                .collect();
            // Connect markdown children linearly, but if a child is a branching node,
            // connect each branch endpoint to the next markdown sibling
            if md_ids_only.len() >= 2 {
                for i in 0..md_ids_only.len() - 1 {
                    let cur = &md_ids_only[i];
                    let next = &md_ids_only[i + 1];
                    if let Some(branch_ends) = self.branch_endpoints.get(cur).cloned() {
                        for end_id in branch_ends {
                            self.connect_pair(&end_id, next, 4);
                        }
                    } else {
                        self.connect_pair(cur, next, 4);
                    }
                }
            }

            // 2) Render nested child scopes (e.g., loop bodies) and collect their start reps and end endpoints
            let mut nested_rep_with_pos: Vec<(
                (&std::path::Path, usize),
                String,
                Vec<String>,
                &RenderableHeader,
            )> = Vec::new();
            if let Some(nested_child_ids) = container_to_children.get(header.id.as_str()) {
                let mut nested_children_headers: Vec<&RenderableHeader> = nested_child_ids
                    .iter()
                    .filter_map(|cid| by_id.get(*cid).copied())
                    .collect();
                nested_children_headers.sort_by(|a, b| {
                    a.span
                        .file
                        .path_buf()
                        .cmp(b.span.file.path_buf())
                        .then(a.span.start.cmp(&b.span.start))
                });

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
                    // Representative for nested child root
                    let start_rep_id = self.render_header_with_hierarchy(
                        index,
                        child_root,
                        by_id,
                        container_to_children,
                        markdown_children,
                        has_markdown_parent,
                    );
                    // Register span for the representative of this nested child root
                    self.register_rep_span(&start_rep_id, &child_root.span);
                    let pos_key = (
                        child_root.span.file.path_buf().as_path(),
                        child_root.span.start,
                    );
                    // Determine end endpoints for this nested child scope (last header's rep or its endpoints)
                    let scope_headers: Vec<&RenderableHeader> =
                        index.headers_in_scope_iter(child_scope).collect();
                    let end_endpoints: Vec<String> = if let Some(last_hdr) = scope_headers.last() {
                        let last_rep = self.render_header_with_hierarchy(
                            index,
                            last_hdr,
                            by_id,
                            container_to_children,
                            markdown_children,
                            has_markdown_parent,
                        );
                        self.branch_endpoints
                            .get(&last_rep)
                            .cloned()
                            .unwrap_or_else(|| vec![last_rep])
                    } else {
                        vec![start_rep_id.clone()]
                    };
                    nested_rep_with_pos.push((
                        pos_key,
                        start_rep_id.clone(),
                        end_endpoints,
                        child_root,
                    ));

                    // Render call targets for nested child root header inside this container
                    self.render_calls_for_header(
                        index,
                        child_root,
                        Some(&subgraph_id),
                        &start_rep_id,
                        4,
                    );
                }
            }

            // Render any calls associated with this header within the container, anchored to the container's first internal rep id
            // Determine first rep id and outward endpoints across markdown and nested children
            let mut all_items: Vec<((&std::path::Path, usize), String, Vec<String>)> = Vec::new();
            for (pos, id, _h) in md_rep_and_headers.iter() {
                let endpoints = self
                    .branch_endpoints
                    .get(id)
                    .cloned()
                    .unwrap_or_else(|| vec![id.clone()]);
                all_items.push((*pos, id.clone(), endpoints));
            }
            for (pos, start_id, end_endpoints, _h) in nested_rep_with_pos.iter() {
                all_items.push((*pos, start_id.clone(), end_endpoints.clone()));
            }
            all_items.sort_by(|a, b| a.0.cmp(&b.0));

            let (container_first_rep_id, outward_endpoints) = if !all_items.is_empty() {
                let first = all_items.first().unwrap().1.clone();
                let last_endpoints = all_items.last().unwrap().2.clone();
                (first, last_endpoints)
            } else {
                // Fallback to a styled node for the container header if somehow empty
                let fallback = self.ensure_node_styled(index, header);
                self.register_rep_span(&fallback, &header.span);
                (fallback.clone(), vec![fallback])
            };

            // Do not render calls for the container header itself in non-branching containers;
            // calls should be associated with concrete child headers within.

            // Render calls for markdown children using their existing rep ids
            for (_pos, rep_id, ch) in md_rep_and_headers.iter() {
                self.render_calls_for_header(index, ch, Some(&subgraph_id), rep_id, 4);
            }

            // Close subgraph
            self.content.push("  end".to_string());

            // Propagate container endpoints outward: from first internal rep id to the last's endpoints
            self.branch_endpoints
                .insert(container_first_rep_id.clone(), outward_endpoints);
            // Cache before returning
            self.header_rep_cache
                .insert(header.id.clone(), container_first_rep_id.clone());
            // Return the first internal representative id as the container's rep id
            return container_first_rep_id;
        }

        // If subgraph already rendered earlier, still return a sensible representative
        // Choose the first header in this scope as representative
        if let Some(first_md) = markdown_children
            .get(header.id.as_str())
            .and_then(|v| v.first().copied())
        {
            let rep = self.render_header_with_hierarchy(
                index,
                first_md,
                by_id,
                container_to_children,
                markdown_children,
                has_markdown_parent,
            );
            self.register_rep_span(&rep, &first_md.span);
            self.header_rep_cache.insert(header.id.clone(), rep.clone());
            return rep;
        }
        // Otherwise, if there are nested children, use the first one's representative
        if let Some(first_nested_id) = container_to_children
            .get(header.id.as_str())
            .and_then(|v| v.first().copied())
        {
            if let Some(child_root) = by_id.get(first_nested_id) {
                let rep = self.render_header_with_hierarchy(
                    index,
                    child_root,
                    by_id,
                    container_to_children,
                    markdown_children,
                    has_markdown_parent,
                );
                self.register_rep_span(&rep, &child_root.span);
                self.header_rep_cache.insert(header.id.clone(), rep.clone());
                return rep;
            }
        }
        // Fallback
        let rep = self.ensure_node_styled(index, header);
        self.register_rep_span(&rep, &header.span);
        self.header_rep_cache.insert(header.id.clone(), rep.clone());
        rep
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
    fn ensure_node_styled(&mut self, _index: &HeaderIndex, header: &RenderableHeader) -> String {
        let id = self.ensure_node(header);
        // styling based on scope kind removed in simplified model
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
        // Apply decisionNode class once
        if self.styled_node_ids.insert(id.clone()) {
            self.content.push(format!("  class {} decisionNode;", id));
        }
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

    /// Connect a single pair of ids without allocating a temporary Vec
    fn connect_pair(&mut self, a: &str, b: &str, indent_spaces: usize) {
        let indent = " ".repeat(indent_spaces);
        if self.emitted_edges.insert((a.to_string(), b.to_string())) {
            self.content.push(format!("{}{} --> {}", indent, a, b));
        }
    }

    fn next_id(&mut self, prefix: &str) -> String {
        let id = format!("{}{}", prefix, self.id_counter);
        self.id_counter += 1;
        id
    }

    fn register_rep_span(&mut self, rep_id: &str, span: &internal_baml_diagnostics::Span) {
        // Only record once per rep id
        if self.rep_id_to_span.contains_key(rep_id) {
            return;
        }
        let serialized = SerializedSpan::serialize(span);
        self.rep_id_to_span.insert(rep_id.to_string(), serialized);
    }
}

#[inline]
fn escape_label(s: &str) -> String {
    s.replace('"', "&quot;")
}

impl BamlVisDiagramGenerator {
    /// Render helper call nodes for a header and connect from the header representative id.
    /// If `container_subgraph_id` is provided, nodes are deduped within that container; otherwise, globally.
    fn render_calls_for_header(
        &mut self,
        index: &HeaderIndex,
        header: &RenderableHeader,
        container_subgraph_id: Option<&str>,
        header_rep_id: &str,
        indent_spaces: usize,
    ) {
        if !SHOW_CALL_NODES {
            return;
        }
        if let Some(callees) = index.header_calls.get(&header.hid) {
            for callee in callees {
                let call_node_id = if let Some(_container_id) = container_subgraph_id {
                    // Deduplicate per header within a container, not across different headers
                    let key = (format!("hdr:{}", header.id), callee.clone());
                    if let Some(id) = self.call_node_ids.get(&key) {
                        id.clone()
                    } else {
                        let id = self.next_id("n");
                        self.call_node_ids.insert(key, id.clone());
                        self.content.push(format!(
                            "{}{}[\"{}\"]",
                            " ".repeat(indent_spaces),
                            id,
                            escape_label(callee)
                        ));
                        if SHOW_CALL_NODES {
                            self.content.push(format!(
                                "{}class {} callNode;",
                                " ".repeat(indent_spaces),
                                id
                            ));
                        }
                        id
                    }
                } else {
                    // Deduplicate per header at the top level as well
                    let key = (format!("hdr:{}", header.id), callee.clone());
                    if let Some(id) = self.call_node_ids.get(&key) {
                        id.clone()
                    } else {
                        let id = self.next_id("n");
                        self.call_node_ids.insert(key, id.clone());
                        self.content
                            .push(format!("  {}[\"{}\"]", id, escape_label(callee)));
                        if SHOW_CALL_NODES {
                            self.content.push(format!("  class {} callNode;", id));
                        }
                        id
                    }
                };
                // Reverse: function (callee) -> header (caller)
                self.connect_pair(&call_node_id, header_rep_id, indent_spaces);
            }
        }
    }
}
