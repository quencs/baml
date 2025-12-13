//! Typed Next-Key Streaming Parser
//!
//! This module implements a type-directed streaming parser that avoids the exponential
//! AnyOf blowup of the legacy parser by using bounded beam search.
//!
//! Key design:
//! - Directly produces BamlValueWithFlags without intermediate jsonish::Value
//! - Uses ExpectedTypeSet (bounded beam) instead of nested AnyOf
//! - Incrementally parses with next_keys() guidance
//! - Preserves all coercion behaviors from the existing deserializer tests

mod coerce;
mod expected_set;
mod extract;
mod frames;
mod lexer;
mod parser;
mod schema_index;
mod session;

pub use expected_set::{Candidate, ExpectedTypeSet, DEFAULT_BEAM_K};
pub use frames::{ArrayFrame, CompletionState, ObjectFrame, ParsedValue, ParsedValueKind, ValueFrame};
pub use lexer::{Lexer, QuoteStyle, Token};
pub use parser::{KeyHint, ParseUpdate, TypedStreamParser};
pub use schema_index::{FieldInfo, PrimitiveKind, SchemaIndex, TypeId, TypeInfo, TypeKind};
pub use session::ParseSession;

use crate::deserializer::coercer::{run_user_checks, validate_asserts, ParsingError};
use crate::deserializer::deserialize_flags::Flag;
use crate::deserializer::types::BamlValueWithFlags;
use baml_types::{BamlValue, TypeIR, StreamingMode};
use internal_baml_jinja::types::OutputFormatContent;

/// Main entry point: parse raw string to typed value
///
/// This function replaces the legacy `jsonish::parse()` + `TypeIR::coerce()` pipeline.
pub fn parse(
    of: &OutputFormatContent,
    root: &TypeIR,
    raw: &str,
    streaming: bool,
) -> anyhow::Result<BamlValueWithFlags> {
    let parser = TypedStreamParser::new_with_context(root, of, DEFAULT_BEAM_K);
    let mut session = parser.new_session();

    // For non-JSON-ish input, try raw string coercion first
    // This handles cases like "1 cup unsalted butter, room temperature" for int|string targets
    let trimmed = raw.trim();
    if !extract::looks_like_json(trimmed) {
        if let Ok(value) = try_raw_string_coercion(&parser.schema, trimmed, streaming) {
            if streaming {
                return Ok(value);
            }
            // Apply constraints to the string coercion result
            match apply_constraints(of, root, value) {
                Ok(v) => return Ok(v),
                Err(_) => {}  // Continue to span extraction
            }
        }
    }

    // Extract candidate spans
    let spans = extract::extract_spans(raw, 2);

    // Try each span, pick best result
    let mut best_result: Option<(i32, BamlValueWithFlags)> = None;

    for span in spans {
        let segment = &raw[span.range.clone()];

        // Reset session for this span
        session = parser.new_session();

        if let Err(_) = parser.ingest(&mut session, segment) {
            continue;
        }

        match parser.finish(&session, streaming) {
            Ok(value) => {
                // Apply constraint checking (skip during streaming)
                let value = if streaming {
                    value
                } else {
                    match apply_constraints(of, root, value) {
                        Ok(v) => v,
                        Err(_) => continue,
                    }
                };

                let score = compute_quality_score(&value, &span);
                if best_result.as_ref().map(|(s, _)| score > *s).unwrap_or(true) {
                    best_result = Some((score, value));
                }
            }
            Err(_) => continue,
        }
    }

    // Try raw string coercion for enum/primitive types when:
    // 1. No good result was found, OR
    // 2. The best result is Null (which might be from parsing {} or other non-meaningful spans)
    let should_try_raw_coercion = best_result.is_none()
        || best_result.as_ref().map(|(_, v)| matches!(v, BamlValueWithFlags::Null(..)))
            .unwrap_or(false);

    if should_try_raw_coercion {
        if let Ok(value) = try_raw_string_coercion(&parser.schema, raw.trim(), streaming) {
            // Apply constraints to the string coercion result too
            let value = if streaming {
                value
            } else {
                match apply_constraints(of, root, value) {
                    Ok(v) => v,
                    Err(_) => {
                        // Coercion failed constraints, fall back to best span result
                        return best_result
                            .map(|(_, v)| v)
                            .ok_or_else(|| anyhow::anyhow!("Failed to parse any span"));
                    }
                }
            };
            return Ok(value);
        }
    }

    best_result
        .map(|(_, v)| v)
        .ok_or_else(|| anyhow::anyhow!("Failed to parse any span"))
}

/// Apply constraint checking (asserts and checks) to a parsed value
fn apply_constraints(
    of: &OutputFormatContent,
    target_type: &TypeIR,
    mut value: BamlValueWithFlags,
) -> anyhow::Result<BamlValueWithFlags> {
    // Get constraints from the type's metadata
    let type_constraints = target_type.meta().constraints.clone();

    // Also get class-level constraints if this is a class type
    let class_constraints = match target_type {
        TypeIR::Class { name, mode, .. } => {
            of.classes.get(&(name.clone(), *mode))
                .or_else(|| of.classes.get(&(name.clone(), StreamingMode::NonStreaming)))
                .map(|c| c.constraints.clone())
                .unwrap_or_default()
        }
        _ => vec![],
    };

    // Combine all constraints
    let all_constraints: Vec<_> = type_constraints.into_iter()
        .chain(class_constraints.into_iter())
        .collect();

    if all_constraints.is_empty() {
        return Ok(value);
    }

    // Convert to BamlValue for constraint evaluation
    let baml_value: BamlValue = value.clone().into();

    // Evaluate all constraints
    let constraint_results = run_user_checks(&baml_value, target_type)
        .map_err(|e| anyhow::anyhow!("Failed to evaluate constraints: {e}"))?;

    // Validate asserts (fail if any assert failed)
    validate_asserts(&constraint_results)
        .map_err(|e| anyhow::anyhow!("Assert failed: {}", e.reason))?;

    // Add check results as flags
    let check_results: Vec<_> = constraint_results
        .into_iter()
        .filter_map(|(constraint, result)| {
            constraint.as_check()
                .map(|(label, expr)| (label, expr, result))
        })
        .collect();

    if !check_results.is_empty() {
        value.add_flag(Flag::ConstraintResults(check_results));
    }

    Ok(value)
}

/// Try to coerce raw input as a string value for primitive targets
fn try_raw_string_coercion(
    schema: &schema_index::SchemaIndex,
    raw: &str,
    streaming: bool,
) -> anyhow::Result<BamlValueWithFlags> {
    use frames::{CompletionState, ParsedValue, ParsedValueKind};

    // Create a ParsedValue containing the raw string
    let parsed = ParsedValue {
        value: ParsedValueKind::String(raw.to_string()),
        completion: CompletionState::Complete,
        type_id: Some(schema.root_id()),
    };

    // Try to coerce it to the target type
    coerce::convert_to_baml_value(schema, schema.root_id(), &parsed, streaming)
}

fn compute_quality_score(value: &BamlValueWithFlags, span: &extract::CandidateSpan) -> i32 {
    use crate::deserializer::deserialize_flags::Flag;

    let mut score = span.score;

    // Boost for complete values
    let conditions = value.conditions();
    if !conditions
        .flags
        .iter()
        .any(|f| matches!(f, Flag::Incomplete | Flag::Pending))
    {
        score += 10;
    }

    // Penalize repairs
    let repair_count = conditions
        .flags
        .iter()
        .filter(|f| matches!(f, Flag::ObjectFromFixedJson(_)))
        .count();
    score -= repair_count as i32 * 2;

    score
}

#[cfg(test)]
mod tests {
    use super::*;
    use baml_types::{type_meta, TypeValue};

    /// Create a minimal OutputFormatContent for testing primitive types
    fn empty_of(target: &TypeIR) -> OutputFormatContent {
        internal_baml_jinja::types::Builder::new(target.clone()).build()
    }

    #[test]
    fn test_module_compiles() {
        // Basic compilation test
        assert!(true);
    }

    #[test]
    fn test_parse_simple_string() {
        let ty = TypeIR::Primitive(TypeValue::String, type_meta::IR::default());
        let of = empty_of(&ty);
        let result = parse(&of, &ty, r#""hello world""#, false);
        assert!(result.is_ok());
        let value = result.unwrap();
        match value {
            BamlValueWithFlags::String(s) => {
                assert_eq!(s.value(), "hello world");
            }
            _ => panic!("Expected string value"),
        }
    }

    #[test]
    fn test_parse_simple_int() {
        let ty = TypeIR::Primitive(TypeValue::Int, type_meta::IR::default());
        let of = empty_of(&ty);
        let result = parse(&of, &ty, "42", false);
        assert!(result.is_ok());
        let value = result.unwrap();
        match value {
            BamlValueWithFlags::Int(i) => {
                assert_eq!(i.value(), &42);
            }
            _ => panic!("Expected int value"),
        }
    }

    #[test]
    fn test_parse_simple_bool() {
        let ty = TypeIR::Primitive(TypeValue::Bool, type_meta::IR::default());
        let of = empty_of(&ty);
        let result = parse(&of, &ty, "true", false);
        assert!(result.is_ok());
        let value = result.unwrap();
        match value {
            BamlValueWithFlags::Bool(b) => {
                assert_eq!(b.value(), &true);
            }
            _ => panic!("Expected bool value"),
        }
    }

    #[test]
    fn test_parse_list_of_ints() {
        let elem_ty = TypeIR::Primitive(TypeValue::Int, type_meta::IR::default());
        let ty = TypeIR::List(Box::new(elem_ty), type_meta::IR::default());
        let of = empty_of(&ty);
        let result = parse(&of, &ty, "[1, 2, 3]", false);
        assert!(result.is_ok());
        let value = result.unwrap();
        match value {
            BamlValueWithFlags::List(_, _, items) => {
                assert_eq!(items.len(), 3);
            }
            _ => panic!("Expected list value"),
        }
    }
}
