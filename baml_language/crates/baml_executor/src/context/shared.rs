//! Shared call context - session-scoped state.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use crate::types::BamlValue;

/// Session-scoped context, shared across multiple calls.
#[derive(Debug, Clone, Default)]
pub struct SharedCallContext {
    call_stack: Arc<Mutex<Vec<CallStackEntry>>>,
    global_tags: Arc<Mutex<HashMap<String, BamlValue>>>,
}

/// Entry in the call stack.
#[derive(Debug, Clone)]
pub struct CallStackEntry {
    pub call_uuid: uuid::Uuid,
    pub function_name: String,
    pub tags: HashMap<String, BamlValue>,
}

impl SharedCallContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push_call(&self, function_name: impl Into<String>) -> uuid::Uuid {
        let call_uuid = uuid::Uuid::new_v4();
        let entry = CallStackEntry {
            call_uuid,
            function_name: function_name.into(),
            tags: HashMap::new(),
        };

        if let Ok(mut stack) = self.call_stack.lock() {
            stack.push(entry);
        }

        call_uuid
    }

    pub fn pop_call(&self) {
        if let Ok(mut stack) = self.call_stack.lock() {
            stack.pop();
        }
    }

    pub fn call_depth(&self) -> usize {
        self.call_stack.lock().map(|s| s.len()).unwrap_or(0)
    }

    pub fn set_global_tag(&self, key: impl Into<String>, value: BamlValue) {
        if let Ok(mut tags) = self.global_tags.lock() {
            tags.insert(key.into(), value);
        }
    }

    pub fn get_global_tag(&self, key: &str) -> Option<BamlValue> {
        self.global_tags
            .lock()
            .ok()
            .and_then(|tags| tags.get(key).cloned())
    }
}
