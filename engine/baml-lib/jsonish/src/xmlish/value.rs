use baml_types::CompletionState;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// XML value representation that can handle incomplete/streaming XML data
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Value {
    /// Text content within XML elements
    Text(String, CompletionState),
    /// XML element with tag name, attributes, and children
    Element {
        tag: String,
        attributes: HashMap<String, String>,
        children: Vec<Value>,
        completion_state: CompletionState,
    },
    /// Raw XML fragment that couldn't be parsed completely
    Fragment(String, CompletionState),
    /// Multiple possible interpretations of the XML
    AnyOf(Vec<Value>, String),
}

impl Value {
    /// Get the completion state of this value
    pub fn completion_state(&self) -> &CompletionState {
        match self {
            Value::Text(_, state) => state,
            Value::Element { completion_state, .. } => completion_state,
            Value::Fragment(_, state) => state,
            Value::AnyOf(values, _) => {
                // If any value is complete, consider the whole thing complete
                values.iter().find_map(|v| match v.completion_state() {
                    CompletionState::Complete => Some(&CompletionState::Complete),
                    _ => None,
                }).unwrap_or(&CompletionState::Incomplete)
            }
        }
    }

    /// Create a new text value
    pub fn text(content: String) -> Self {
        Value::Text(content, CompletionState::Complete)
    }

    /// Create a new element value
    pub fn element(tag: String, attributes: HashMap<String, String>, children: Vec<Value>) -> Self {
        Value::Element {
            tag,
            attributes,
            children,
            completion_state: CompletionState::Complete,
        }
    }

    /// Create a new incomplete element
    pub fn incomplete_element(tag: String, attributes: HashMap<String, String>, children: Vec<Value>) -> Self {
        Value::Element {
            tag,
            attributes,
            children,
            completion_state: CompletionState::Incomplete,
        }
    }

    /// Create a new fragment
    pub fn fragment(content: String) -> Self {
        Value::Fragment(content, CompletionState::Incomplete)
    }
}