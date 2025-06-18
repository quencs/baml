use anyhow::Result;
use std::collections::HashSet;

use baml_types::{
    ir_type::{Type, TypeStreaming},
    BamlMap, BamlMedia, BamlValue, BamlValueWithMeta, Constraint, FieldType, JinjaExpression,
};
use serde_json::json;
use strsim::jaro;

use super::{
    coercer::ParsingError,
    deserialize_flags::{DeserializerConditions, Flag},
    score::WithScore,
};

/// A deserialization target for full (non-streaming values, which captures
/// the parser scoring at each node in the nested value.
pub type BamlValueWithFlags = BamlValueWithMeta<(DeserializerConditions, Type)>;

/// A deserialization target for streaming values, which captures
/// the parser scoring at each node in the nested value.
pub type BamlValueStreamingWithFlags = BamlValueWithMeta<(DeserializerConditions, TypeStreaming)>;

/// A convenience type for values with flags for string matching operations.
pub type ValueWithFlags<T> = (T, FieldType, DeserializerConditions);

/// A trait for Metadata types that contain some sort of "Type-like" field,
/// (e.g. `Type` or `TypeStreaming`).
pub trait HasType {
    /// The type of the "Type-like" field.
    type Type: std::fmt::Display;

    /// Get a reference to the type.
    fn r#type(&self) -> &Self::Type;

    /// Get a mutable reference to the type.
    fn type_mut(&mut self) -> &mut Self::Type;
}

/// `BamlValueWithFlags` metadata implements `HasType`. The type is `Type`, the
/// base BAML type.
impl HasType for (DeserializerConditions, Type) {
    type Type = Type;
    fn r#type(&self) -> &Type {
        &self.1
    }
    fn type_mut(&mut self) -> &mut Type {
        &mut self.1
    }
}

/// `BamlValueStreamingWithFlags` metadata implements `HasType`. The type is
/// `TypeStreaming`, the streaming BAML type.
impl HasType for (DeserializerConditions, TypeStreaming) {
    type Type = TypeStreaming;
    fn r#type(&self) -> &TypeStreaming {
        &self.1
    }
    fn type_mut(&mut self) -> &mut TypeStreaming {
        &mut self.1
    }
}

/// Default implementation for base type metadata
impl Default for (DeserializerConditions, Type) {
    fn default() -> Self {
        (DeserializerConditions::default(), Type::null())
    }
}

/// Default implementation for streaming type metadata
impl Default for (DeserializerConditions, TypeStreaming) {
    fn default() -> Self {
        (DeserializerConditions::default(), TypeStreaming::null())
    }
}

/// A trait for Metadata types that contain a `DeserializerConditions` field.
pub trait HasFlags {
    /// Get a reference to the flags.
    fn flags(&self) -> &DeserializerConditions;

    /// Get a mutable reference to the flags.
    fn flags_mut(&mut self) -> &mut DeserializerConditions;

    /// Get the score of the flags under a value, including the score of all
    /// flags under the value's children.
    fn score(value: &BamlValueWithMeta<Self>) -> i32
    where
        Self: Sized,
    {
        value.iter().map(|v| v.meta().flags().score()).sum()
    }

    /// Get the explanation of the flags under a value, including the explanation
    /// of all flags under the value's children.
    fn explanation_json(value: &BamlValueWithMeta<Self>) -> Vec<serde_json::Value>
    where
        Self: Sized + HasType,
    {
        let mut expl = vec![];
        Self::explanation_impl(value, vec!["<root>".to_string()], &mut expl);
        expl.into_iter().map(|e| e.to_ui_json()).collect::<Vec<_>>()
    }

    // A helper function used by `explanation_json` to recursively build up the
    // explanation of a value.
    fn explanation_impl(
        value: &BamlValueWithMeta<Self>,
        scope: Vec<String>,
        expls: &mut Vec<ParsingError>,
    ) where
        Self: HasType + Sized,
    {
        let shallow_causes = value.meta().flags().explanation();
        let type_name = value.meta().r#type().to_string();
        if !shallow_causes.is_empty() {
            expls.push(ParsingError {
                scope: scope.clone(),
                reason: format!("error while parsing {}", type_name),
                causes: shallow_causes,
            });
        }
        match value {
            BamlValueWithMeta::String(_, _) => {}
            BamlValueWithMeta::Int(_, _) => {}
            BamlValueWithMeta::Float(_, _) => {}
            BamlValueWithMeta::Bool(_, _) => {}
            BamlValueWithMeta::List(values, _) => {
                for (i, value) in values.iter().enumerate() {
                    let mut scope = scope.clone();
                    scope.push(format!("parsed:{}", i));
                    Self::explanation_impl(&value, scope, expls);
                }
            }
            BamlValueWithMeta::Map(kvs, flags) => {
                for (k, v) in kvs.iter() {
                    let causes = v.meta().flags().explanation();
                    if !causes.is_empty() {
                        expls.push(ParsingError {
                            scope: scope.clone(),
                            reason: format!("error while parsing value for map key '{}'", k),
                            causes,
                        });
                    }
                    let mut scope = scope.clone();
                    scope.push(format!("parsed:{}", k));
                    Self::explanation_impl(&v, scope, expls);
                }
            }
            BamlValueWithMeta::Enum(enum_name, target, v) => {}
            BamlValueWithMeta::Class(class_name, fields, _) => {
                for (k, v) in fields.iter() {
                    let mut scope = scope.clone();
                    scope.push(k.to_string());
                    Self::explanation_impl(&v, scope, expls);
                }
            }

            BamlValueWithMeta::Null(_) => {}
            BamlValueWithMeta::Media(target, v) => {}
        }
    }
}

impl HasFlags for (DeserializerConditions, Type) {
    fn flags(&self) -> &DeserializerConditions {
        &self.0
    }
    fn flags_mut(&mut self) -> &mut DeserializerConditions {
        &mut self.0
    }
}

impl HasFlags for (DeserializerConditions, TypeStreaming) {
    fn flags(&self) -> &DeserializerConditions {
        &self.0
    }
    fn flags_mut(&mut self) -> &mut DeserializerConditions {
        &mut self.0
    }
}

pub trait ParsingErrorToUiJson {
    fn to_ui_json(&self) -> serde_json::Value;
}

impl ParsingErrorToUiJson for ParsingError {
    fn to_ui_json(&self) -> serde_json::Value {
        json!({
            if self.scope.is_empty() {
                "<root>".to_string()
            } else {
                self.scope.join(".")
            }: self.reason,
            "causes": self.causes.iter().map(|c| c.to_ui_json()).collect::<Vec<_>>(),
        })
    }
}
