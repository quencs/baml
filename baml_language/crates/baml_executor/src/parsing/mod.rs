//! Output parsing - convert LLM responses to BAML types.

use baml_program::Ty;

use crate::{errors::ParseOutputError, types::BamlValue};

pub fn parse_output(content: &str, _output_type: &Ty) -> Result<BamlValue, ParseOutputError> {
    match serde_json::from_str::<serde_json::Value>(content) {
        Ok(json) => Ok(json_to_baml_value(json)),
        Err(_) => Ok(BamlValue::String(content.to_string())),
    }
}

pub fn parse_output_partial(
    content: &str,
    _output_type: &Ty,
) -> Result<BamlValue, ParseOutputError> {
    match serde_json::from_str::<serde_json::Value>(content) {
        Ok(json) => Ok(json_to_baml_value(json)),
        Err(_) => Ok(BamlValue::String(content.to_string())),
    }
}

fn json_to_baml_value(json: serde_json::Value) -> BamlValue {
    match json {
        serde_json::Value::Null => BamlValue::Null,
        serde_json::Value::Bool(b) => BamlValue::Bool(b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                BamlValue::Int(i)
            } else if let Some(f) = n.as_f64() {
                BamlValue::Float(f)
            } else {
                BamlValue::Float(0.0)
            }
        }
        serde_json::Value::String(s) => BamlValue::String(s),
        serde_json::Value::Array(arr) => {
            BamlValue::List(arr.into_iter().map(json_to_baml_value).collect())
        }
        serde_json::Value::Object(obj) => {
            let map = obj
                .into_iter()
                .map(|(k, v)| (k, json_to_baml_value(v)))
                .collect();
            BamlValue::Map(map)
        }
    }
}
