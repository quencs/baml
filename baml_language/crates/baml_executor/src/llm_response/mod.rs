//! LLM response types - unified format for all providers.

use std::time::{Duration, SystemTime};

use baml_llm_interface::RenderedPrompt;
use indexmap::IndexMap;

#[derive(Debug, Clone)]
pub enum LLMResponse {
    Success(LLMCompleteResponse),
    LLMFailure(LLMErrorResponse),
    UserFailure(String),
    InternalFailure(String),
    Cancelled(String),
}

impl LLMResponse {
    pub fn is_success(&self) -> bool {
        matches!(self, LLMResponse::Success(_))
    }

    pub fn content(&self) -> Option<&str> {
        match self {
            LLMResponse::Success(resp) => Some(&resp.content),
            _ => None,
        }
    }

    pub fn error_message(&self) -> Option<&str> {
        match self {
            LLMResponse::LLMFailure(e) => Some(&e.message),
            LLMResponse::UserFailure(s) => Some(s),
            LLMResponse::InternalFailure(s) => Some(s),
            LLMResponse::Cancelled(s) => Some(s),
            LLMResponse::Success(_) => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct LLMCompleteResponse {
    pub client: String,
    pub model: String,
    pub prompt: RenderedPrompt,
    pub request_options: IndexMap<String, serde_json::Value>,
    pub content: String,
    pub start_time: SystemTime,
    pub latency: Duration,
    pub metadata: LLMResponseMetadata,
}

#[derive(Debug, Clone, Default)]
pub struct LLMResponseMetadata {
    pub baml_is_complete: bool,
    pub finish_reason: Option<String>,
    pub prompt_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub total_tokens: Option<u64>,
    pub cached_input_tokens: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct LLMErrorResponse {
    pub client: String,
    pub model: Option<String>,
    pub prompt: RenderedPrompt,
    pub request_options: IndexMap<String, serde_json::Value>,
    pub start_time: SystemTime,
    pub latency: Duration,
    pub message: String,
    pub code: ErrorCode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    InvalidAuthentication,
    NotSupported,
    RateLimited,
    ServerError,
    ServiceUnavailable,
    Timeout,
    Other(u16),
}

impl ErrorCode {
    pub fn from_status(status: u16) -> Self {
        match status {
            401 => ErrorCode::InvalidAuthentication,
            403 => ErrorCode::NotSupported,
            429 => ErrorCode::RateLimited,
            500 => ErrorCode::ServerError,
            503 => ErrorCode::ServiceUnavailable,
            other => ErrorCode::Other(other),
        }
    }

    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            ErrorCode::RateLimited | ErrorCode::ServerError | ErrorCode::ServiceUnavailable
        )
    }
}

impl std::fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorCode::InvalidAuthentication => write!(f, "invalid_authentication"),
            ErrorCode::NotSupported => write!(f, "not_supported"),
            ErrorCode::RateLimited => write!(f, "rate_limited"),
            ErrorCode::ServerError => write!(f, "server_error"),
            ErrorCode::ServiceUnavailable => write!(f, "service_unavailable"),
            ErrorCode::Timeout => write!(f, "timeout"),
            ErrorCode::Other(code) => write!(f, "error_{}", code),
        }
    }
}
