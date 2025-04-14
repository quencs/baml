use serde::{Deserialize, Serialize};

use crate::ast::types::type_reference::TypeReference;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Value {
    pub r#type: TypeReference,
    pub value: ValueContent,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type", content = "data")]
enum ValueContent {
    String(String),
    Float(f64),
    Int(i64),
    Boolean(bool),
    List(Vec<Value>),
    Map(Vec<(String, Value)>),
    Class {
        fields: Vec<(String, Value)>,
    },
    Enum {
        value: String,
    },
    Media {
        value: MediaValue,
        mime_type: Option<String>,
    },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type", content = "data")]
enum MediaValue {
    Url(String),
    Base64(String),
    FilePath(String),
}
