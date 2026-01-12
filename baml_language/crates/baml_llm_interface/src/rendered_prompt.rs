//! Rendered prompt types.

use serde::Serialize;

use crate::ChatMessagePart;

/// A rendered chat message.
#[derive(Debug, PartialEq, Serialize, Clone)]
pub struct RenderedChatMessage {
    /// The role of this message.
    pub role: String,
    /// Whether duplicate roles are allowed.
    pub allow_duplicate_role: bool,
    /// The message parts.
    pub parts: Vec<ChatMessagePart>,
}

/// The result of rendering a prompt template.
#[derive(Debug, PartialEq, Clone, Serialize)]
pub enum RenderedPrompt {
    /// A completion prompt (single string).
    Completion(String),
    /// A chat prompt (multiple messages with roles).
    Chat(Vec<RenderedChatMessage>),
}

impl std::fmt::Display for RenderedPrompt {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            RenderedPrompt::Completion(s) => write!(f, "{s}"),
            RenderedPrompt::Chat(messages) => {
                for message in messages {
                    writeln!(
                        f,
                        "{}: {}",
                        message.role,
                        message
                            .parts
                            .iter()
                            .map(ChatMessagePart::to_string)
                            .collect::<Vec<String>>()
                            .join("")
                    )?;
                }
                Ok(())
            }
        }
    }
}
