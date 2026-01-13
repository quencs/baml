//! Error types for the BAML executor.

use thiserror::Error;

/// Format a list of errors for display.
fn format_errors(errors: &[RuntimeError]) -> String {
    if errors.is_empty() {
        "no errors recorded".to_string()
    } else {
        errors
            .iter()
            .map(|e| e.to_string())
            .collect::<Vec<_>>()
            .join("; ")
    }
}

/// Main runtime error type.
#[derive(Debug, Error, Clone)]
pub enum RuntimeError {
    #[error("Preparation failed: {0}")]
    Prepare(#[from] PrepareError),

    #[error("Prompt rendering failed: {0}")]
    Render(String),

    #[error("Request building failed: {0}")]
    BuildRequest(#[from] BuildRequestError),

    #[error("HTTP request failed: {0}")]
    Http(#[from] HttpError),

    #[error("Response parsing failed: {0}")]
    ParseResponse(#[from] ParseResponseError),

    #[error("Output parsing failed: {0}")]
    ParseOutput(#[from] ParseOutputError),

    #[error("Media resolution failed: {0}")]
    MediaResolve(#[from] MediaResolveError),

    #[error("Orchestration exhausted: all {attempts} attempts failed: {}", format_errors(.errors))]
    OrchestrationExhausted {
        attempts: usize,
        errors: Vec<RuntimeError>,
    },

    #[error("Cancelled")]
    Cancelled,

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Function not found: {0}")]
    FunctionNotFound(String),

    #[error("LLM failure: {message}")]
    LlmFailure {
        message: String,
        code: Option<String>,
    },

    #[error("Internal error: {0}")]
    Internal(String),
}

/// Error during function preparation.
#[derive(Debug, Error, Clone)]
pub enum PrepareError {
    #[error("Function not found: {function_name}")]
    FunctionNotFound { function_name: String },

    #[error("Invalid parameters for {function_name}: {message}")]
    InvalidParams {
        function_name: String,
        message: String,
    },

    #[error("Invalid schema: {message}")]
    InvalidSchema { message: String },

    #[error("Type mismatch for parameter {param_name}: expected {expected}, got {actual}")]
    TypeMismatch {
        param_name: String,
        expected: String,
        actual: String,
    },
}

/// Error during prompt rendering.
#[derive(Debug, Error, Clone)]
pub enum RenderError {
    #[error("Template rendering failed: {message}")]
    TemplateError { message: String },

    #[error("Missing variable: {name}")]
    MissingVariable { name: String },

    #[error("Invalid template: {message}")]
    InvalidTemplate { message: String },
}

/// Error building the HTTP request.
#[derive(Debug, Error, Clone)]
pub enum BuildRequestError {
    #[error("Invalid URL: {url}")]
    InvalidUrl { url: String },

    #[error("Missing API key for client: {client}")]
    MissingApiKey { client: String },

    #[error("Serialization failed: {message}")]
    SerializationError { message: String },

    #[error("Unsupported provider: {provider}")]
    UnsupportedProvider { provider: String },
}

/// HTTP-level errors.
#[derive(Debug, Error, Clone)]
pub struct HttpError {
    pub message: String,
    pub status_code: Option<u16>,
}

impl std::fmt::Display for HttpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.status_code {
            Some(code) => write!(f, "HTTP {} - {}", code, self.message),
            None => write!(f, "HTTP error: {}", self.message),
        }
    }
}

/// Error parsing provider response.
#[derive(Debug, Error, Clone)]
pub enum ParseResponseError {
    #[error("Invalid JSON response: {message}")]
    InvalidJson { message: String },

    #[error("Missing field in response: {field}")]
    MissingField { field: String },

    #[error("Unexpected response format: {message}")]
    UnexpectedFormat { message: String },
}

/// Error parsing LLM output to BAML types.
#[derive(Debug, Error, Clone)]
pub enum ParseOutputError {
    #[error("Failed to parse output as {expected_type}: {message}")]
    TypeMismatch {
        expected_type: String,
        message: String,
    },

    #[error("Incomplete output: {message}")]
    IncompleteOutput { message: String },

    #[error("Invalid JSON in output: {message}")]
    InvalidJson { message: String },
}

/// Error resolving media content.
#[derive(Debug, Error, Clone)]
pub enum MediaResolveError {
    #[error("Failed to fetch URL {url}: {message}")]
    FetchError { url: String, message: String },

    #[error("Failed to read file {path}: {message}")]
    FileReadError { path: String, message: String },

    #[error("Unsupported media type: {mime_type}")]
    UnsupportedMediaType { mime_type: String },
}

/// Trait for determining if an error should trigger a retry.
pub trait RetryableError {
    fn is_retryable(&self) -> bool;
}

impl RetryableError for HttpError {
    fn is_retryable(&self) -> bool {
        match self.status_code {
            Some(429) => true,       // Rate limited
            Some(500..=599) => true, // Server errors
            _ => false,
        }
    }
}

impl RetryableError for RuntimeError {
    fn is_retryable(&self) -> bool {
        match self {
            RuntimeError::Http(e) => e.is_retryable(),
            RuntimeError::ParseResponse(_) => false,
            RuntimeError::ParseOutput(_) => false,
            RuntimeError::Cancelled => false,
            RuntimeError::OrchestrationExhausted { .. } => false,
            RuntimeError::LlmFailure { code, .. } => {
                // Rate limits and server errors are retryable
                code.as_deref() == Some("rate_limited")
                    || code.as_deref() == Some("server_error")
                    || code.as_deref() == Some("service_unavailable")
            }
            RuntimeError::Internal(_) => false,
            _ => false,
        }
    }
}
