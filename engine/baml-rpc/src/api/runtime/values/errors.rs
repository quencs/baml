use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum BamlError {
    /// For any exceptions that are not handled by the BAML runtime
    ExternalException {
        message: String,
    },
    Base {
        message: String,
    },
    InvalidArgument {
        message: String,
    },
    Client {
        message: String,
    },
    ClientHttp {
        message: String,
        status_code: i32,
    },
    ClientFinishReason {
        finish_reason: String,
        message: String,
        prompt: String,
        raw_output: String,
    },
    Validation {
        raw_output: String,
        message: String,
        prompt: String,
    },
}
