use baml_types::HasFieldType;

use crate::tracingv2::storage::interface::TraceEventWithMeta;

mod errors;
mod trace_data;
mod types;

trait IntoRpcEvent<'a, RpcOutputType> {
    fn into_rpc_event(&'a self) -> RpcOutputType;
}

pub(super) fn to_rpc_event<'a>(
    event: &'a TraceEventWithMeta,
) -> baml_rpc::runtime_api::TraceEvent<'a> {
    let timestamp = baml_rpc::EpochMsTimestamp::try_from(event.timestamp)
        .expect("Failed to convert timestamp to EpochMsTimestamp");
    baml_rpc::runtime_api::TraceEvent {
        span_id: event.span_id.clone(),
        content_event_id: event.content_span_id.clone(),
        span_chain: event.span_chain.clone(),
        timestamp,
        content: event.content.into_rpc_event(),
    }
}

impl<'a, T: HasFieldType> IntoRpcEvent<'a, baml_rpc::runtime_api::TraceData<'a>>
    for baml_types::tracing::events::TraceData<'a, T>
{
    fn into_rpc_event(&'a self) -> baml_rpc::runtime_api::TraceData<'a> {
        use baml_types::tracing::events::TraceData;

        match self {
            TraceData::FunctionStart(function_start) => function_start.into_rpc_event(),
            TraceData::FunctionEnd(function_end) => function_end.into_rpc_event(),
            TraceData::LLMRequest(logged_llmrequest) => {
                baml_rpc::runtime_api::TraceData::Intermediate(logged_llmrequest.into_rpc_event())
            }
            TraceData::RawLLMRequest(httprequest) => {
                baml_rpc::runtime_api::TraceData::Intermediate(httprequest.into_rpc_event())
            }
            TraceData::RawLLMResponse(httpresponse) => {
                baml_rpc::runtime_api::TraceData::Intermediate(httpresponse.into_rpc_event())
            }
            TraceData::LLMResponse(logged_llmresponse) => {
                baml_rpc::runtime_api::TraceData::Intermediate(logged_llmresponse.into_rpc_event())
            }
        }
    }
}
