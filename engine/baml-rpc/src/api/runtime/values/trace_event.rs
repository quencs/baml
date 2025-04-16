use std::{borrow::Cow, collections::HashMap};

use crate::{
    ast::{evaluation_context::TypeBuilderValue, tops::BamlFunctionId},
    base::EpochMsTimestamp,
};
use serde::{Deserialize, Serialize};

use baml_ids::{ContentSpanId, SpanId};

use super::Media;

/// This is intentionally VERY similar to TraceEvent in
/// baml-lib/baml-types/src/tracing/events.rs
/// If the convertion from baml-types to baml-rpc is not possible,
/// WE HAVE A BREAKING CHANGE.
#[derive(Debug, Serialize, Deserialize)]
pub struct TraceEvent<'a> {
    /*
     * (span_id, content_event_id) is a unique identifier for a log event
     * The query (span_id, *) gets all logs for a function call
     */
    pub span_id: SpanId,

    // a unique identifier for this particular content
    pub content_event_id: ContentSpanId,

    // The chain of spans that lead to this log event
    // Includes span_id at the last position (content_event_id is not included)
    pub span_chain: Vec<SpanId>,

    // The timestamp of the log
    #[serde(rename = "timestamp_epoch_ms")]
    pub timestamp: EpochMsTimestamp,

    // The content of the log
    pub content: TraceData<'a>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum TraceData<'a> {
    FunctionStart {
        function_display_name: String,
        args: Vec<(String, super::Value<'a>)>,
        tags: TraceTags,
        /// Only sent for BAML defined functions
        baml_function_content: Option<BamlFunctionStart>,
    },
    /// Terminal Event
    FunctionEnd(FunctionEnd<'a>),

    /// Intermediate events between start and end
    Intermediate(IntermediateData<'a>),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BamlFunctionStart {
    pub function_id: BamlFunctionId,
    pub eval_context: EvaluationContext,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum FunctionEnd<'a> {
    Success { result: super::Value<'a> },
    Error { error: super::BamlError<'a> },
}

pub type TraceTags = std::collections::HashMap<String, serde_json::Value>;

#[derive(Debug, Serialize, Deserialize)]
pub struct EvaluationContext {
    pub tags: TraceTags,

    pub type_builder: Option<TypeBuilderValue>,
    // TODO(hellovai): add this
    // pub client_registry: Option<ClientRegistryValue>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum IntermediateData<'a> {
    /// These are all resolved from the client
    LLMRequest {
        client_name: String,
        client_provider: String,
        params: HashMap<String, Cow<'a, serde_json::Value>>,
        prompt: Vec<LLMChatMessage<'a>>,
    },
    RawLLMRequest {
        url: String,
        method: String,
        headers: HashMap<String, String>,
        body: HTTPBody<'a>,
    },
    RawLLMResponse {
        status: u16,
        headers: Option<HashMap<String, String>>,
        body: HTTPBody<'a>,
    },
    LLMResponse {
        #[serde(skip_serializing_if = "Option::is_none")]
        model: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        finish_reason: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        usage: Option<LLMUsage>,

        #[serde(skip_serializing_if = "Option::is_none")]
        raw_text_output: Option<Cow<'a, str>>,
    },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HTTPBody<'a> {
    pub raw: Cow<'a, [u8]>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct LLMChatMessage<'a> {
    pub role: String,
    pub content: Vec<LLMChatMessagePart<'a>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LLMChatMessagePart<'a> {
    Text(Cow<'a, str>),
    Media(Media<'a>),
    WithMeta(
        Box<LLMChatMessagePart<'a>>,
        HashMap<String, serde_json::Value>,
    ),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct LLMUsage {
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub total_tokens: Option<u64>,
}
