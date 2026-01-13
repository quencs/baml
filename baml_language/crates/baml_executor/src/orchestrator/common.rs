//! Common orchestration utilities shared between call and stream.

use baml_llm_interface::RenderedPrompt;

use super::{ClientConfig, OrchestratorNode, ProviderType};
use crate::{
    errors::RuntimeError,
    llm_request::openai::{OpenAiClientConfig, OpenAiRequest},
};

pub struct PreparedNodeRequest {
    pub rendered_prompt: RenderedPrompt,
    pub request: OpenAiRequest,
}

pub fn prepare_node_request(
    prompt: &RenderedPrompt,
    node: &OrchestratorNode,
    env_vars: &std::collections::HashMap<String, String>,
    stream: bool,
) -> Result<PreparedNodeRequest, RuntimeError> {
    let client_config = build_client_config(&node.client, env_vars)?;

    let request = match node.client.provider {
        ProviderType::OpenAi => OpenAiRequest::from_rendered(prompt, &client_config, stream)?,
        ProviderType::Anthropic => {
            return Err(RuntimeError::BuildRequest(
                crate::errors::BuildRequestError::UnsupportedProvider {
                    provider: "anthropic".to_string(),
                },
            ));
        }
    };

    Ok(PreparedNodeRequest {
        rendered_prompt: prompt.clone(),
        request,
    })
}

fn build_client_config(
    client: &ClientConfig,
    env_vars: &std::collections::HashMap<String, String>,
) -> Result<OpenAiClientConfig, RuntimeError> {
    let api_key = client
        .options
        .get("api_key")
        .and_then(|v| v.as_str())
        .map(String::from)
        .or_else(|| env_vars.get("OPENAI_API_KEY").cloned())
        .unwrap_or_default();

    let base_url = client
        .options
        .get("base_url")
        .and_then(|v| v.as_str())
        .unwrap_or("https://api.openai.com/v1")
        .to_string();

    let model = client
        .options
        .get("model")
        .and_then(|v| v.as_str())
        .unwrap_or("gpt-4")
        .to_string();

    let temperature = client
        .options
        .get("temperature")
        .and_then(|v| v.as_f64())
        .map(|t| t as f32);

    let max_tokens = client
        .options
        .get("max_tokens")
        .and_then(|v| v.as_u64())
        .map(|t| t as u32);

    Ok(OpenAiClientConfig {
        base_url,
        api_key,
        model,
        temperature,
        max_tokens,
        timeout: Some(std::time::Duration::from_secs(60)),
    })
}
