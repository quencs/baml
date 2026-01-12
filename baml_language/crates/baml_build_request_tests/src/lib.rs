//! Test utilities for BAML request building pipeline.
//!
//! This crate provides snapshot tests for:
//! - `render_prompt`: Prompt rendering with Jinja templates
//! - `render_raw_curl`: Raw curl command generation
//! - `build_request`: HTTP request construction
//!
//! ## Naming Convention
//!
//! Test fixtures use PascalCase filenames that directly map to function and test names:
//! - File: `TestCaseName.baml`
//! - Function: `FnTestCaseName`
//! - Test: `TestTestCaseName`
//!
//! For example, `OutputEnum.baml` contains `FnOutputEnum` and `TestOutputEnum`.

use std::path::Path;

use baml_db::{Setter, SourceFile, baml_workspace::Project};
use baml_program::{
    BamlMap, BamlProgram,
    context::DynamicBamlContext,
    prompt::{MediaContent, MessagePart, RenderedMessage, RenderedPrompt, Role},
};
use baml_project::ProjectDatabase as RootDatabase;
use serde::Serialize;

/// Load a BAML file and create a database with proper project setup.
///
/// This creates a database, adds the file, and wires the file into the project
/// so that HIR queries can discover it.
pub fn load_baml_file(content: &str) -> (RootDatabase, SourceFile, Project) {
    let mut db = RootDatabase::default();

    // Create the project first
    let project = db.set_project_root(Path::new("/test"));

    // Add the file to the database
    let source = db.add_file("test.baml", content);

    // Wire the file into the project's file list
    project.set_files(&mut db).to(vec![source]);

    (db, source, project)
}

/// Derive the function name from a PascalCase fixture filename.
///
/// For `OutputEnum.baml`, returns `FnOutputEnum`.
pub fn derive_function_name(fixture_name: &str) -> String {
    let base = fixture_name.trim_end_matches(".baml");
    format!("Fn{}", base)
}

/// Derive the test name from a PascalCase fixture filename.
///
/// For `OutputEnum.baml`, returns `TestOutputEnum`.
pub fn derive_test_name(fixture_name: &str) -> String {
    let base = fixture_name.trim_end_matches(".baml");
    format!("Test{}", base)
}

/// Snapshot of a rendered prompt.
#[derive(Debug, Serialize)]
pub struct PromptSnapshot {
    pub fixture: String,
    pub function: String,
    pub prompt: RenderedPromptSnapshot,
}

/// Serializable version of RenderedPrompt.
#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum RenderedPromptSnapshot {
    Completion { text: String },
    Chat { messages: Vec<ChatMessageSnapshot> },
}

/// Serializable chat message.
#[derive(Debug, Serialize)]
pub struct ChatMessageSnapshot {
    pub role: String,
    pub content: Vec<ChatMessagePartSnapshot>,
}

/// Serializable chat message part.
#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum ChatMessagePartSnapshot {
    Text {
        text: String,
    },
    Media {
        media_type: String,
    },
    WithMeta {
        inner: Box<ChatMessagePartSnapshot>,
        meta: serde_json::Value,
    },
}

impl From<&RenderedPrompt> for RenderedPromptSnapshot {
    fn from(prompt: &RenderedPrompt) -> Self {
        // Check if this looks like a completion (single user message with no chat markers)
        // or a chat prompt (multiple messages or explicit roles)
        if prompt.messages.len() == 1 && prompt.messages[0].role == Role::User {
            // Single user message - treat as completion
            RenderedPromptSnapshot::Completion {
                text: prompt.messages[0].text_content(),
            }
        } else {
            // Multiple messages or explicit roles - treat as chat
            RenderedPromptSnapshot::Chat {
                messages: prompt
                    .messages
                    .iter()
                    .map(ChatMessageSnapshot::from)
                    .collect(),
            }
        }
    }
}

impl From<&RenderedMessage> for ChatMessageSnapshot {
    fn from(msg: &RenderedMessage) -> Self {
        ChatMessageSnapshot {
            role: msg.role.as_str().to_string(),
            content: msg
                .parts
                .iter()
                .map(ChatMessagePartSnapshot::from)
                .collect(),
        }
    }
}

impl From<&MessagePart> for ChatMessagePartSnapshot {
    fn from(part: &MessagePart) -> Self {
        match part {
            MessagePart::Text(text) => ChatMessagePartSnapshot::Text { text: text.clone() },
            MessagePart::Media(media) => ChatMessagePartSnapshot::Media {
                media_type: match media {
                    MediaContent::Url { media_type, .. } => format!("{:?}", media_type),
                    MediaContent::Base64 { media_type, .. } => format!("{:?}", media_type),
                    MediaContent::FilePath { media_type, .. } => format!("{:?}", media_type),
                },
            },
            MessagePart::WithMeta { part, meta } => ChatMessagePartSnapshot::WithMeta {
                inner: Box::new(ChatMessagePartSnapshot::from(part.as_ref())),
                meta: serde_json::to_value(meta).unwrap_or_default(),
            },
        }
    }
}

/// Render a prompt for a fixture file using BamlProgram.
///
/// The function name is derived from the PascalCase fixture name:
/// - `OutputEnum.baml` -> `FnOutputEnum`
pub fn render_prompt_for_fixture(
    baml_content: &str,
    fixture_name: &str,
) -> anyhow::Result<RenderedPrompt> {
    let (db, _source, project) = load_baml_file(baml_content);

    // Create the runtime
    let runtime = BamlProgram::with_project(db, project);

    // Derive function name from fixture name
    let func_name = derive_function_name(fixture_name);

    // Prepare the function with empty args
    let args = BamlMap::new();
    let prepared = runtime
        .prepare_function(&func_name, args)
        .map_err(|e| anyhow::anyhow!("Failed to prepare function '{}': {}", func_name, e))?;

    // Render the prompt through the runtime
    let dynamic_ctx = DynamicBamlContext::new();
    runtime
        .render_prompt(&prepared, &dynamic_ctx)
        .map_err(|e| anyhow::anyhow!("Failed to render prompt: {}", e))
}
