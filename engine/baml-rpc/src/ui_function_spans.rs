use crate::base::EpochMsTimestamp;
use crate::rpc::ApiEndpoint;
use crate::ProjectId;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Filter {
    pub env_id: Option<String>,
    pub person_id: Option<String>,
    pub api_key: Option<String>,
    pub client: Option<String>,
    pub function_id: Option<String>,
    pub function_name: Option<String>,
    pub session_id: Option<String>,
    pub call_type: Option<String>,
    #[ts(type = "number | null")]
    pub start_at: Option<EpochMsTimestamp>,
    #[ts(type = "number | null")]
    pub end_at: Option<EpochMsTimestamp>,
    pub relative_time: Option<String>,
    pub span_id: Option<String>,
    pub streamed: Option<bool>,
    pub status: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ListFunctionSpansRequest {
    #[ts(type = "string")]
    pub project_id: ProjectId,
    pub function_call_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ListFunctionSpansResponse {
    pub function_spans: Vec<api::FunctionSpan>,
    // TODO: add function definitions and type definitions
    // #[ts(type = "any")]
    // function_definitions: Vec<BamlFunctionDefinition>,
    // #[ts(type = "any")]
    // type_definitions: Vec<BamlTypeDefinition>,
}

pub struct ListFunctionSpans;

impl ApiEndpoint for ListFunctionSpans {
    type Request<'a> = ListFunctionSpansRequest;
    type Response<'a> = ListFunctionSpansResponse;

    const PATH: &'static str = "/v1/function-spans";
}

pub mod api {
    use serde::{Deserialize, Serialize};
    use ts_rs::TS;

    use crate::base::EpochMsTimestamp;

    #[derive(Debug, Serialize, Deserialize, TS)]
    #[ts(export)]
    pub struct FunctionSpan {
        pub function_span_id: String,
        pub source: String,
        pub function_id: String,
        #[serde(rename = "start_epoch_ms")]
        #[ts(type = "number | null")]
        pub start_time: Option<EpochMsTimestamp>,
        #[serde(rename = "end_epoch_ms")]
        #[ts(type = "number | null")]
        pub end_time: Option<EpochMsTimestamp>,
        #[ts(type = "any")]
        pub baml_options: serde_json::Value,
        pub inputs: Vec<FunctionInput>,
        #[ts(type = "any")]
        pub output: serde_json::Value,
        pub status: String,
        #[ts(type = "any")]
        pub error: serde_json::Value,
    }

    #[derive(Debug, Deserialize, Serialize, TS)]
    #[ts(export)]
    pub struct FunctionInput {
        pub field: String,
        #[ts(type = "any")]
        pub value: serde_json::Value,
    }
}
