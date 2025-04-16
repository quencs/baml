use std::borrow::Cow;

use serde::{Deserialize, Serialize};

use crate::ast::types::type_reference::TypeReference;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Value<'a> {
    pub r#type: TypeReference,
    pub value: ValueContent<'a>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type", content = "data")]
pub enum ValueContent<'a> {
    String(Cow<'a, str>),
    Float(f64),
    Int(i64),
    Boolean(bool),
    List(Vec<Value<'a>>),
    Map(Vec<(String, Value<'a>)>),
    Class { fields: Vec<(String, Value<'a>)> },
    Enum { value: String },
    Media(Media<'a>),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Media<'a> {
    pub mime_type: Option<String>,
    pub value: MediaValue<'a>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type", content = "data")]
pub enum MediaValue<'a> {
    Url(Cow<'a, str>),
    Base64(Cow<'a, str>),
    FilePath(Cow<'a, str>),
}
