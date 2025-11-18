use std::collections::{BTreeMap, HashMap};

use serde::{Deserialize, Serialize};

use crate::{RuntimeNodeType, VizExecDelta, VizExecEvent};

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

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReducerState {
    pub nodes: BTreeMap<String, LexicalState>,
    pub frames: Vec<Frame>,
}

/// Simple reducer that models a lexical node's lifecycle.
#[derive(Default, Debug)]
pub struct VizStateReducer {
    states: HashMap<String, LexicalState>,
    frames: Vec<Frame>,
}

impl VizStateReducer {
    pub fn apply(&mut self, lexical_id: String, viz_event: &VizExecEvent) -> Vec<StateUpdate> {
        let next = match viz_event.event {
            VizExecDelta::Enter => LexicalState::Running,
            VizExecDelta::Exit => LexicalState::Completed,
        };

        match viz_event.event {
            VizExecDelta::Enter => self.frames.push(Frame {
                lexical_id: viz_event.lexical_id.clone(),
                node_type: viz_event.node_type.clone(),
                label: viz_event.label.clone(),
                header_level: viz_event.header_level,
            }),
            VizExecDelta::Exit => {
                while let Some(frame) = self.frames.pop() {
                    if frame.lexical_id == viz_event.lexical_id {
                        break;
                    }
                }
            }
        }

        self.states.insert(lexical_id.clone(), next);
        vec![StateUpdate {
            lexical_id,
            new_state: next,
        }]
    }

    pub fn dump(&self) -> ReducerState {
        ReducerState {
            nodes: self
                .states
                .iter()
                .map(|(key, state)| (key.clone(), *state))
                .collect(),
            frames: self.frames.clone(),
        }
    }
}
