use serde::{Deserialize, Serialize};

use crate::{PathSegment, RuntimeNodeType, VizExecEvent};

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
    /// Stable node id (matches control_flow.rs pre-order allocation).
    pub node_id: u32,
    pub lexical_id: String,
    pub new_state: LexicalState,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Frame {
    pub node_id: u32,
    pub lexical_segment: PathSegment,
    pub lexical_id: String,
    pub node_type: RuntimeNodeType,
    pub label: String,
    pub header_level: Option<u8>,
}

/// Simple reducer that models a lexical node's lifecycle.
#[derive(Default, Debug)]
pub struct VizStateReducer {
    frames_by_function: std::collections::HashMap<String, Vec<Frame>>,
}

impl VizStateReducer {
    /// Apply a single viz exec event and return the resulting state updates.
    pub fn apply(&mut self, function_name: &str, viz_event: &VizExecEvent) -> Vec<StateUpdate> {
        if !self.frames_by_function.contains_key(function_name) {
            self.frames_by_function
                .insert(function_name.to_string(), Vec::new());
        }
        let frames = self
            .frames_by_function
            .get_mut(function_name)
            .expect("just inserted or existed");
        Self::dispatch(function_name, frames, viz_event)
    }

    /// Current stack of frames (root at index 0) for all functions.
    pub fn dump(&self) -> Vec<Frame> {
        self.frames_by_function
            .values()
            .flat_map(|f| f.clone())
            .collect()
    }

    /// Current lexical_id stack (root at index 0) for all functions (flattened).
    pub fn lexical_stack(&self) -> Vec<String> {
        self.frames_by_function
            .values()
            .flat_map(|f| f.iter().map(|fr| fr.lexical_id.clone()))
            .collect()
    }

    fn dispatch(
        function_name: &str,
        frames: &mut Vec<Frame>,
        viz_event: &VizExecEvent,
    ) -> Vec<StateUpdate> {
        match viz_event.event {
            crate::VizExecDelta::Enter => handle_enter(function_name, frames, viz_event),
            crate::VizExecDelta::Exit => handle_exit(frames, viz_event),
        }
    }
}

fn handle_enter(
    function: &str,
    frames: &mut Vec<Frame>,
    viz_event: &VizExecEvent,
) -> Vec<StateUpdate> {
        let mut updates = Vec::new();

        // Pop headers at or deeper than the incoming header level; this mirrors
        // HirTraversalContext::pop_headers_to_level.
        if viz_event.node_type == RuntimeNodeType::HeaderContextEnter {
            let new_level = viz_event.header_level.unwrap_or(1).max(1);
            while let Some(top) = frames.last() {
                if top.node_type == RuntimeNodeType::HeaderContextEnter {
                    let top_level = top.header_level.unwrap_or(1);
                    if top_level >= new_level {
                        let frame = frames.pop().expect("frame to exist");
                        updates.push(StateUpdate {
                            node_id: frame.node_id,
                            lexical_id: frame.lexical_id.clone(),
                            new_state: LexicalState::Completed,
                        });
                        continue;
                    }
                }
                break;
            }
        }

    let lexical_id = {
        let mut segments: Vec<PathSegment> =
            frames.iter().map(|f| f.lexical_segment.clone()).collect();
        segments.push(viz_event.path_segment.clone());
        encode_segments(function, &segments)
    };

        let frame = Frame {
            node_id: viz_event.node_id,
            lexical_segment: viz_event.path_segment.clone(),
            lexical_id: lexical_id.clone(),
            node_type: viz_event.node_type.clone(),
            label: viz_event.label.clone(),
            header_level: viz_event.header_level,
        };
        frames.push(frame);

        updates.push(StateUpdate {
            node_id: viz_event.node_id,
            lexical_id,
            new_state: LexicalState::Running,
        });

        updates
    }

fn handle_exit(frames: &mut Vec<Frame>, viz_event: &VizExecEvent) -> Vec<StateUpdate> {
        let mut updates = Vec::new();
        let mut popped: Vec<Frame> = Vec::new();
        let mut found = false;

        while let Some(frame) = frames.pop() {
            let matches = frame.lexical_segment == viz_event.path_segment;
            popped.push(frame);
            if matches {
                found = true;
                break;
            }
        }

        if !found {
            // No matching frame; restore stack and emit nothing.
            for frame in popped.into_iter().rev() {
                frames.push(frame);
            }
            return updates;
        }

        for frame in popped {
            updates.push(StateUpdate {
                node_id: frame.node_id,
                lexical_id: frame.lexical_id.clone(),
                new_state: LexicalState::Completed,
            });
        }

        updates
}

/// Encode a full lexical id from function name + segments (matches control_flow.rs).
pub fn encode_segments(function: &str, segments: &[PathSegment]) -> String {
    let mut encoded = String::from(function);
    for segment in segments {
        encoded.push('|');
        encoded.push_str(&segment.encode());
    }
    encoded
}
