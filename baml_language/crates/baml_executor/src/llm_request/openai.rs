//! OpenAI-specific request types.

use std::{collections::HashMap, time::Duration};

use baml_llm_interface::{ChatMessagePart, RenderedChatMessage, RenderedPrompt};
use baml_program::MediaContent;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::{errors::BuildRequestError, render_options::RenderOptions};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
}

impl HttpMethod {
    pub fn as_str(&self) -> &str {
        match self {
            HttpMethod::Get => "GET",
            HttpMethod::Post => "POST",
            HttpMethod::Put => "PUT",
            HttpMethod::Delete => "DELETE",
        }
    }
}

#[derive(Debug, Clone)]
pub enum HeaderValue {
    Plain(String),
    Secret(String),
}

impl HeaderValue {
    pub fn render(&self, expose_secrets: bool) -> String {
        match self {
            HeaderValue::Plain(s) => s.clone(),
            HeaderValue::Secret(s) => {
                if expose_secrets {
                    s.clone()
                } else {
                    "[REDACTED]".to_string()
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct OpenAiRequest {
    pub method: HttpMethod,
    pub url: String,
    pub headers: HashMap<String, HeaderValue>,
    pub body: serde_json::Value,
    pub timeout: Option<Duration>,
    pub media_resolved: bool,
    pub stream: bool,
}

#[derive(Debug, Clone)]
pub struct OpenAiClientConfig {
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub timeout: Option<Duration>,
}

impl Default for OpenAiClientConfig {
    fn default() -> Self {
        Self {
            base_url: "https://api.openai.com/v1".to_string(),
            api_key: String::new(),
            model: "gpt-4".to_string(),
            temperature: None,
            max_tokens: None,
            timeout: Some(Duration::from_secs(60)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAiMessage {
    pub role: String,
    pub content: OpenAiContent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OpenAiContent {
    Text(String),
    Parts(Vec<OpenAiContentPart>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum OpenAiContentPart {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image_url")]
    ImageUrl { image_url: OpenAiImageUrl },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAiImageUrl {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

impl OpenAiRequest {
    pub fn from_rendered(
        rendered: &RenderedPrompt,
        config: &OpenAiClientConfig,
        stream: bool,
    ) -> Result<Self, BuildRequestError> {
        let messages: Vec<OpenAiMessage> = match rendered {
            RenderedPrompt::Completion { text } => {
                vec![OpenAiMessage {
                    role: "user".to_string(),
                    content: OpenAiContent::Text(text.clone()),
                }]
            }
            RenderedPrompt::Chat { messages } => {
                messages.iter().map(Self::convert_message).collect()
            }
        };

        let mut body = serde_json::json!({
            "model": config.model,
            "messages": messages,
        });

        if let Some(temp) = config.temperature {
            body["temperature"] = serde_json::json!(temp);
        }
        if let Some(max_tokens) = config.max_tokens {
            body["max_tokens"] = serde_json::json!(max_tokens);
        }
        if stream {
            body["stream"] = serde_json::json!(true);
        }

        let mut headers = HashMap::new();
        headers.insert(
            "Content-Type".to_string(),
            HeaderValue::Plain("application/json".to_string()),
        );
        headers.insert(
            "Authorization".to_string(),
            HeaderValue::Secret(format!("Bearer {}", config.api_key)),
        );

        Ok(Self {
            method: HttpMethod::Post,
            url: format!("{}/chat/completions", config.base_url),
            headers,
            body,
            timeout: config.timeout,
            media_resolved: false,
            stream,
        })
    }

    fn convert_message(msg: &RenderedChatMessage) -> OpenAiMessage {
        let role = msg.role.clone();
        let has_media = msg
            .parts
            .iter()
            .any(|p| matches!(p, ChatMessagePart::Media { .. }));

        let content = if has_media {
            let parts: Vec<OpenAiContentPart> = msg.parts.iter().map(Self::convert_part).collect();
            OpenAiContent::Parts(parts)
        } else {
            let text = msg
                .parts
                .iter()
                .filter_map(|p| p.as_text().cloned())
                .collect::<Vec<_>>()
                .join("");
            OpenAiContent::Text(text)
        };

        OpenAiMessage { role, content }
    }

    fn convert_part(part: &ChatMessagePart) -> OpenAiContentPart {
        match part {
            ChatMessagePart::Text { text } => OpenAiContentPart::Text { text: text.clone() },
            ChatMessagePart::Media { media } => {
                let url = match &media.content {
                    MediaContent::Url(url) => url.clone(),
                    MediaContent::Base64(b64) => {
                        // For simplified MediaContent, assume base64 is already a data URL
                        // or just the raw base64 data. The media_type from MediaKind gives the category.
                        let mime = match media.media_type {
                            baml_program::MediaKind::Image => "image/png",
                            baml_program::MediaKind::Audio => "audio/wav",
                            baml_program::MediaKind::Video => "video/mp4",
                            baml_program::MediaKind::Pdf => "application/pdf",
                        };
                        format!("data:{};base64,{}", mime, b64)
                    }
                    MediaContent::File(path) => {
                        format!("file://{}", path.display())
                    }
                };
                OpenAiContentPart::ImageUrl {
                    image_url: OpenAiImageUrl { url, detail: None },
                }
            }
            ChatMessagePart::WithMeta { inner, .. } => Self::convert_part(inner),
        }
    }

    pub fn has_unresolved_media(&self) -> bool {
        !self.media_resolved
    }

    pub fn to_curl(&self, options: &RenderOptions) -> String {
        let mut parts = vec![format!("curl -X {}", self.method.as_str())];

        for (key, value) in &self.headers {
            let rendered_value = value.render(options.expose_secrets);
            parts.push(format!("-H '{}: {}'", key, rendered_value));
        }

        let body_str = serde_json::to_string_pretty(&self.body).unwrap_or_default();
        parts.push(format!("-d '{}'", body_str.replace('\'', "'\\''")));
        parts.push(format!("'{}'", self.url));

        parts.join(" \\\n  ")
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct OpenAiChatCompletionResponse {
    pub id: String,
    pub object: String,
    #[serde(default)]
    pub created: u64,
    pub model: String,
    pub choices: Vec<OpenAiChoice>,
    #[serde(default)]
    pub usage: Option<OpenAiUsage>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OpenAiChoice {
    pub index: u32,
    pub message: OpenAiResponseMessage,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OpenAiResponseMessage {
    pub role: String,
    pub content: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OpenAiUsage {
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_tokens: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OpenAiErrorResponse {
    pub error: OpenAiError,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OpenAiError {
    pub message: String,
    #[serde(rename = "type")]
    pub error_type: Option<String>,
    pub param: Option<String>,
    pub code: Option<String>,
}

pub fn parse_openai_response(
    body: &str,
    status_code: u16,
    client_name: &str,
    prompt: RenderedPrompt,
    start_time: std::time::SystemTime,
    latency: Duration,
) -> crate::llm_response::LLMResponse {
    use crate::llm_response::*;

    if !(200..300).contains(&status_code) {
        let message = if let Ok(err_resp) = serde_json::from_str::<OpenAiErrorResponse>(body) {
            err_resp.error.message
        } else {
            body.to_string()
        };

        return LLMResponse::LLMFailure(LLMErrorResponse {
            client: client_name.to_string(),
            model: None,
            prompt,
            request_options: IndexMap::new(),
            start_time,
            latency,
            message,
            code: ErrorCode::from_status(status_code),
        });
    }

    match serde_json::from_str::<OpenAiChatCompletionResponse>(body) {
        Ok(response) => {
            let content = response
                .choices
                .first()
                .and_then(|c| c.message.content.clone())
                .unwrap_or_default();

            let finish_reason = response
                .choices
                .first()
                .and_then(|c| c.finish_reason.clone());

            let is_complete = finish_reason.as_deref() == Some("stop");

            let metadata = LLMResponseMetadata {
                baml_is_complete: is_complete,
                finish_reason,
                prompt_tokens: response.usage.as_ref().map(|u| u.prompt_tokens),
                output_tokens: response.usage.as_ref().map(|u| u.completion_tokens),
                total_tokens: response.usage.as_ref().map(|u| u.total_tokens),
                cached_input_tokens: None,
            };

            LLMResponse::Success(LLMCompleteResponse {
                client: client_name.to_string(),
                model: response.model,
                prompt,
                request_options: IndexMap::new(),
                content,
                start_time,
                latency,
                metadata,
            })
        }
        Err(e) => LLMResponse::InternalFailure(format!("Failed to parse OpenAI response: {}", e)),
    }
}

#[derive(Debug, Clone)]
pub enum SseEvent {
    Data(String),
    Done,
    Error(String),
}

#[derive(Debug, Clone, Deserialize)]
pub struct OpenAiStreamChunk {
    pub id: String,
    pub object: String,
    #[serde(default)]
    pub created: u64,
    pub model: String,
    pub choices: Vec<OpenAiStreamChoice>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OpenAiStreamChoice {
    pub index: u32,
    pub delta: OpenAiDelta,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OpenAiDelta {
    pub role: Option<String>,
    pub content: Option<String>,
}

pub fn parse_sse_line(line: &str) -> Option<SseEvent> {
    let line = line.trim();

    if line.is_empty() {
        return None;
    }

    if let Some(data) = line.strip_prefix("data: ") {
        if data == "[DONE]" {
            return Some(SseEvent::Done);
        }

        match serde_json::from_str::<OpenAiStreamChunk>(data) {
            Ok(chunk) => {
                let content = chunk
                    .choices
                    .first()
                    .and_then(|c| c.delta.content.clone())
                    .unwrap_or_default();

                if content.is_empty() {
                    if chunk
                        .choices
                        .first()
                        .and_then(|c| c.finish_reason.as_ref())
                        .is_some()
                    {
                        return Some(SseEvent::Done);
                    }
                    return None;
                }

                Some(SseEvent::Data(content))
            }
            Err(e) => Some(SseEvent::Error(format!("Failed to parse SSE chunk: {}", e))),
        }
    } else {
        None
    }
}
