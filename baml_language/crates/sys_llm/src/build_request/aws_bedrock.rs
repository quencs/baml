//! AWS Bedrock Converse request builder (text-only).

use aws_credential_types::provider::ProvideCredentials;
use baml_builtins::{PromptAst, PromptAstSimple};
use indexmap::IndexMap;
use serde::Serialize;

use super::{
    BuildRequestError, LlmPrimitiveClient, LlmRequestBuilder, RawHttpRequest, get_string_option,
};

pub(crate) struct AwsBedrockBuilder;

/// Default region when none is specified.
const DEFAULT_REGION: &str = "us-east-1";

/// Block on a future, handling both "inside a tokio runtime" and "no runtime" cases.
fn block_on_future<F: std::future::Future>(fut: F) -> F::Output {
    match tokio::runtime::Handle::try_current() {
        Ok(handle) => tokio::task::block_in_place(|| handle.block_on(fut)),
        Err(_) => tokio::runtime::Runtime::new()
            .expect("failed to create tokio runtime for AWS config")
            .block_on(fut),
    }
}

#[derive(Serialize)]
struct ConverseMessage {
    role: String,
    content: Vec<ContentBlock>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
enum ContentBlock {
    Text(String),
}

/// Resolved AWS credentials + region, either from explicit options or the
/// default AWS credential chain (env vars, profiles, IMDS, etc.).
struct ResolvedAwsConfig {
    credentials: aws_credential_types::Credentials,
    region: String,
}

impl AwsBedrockBuilder {
    /// Resolve AWS credentials and region.
    ///
    /// If `access_key_id` / `secret_access_key` are set in client options, those
    /// are used directly. Otherwise we fall back to the full default credential
    /// chain provided by `aws_config` (env vars, `~/.aws/credentials`,
    /// `~/.aws/config`, IMDS, ECS task role, etc.).
    ///
    /// Region resolution order:
    ///   1. Explicit `region` option on the client.
    ///   2. Default chain from `aws_config` (`AWS_REGION` / `AWS_DEFAULT_REGION` / profile).
    ///   3. Hard-coded fallback `us-east-1`.
    fn resolve_aws_config(
        client: &LlmPrimitiveClient,
    ) -> Result<ResolvedAwsConfig, BuildRequestError> {
        // Explicit credentials in client options — skip the async config loader.
        if let Some(access_key) = get_string_option(client, "access_key_id") {
            let secret_key = get_string_option(client, "secret_access_key")
                .ok_or_else(|| BuildRequestError::MissingOption("secret_access_key".into()))?;
            let session_token = get_string_option(client, "session_token");
            let region =
                get_string_option(client, "region").unwrap_or_else(|| DEFAULT_REGION.to_string());
            let creds = aws_credential_types::Credentials::new(
                access_key,
                secret_key,
                session_token,
                None,
                "baml-bedrock-explicit",
            );
            return Ok(ResolvedAwsConfig {
                credentials: creds,
                region,
            });
        }

        // Fall back to the default AWS credential/region chain.
        let sdk_config = block_on_future(aws_config::load_defaults(
            aws_config::BehaviorVersion::latest(),
        ));

        let region = get_string_option(client, "region")
            .or_else(|| {
                sdk_config
                    .region()
                    .map(|r: &aws_config::Region| r.to_string())
            })
            .unwrap_or_else(|| DEFAULT_REGION.to_string());

        let provider = sdk_config.credentials_provider().ok_or_else(|| {
            BuildRequestError::MissingOption(
                "No AWS credentials found. Set access_key_id/secret_access_key options \
                 or configure AWS credentials via environment variables, \
                 ~/.aws/credentials, or IAM role."
                    .into(),
            )
        })?;

        let credentials = block_on_future(provider.provide_credentials()).map_err(
            |e: aws_credential_types::provider::error::CredentialsError| {
                BuildRequestError::InvalidOption {
                    key: "credentials".into(),
                    reason: e.to_string(),
                }
            },
        )?;

        Ok(ResolvedAwsConfig {
            credentials,
            region,
        })
    }

    /// Sign a raw HTTP request using AWS `SigV4`.
    fn sign_request(
        raw: &mut RawHttpRequest,
        credentials: &aws_credential_types::Credentials,
        region: &str,
    ) -> Result<(), BuildRequestError> {
        use std::time::SystemTime;

        use aws_sigv4::{
            http_request::{SignableBody, SignableRequest, SigningSettings, sign},
            sign::v4,
        };

        let identity = credentials.clone().into();

        let signing_params = v4::SigningParams::builder()
            .identity(&identity)
            .region(region)
            .name("bedrock")
            .time(SystemTime::now())
            .settings(SigningSettings::default())
            .build()
            .map_err(|e| BuildRequestError::InvalidOption {
                key: "signing".into(),
                reason: e.to_string(),
            })?
            .into();

        let signable = SignableRequest::new(
            &raw.method,
            &raw.url,
            raw.headers.iter().map(|(k, v)| (k.as_str(), v.as_str())),
            SignableBody::Bytes(raw.body.as_bytes()),
        )
        .map_err(|e| BuildRequestError::InvalidOption {
            key: "signing".into(),
            reason: e.to_string(),
        })?;

        let (signing_instructions, _signature) = sign(signable, &signing_params)
            .map_err(|e| BuildRequestError::InvalidOption {
                key: "signing".into(),
                reason: e.to_string(),
            })?
            .into_parts();

        // Apply the signing headers to the raw request.
        for (name, value) in signing_instructions.headers() {
            raw.headers.insert(name.to_string(), value.to_string());
        }

        Ok(())
    }
}

impl LlmRequestBuilder for AwsBedrockBuilder {
    fn provider_skip_keys(&self) -> &'static [&'static str] {
        &[
            "region",
            "inference_profile",
            "access_key_id",
            "secret_access_key",
            "session_token",
        ]
    }

    fn build_url(&self, client: &LlmPrimitiveClient) -> Result<String, BuildRequestError> {
        let model = get_string_option(client, "model")
            .or_else(|| get_string_option(client, "inference_profile"))
            .ok_or_else(|| BuildRequestError::MissingOption("model".into()))?;
        let region =
            get_string_option(client, "region").unwrap_or_else(|| DEFAULT_REGION.to_string());
        let base_url = get_string_option(client, "base_url")
            .unwrap_or_else(|| format!("https://bedrock-runtime.{region}.amazonaws.com"));
        Ok(format!("{base_url}/model/{model}/converse"))
    }

    fn build_auth_headers(&self, _client: &LlmPrimitiveClient) -> IndexMap<String, String> {
        // Auth is handled via SigV4 signing in build_request; no static auth headers.
        IndexMap::new()
    }

    fn build_request(
        &self,
        client: &LlmPrimitiveClient,
        prompt: bex_vm_types::PromptAst,
        stream: bool,
    ) -> Result<RawHttpRequest, BuildRequestError> {
        let resolved = Self::resolve_aws_config(client)?;

        // Use the resolved region for the URL.
        let model = get_string_option(client, "model")
            .or_else(|| get_string_option(client, "inference_profile"))
            .ok_or_else(|| BuildRequestError::MissingOption("model".into()))?;
        let base_url = get_string_option(client, "base_url").unwrap_or_else(|| {
            format!("https://bedrock-runtime.{}.amazonaws.com", resolved.region)
        });
        let url = format!("{base_url}/model/{model}/converse");

        let headers = self.build_headers(client);
        let body = self.build_body(client, prompt, stream)?;
        let mut raw = RawHttpRequest {
            method: "POST".to_string(),
            url,
            headers,
            body,
        };

        Self::sign_request(&mut raw, &resolved.credentials, &resolved.region)?;

        Ok(raw)
    }

    fn build_body(
        &self,
        client: &LlmPrimitiveClient,
        prompt: bex_vm_types::PromptAst,
        _stream: bool,
    ) -> Result<String, BuildRequestError> {
        let mut body = serde_json::Map::new();
        // Bedrock Converse does NOT put "model" in the body — it's in the URL.
        body.extend(self.build_prompt_body(client, prompt)?);
        self.forward_options(client, &mut body);
        serde_json::to_string(&body).map_err(|e| BuildRequestError::InvalidOption {
            key: "body".into(),
            reason: e.to_string(),
        })
    }

    fn build_prompt_body(
        &self,
        _client: &LlmPrimitiveClient,
        prompt: bex_vm_types::PromptAst,
    ) -> Result<serde_json::Map<String, serde_json::Value>, BuildRequestError> {
        let mut map = serde_json::Map::new();
        let (system, messages) = extract_system_and_messages(prompt);
        if !system.is_empty() {
            map.insert(
                "system".to_string(),
                serde_json::to_value(system).expect("infallible"),
            );
        }
        map.insert(
            "messages".to_string(),
            serde_json::to_value(messages).expect("infallible"),
        );
        Ok(map)
    }
}

fn content_blocks(content: &PromptAstSimple) -> Vec<ContentBlock> {
    match content {
        PromptAstSimple::String(s) => vec![ContentBlock::Text(s.clone())],
        PromptAstSimple::Multiple(parts) => parts.iter().flat_map(|p| content_blocks(p)).collect(),
        PromptAstSimple::Media(_) => vec![], // text-only for now
    }
}

fn extract_system_and_messages(
    prompt: bex_vm_types::PromptAst,
) -> (Vec<ContentBlock>, Vec<ConverseMessage>) {
    let mut system = Vec::new();
    let mut messages = Vec::new();

    let items = match prompt.as_ref() {
        PromptAst::Vec(v) => v.clone(),
        _ => vec![prompt],
    };

    for item in &items {
        match item.as_ref() {
            PromptAst::Message { role, content, .. } if role == "system" => {
                system.extend(content_blocks(content));
            }
            PromptAst::Message { role, content, .. } => {
                messages.push(ConverseMessage {
                    role: role.clone(),
                    content: content_blocks(content),
                });
            }
            PromptAst::Simple(content) => {
                messages.push(ConverseMessage {
                    role: "user".to_string(),
                    content: content_blocks(content),
                });
            }
            PromptAst::Vec(_) => unreachable!(),
        }
    }

    (system, messages)
}
