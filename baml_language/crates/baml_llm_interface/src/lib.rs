//! LLM interface types for BAML.
//!
//! This crate provides provider-agnostic types for representing prompts
//! and messages sent to LLMs. It decouples the rendering logic from the
//! BAML type system.
//!
//! Main types:
//! - `RenderedPrompt` - The result of rendering a prompt template
//! - `RenderedChatMessage` - A single chat message with role and parts
//! - `ChatMessagePart` - Text, media, or metadata-wrapped content
//! - `LlmClientSpec` - Specification for an LLM client

mod chat_message_part;
mod llm_client_spec;
mod rendered_prompt;

pub use chat_message_part::ChatMessagePart;
pub use llm_client_spec::LlmClientSpec;
pub use rendered_prompt::{RenderedChatMessage, RenderedPrompt};
