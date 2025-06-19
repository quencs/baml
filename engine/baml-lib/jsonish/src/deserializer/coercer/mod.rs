mod array_helper;
mod coerce_array;
mod coerce_literal;
mod coerce_map;
mod coerce_primitive;
mod coerce_union;
mod field_type;
mod ir_ref;
mod match_string;

use std::collections::{HashMap, HashSet};

use anyhow::Result;

use baml_types::{ir_type::TypeGeneric, BamlValue, BamlValueWithMeta, Constraint, JinjaExpression};
use internal_baml_jinja::types::OutputFormatContent;

use internal_baml_core::ir::{jinja_helpers::evaluate_predicate, FieldType};

use crate::jsonish;

use super::types::{
    BamlValueStreamingWithFlags, BamlValueWithFlags, HasConstraints, HasFlags, HasType,
};

pub struct ParsingContext<'a> {
    pub scope: Vec<String>,
    visited: HashSet<(String, jsonish::Value)>,
    pub of: &'a OutputFormatContent,
    // pub allow_partials: bool,
}

impl ParsingContext<'_> {
    pub fn display_scope(&self) -> String {
        if self.scope.is_empty() {
            return "<root>".to_string();
        }
        self.scope.join(".")
    }

    pub(crate) fn new(of: &OutputFormatContent) -> ParsingContext<'_> {
        ParsingContext {
            scope: Vec::new(),
            visited: HashSet::new(),
            of,
        }
    }

    pub(crate) fn enter_scope(&self, scope: &str) -> ParsingContext {
        let mut new_scope = self.scope.clone();
        new_scope.push(scope.to_string());
        ParsingContext {
            scope: new_scope,
            visited: self.visited.clone(),
            of: self.of,
        }
    }

    // TODO: This function and `enter_scope` are clonning both the scope vector
    // and visited hash set each time. Maybe it can be optimized with interior
    // mutability or something.
    pub(crate) fn visit_class_value_pair(
        &self,
        cls_value_pair: (String, jsonish::Value),
    ) -> ParsingContext {
        let mut new_visited = self.visited.clone();
        new_visited.insert(cls_value_pair);
        ParsingContext {
            scope: self.scope.clone(),
            visited: new_visited,
            of: self.of,
        }
    }

    pub(crate) fn error_too_many_matches<T>(
        &self,
        target: &TypeGeneric<T>,
        options: impl IntoIterator<Item = TypeGeneric<T>>,
    ) -> ParsingError
    where
        TypeGeneric<T>: std::fmt::Display,
    {
        ParsingError {
            reason: format!(
                "Too many matches for {}. Got: {}",
                target,
                options.into_iter().fold("".to_string(), |acc, f| {
                    if acc.is_empty() {
                        return f.to_string();
                    }
                    format!("{}, {}", acc, f)
                })
            ),
            scope: self.scope.clone(),
            causes: vec![],
        }
    }

    pub(crate) fn error_merge_multiple<'a>(
        &self,
        summary: &str,
        error: impl IntoIterator<Item = &'a ParsingError>,
    ) -> ParsingError {
        ParsingError {
            reason: summary.to_string(),
            scope: self.scope.clone(),
            causes: error.into_iter().cloned().collect(),
        }
    }

    pub(crate) fn error_unexpected_empty_array(&self, target: &FieldType) -> ParsingError {
        ParsingError {
            reason: format!("Expected {}, got empty array", target),
            scope: self.scope.clone(),
            causes: vec![],
        }
    }

    pub(crate) fn error_unexpected_null<T>(&self, target: &TypeGeneric<T>) -> ParsingError
    where
        TypeGeneric<T>: std::fmt::Display,
    {
        ParsingError {
            reason: format!("Expected {}, got null", target),
            scope: self.scope.clone(),
            causes: vec![],
        }
    }

    pub(crate) fn error_image_not_supported(&self) -> ParsingError {
        ParsingError {
            reason: "Image type is not supported here".to_string(),
            scope: self.scope.clone(),
            causes: vec![],
        }
    }

    pub(crate) fn error_audio_not_supported(&self) -> ParsingError {
        ParsingError {
            reason: "Audio type is not supported here".to_string(),
            scope: self.scope.clone(),
            causes: vec![],
        }
    }

    pub(crate) fn error_map_must_have_supported_key(&self, key_type: &FieldType) -> ParsingError {
        ParsingError {
            reason: format!(
                "Maps may only have strings, enums or literal strings for keys, but got {key_type}"
            ),
            scope: self.scope.clone(),
            causes: vec![],
        }
    }

    pub(crate) fn error_missing_required_field(
        &self,
        unparsed: Vec<(String, &ParsingError)>,
        missing: Vec<String>,
        _item: Option<&crate::jsonish::Value>,
    ) -> ParsingError {
        ParsingError {
            reason: format!(
                "Failed while parsing required fields: missing={}, unparsed={}",
                missing.len(),
                unparsed.len()
            ),
            scope: self.scope.clone(),
            causes: missing
                .into_iter()
                .map(|k| ParsingError {
                    scope: self.scope.clone(),
                    reason: format!("Missing required field: {}", k),
                    causes: vec![],
                })
                .chain(unparsed.into_iter().map(|(k, e)| ParsingError {
                    scope: self.scope.clone(),
                    reason: format!("Failed to parse field {}: {}", k, e),
                    causes: vec![e.clone()],
                }))
                .collect(),
        }
    }

    pub(crate) fn error_unexpected_type<T: std::fmt::Display>(
        &self,
        target: &TypeGeneric<T>,
        got: &crate::jsonish::Value,
    ) -> ParsingError
    where
        TypeGeneric<T>: std::fmt::Display,
    {
        ParsingError {
            reason: format!(
                "Expected {}, got {}.",
                match target {
                    TypeGeneric::Enum { .. } => format!("{} enum value", target),
                    TypeGeneric::Class { .. } => format!("{}", target),
                    _ => format!("{target}"),
                },
                got
            ),
            scope: self.scope.clone(),
            causes: vec![],
        }
    }

    pub(crate) fn error_internal<T: std::fmt::Display>(&self, error: T) -> ParsingError {
        ParsingError {
            reason: format!("Internal error: {}", error),
            scope: self.scope.clone(),
            causes: vec![],
        }
    }

    pub(crate) fn error_circular_reference(
        &self,
        cls: &str,
        value: &jsonish::Value,
    ) -> ParsingError {
        ParsingError {
            reason: format!("Circular reference detected for class-value pair {cls} <-> {value}"),
            scope: self.scope.clone(),
            causes: vec![],
        }
    }
}

#[derive(Debug, Clone)]
pub struct ParsingError {
    pub scope: Vec<String>,
    pub reason: String,
    pub causes: Vec<ParsingError>,
}

impl std::fmt::Display for ParsingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}: {}",
            if self.scope.is_empty() {
                "<root>".to_string()
            } else {
                self.scope.join(".")
            },
            self.reason
        )?;
        for cause in &self.causes {
            write!(f, "\n  - {}", format!("{}", cause).replace("\n", "\n  "))?;
        }
        Ok(())
    }
}

impl std::error::Error for ParsingError {}

pub trait TypeCoercer<T, M: HasType<Meta = T> + HasFlags> {
    fn coerce(
        &self,
        ctx: &ParsingContext,
        target: &TypeGeneric<T>,
        value: Option<&crate::jsonish::Value>,
    ) -> Result<BamlValueWithMeta<M>, ParsingError>;
}

pub trait DefaultValue<T, M: HasType<Meta = T> + HasFlags> {
    fn default_value(&self, error: Option<&ParsingError>) -> Option<BamlValueWithMeta<M>>;
}

/// Run all checks and asserts for a value at a given type.
/// This function only runs checks on the top-level node of the `BamlValue`.
/// Checks on nested fields, list items etc. are not run here.
///
/// For a function that traverses a whole `BamlValue` looking for failed asserts,
/// see `first_failing_assert_nested`.
pub fn run_user_checks<T, M: HasType<Meta = T> + HasFlags>(
    baml_value: &BamlValue,
    type_: &TypeGeneric<T>,
) -> Result<Vec<(Constraint, bool)>>
where
    TypeGeneric<T>: std::fmt::Display,
    T: HasConstraints,
{
    eprintln!("baml_value: {:?} type: {}", baml_value, type_);
    let res = type_
        .meta()
        .constraints()
        .iter()
        .map(|constraint| {
            let result = evaluate_predicate(baml_value, &constraint.expression)?;
            Ok((constraint.clone(), result))
        })
        .collect::<Result<Vec<_>>>();
    eprintln!("res: {:?}", res);
    res
}
