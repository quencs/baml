//! Orchestrator - manages retry/fallback logic for LLM calls.

mod common;

use std::time::Duration;

use baml_llm_interface::RenderedPrompt;
pub use common::*;

use crate::{errors::RuntimeError, llm_response::LLMResponse, types::BamlValue};

#[derive(Debug, Clone)]
pub struct OrchestratorConfig {
    pub nodes: Vec<OrchestratorNode>,
}

impl OrchestratorConfig {
    pub fn single(node: OrchestratorNode) -> Self {
        Self { nodes: vec![node] }
    }

    pub fn with_fallback(nodes: Vec<OrchestratorNode>) -> Self {
        Self { nodes }
    }
}

#[derive(Debug, Clone)]
pub struct OrchestratorNode {
    pub client: ClientConfig,
    pub scope: OrchestrationScope,
    pub delay: Option<Duration>,
}

#[derive(Debug, Clone)]
pub struct ClientConfig {
    pub name: String,
    pub provider: ProviderType,
    pub options: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderType {
    OpenAi,
    Anthropic,
}

#[derive(Debug, Clone)]
pub enum OrchestrationScope {
    Direct,
    Retry { attempt: usize },
    Fallback { from_client: String },
}

#[derive(Debug, Clone)]
pub struct OrchestrationResult {
    pub response: Option<ParsedResponse>,
    pub attempts: Vec<OrchestrationAttempt>,
    pub total_duration: Duration,
}

#[derive(Debug, Clone)]
pub struct ParsedResponse {
    pub value: BamlValue,
    pub raw_content: String,
    pub model: String,
    pub metadata: crate::llm_response::LLMResponseMetadata,
}

#[derive(Debug, Clone)]
pub struct OrchestrationAttempt {
    pub node: OrchestratorNode,
    pub rendered_prompt: Option<RenderedPrompt>,
    pub llm_response: Option<LLMResponse>,
    pub parsed_output: Option<Result<BamlValue, String>>,
    pub error: Option<RuntimeError>,
    pub duration: Duration,
}

impl OrchestrationAttempt {
    pub fn new(node: OrchestratorNode) -> Self {
        Self {
            node,
            rendered_prompt: None,
            llm_response: None,
            parsed_output: None,
            error: None,
            duration: Duration::ZERO,
        }
    }

    pub fn failed(node: OrchestratorNode, error: RuntimeError, duration: Duration) -> Self {
        Self {
            node,
            rendered_prompt: None,
            llm_response: None,
            parsed_output: None,
            error: Some(error),
            duration,
        }
    }
}

pub struct FunctionResultStream {
    pub node: OrchestratorNode,
    pub node_index: usize,
    pub remaining_nodes: Vec<OrchestratorNode>,
    pub accumulated_content: String,
    pub attempts: Vec<OrchestrationAttempt>,
    pub start_time: std::time::Instant,
    pub output_type: baml_program::Ty,
}

impl FunctionResultStream {
    pub fn content(&self) -> &str {
        &self.accumulated_content
    }

    pub fn append_content(&mut self, chunk: &str) {
        self.accumulated_content.push_str(chunk);
    }

    pub fn parse_partial(
        &self,
    ) -> Result<crate::types::PartialResult, crate::errors::ParseOutputError> {
        let value =
            crate::parsing::parse_output_partial(&self.accumulated_content, &self.output_type)?;
        Ok(crate::types::PartialResult {
            value,
            raw_content: self.accumulated_content.clone(),
        })
    }

    pub fn finalize(self) -> Result<OrchestrationResult, RuntimeError> {
        let value = crate::parsing::parse_output(&self.accumulated_content, &self.output_type)?;

        Ok(OrchestrationResult {
            response: Some(ParsedResponse {
                value,
                raw_content: self.accumulated_content,
                model: "unknown".to_string(),
                metadata: crate::llm_response::LLMResponseMetadata::default(),
            }),
            attempts: self.attempts,
            total_duration: self.start_time.elapsed(),
        })
    }
}

pub async fn orchestrate_call(
    prompt: &RenderedPrompt,
    config: &OrchestratorConfig,
    env_vars: &std::collections::HashMap<String, String>,
    output_type: &baml_program::Ty,
    is_cancelled: impl Fn() -> bool,
) -> Result<OrchestrationResult, RuntimeError> {
    use std::time::Instant;

    use crate::{errors::RetryableError, llm_request::execute_openai_request_async};

    let mut attempts = Vec::new();
    let start = Instant::now();

    for (node_index, node) in config.nodes.iter().enumerate() {
        if is_cancelled() {
            return Err(RuntimeError::Cancelled);
        }

        if let Some(delay) = node.delay {
            tokio::time::sleep(delay).await;
        }

        let attempt_start = Instant::now();
        let mut attempt = OrchestrationAttempt::new(node.clone());

        let prepared = match prepare_node_request(prompt, node, env_vars, false) {
            Ok(p) => {
                attempt.rendered_prompt = Some(p.rendered_prompt.clone());
                p
            }
            Err(e) => {
                attempt.error = Some(e);
                attempt.duration = attempt_start.elapsed();
                attempts.push(attempt);
                continue;
            }
        };

        // Execute the HTTP request
        match execute_openai_request_async(&prepared.request, &node.client.name, prompt.clone())
            .await
        {
            Ok(result) => {
                attempt.llm_response = Some(result.response.clone());
                attempt.duration = result.latency;

                match result.response {
                    LLMResponse::Success(success) => {
                        // Parse the output
                        match crate::parsing::parse_output(&success.content, output_type) {
                            Ok(value) => {
                                attempt.parsed_output = Some(Ok(value.clone()));
                                attempts.push(attempt);

                                return Ok(OrchestrationResult {
                                    response: Some(ParsedResponse {
                                        value,
                                        raw_content: success.content,
                                        model: success.model,
                                        metadata: success.metadata,
                                    }),
                                    attempts,
                                    total_duration: start.elapsed(),
                                });
                            }
                            Err(parse_error) => {
                                attempt.parsed_output = Some(Err(parse_error.to_string()));
                                let error = RuntimeError::ParseOutput(parse_error);

                                if error.is_retryable() && node_index + 1 < config.nodes.len() {
                                    attempt.error = Some(error);
                                    attempts.push(attempt);
                                    continue;
                                } else {
                                    attempt.error = Some(error);
                                    attempts.push(attempt);
                                    break;
                                }
                            }
                        }
                    }
                    LLMResponse::LLMFailure(failure) => {
                        let error = RuntimeError::LlmFailure {
                            message: failure.message.clone(),
                            code: Some(failure.code.to_string()),
                        };

                        if error.is_retryable() && node_index + 1 < config.nodes.len() {
                            attempt.error = Some(error);
                            attempts.push(attempt);
                            continue;
                        } else {
                            attempt.error = Some(error);
                            attempts.push(attempt);
                            break;
                        }
                    }
                    LLMResponse::InternalFailure(msg) => {
                        let error = RuntimeError::Internal(msg);

                        if error.is_retryable() && node_index + 1 < config.nodes.len() {
                            attempt.error = Some(error);
                            attempts.push(attempt);
                            continue;
                        } else {
                            attempt.error = Some(error);
                            attempts.push(attempt);
                            break;
                        }
                    }
                    LLMResponse::UserFailure(msg) => {
                        let error = RuntimeError::Validation(msg);
                        attempt.error = Some(error);
                        attempts.push(attempt);
                        break;
                    }
                    LLMResponse::Cancelled(_) => {
                        return Err(RuntimeError::Cancelled);
                    }
                }
            }
            Err(error) => {
                if error.is_retryable() && node_index + 1 < config.nodes.len() {
                    attempt.error = Some(error);
                    attempt.duration = attempt_start.elapsed();
                    attempts.push(attempt);
                    continue;
                } else {
                    attempt.error = Some(error);
                    attempt.duration = attempt_start.elapsed();
                    attempts.push(attempt);
                    break;
                }
            }
        }
    }

    Err(RuntimeError::OrchestrationExhausted {
        attempts: attempts.len(),
        errors: attempts.iter().filter_map(|a| a.error.clone()).collect(),
    })
}

pub fn orchestrate_stream(
    prompt: &RenderedPrompt,
    config: OrchestratorConfig,
    env_vars: &std::collections::HashMap<String, String>,
    output_type: baml_program::Ty,
    is_cancelled: impl Fn() -> bool,
) -> Result<FunctionResultStream, RuntimeError> {
    use std::time::Instant;

    let mut attempts = Vec::new();
    let start = Instant::now();

    for (node_index, node) in config.nodes.iter().enumerate() {
        if is_cancelled() {
            return Err(RuntimeError::Cancelled);
        }

        if let Some(delay) = node.delay {
            std::thread::sleep(delay);
        }

        let attempt_start = Instant::now();

        match prepare_node_request(prompt, node, env_vars, true) {
            Ok(_prepared) => {
                return Ok(FunctionResultStream {
                    node: node.clone(),
                    node_index,
                    remaining_nodes: config.nodes[node_index + 1..].to_vec(),
                    accumulated_content: String::new(),
                    attempts,
                    start_time: start,
                    output_type,
                });
            }
            Err(e) => {
                attempts.push(OrchestrationAttempt::failed(
                    node.clone(),
                    e,
                    attempt_start.elapsed(),
                ));
                continue;
            }
        }
    }

    Err(RuntimeError::OrchestrationExhausted {
        attempts: attempts.len(),
        errors: attempts.iter().filter_map(|a| a.error.clone()).collect(),
    })
}
