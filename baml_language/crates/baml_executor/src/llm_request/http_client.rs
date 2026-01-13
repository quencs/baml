//! HTTP client for executing LLM requests.
//!
//! This module provides HTTP execution for both native (reqwest) and WASM (fetch) targets.

use std::time::{Duration, Instant, SystemTime};

use baml_llm_interface::RenderedPrompt;

use super::openai::{OpenAiRequest, parse_openai_response};
use crate::{errors::RuntimeError, llm_response::LLMResponse};

/// Result of executing an HTTP request.
pub struct HttpExecutionResult {
    pub response: LLMResponse,
    pub raw_body: String,
    pub status_code: u16,
    pub latency: Duration,
}

/// Execute an OpenAI request asynchronously and return the response.
#[cfg(all(feature = "native", not(target_arch = "wasm32")))]
pub async fn execute_openai_request_async(
    request: &OpenAiRequest,
    client_name: &str,
    prompt: RenderedPrompt,
) -> Result<HttpExecutionResult, RuntimeError> {
    let start_time = SystemTime::now();
    let start = Instant::now();

    let client = reqwest::Client::builder()
        .timeout(request.timeout.unwrap_or(Duration::from_secs(60)))
        .build()
        .map_err(|e| {
            RuntimeError::Http(crate::errors::HttpError {
                message: format!("Failed to create HTTP client: {}", e),
                status_code: None,
            })
        })?;

    let mut req_builder = client.post(&request.url);

    for (key, value) in &request.headers {
        req_builder = req_builder.header(key, value.render(true));
    }

    req_builder = req_builder.json(&request.body);

    let response = req_builder.send().await.map_err(|e| {
        RuntimeError::Http(crate::errors::HttpError {
            message: format!("HTTP request failed: {}", e),
            status_code: e.status().map(|s| s.as_u16()),
        })
    })?;

    let status_code = response.status().as_u16();
    let raw_body = response.text().await.map_err(|e| {
        RuntimeError::Http(crate::errors::HttpError {
            message: format!("Failed to read response body: {}", e),
            status_code: Some(status_code),
        })
    })?;

    let latency = start.elapsed();

    let llm_response = parse_openai_response(
        &raw_body,
        status_code,
        client_name,
        prompt,
        start_time,
        latency,
    );

    Ok(HttpExecutionResult {
        response: llm_response,
        raw_body,
        status_code,
        latency,
    })
}

/// Execute an OpenAI request synchronously using tokio block_on.
#[cfg(all(feature = "native", not(target_arch = "wasm32")))]
pub fn execute_openai_request(
    request: &OpenAiRequest,
    client_name: &str,
    prompt: RenderedPrompt,
) -> Result<HttpExecutionResult, RuntimeError> {
    tokio::runtime::Handle::try_current()
        .map_err(|_| RuntimeError::Internal("No tokio runtime available".to_string()))?
        .block_on(execute_openai_request_async(request, client_name, prompt))
}

/// Execute an OpenAI request (WASM stub - not implemented).
///
/// On WASM targets, HTTP requests should be made via JavaScript bindings.
#[cfg(any(not(feature = "native"), target_arch = "wasm32"))]
pub fn execute_openai_request(
    _request: &OpenAiRequest,
    _client_name: &str,
    _prompt: RenderedPrompt,
) -> Result<HttpExecutionResult, RuntimeError> {
    Err(RuntimeError::Http(crate::errors::HttpError {
        message: "HTTP execution not available in WASM - use JavaScript fetch bindings".to_string(),
        status_code: None,
    }))
}

/// Execute an OpenAI request asynchronously (WASM stub - not implemented).
#[cfg(any(not(feature = "native"), target_arch = "wasm32"))]
pub async fn execute_openai_request_async(
    _request: &OpenAiRequest,
    _client_name: &str,
    _prompt: RenderedPrompt,
) -> Result<HttpExecutionResult, RuntimeError> {
    Err(RuntimeError::Http(crate::errors::HttpError {
        message: "HTTP execution not available in WASM - use JavaScript fetch bindings".to_string(),
        status_code: None,
    }))
}
