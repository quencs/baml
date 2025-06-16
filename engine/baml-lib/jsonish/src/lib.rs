pub mod helpers;
pub mod tests;

use anyhow::Result;
use indexmap::IndexMap;
pub mod deserializer;
use std::collections::HashMap;
pub mod jsonish;
pub mod xmlish;

use baml_types::{
    BamlValue, BamlValueWithMeta, FieldType, HasFieldType, JinjaExpression, ResponseCheck,
};
use deserializer::{
    coercer::{ParsingContext, ParsingError, TypeCoercer},
    deserialize_flags::DeserializerConditions,
};

pub use deserializer::types::BamlValueWithFlags;
use internal_baml_core::ir::TypeValue;
use internal_baml_jinja::types::OutputFormatContent;

use crate::deserializer::score::WithScore;
use baml_types::{Completion, CompletionState};
use deserializer::deserialize_flags::Flag;
use deserializer::types::ParsingErrorToUiJson;
use jsonish::Value;
use serde::{ser::SerializeMap, ser::SerializeStruct, Serialize, Serializer};

#[derive(Clone, Debug)]
pub struct ResponseBamlValue(pub BamlValueWithMeta<ResponseValueMeta>);

#[derive(Clone, Debug)]
pub struct ResponseValueMeta(
    pub Vec<Flag>,
    pub Vec<ResponseCheck>,
    pub Completion,
    pub FieldType,
);

impl From<FieldType> for ResponseValueMeta {
    fn from(field_type: FieldType) -> Self {
        ResponseValueMeta(vec![], vec![], Completion::default(), field_type)
    }
}

impl baml_types::HasFieldType for ResponseValueMeta {
    fn field_type<'a>(&'a self) -> &'a FieldType {
        &self.3
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum SerializeMode {
    Final,
    Partial,
}

/// A special-purpose wrapper for specifying the serialization format of a
/// `ResponseBamlValue`. You should construct these from `ResponseBamlValue`
/// with the `serialize_final` or `serialize_partial` method.
pub struct SerializeResponseBamlValue<'a> {
    pub value: &'a BamlValueWithMeta<ResponseValueMeta>,
    pub serialize_mode: SerializeMode,
}

impl ResponseBamlValue {
    /// Prepare a `ResponseBamlValue` for "final" serialization (serialization
    /// with no stream-state metadata).
    pub fn serialize_final<'a>(&'a self) -> SerializeResponseBamlValue<'a> {
        SerializeResponseBamlValue {
            value: &self.0,
            serialize_mode: SerializeMode::Final,
        }
    }

    /// Prepare a `ResponseBamlValue` for "partial" serialization (serialization
    /// with stream-state metadata).
    pub fn serialize_partial<'a>(&'a self) -> SerializeResponseBamlValue<'a> {
        SerializeResponseBamlValue {
            value: &self.0,
            serialize_mode: SerializeMode::Partial,
        }
    }
}

impl serde::Serialize for SerializeResponseBamlValue<'_> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use BamlValueWithMeta::*;
        let serialize_mode = &self.serialize_mode;
        match &self.value {
            String(s, ref meta) => serialize_with_meta(&s, &meta, serialize_mode, serializer),
            Int(i, ref meta) => serialize_with_meta(&i, &meta, serialize_mode, serializer),
            Float(f, ref meta) => serialize_with_meta(&f, &meta, serialize_mode, serializer),
            Bool(b, ref meta) => serialize_with_meta(&b, &meta, serialize_mode, serializer),
            Media(v, ref meta) => serialize_with_meta(&v, &meta, serialize_mode, serializer),
            Enum(ref _name, v, ref meta) => {
                serialize_with_meta(&v, meta, serialize_mode, serializer)
            }
            Map(items, ref meta) => {
                let new_items = items
                    .into_iter()
                    .map(|(k, v)| {
                        (
                            k.clone(),
                            SerializeResponseBamlValue {
                                value: &v,
                                serialize_mode: serialize_mode.clone(),
                            },
                        )
                    })
                    .collect::<IndexMap<std::string::String, SerializeResponseBamlValue<'_>>>();
                serialize_with_meta(&new_items, &meta, serialize_mode, serializer)
            }
            List(items, ref meta) => {
                let new_items = items
                    .into_iter()
                    .map(|v| SerializeResponseBamlValue {
                        value: v,
                        serialize_mode: serialize_mode.clone(),
                    })
                    .collect::<Vec<_>>();
                serialize_with_meta(&new_items, &meta, serialize_mode, serializer)
            }
            Class(_name, fields, ref meta) => {
                let new_fields = fields
                    .into_iter()
                    .map(|(k, v)| {
                        let subvalue_serialize_mode =
                            match (&serialize_mode, v.meta().2.required_done) {
                                (SerializeMode::Final, _) => SerializeMode::Final,
                                (SerializeMode::Partial, true) => SerializeMode::Final,
                                (SerializeMode::Partial, false) => SerializeMode::Partial,
                            };
                        (
                            k,
                            SerializeResponseBamlValue {
                                value: v,
                                serialize_mode: subvalue_serialize_mode,
                            },
                        )
                    })
                    .collect::<IndexMap<_, _>>();
                serialize_with_meta(&new_fields, &meta, serialize_mode, serializer)
            }
            Null(ref meta) => serialize_with_meta(&(), &meta, serialize_mode, serializer),
        }
    }
}

/// This newtype wrapper exists solely for the purpose of defining a
/// `Serialize` impl.
pub struct ResponseChecksMetadata<'a, T: Serialize>(pub (&'a T, &'a Vec<ResponseCheck>));

impl<'a, T: Serialize> serde::Serialize for ResponseChecksMetadata<'a, T> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let checks_map: HashMap<_, _> = self
            .0
             .1
            .iter()
            .map(|check| (check.name.clone(), check))
            .collect();
        let mut state = serializer.serialize_struct("Checked", 2)?;
        state.serialize_field("value", &self.0 .0)?;
        state.serialize_field("checks", &checks_map)?;
        state.end()
    }
}

fn serialize_with_meta<S: Serializer, T: Serialize>(
    value: &T,
    meta: &ResponseValueMeta,
    serialize_mode: &SerializeMode,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    let should_display_stream_state =
        meta.2.display && matches!(serialize_mode, SerializeMode::Partial);
    match (meta.1.len(), should_display_stream_state) {
        (0, false) => value.serialize(serializer),
        (_, false) => ResponseChecksMetadata((value, &meta.1)).serialize(serializer),
        (0, true) => {
            let mut state = serializer.serialize_struct("StreamState", 2)?;
            state.serialize_field("state", &meta.2.state)?;
            state.serialize_field("value", value)?;
            state.end()
        }
        (_, true) => {
            let mut outer_value = serializer.serialize_struct("StreamState", 2)?;
            outer_value.serialize_field("state", &meta.2.state)?;
            outer_value.serialize_field("value", &ResponseChecksMetadata((value, &meta.1)))?;
            outer_value.end()
        }
    }
}

pub fn from_str(
    of: &OutputFormatContent,
    target: &FieldType,
    raw_string: &str,
    allow_partials: bool,
) -> Result<BamlValueWithFlags> {
    if matches!(target, FieldType::Primitive(TypeValue::String)) {
        return Ok(BamlValueWithFlags::String(
            (raw_string.to_string(), target).into(),
        ));
    }

    // When the schema is just a string, i should really just return the raw_string w/o parsing it.
    let value = jsonish::parse(raw_string, jsonish::ParseOptions::default())?;

    // Pick the schema that is the most specific.
    log::debug!("Parsed JSONish (step 1 of parsing): {:#?}", value);
    let ctx = ParsingContext::new(of, allow_partials);

    // Determine the best way to get the desired schema from the parsed schema.

    // Lets try to now coerce the value into the expected schema.
    let parsed_value: BamlValueWithFlags = match target.coerce(&ctx, target, Some(&value)) {
        Ok(v) => {
            if v.conditions()
                .flags()
                .iter()
                .any(|f| matches!(f, Flag::InferedObject(jsonish::Value::String(_, _))))
            {
                anyhow::bail!("Failed to coerce value: {:?}", v.conditions().flags());
            }

            Ok::<BamlValueWithFlags, anyhow::Error>(v)
        }
        Err(e) => anyhow::bail!("Failed to coerce value: {}", e),
    }?;

    Ok(parsed_value)
}

pub fn from_str_xml(
    of: &OutputFormatContent,
    target: &FieldType,
    raw_string: &str,
    allow_partials: bool,
) -> Result<BamlValueWithFlags> {
    if matches!(target, FieldType::Primitive(TypeValue::String)) {
        return Ok(BamlValueWithFlags::String(
            (raw_string.to_string(), target).into(),
        ));
    }

    // Parse as XML instead of JSON
    let xml_value = xmlish::parse(raw_string, xmlish::ParseOptions::default())?;

    // Convert XML value to JSONish value for coercion
    let jsonish_value = xml_to_jsonish_value(xml_value)?;

    log::debug!(
        "Parsed XML and converted to JSONish (step 1 of parsing): {:#?}",
        jsonish_value
    );
    let ctx = ParsingContext::new(of, allow_partials);

    // Coerce the converted value into the expected schema
    let parsed_value: BamlValueWithFlags = match target.coerce(&ctx, target, Some(&jsonish_value)) {
        Ok(v) => {
            if v.conditions()
                .flags()
                .iter()
                .any(|f| matches!(f, Flag::InferedObject(jsonish::Value::String(_, _))))
            {
                anyhow::bail!("Failed to coerce value: {:?}", v.conditions().flags());
            }

            Ok::<BamlValueWithFlags, anyhow::Error>(v)
        }
        Err(e) => anyhow::bail!("Failed to coerce value: {}", e),
    }?;

    Ok(parsed_value)
}

/// Convert XML value to JSONish value for type coercion
fn xml_to_jsonish_value(xml_value: xmlish::Value) -> Result<jsonish::Value> {
    use baml_types::CompletionState;

    match xml_value {
        xmlish::Value::Text(text, completion_state) => {
            Ok(jsonish::Value::String(text, completion_state))
        }
        xmlish::Value::Element {
            tag,
            attributes,
            children,
            completion_state,
        } => {
            // Convert XML element to JSON object
            let mut obj_map = indexmap::IndexMap::new();

            // Add attributes as fields with @ prefix
            for (key, value) in &attributes {
                let attr_key = format!("@{}", key);
                obj_map.insert(
                    attr_key,
                    jsonish::Value::String(value.clone(), completion_state.clone()),
                );
            }

            // Special case: if the element contains only text content and no attributes,
            // flatten it to just the string value
            if children.len() == 1 && attributes.is_empty() {
                if let xmlish::Value::Text(text, _) = &children[0] {
                    return Ok(jsonish::Value::String(text.clone(), completion_state));
                }
            }

            // Collect text content and group child elements by tag name
            let mut text_content = String::new();
            let mut child_groups: std::collections::HashMap<String, Vec<jsonish::Value>> =
                std::collections::HashMap::new();

            for child in children {
                match &child {
                    xmlish::Value::Text(text, _) => {
                        text_content.push_str(text);
                    }
                    xmlish::Value::Element { tag: child_tag, .. } => {
                        let child_value = xml_to_jsonish_value(child.clone())?;
                        child_groups
                            .entry(child_tag.clone())
                            .or_insert_with(Vec::new)
                            .push(child_value);
                    }
                    _ => {
                        let child_value = xml_to_jsonish_value(child.clone())?;
                        child_groups
                            .entry("_unknown".to_string())
                            .or_insert_with(Vec::new)
                            .push(child_value);
                    }
                }
            }

            // Only add _text field if there are child elements AND text content
            if !text_content.trim().is_empty() && !child_groups.is_empty() {
                obj_map.insert(
                    "_text".to_string(),
                    jsonish::Value::String(text_content, completion_state.clone()),
                );
            }

            for (tag_name, values) in child_groups {
                if values.len() == 1 {
                    obj_map.insert(tag_name, values.into_iter().next().unwrap());
                } else {
                    obj_map.insert(
                        tag_name,
                        jsonish::Value::Array(values, completion_state.clone()),
                    );
                }
            }

            // Convert IndexMap to Vec for jsonish::Value::Object
            let obj_vec: Vec<(String, jsonish::Value)> = obj_map.into_iter().collect();
            Ok(jsonish::Value::Object(obj_vec, completion_state))
        }
        xmlish::Value::Fragment(text, completion_state) => {
            // Treat fragments as strings
            Ok(jsonish::Value::String(text, completion_state))
        }
        xmlish::Value::AnyOf(values, original) => {
            // Convert each possibility
            let converted_values = values
                .into_iter()
                .map(xml_to_jsonish_value)
                .collect::<Result<Vec<_>>>()?;
            Ok(jsonish::Value::AnyOf(converted_values, original))
        }
    }
}

impl ResponseBamlValue {
    pub fn score(&self) -> i32 {
        self.0.iter().map(|node| node.meta().0.score()).sum()
    }

    pub fn explanation_json(&self) -> Vec<serde_json::Value> {
        let mut expl = vec![];
        self.explanation_impl(vec!["<root>".to_string()], &mut expl);
        expl.into_iter().map(|e| e.to_ui_json()).collect::<Vec<_>>()
    }

    fn explanation_impl(&self, scope: Vec<String>, expls: &mut Vec<ParsingError>) {
        self.0.iter().for_each(|node| {
            let message = match node {
                BamlValueWithMeta::String(_, _) => "error while parsing string".to_string(),
                BamlValueWithMeta::Int(_, _) => "error while parsing int".to_string(),
                BamlValueWithMeta::Float(_, _) => "error while parsing float".to_string(),
                BamlValueWithMeta::Bool(_, _) => "error while parsing bool".to_string(),
                BamlValueWithMeta::List(_, _) => "error while parsing list".to_string(),
                BamlValueWithMeta::Map(_, _) => "error while parsing map".to_string(),
                BamlValueWithMeta::Enum(enum_name, _, _) => {
                    format!("error while parsing {enum_name} enum value")
                }
                BamlValueWithMeta::Class(class_name, _, _) => {
                    format!("error while parsing class {class_name}")
                }
                BamlValueWithMeta::Null(_) => "error while parsing null".to_string(),
                BamlValueWithMeta::Media(_, _) => "error while parsing media".to_string(),
            };
            let parsing_error = ParsingError {
                scope: scope.clone(),
                reason: message,
                causes: DeserializerConditions {
                    flags: node.meta().0.clone(),
                }
                .explanation(),
            };
            if node.meta().0.len() > 0 {
                expls.push(parsing_error)
            }
        })
    }
}

impl From<ResponseBamlValue> for BamlValue {
    fn from(v: ResponseBamlValue) -> BamlValue {
        v.0.into()
    }
}

impl WithScore for ResponseBamlValue {
    fn score(&self) -> i32 {
        self.0.iter().map(|node| node.meta().0.score()).sum()
    }
}

#[cfg(test)]
mod xml_tests {
    use super::*;
    use crate::xmlish;
    use baml_types::FieldType;
    use internal_baml_jinja::types::OutputFormatContent;

    #[test]
    fn test_xml_parsing_simple() {
        let xml = "<person><name>John</name><age>30</age></person>";
        let result = xmlish::parse(xml, xmlish::ParseOptions::default()).unwrap();

        match result {
            xmlish::Value::Element { tag, children, .. } => {
                assert_eq!(tag, "person");
                assert_eq!(children.len(), 2);
            }
            _ => panic!("Expected XML element"),
        }
    }

    #[test]
    fn test_xml_to_jsonish_conversion() {
        let xml_value = xmlish::Value::element(
            "person".to_string(),
            std::collections::HashMap::new(),
            vec![
                xmlish::Value::element(
                    "name".to_string(),
                    std::collections::HashMap::new(),
                    vec![xmlish::Value::text("John".to_string())],
                ),
                xmlish::Value::element(
                    "age".to_string(),
                    std::collections::HashMap::new(),
                    vec![xmlish::Value::text("30".to_string())],
                ),
            ],
        );

        let jsonish_value = xml_to_jsonish_value(xml_value).unwrap();

        match jsonish_value {
            jsonish::Value::Object(obj, _) => {
                assert!(obj.len() >= 2); // Should have at least name and age fields
            }
            _ => panic!("Expected JSON object"),
        }
    }

    #[test]
    fn test_nested_xml_parsing_complex() {
        let xml = r#"<TestClassNested>
  <prop1>Sample Text</prop1>
  <prop2>
    <InnerClass>
      <prop1>Inner Text</prop1>
      <prop2>Another piece of text</prop2>
      <inner>
        <InnerClass2>
          <prop2>42</prop2>
          <prop3>3.14</prop3>
        </InnerClass2>
      </inner>
    </InnerClass>
  </prop2>
</TestClassNested>"#;

        // First test XML parsing
        let xml_result = xmlish::parse(xml, xmlish::ParseOptions::default()).unwrap();

        match &xml_result {
            xmlish::Value::Element { tag, children, .. } => {
                assert_eq!(tag, "TestClassNested");
                assert_eq!(children.len(), 2); // prop1 and prop2
            }
            _ => panic!("Expected XML element"),
        }

        // Test XML to JSON conversion
        let jsonish_result = xml_to_jsonish_value(xml_result).unwrap();

        // Verify the structure matches expectations
        match &jsonish_result {
            jsonish::Value::Object(obj, _) => {
                let prop1_found = obj.iter().any(|(k, _)| k == "prop1");
                let prop2_found = obj.iter().any(|(k, _)| k == "prop2");
                assert!(prop1_found, "prop1 not found in converted object");
                assert!(prop2_found, "prop2 not found in converted object");

                // Check prop1 value
                let prop1_value = obj.iter().find(|(k, _)| k == "prop1").unwrap().1.clone();
                match prop1_value {
                    jsonish::Value::String(s, _) => {
                        assert_eq!(s, "Sample Text");
                    }
                    _ => panic!("Expected prop1 to be a string"),
                }

                // Check prop2 nested structure
                let prop2_value = obj.iter().find(|(k, _)| k == "prop2").unwrap().1.clone();
                match prop2_value {
                    jsonish::Value::Object(prop2_obj, _) => {
                        // prop2 should contain InnerClass
                        let inner_class_found = prop2_obj.iter().any(|(k, _)| k == "InnerClass");
                        assert!(inner_class_found, "InnerClass not found in prop2");

                        // Get the InnerClass object
                        let inner_class_value = prop2_obj
                            .iter()
                            .find(|(k, _)| k == "InnerClass")
                            .unwrap()
                            .1
                            .clone();
                        match inner_class_value {
                            jsonish::Value::Object(inner_obj, _) => {
                                let inner_prop1_found = inner_obj.iter().any(|(k, _)| k == "prop1");
                                let inner_prop2_found = inner_obj.iter().any(|(k, _)| k == "prop2");
                                let inner_inner_found = inner_obj.iter().any(|(k, _)| k == "inner");
                                assert!(inner_prop1_found, "prop1 not found in InnerClass object");
                                assert!(inner_prop2_found, "prop2 not found in InnerClass object");
                                assert!(inner_inner_found, "inner not found in InnerClass object");

                                // Check inner.InnerClass2
                                let inner_inner_value = inner_obj
                                    .iter()
                                    .find(|(k, _)| k == "inner")
                                    .unwrap()
                                    .1
                                    .clone();
                                match inner_inner_value {
                                    jsonish::Value::Object(inner_inner_obj, _) => {
                                        let inner_class2_found =
                                            inner_inner_obj.iter().any(|(k, _)| k == "InnerClass2");
                                        assert!(
                                            inner_class2_found,
                                            "InnerClass2 not found in inner object"
                                        );

                                        let inner_class2_value = inner_inner_obj
                                            .iter()
                                            .find(|(k, _)| k == "InnerClass2")
                                            .unwrap()
                                            .1
                                            .clone();
                                        match inner_class2_value {
                                            jsonish::Value::Object(inner2_obj, _) => {
                                                let inner2_prop2_found =
                                                    inner2_obj.iter().any(|(k, _)| k == "prop2");
                                                let inner2_prop3_found =
                                                    inner2_obj.iter().any(|(k, _)| k == "prop3");
                                                assert!(
                                                    inner2_prop2_found,
                                                    "prop2 not found in InnerClass2 object"
                                                );
                                                assert!(
                                                    inner2_prop3_found,
                                                    "prop3 not found in InnerClass2 object"
                                                );
                                            }
                                            _ => panic!("Expected InnerClass2 to be an object"),
                                        }
                                    }
                                    _ => panic!("Expected inner to be an object"),
                                }
                            }
                            _ => panic!("Expected InnerClass to be an object"),
                        }
                    }
                    _ => panic!("Expected prop2 to be an object"),
                }
            }
            _ => panic!("Expected root to be an object, got: {:?}", jsonish_result),
        }
    }

    #[test]
    fn test_xml_detection_logic() {
        let xml_content = r#"<TestClassNested>
  <prop1>Sample Text</prop1>
  <prop2>
    <InnerClass>
      <prop1>Inner Text Content</prop1>
      <prop2>Another Text Content</prop2>
      <inner>
        <InnerClass2>
          <prop2>42</prop2>
          <prop3>3.14</prop3>
        </InnerClass2>
      </inner>
    </InnerClass>
  </prop2>
</TestClassNested>"#;

        let json_content = r#"{"prop1": "Sample Text", "prop2": {"prop1": "Inner Text"}}"#;

        // Test XML detection logic
        let is_xml_1 =
            xml_content.trim_start().starts_with('<') && xml_content.trim_end().ends_with('>');
        let is_xml_2 =
            json_content.trim_start().starts_with('<') && json_content.trim_end().ends_with('>');

        assert!(is_xml_1, "Should detect XML content");
        assert!(!is_xml_2, "Should not detect JSON content as XML");
    }
}
