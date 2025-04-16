use std::borrow::Cow;

use super::{IntoRpcEvent, TypeLookup};

impl<'a> IntoRpcEvent<'a, baml_rpc::runtime_api::BamlError<'a>>
    for baml_types::tracing::events::BamlError<'a>
{
    fn into_rpc_event(
        &'a self,
        lookup: &(impl TypeLookup + ?Sized),
    ) -> baml_rpc::runtime_api::BamlError<'a> {
        match self {
            baml_types::tracing::events::BamlError::External { message } => {
                baml_rpc::runtime_api::BamlError::ExternalException {
                    message: Cow::Borrowed(message),
                }
            }
            baml_types::tracing::events::BamlError::Internal { message } => {
                baml_rpc::runtime_api::BamlError::InternalException {
                    message: Cow::Borrowed(message),
                }
            }
            baml_types::tracing::events::BamlError::Base { message } => {
                baml_rpc::runtime_api::BamlError::Base {
                    message: Cow::Borrowed(message),
                }
            }
            baml_types::tracing::events::BamlError::InvalidArgument { message } => {
                baml_rpc::runtime_api::BamlError::InvalidArgument {
                    message: Cow::Borrowed(message),
                }
            }
            baml_types::tracing::events::BamlError::Client { message } => {
                baml_rpc::runtime_api::BamlError::Client {
                    message: Cow::Borrowed(message),
                }
            }
            baml_types::tracing::events::BamlError::ClientHttp {
                message,
                status_code,
            } => baml_rpc::runtime_api::BamlError::ClientHttp {
                message: Cow::Borrowed(message),
                status_code: *status_code,
            },
            baml_types::tracing::events::BamlError::ClientFinishReason {
                finish_reason,
                message,
                prompt,
                raw_output,
            } => baml_rpc::runtime_api::BamlError::ClientFinishReason {
                finish_reason: Cow::Borrowed(finish_reason),
                message: Cow::Borrowed(message),
                prompt: Cow::Borrowed(prompt),
                raw_output: Cow::Borrowed(raw_output),
            },
            baml_types::tracing::events::BamlError::Validation {
                raw_output,
                message,
                prompt,
            } => baml_rpc::runtime_api::BamlError::Validation {
                raw_output: Cow::Borrowed(raw_output),
                message: Cow::Borrowed(message),
                prompt: Cow::Borrowed(prompt),
            },
        }
    }
}
