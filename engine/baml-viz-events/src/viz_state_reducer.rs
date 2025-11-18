use serde::{Deserialize, Serialize};

use crate::{RuntimeNodeType, VizExecEvent};

/// Tracking state for a lexical node while replaying runtime events.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LexicalState {
    NotRunning,
    Running,
    Completed,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct StateUpdate {
    pub lexical_id: String,
    pub new_state: LexicalState,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Frame {
    pub lexical_id: String,
    pub node_type: RuntimeNodeType,
    pub label: String,
    pub header_level: Option<u8>,
}

/// Simple reducer that models a lexical node's lifecycle.
#[derive(Default, Debug)]
pub struct VizStateReducer {
    frames: Vec<Frame>,
}

impl VizStateReducer {
    pub fn apply(&mut self, _viz_event: &VizExecEvent) -> Vec<StateUpdate> {
        vec![]
    }

    pub fn dump(&self) -> Vec<Frame> {
        self.frames.clone()
    }
}
