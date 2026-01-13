//! Per-call context - configuration consumed by each call.

use std::collections::HashMap;

use crate::types::BamlValue;

/// Per-call configuration, consumed by each call.
#[derive(Debug, Clone, Default)]
pub struct PerCallContext {
    /// Environment variables for this call.
    pub env_vars: HashMap<String, String>,
    /// Per-call tags.
    pub tags: HashMap<String, BamlValue>,
    /// Whether the call has been cancelled.
    cancelled: bool,
}

impl PerCallContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_env_vars(mut self, env_vars: HashMap<String, String>) -> Self {
        self.env_vars = env_vars;
        self
    }

    pub fn with_tag(mut self, key: impl Into<String>, value: BamlValue) -> Self {
        self.tags.insert(key.into(), value);
        self
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled
    }

    pub fn cancel(&mut self) {
        self.cancelled = true;
    }

    pub fn get_env(&self, key: &str) -> Option<&str> {
        self.env_vars.get(key).map(|s| s.as_str())
    }
}
