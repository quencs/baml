//! BAML Executor - Runtime execution engine for BAML functions.
//!
//! This crate provides the execution infrastructure for BAML functions,
//! including prompt rendering, LLM communication, and response parsing.
//!
//! # Architecture
//!
//! The executor follows a pipeline architecture:
//!
//! 1. `prepare_function()` - Validates inputs and prepares execution context
//! 2. `render_prompt()` - Renders the Jinja template to provider-agnostic messages
//! 3. `build_provider_request()` - Converts to provider-specific format (e.g., OpenAI)
//! 4. `resolve_media()` - Resolves media URLs to inline data if needed
//! 5. `execute()` - Sends HTTP request to LLM provider
//! 6. `parse_response()` - Parses provider response to unified format
//! 7. `parse_output()` - Parses LLM content to BAML types
//!
//! # Feature Flags
//!
//! - `native`: Enables native async runtime with tokio and reqwest (default)
//! - `wasm`: Enables WASM-compatible async with wasm-bindgen-futures

pub mod context;
pub mod errors;
pub mod executor;
pub mod function_lookup;
pub mod llm_request;
pub mod llm_response;
pub mod orchestrator;
pub mod parsing;

mod prepared_function;
mod render_options;
mod types;

pub use errors::*;
pub use executor::*;
pub use prepared_function::*;
pub use render_options::*;
pub use types::*;
