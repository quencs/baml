use indexmap::IndexMap;

use super::{ControlFlowVisualization, Edge, Node, NodeId};

/// Result of flattening a `ControlFlowVisualization`.
#[derive(Clone, Debug, Default)]
pub struct FlattenedControlFlowVisualization {
    pub nodes: IndexMap<NodeId, Node>,
    pub edges_by_src: IndexMap<NodeId, Vec<Edge>>,
}

impl From<ControlFlowVisualization> for FlattenedControlFlowVisualization {
    fn from(value: ControlFlowVisualization) -> Self {
        Self {
            nodes: value.nodes,
            edges_by_src: value.edges_by_src,
        }
    }
}

/// Run the three-pass flattening pipeline described in goal5b.
pub fn flatten_control_flow(viz: &ControlFlowVisualization) -> FlattenedControlFlowVisualization {
    let pass_one = remove_implicit_nodes(viz);
    let pass_two = expand_branch_groups(&pass_one);
    let pass_three = flatten_branch_arms_and_scopes(&pass_two);
    pass_three.into()
}

/// Pass 1: remove implicit nodes that do not contribute headerized work.
pub fn remove_implicit_nodes(viz: &ControlFlowVisualization) -> ControlFlowVisualization {
    // Placeholder implementation – real logic arrives in later iterations.
    viz.clone()
}

/// Pass 2: ensure BranchGroup / BranchArm nodes have the correct fan-out edges.
pub fn expand_branch_groups(viz: &ControlFlowVisualization) -> ControlFlowVisualization {
    viz.clone()
}

/// Pass 3: flatten BranchArm / OtherScope nodes so header nodes appear at the correct depth.
pub fn flatten_branch_arms_and_scopes(viz: &ControlFlowVisualization) -> ControlFlowVisualization {
    viz.clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    use internal_baml_core::ast::Span;

    fn sample_viz() -> ControlFlowVisualization {
        let mut viz = ControlFlowVisualization::default();

        let root_id = NodeId::new(0);
        viz.nodes.insert(
            root_id,
            Node::root(
                root_id,
                "root|root:0".to_string(),
                Span::fake(),
                "root".to_string(),
            ),
        );

        viz
    }

    #[test]
    fn remove_implicit_nodes_keeps_empty_graph() {
        let viz = ControlFlowVisualization::default();
        let result = remove_implicit_nodes(&viz);
        assert_eq!(viz.nodes.len(), result.nodes.len());
        assert_eq!(viz.edges_by_src.len(), result.edges_by_src.len());
    }

    #[test]
    fn expand_branch_groups_is_noop_for_now() {
        let viz = sample_viz();
        let result = expand_branch_groups(&viz);
        assert_eq!(viz.nodes.len(), result.nodes.len());
    }

    #[test]
    fn flatten_branch_arms_and_scopes_is_noop_for_now() {
        let viz = sample_viz();
        let result = flatten_branch_arms_and_scopes(&viz);
        assert_eq!(viz.nodes.len(), result.nodes.len());
    }
}
