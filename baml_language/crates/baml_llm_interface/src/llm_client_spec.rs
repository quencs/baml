//! LLM client specification types.

use indexmap::IndexMap;
use serde::Serialize;
use std::collections::HashMap;

/// Specification for an LLM client.
///
/// This describes the capabilities and configuration of an LLM client,
/// including role handling and provider-specific options.
#[derive(Clone, Debug, Serialize)]
pub struct LlmClientSpec {
    /// The name of the client.
    pub name: String,
    /// The provider (e.g., "openai", "anthropic").
    pub provider: String,
    /// Default role for messages without explicit role.
    pub default_role: String,
    /// Allowed roles for this client.
    pub allowed_roles: Vec<String>,
    /// Role remapping (e.g., "user" -> "human" for Anthropic).
    pub remap_role: HashMap<String, String>,
    /// Additional client options.
    pub options: IndexMap<String, serde_json::Value>,
}

impl Default for LlmClientSpec {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
            provider: "openai".to_string(),
            default_role: "system".to_string(),
            allowed_roles: vec![
                "system".to_string(),
                "user".to_string(),
                "assistant".to_string(),
            ],
            remap_role: HashMap::new(),
            options: IndexMap::new(),
        }
    }
}
