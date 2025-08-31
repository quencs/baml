//! BAML Rust Integration Tests
//!
//! This crate contains comprehensive integration tests for the BAML Rust client,
//! testing the CFFI-based client implementation against real BAML functions.

pub mod utils;

// Re-export generated client and types - temporarily disabled
// pub use baml_client::*;

// Re-export commonly used types
pub use serde_json::Value as JsonValue;
pub use std::collections::HashMap;

// Re-export from baml_client_rust
pub use baml_client_rust::{BamlClient, BamlClientBuilder, BamlResult, BamlContext};

/// Test configuration and setup utilities
pub mod test_config {
    use super::*;

    /// Get OpenAI API key from environment or use test key
    pub fn get_openai_api_key() -> String {
        std::env::var("OPENAI_API_KEY").unwrap_or_else(|_| "test-key".to_string())
    }

    /// Setup basic test client with environment configuration
    pub fn setup_test_client() -> BamlResult<BamlClient> {
        BamlClientBuilder::new()
            .env_var("OPENAI_API_KEY", get_openai_api_key())
            .build()
    }

    /// Setup test client from directory
    #[cfg(not(target_arch = "wasm32"))]
    pub fn setup_test_client_from_directory<P: AsRef<std::path::Path>>(
        path: P,
    ) -> BamlResult<BamlClient> {
        let mut env_vars = std::env::vars().collect::<std::collections::HashMap<String, String>>();
        env_vars.insert("OPENAI_API_KEY".to_string(), get_openai_api_key());

        BamlClient::from_directory(path, env_vars)
    }
}

/// Initialize logging for tests
pub fn init_test_logging() {
    let _ = env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .is_test(true)
        .try_init();
}
