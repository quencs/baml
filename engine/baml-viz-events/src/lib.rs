mod viz_state_reducer;

use serde::{Deserialize, Serialize};

pub use viz_state_reducer::{Frame, LexicalState, StateUpdate, VizStateReducer};

/// Indicates whether execution is entering or exiting a context node.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VizExecDelta {
    Enter,
    Exit,
}

/// Structural node types that show up in the runtime control-flow visualization.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeNodeType {
    FunctionRoot,
    HeaderContextEnter,
    BranchGroup,
    BranchArm,
    Loop,
    OtherScope,
}

/// A single control-flow context event emitted by the runtime.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct VizExecEvent {
    /// Enter/exit transition.
    pub event: VizExecDelta,
    /// The logical node type being visited.
    pub node_type: RuntimeNodeType,
    /// Unique lexical identifier scoped to the function graph.
    pub lexical_id: String,
    /// Human-readable label for the node (header text or synthetic scope label).
    pub label: String,
    /// Header level (only meaningful for header nodes).
    pub header_level: Option<u8>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_viz_exec_event() {
        let original = VizExecEvent {
            event: VizExecDelta::Enter,
            node_type: RuntimeNodeType::HeaderContextEnter,
            lexical_id: "checkout|hdr:verify-payment:1".to_string(),
            label: "//# Verify payment".to_string(),
            header_level: Some(1),
        };

        let serialized = serde_json::to_string(&original).expect("serialize viz exec event");
        let restored: VizExecEvent =
            serde_json::from_str(&serialized).expect("deserialize viz exec event");

        assert_eq!(restored, original);
    }
}
