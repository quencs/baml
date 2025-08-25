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
use baml_types::{BamlValue, Constraint, JinjaExpression};
use internal_baml_core::ir::{
    jinja_helpers::evaluate_predicate, 
    baml_helpers::evaluate_native_predicate,
    TypeIR
};
use internal_baml_jinja::types::OutputFormatContent;

use super::types::BamlValueWithFlags;
use crate::jsonish;

pub struct ParsingContext<'a> {
    pub scope: Vec<String>,
    visited_during_coerce: HashSet<(String, jsonish::Value)>,
    visited_during_try_cast: HashSet<(String, jsonish::Value)>,
    /// THIS IS A TEMPORARY HACK (ask vaibhav)
    pub do_not_use_mode: baml_types::StreamingMode,
    pub of: &'a OutputFormatContent,
}

impl ParsingContext<'_> {
    pub fn display_scope(&self) -> String {
        if self.scope.is_empty() {
            return "<root>".to_string();
        }
        self.scope.join(".")
    }

    pub(crate) fn new(
        of: &OutputFormatContent,
        mode: baml_types::StreamingMode,
    ) -> ParsingContext<'_> {
        ParsingContext {
            scope: Vec::new(),
            visited_during_coerce: HashSet::new(),
            visited_during_try_cast: HashSet::new(),
            do_not_use_mode: mode,
            of,
        }
    }

    pub(crate) fn enter_scope(&self, scope: &str) -> ParsingContext<'_> {
        let mut new_scope = self.scope.clone();
        new_scope.push(scope.to_string());
        ParsingContext {
            scope: new_scope,
            visited_during_coerce: self.visited_during_coerce.clone(),
            visited_during_try_cast: self.visited_during_try_cast.clone(),
            of: self.of,
            do_not_use_mode: self.do_not_use_mode,
        }
    }

    // TODO: This function and `enter_scope` are clonning both the scope vector
    // and visited hash set each time. Maybe it can be optimized with interior
    // mutability or something.
    pub(crate) fn visit_class_value_pair(
        &self,
        cls_value_pair: (String, jsonish::Value),
        is_coerce: bool,
    ) -> ParsingContext<'_> {
        let mut new_visited_coerce = self.visited_during_coerce.clone();
        let mut new_visited_try_cast = self.visited_during_try_cast.clone();
        if is_coerce {
            new_visited_coerce.insert(cls_value_pair);
        } else {
            new_visited_try_cast.insert(cls_value_pair);
        }
        ParsingContext {
            scope: self.scope.clone(),
            visited_during_coerce: new_visited_coerce,
            visited_during_try_cast: new_visited_try_cast,
            of: self.of,
            do_not_use_mode: self.do_not_use_mode,
        }
    }

    pub(crate) fn error_too_many_matches<T: std::fmt::Display>(
        &self,
        target: &TypeIR,
        options: impl IntoIterator<Item = T>,
    ) -> ParsingError {
        ParsingError {
            reason: format!(
                "Too many matches for {}. Got: {}",
                target,
                options.into_iter().fold("".to_string(), |acc, f| {
                    if acc.is_empty() {
                        return f.to_string();
                    }
                    format!("{acc}, {f}")
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

    pub(crate) fn error_unexpected_empty_array(&self, target: &TypeIR) -> ParsingError {
        ParsingError {
            reason: format!("Expected {target}, got empty array"),
            scope: self.scope.clone(),
            causes: vec![],
        }
    }

    pub(crate) fn error_unexpected_null(&self, target: &TypeIR) -> ParsingError {
        ParsingError {
            reason: format!("Expected {target}, got null"),
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

    pub(crate) fn error_pdf_not_supported(&self) -> ParsingError {
        ParsingError {
            reason: "Pdf type is not supported here".to_string(),
            scope: self.scope.clone(),
            causes: vec![],
        }
    }

    pub(crate) fn error_video_not_supported(&self) -> ParsingError {
        ParsingError {
            reason: "Video type is not supported here".to_string(),
            scope: self.scope.clone(),
            causes: vec![],
        }
    }

    pub(crate) fn error_map_must_have_supported_key(&self, key_type: &TypeIR) -> ParsingError {
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
                    reason: format!("Missing required field: {k}"),
                    causes: vec![],
                })
                .chain(unparsed.into_iter().map(|(k, e)| ParsingError {
                    scope: self.scope.clone(),
                    reason: format!("Failed to parse field {k}: {e}"),
                    causes: vec![e.clone()],
                }))
                .collect(),
        }
    }

    pub(crate) fn error_unexpected_type<T: std::fmt::Display + std::fmt::Debug>(
        &self,
        target: &TypeIR,
        got: &T,
    ) -> ParsingError {
        ParsingError {
            reason: format!(
                "Expected {}, got {:?}.",
                match target {
                    TypeIR::Enum { .. } => format!("{target} enum value"),
                    TypeIR::Class { .. } => format!("{target}"),
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
            reason: format!("Internal error: {error}"),
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
            write!(f, "\n  - {}", format!("{cause}").replace("\n", "\n  "))?;
        }
        Ok(())
    }
}

impl std::error::Error for ParsingError {}

pub trait TypeCoercer {
    fn coerce(
        &self,
        ctx: &ParsingContext,
        target: &TypeIR,
        value: Option<&crate::jsonish::Value>,
    ) -> Result<BamlValueWithFlags, ParsingError>;

    fn try_cast(
        &self,
        ctx: &ParsingContext,
        target: &TypeIR,
        value: Option<&crate::jsonish::Value>,
    ) -> Option<BamlValueWithFlags>;
}

pub trait DefaultValue {
    fn default_value(&self, error: Option<&ParsingError>) -> Option<BamlValueWithFlags>;
}

/// Run all checks and asserts for a value at a given type.
/// This function only runs checks on the top-level node of the `BamlValue`.
/// Checks on nested fields, list items etc. are not run here.
///
/// For a function that traverses a whole `BamlValue` looking for failed asserts,
/// see `first_failing_assert_nested`.
pub fn run_user_checks(baml_value: &BamlValue, type_: &TypeIR) -> Result<Vec<(Constraint, bool)>> {
    let res = type_
        .meta()
        .constraints
        .iter()
        .map(|constraint| {
            let result = match &constraint.expression {
                baml_types::ConstraintExpression::Jinja(jinja_expr) => {
                    evaluate_predicate(baml_value, jinja_expr)?
                }
                baml_types::ConstraintExpression::Native(native_expr) => {
                    // Use native expression evaluator for BAML constraint expressions
                    let context = HashMap::new(); // Add any additional context as needed
                    
                    // For Phase 2, we use a placeholder evaluator that parses from string
                    // In future phases, this will use proper THIR expression evaluation
                    match native_expr.parse::<bool>() {
                        Ok(bool_result) => bool_result,
                        Err(_) => {
                            // If it's not a simple boolean, use the native evaluator
                            // For now, this is a placeholder implementation
                            log::debug!("Evaluating native constraint: {}", native_expr);
                            
                            // Create a placeholder Expr for the evaluator
                            // In full implementation, this will come from THIR
                            use baml_types::expr::{Expr, ExprMetadata};
                            use baml_types::BamlValueWithMeta;
                            use internal_baml_core::internal_baml_diagnostics::Span;
                            let placeholder_expr = Expr::Atom(BamlValueWithMeta::Bool(true, 
                                (Span::fake(), None)));
                            
                            evaluate_native_predicate(baml_value, &context, &placeholder_expr)
                                .unwrap_or_else(|e| {
                                    log::warn!("Native constraint evaluation failed: {}", e);
                                    false
                                })
                        }
                    }
                }
            };
            Ok((constraint.clone(), result))
        })
        .collect::<Result<Vec<_>>>();
    res
}
