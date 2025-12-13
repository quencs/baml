//! Coercion Layer
//!
//! Converts ParsedValue (with CompletionState) to BamlValueWithFlags.
//! Implements type-directed coercion matching the existing deserializer behaviors.

use baml_types::{BamlMap, BamlValue, LiteralValue, StreamingMode, TypeIR, TypeValue, type_meta, ir_type::UnionConstructor};

use super::frames::{CompletionState, ParsedValue, ParsedValueKind};
use super::schema_index::{LiteralKind, PrimitiveKind, SchemaIndex, TypeId, TypeInfo, TypeKind};
use crate::deserializer::coercer::{run_user_checks, validate_asserts};
use crate::deserializer::deserialize_flags::{DeserializerConditions, Flag};
use crate::deserializer::types::{BamlValueWithFlags, ValueWithFlags};

/// Create type metadata with proper done flag
fn make_meta(streaming: bool) -> type_meta::IR {
    let mut meta = type_meta::IR::default();
    // When not streaming, mark as done
    if !streaming {
        meta.streaming_behavior.done = true;
    }
    meta
}

/// Helper to create a TypeIR for primitives
fn primitive_type(prim: TypeValue, streaming: bool) -> TypeIR {
    TypeIR::Primitive(prim, make_meta(streaming))
}

/// Helper to create a TypeIR for classes
fn class_type(name: &str, streaming: bool) -> TypeIR {
    TypeIR::Class {
        name: name.to_string(),
        mode: StreamingMode::NonStreaming,
        dynamic: false,
        meta: make_meta(streaming),
    }
}

/// Helper to create a TypeIR for enums
fn enum_type(name: &str, streaming: bool) -> TypeIR {
    TypeIR::Enum {
        name: name.to_string(),
        dynamic: false,
        meta: make_meta(streaming),
    }
}

/// Helper to create a TypeIR for lists
fn list_type(elem: TypeIR, streaming: bool) -> TypeIR {
    TypeIR::List(Box::new(elem), make_meta(streaming))
}

/// Helper to create a TypeIR for maps
fn map_type(key: TypeIR, val: TypeIR, streaming: bool) -> TypeIR {
    TypeIR::Map(Box::new(key), Box::new(val), make_meta(streaming))
}

/// Helper to create literal type
fn literal_type(lit: LiteralValue, streaming: bool) -> TypeIR {
    TypeIR::Literal(lit, make_meta(streaming))
}

/// Maximum recursion depth for class-wrapping
const MAX_WRAP_DEPTH: u8 = 10;

/// Convert a ParsedValue to BamlValueWithFlags
pub fn convert_to_baml_value(
    schema: &SchemaIndex,
    type_id: TypeId,
    parsed: &ParsedValue,
    streaming: bool,
) -> anyhow::Result<BamlValueWithFlags> {
    let mut result = convert_to_baml_value_impl(schema, type_id, parsed, streaming, 0, true)?;

    // Apply constraints only when not streaming (partial data can't be validated)
    if !streaming {
        let info = schema.get(type_id);
        if let Some(target_type) = info.map(|i| &i.source_type) {
            if !target_type.meta().constraints.is_empty() {
                // Convert to BamlValue for constraint evaluation
                let baml_value: BamlValue = result.clone().into();

                // Evaluate constraints
                let constraint_results = run_user_checks(&baml_value, target_type)
                    .map_err(|e| anyhow::anyhow!("Failed to evaluate constraints: {e}"))?;

                // Validate asserts (fail if any assert failed)
                validate_asserts(&constraint_results)
                    .map_err(|e| anyhow::anyhow!("Assert failed: {}", e.reason))?;

                // Add check results as flags
                let check_results: Vec<_> = constraint_results
                    .into_iter()
                    .filter_map(|(constraint, res)| {
                        constraint.as_check()
                            .map(|(label, expr)| (label, expr, res))
                    })
                    .collect();

                if !check_results.is_empty() {
                    result.add_flag(Flag::ConstraintResults(check_results));
                }
            }
        }
    }

    Ok(result)
}

/// Convert a ParsedValue to BamlValueWithFlags (without constraint checking)
///
/// `allow_wrapping` - whether to allow wrapping primitives into classes.
/// Set to false during union variant iteration to prefer simpler matches.
fn convert_to_baml_value_impl(
    schema: &SchemaIndex,
    type_id: TypeId,
    parsed: &ParsedValue,
    streaming: bool,
    depth: u8,
    allow_wrapping: bool,
) -> anyhow::Result<BamlValueWithFlags> {
    // Guard against infinite recursion
    if depth > MAX_WRAP_DEPTH {
        return Err(anyhow::anyhow!("Maximum coercion depth exceeded"));
    }

    log::debug!(
        "convert_to_baml_value_impl: type_id={}, parsed={:?}, depth={}",
        type_id,
        parsed.value,
        depth
    );

    let info = schema.get(type_id);

    // Get the original TypeIR from the schema (preserves finalized metadata)
    let target = info.map(|i| i.source_type.clone())
        .unwrap_or_else(|| primitive_type(TypeValue::Null, streaming));

    // Start with conditions from completion state
    let mut conditions = DeserializerConditions::new();
    apply_completion_flags(parsed.completion, &mut conditions);

    match (info.map(|i| &i.kind), &parsed.value) {
        // Null handling
        (_, ParsedValueKind::Null) => {
            Ok(BamlValueWithFlags::Null(target, conditions))
        }

        (_, ParsedValueKind::Placeholder) if parsed.completion == CompletionState::Pending => {
            conditions.add_flag(Flag::DefaultFromNoValue);
            Ok(BamlValueWithFlags::Null(target, conditions))
        }

        // String primitives
        (Some(TypeKind::Primitive(PrimitiveKind::String)), ParsedValueKind::String(s)) => {
            Ok(BamlValueWithFlags::String(ValueWithFlags {
                value: s.clone(),
                target,
                flags: conditions,
            }))
        }

        // String from Int (coercion)
        (Some(TypeKind::Primitive(PrimitiveKind::String)), ParsedValueKind::Int(n)) => {
            Ok(BamlValueWithFlags::String(ValueWithFlags {
                value: n.to_string(),
                target,
                flags: conditions,
            }))
        }

        // String from Float (coercion)
        (Some(TypeKind::Primitive(PrimitiveKind::String)), ParsedValueKind::Float(f)) => {
            Ok(BamlValueWithFlags::String(ValueWithFlags {
                value: f.to_string(),
                target,
                flags: conditions,
            }))
        }

        // String from Bool (coercion)
        (Some(TypeKind::Primitive(PrimitiveKind::String)), ParsedValueKind::Bool(b)) => {
            Ok(BamlValueWithFlags::String(ValueWithFlags {
                value: b.to_string(),
                target,
                flags: conditions,
            }))
        }

        // Int primitives - with coercion from string
        (Some(TypeKind::Primitive(PrimitiveKind::Int)), ParsedValueKind::Int(n)) => {
            Ok(BamlValueWithFlags::Int(ValueWithFlags {
                value: *n,
                target,
                flags: conditions,
            }))
        }

        (Some(TypeKind::Primitive(PrimitiveKind::Int)), ParsedValueKind::String(s)) => {
            // Handle comma-separated ints: "12,111" -> 12111
            let cleaned = s.replace(',', "");
            let n: i64 = cleaned.parse().map_err(|_| {
                anyhow::anyhow!("Cannot parse '{}' as int", s)
            })?;
            Ok(BamlValueWithFlags::Int(ValueWithFlags {
                value: n,
                target,
                flags: conditions,
            }))
        }

        (Some(TypeKind::Primitive(PrimitiveKind::Int)), ParsedValueKind::Float(f)) => {
            conditions.add_flag(Flag::FloatToInt(*f));
            Ok(BamlValueWithFlags::Int(ValueWithFlags {
                value: *f as i64,
                target,
                flags: conditions,
            }))
        }

        // Float primitives
        (Some(TypeKind::Primitive(PrimitiveKind::Float)), ParsedValueKind::Float(f)) => {
            Ok(BamlValueWithFlags::Float(ValueWithFlags {
                value: *f,
                target,
                flags: conditions,
            }))
        }

        (Some(TypeKind::Primitive(PrimitiveKind::Float)), ParsedValueKind::Int(n)) => {
            Ok(BamlValueWithFlags::Float(ValueWithFlags {
                value: *n as f64,
                target,
                flags: conditions,
            }))
        }

        (Some(TypeKind::Primitive(PrimitiveKind::Float)), ParsedValueKind::String(s)) => {
            // Handle fractions: "1/5" -> 0.2
            let f = if let Some((num, denom)) = s.split_once('/') {
                let n: f64 = num.trim().parse().map_err(|_| {
                    anyhow::anyhow!("Cannot parse '{}' as float", s)
                })?;
                let d: f64 = denom.trim().parse().map_err(|_| {
                    anyhow::anyhow!("Cannot parse '{}' as float", s)
                })?;
                n / d
            } else {
                s.parse().map_err(|_| {
                    anyhow::anyhow!("Cannot parse '{}' as float", s)
                })?
            };
            conditions.add_flag(Flag::StringToFloat(s.clone()));
            Ok(BamlValueWithFlags::Float(ValueWithFlags {
                value: f,
                target,
                flags: conditions,
            }))
        }

        // Bool primitives
        (Some(TypeKind::Primitive(PrimitiveKind::Bool)), ParsedValueKind::Bool(b)) => {
            Ok(BamlValueWithFlags::Bool(ValueWithFlags {
                value: *b,
                target,
                flags: conditions,
            }))
        }

        (Some(TypeKind::Primitive(PrimitiveKind::Bool)), ParsedValueKind::String(s)) => {
            let b = match s.to_lowercase().as_str() {
                "true" | "yes" | "1" => true,
                "false" | "no" | "0" => false,
                _ => {
                    return Err(anyhow::anyhow!("Cannot parse '{}' as bool", s));
                }
            };
            conditions.add_flag(Flag::StringToBool(s.clone()));
            Ok(BamlValueWithFlags::Bool(ValueWithFlags {
                value: b,
                target,
                flags: conditions,
            }))
        }

        // Enum handling
        (Some(TypeKind::Enum { name, values, fuzzy_map }), ParsedValueKind::String(s)) => {
            // Try to find a matching enum value
            match match_enum_value(s, values, fuzzy_map) {
                Some((canonical, mut match_flags)) => {
                    // Add any match flags
                    for flag in match_flags.drain(..) {
                        conditions.add_flag(flag);
                    }

                    Ok(BamlValueWithFlags::Enum(
                        name.clone(),
                        target.clone(),
                        ValueWithFlags {
                            value: canonical,
                            target,
                            flags: conditions,
                        },
                    ))
                }
                None => {
                    // No match found - fail
                    Err(anyhow::anyhow!(
                        "No matching enum value found for '{}' in enum {}",
                        s,
                        name
                    ))
                }
            }
        }

        // Literal handling
        (Some(TypeKind::Literal(lit)), value) => {
            match (lit, value) {
                (LiteralKind::String(expected), ParsedValueKind::String(s)) => {
                    // Check for match: exact, case-insensitive, unaccented, or substring
                    let s_lower = s.to_lowercase();
                    let expected_lower = expected.to_lowercase();
                    let s_unaccented = remove_accents(s).to_lowercase();
                    let expected_unaccented = remove_accents(expected).to_lowercase();

                    // Exact or fuzzy match
                    let exact_match = s == expected
                        || s_lower == expected_lower
                        || s_unaccented == expected_unaccented;

                    // Substring match (check if expected appears in input)
                    let substring_match = !exact_match && (
                        s.contains(expected)
                        || s_lower.contains(&expected_lower)
                        || s_unaccented.contains(&expected_unaccented)
                    );

                    if exact_match || substring_match {
                        if substring_match {
                            conditions.add_flag(Flag::SubstringMatch(s.clone()));
                        }
                        // Return the canonical (expected) value, not the input
                        Ok(BamlValueWithFlags::String(ValueWithFlags {
                            value: expected.clone(),
                            target,
                            flags: conditions,
                        }))
                    } else {
                        Err(anyhow::anyhow!("Literal mismatch: expected '{}', got '{}'", expected, s))
                    }
                }
                (LiteralKind::Int(expected), ParsedValueKind::Int(n)) => {
                    if *n == *expected {
                        Ok(BamlValueWithFlags::Int(ValueWithFlags {
                            value: *n,
                            target,
                            flags: conditions,
                        }))
                    } else {
                        Err(anyhow::anyhow!("Literal mismatch: expected {}, got {}", expected, n))
                    }
                }
                (LiteralKind::Bool(expected), ParsedValueKind::Bool(b)) => {
                    if *b == *expected {
                        Ok(BamlValueWithFlags::Bool(ValueWithFlags {
                            value: *b,
                            target,
                            flags: conditions,
                        }))
                    } else {
                        Err(anyhow::anyhow!("Literal mismatch: expected {}, got {}", expected, b))
                    }
                }
                _ => Err(anyhow::anyhow!("Type mismatch for literal")),
            }
        }

        // Class/Object handling
        (Some(TypeKind::Class { name, fields, required, fuzzy_fields }), ParsedValueKind::Object { fields: parsed_fields, .. }) => {
            // First, check if any of the parsed keys match expected field names
            let any_key_matches = parsed_fields.iter().any(|(k, _)| {
                fields.contains_key(k) ||
                fuzzy_fields.contains_key(&k.to_lowercase()) ||
                fuzzy_fields.contains_key(&remove_accents(k).to_lowercase())
            });

            // If no keys match and wrapping is allowed, try wrapping the entire object into a field
            if !any_key_matches && allow_wrapping {
                // Try to find a field that can accept the entire input object
                for (field_rendered_name, field_info) in fields {
                    // Try coercing the entire parsed object to this field's type
                    if let Ok(coerced) = convert_to_baml_value_impl(schema, field_info.type_id, parsed, streaming, depth + 1, false) {
                        let mut result_fields = BamlMap::new();
                        let output_name = field_info.real_name.clone();
                        result_fields.insert(output_name, coerced);

                        // Fill in other fields as pending/null
                        for (other_rendered, other_info) in fields {
                            if other_rendered != field_rendered_name {
                                let other_output = other_info.real_name.clone();
                                if streaming {
                                    let mut field_conditions = DeserializerConditions::new();
                                    field_conditions.add_flag(Flag::Pending);
                                    field_conditions.add_flag(Flag::DefaultFromNoValue);
                                    let default = default_for_type(schema, other_info.type_id, field_conditions, streaming);
                                    result_fields.insert(other_output, default);
                                } else if !required.contains(other_rendered) {
                                    let mut field_conditions = DeserializerConditions::new();
                                    field_conditions.add_flag(Flag::OptionalDefaultFromNoValue);
                                    let field_target = schema.get(other_info.type_id)
                                        .map(|i| i.source_type.clone())
                                        .unwrap_or_else(|| primitive_type(TypeValue::Null, streaming));
                                    result_fields.insert(other_output, BamlValueWithFlags::Null(field_target, field_conditions));
                                }
                                // If other field is required, we just skip it (this means wrapping only works
                                // for classes where the matching field is the only required field)
                            }
                        }

                        return Ok(BamlValueWithFlags::Class(
                            name.clone(),
                            conditions,
                            target,
                            result_fields,
                        ));
                    }
                }
            }

            // Standard field-by-field matching
            let mut result_fields = BamlMap::new();

            // Process expected fields from schema
            for (field_rendered_name, field_info) in fields {
                // Try to find the parsed field by rendered name (which is used for JSON matching)
                // First try exact match, then fuzzy match
                let parsed_field = parsed_fields.iter()
                    .find(|(k, _)| k == field_rendered_name)
                    .or_else(|| {
                        // Try fuzzy match - normalize the parsed key and look it up
                        parsed_fields.iter().find(|(k, _)| {
                            // Check if this parsed key matches this field via fuzzy lookup
                            let k_lower = k.to_lowercase();
                            let k_unaccented = remove_accents(k).to_lowercase();

                            // The fuzzy_fields map goes from normalized->rendered_name
                            // So we check if our normalized key maps to this field's rendered name
                            fuzzy_fields.get(&k_lower) == Some(field_rendered_name) ||
                            fuzzy_fields.get(&k_unaccented) == Some(field_rendered_name)
                        })
                    });

                // Use the real_name for the output
                let output_name = field_info.real_name.clone();

                match parsed_field {
                    Some((_, val)) => {
                        let coerced = convert_to_baml_value_impl(schema, field_info.type_id, val, streaming, depth + 1, allow_wrapping)?;
                        result_fields.insert(output_name, coerced);
                    }
                    None if streaming => {
                        // Missing field in streaming - create Pending placeholder
                        let mut field_conditions = DeserializerConditions::new();
                        field_conditions.add_flag(Flag::Pending);
                        field_conditions.add_flag(Flag::DefaultFromNoValue);

                        let default = default_for_type(schema, field_info.type_id, field_conditions, streaming);
                        result_fields.insert(output_name, default);
                    }
                    None if !required.contains(field_rendered_name) => {
                        // Optional field missing - get the field's source type for null value
                        let mut field_conditions = DeserializerConditions::new();
                        field_conditions.add_flag(Flag::OptionalDefaultFromNoValue);

                        let field_target = schema.get(field_info.type_id)
                            .map(|i| i.source_type.clone())
                            .unwrap_or_else(|| primitive_type(TypeValue::Null, streaming));
                        let null = BamlValueWithFlags::Null(field_target, field_conditions);
                        result_fields.insert(output_name, null);
                    }
                    None => {
                        // Required field missing - fail in non-streaming mode
                        return Err(anyhow::anyhow!(
                            "Missing required field '{}' (rendered as '{}') in class {}",
                            field_info.real_name,
                            field_rendered_name,
                            name
                        ));
                    }
                }
            }

            Ok(BamlValueWithFlags::Class(
                name.clone(),
                conditions,
                target,
                result_fields,
            ))
        }

        // Class handling - wrap primitive into single-field class
        // This handles cases like: input `true` -> class Foo { foo bool } -> { "foo": true }
        // Only attempt this when allow_wrapping is true (disabled during union variant iteration)
        (Some(TypeKind::Class { name, fields, required, fuzzy_fields: _ }), non_object)
            if !matches!(non_object, ParsedValueKind::Object { .. }) && allow_wrapping =>
        {
            // Try to find a field that can accept this value
            for (field_rendered_name, field_info) in fields {
                // Try coercing the value to this field's type (with wrapping disabled to avoid recursion)
                if let Ok(coerced) = convert_to_baml_value_impl(schema, field_info.type_id, parsed, streaming, depth + 1, false) {
                    let mut result_fields = BamlMap::new();
                    let output_name = field_info.real_name.clone();
                    result_fields.insert(output_name, coerced);

                    // Fill in missing required fields as pending/null
                    for (other_rendered, other_info) in fields {
                        if other_rendered != field_rendered_name {
                            let other_output = other_info.real_name.clone();
                            if streaming {
                                let mut field_conditions = DeserializerConditions::new();
                                field_conditions.add_flag(Flag::Pending);
                                field_conditions.add_flag(Flag::DefaultFromNoValue);
                                let default = default_for_type(schema, other_info.type_id, field_conditions, streaming);
                                result_fields.insert(other_output, default);
                            } else if !required.contains(other_rendered) {
                                // Optional field - fill with null
                                let mut field_conditions = DeserializerConditions::new();
                                field_conditions.add_flag(Flag::OptionalDefaultFromNoValue);
                                let field_target = schema.get(other_info.type_id)
                                    .map(|i| i.source_type.clone())
                                    .unwrap_or_else(|| primitive_type(TypeValue::Null, streaming));
                                result_fields.insert(other_output, BamlValueWithFlags::Null(field_target, field_conditions));
                            } else {
                                // Required field - only one-field classes can be wrapped
                                // Skip this field match and try another
                                continue;
                            }
                        }
                    }

                    return Ok(BamlValueWithFlags::Class(
                        name.clone(),
                        conditions,
                        target,
                        result_fields,
                    ));
                }
            }

            Err(anyhow::anyhow!(
                "Cannot coerce {:?} to class {} - no compatible single field found",
                non_object, name
            ))
        }

        // List handling - array to array
        (Some(TypeKind::List { element }), ParsedValueKind::Array(items)) => {
            log::debug!("Coercing Array with {} items to List", items.len());
            let coerced: Result<Vec<_>, _> = items
                .iter()
                .map(|item| convert_to_baml_value_impl(schema, *element, item, streaming, depth + 1, allow_wrapping))
                .collect();

            Ok(BamlValueWithFlags::List(conditions, target, coerced?))
        }

        // List handling - wrap single value in array
        (Some(TypeKind::List { element }), _) => {
            // Try to coerce the single value to the element type, then wrap in array
            let coerced = convert_to_baml_value_impl(schema, *element, parsed, streaming, depth + 1, allow_wrapping)?;
            Ok(BamlValueWithFlags::List(conditions, target, vec![coerced]))
        }

        // Map handling
        (Some(TypeKind::Map { key: _key_ty, value: val_ty }), ParsedValueKind::Object { fields: parsed_fields, .. }) => {
            let mut result_map: BamlMap<String, (DeserializerConditions, BamlValueWithFlags)> = BamlMap::new();

            for (k, v) in parsed_fields {
                let coerced_val = convert_to_baml_value_impl(schema, *val_ty, v, streaming, depth + 1, allow_wrapping)?;
                result_map.insert(k.clone(), (DeserializerConditions::new(), coerced_val));
            }

            Ok(BamlValueWithFlags::Map(conditions, target, result_map))
        }

        // Union handling - try each variant
        (Some(TypeKind::Union { variants, .. }), _) => {
            for &var_id in variants {
                // Disable wrapping during union variant iteration to prefer simpler matches
                if let Ok(result) = convert_to_baml_value_impl(schema, var_id, parsed, streaming, depth + 1, false) {
                    // Preserve the outer Union target type
                    return Ok(result.with_target(&target));
                }
            }
            Err(anyhow::anyhow!("No union variant matched"))
        }

        // Optional handling - Null case is already handled by the generic null handler above
        (Some(TypeKind::Optional { inner }), _) => {
            // For non-null values in Optional, recursively convert using inner type
            // But return value with the outer Optional target
            let inner_result = convert_to_baml_value_impl(schema, *inner, parsed, streaming, depth + 1, allow_wrapping)?;
            Ok(inner_result.with_target(&target))
        }

        // RecursiveAlias handling - delegate to the resolved inner type
        (Some(TypeKind::RecursiveAlias { name, target: Some(inner_id) }), _) => {
            log::debug!("Coercing RecursiveAlias '{}' -> delegating to TypeId {}", name, inner_id);
            // Parse using the resolved inner type, but keep the recursive alias target
            let inner_result = convert_to_baml_value_impl(schema, *inner_id, parsed, streaming, depth + 1, allow_wrapping)?;
            Ok(inner_result.with_target(&target))
        }

        // RecursiveAlias with no target - can't parse
        (Some(TypeKind::RecursiveAlias { name, target: None }), _) => {
            Err(anyhow::anyhow!("Cannot parse recursive type alias '{}' - type not resolved", name))
        }

        // Fallback for strings - only allow for primitives, enums, literals, or Top
        (Some(kind), ParsedValueKind::String(s)) => {
            match kind {
                TypeKind::Primitive(PrimitiveKind::String) |
                TypeKind::Enum { .. } |
                TypeKind::Literal(LiteralKind::String(_)) |
                TypeKind::Top => {
                    Ok(BamlValueWithFlags::String(ValueWithFlags {
                        value: s.clone(),
                        target,
                        flags: conditions,
                    }))
                }
                _ => Err(anyhow::anyhow!(
                    "Cannot coerce string to {:?}", kind
                )),
            }
        }

        // Fallback for no type info - return string as-is
        (None, ParsedValueKind::String(s)) => {
            Ok(BamlValueWithFlags::String(ValueWithFlags {
                value: s.clone(),
                target,
                flags: conditions,
            }))
        }

        (None, ParsedValueKind::Int(n)) => {
            Ok(BamlValueWithFlags::Int(ValueWithFlags {
                value: *n,
                target,
                flags: conditions,
            }))
        }

        (None, ParsedValueKind::Float(f)) => {
            Ok(BamlValueWithFlags::Float(ValueWithFlags {
                value: *f,
                target,
                flags: conditions,
            }))
        }

        (None, ParsedValueKind::Bool(b)) => {
            Ok(BamlValueWithFlags::Bool(ValueWithFlags {
                value: *b,
                target,
                flags: conditions,
            }))
        }

        // Arrays without type info
        (None, ParsedValueKind::Array(items)) => {
            let coerced: Result<Vec<_>, _> = items
                .iter()
                .map(|item| convert_to_baml_value_impl(schema, schema.root_id(), item, streaming, depth + 1, allow_wrapping))
                .collect();

            Ok(BamlValueWithFlags::List(conditions, target, coerced?))
        }

        // Objects without type info
        (None, ParsedValueKind::Object { fields: parsed_fields, .. }) => {
            let mut result_fields = BamlMap::new();
            for (k, v) in parsed_fields {
                let coerced = convert_to_baml_value_impl(schema, schema.root_id(), v, streaming, depth + 1, allow_wrapping)?;
                result_fields.insert(k.clone(), coerced);
            }

            Ok(BamlValueWithFlags::Class(
                "Object".to_string(),
                conditions,
                target,
                result_fields,
            ))
        }

        _ => Err(anyhow::anyhow!(
            "Cannot coerce {:?} to type {:?}",
            parsed.value,
            info.map(|i| &i.kind)
        )),
    }
}

/// Apply completion state to flags
fn apply_completion_flags(completion: CompletionState, conditions: &mut DeserializerConditions) {
    match completion {
        CompletionState::Complete => {
            // No flag needed
        }
        CompletionState::Incomplete => {
            conditions.add_flag(Flag::Incomplete);
        }
        CompletionState::Pending => {
            conditions.add_flag(Flag::Pending);
        }
    }
}

/// Create a default value for a type with given conditions
fn default_for_type(
    schema: &SchemaIndex,
    type_id: TypeId,
    conditions: DeserializerConditions,
    streaming: bool,
) -> BamlValueWithFlags {
    // Get the original TypeIR to preserve metadata
    let target = schema.get(type_id)
        .map(|i| i.source_type.clone())
        .unwrap_or_else(|| primitive_type(TypeValue::Null, streaming));
    let info = schema.get(type_id);

    match info.map(|i| &i.kind) {
        Some(TypeKind::Primitive(PrimitiveKind::Null)) => {
            BamlValueWithFlags::Null(target, conditions)
        }
        Some(TypeKind::Primitive(PrimitiveKind::String)) => {
            BamlValueWithFlags::String(ValueWithFlags {
                value: String::new(),
                target,
                flags: conditions,
            })
        }
        Some(TypeKind::Primitive(PrimitiveKind::Int)) => {
            BamlValueWithFlags::Int(ValueWithFlags {
                value: 0,
                target,
                flags: conditions,
            })
        }
        Some(TypeKind::Primitive(PrimitiveKind::Float)) => {
            BamlValueWithFlags::Float(ValueWithFlags {
                value: 0.0,
                target,
                flags: conditions,
            })
        }
        Some(TypeKind::Primitive(PrimitiveKind::Bool)) => {
            BamlValueWithFlags::Bool(ValueWithFlags {
                value: false,
                target,
                flags: conditions,
            })
        }
        Some(TypeKind::List { element }) => {
            BamlValueWithFlags::List(conditions, target, vec![])
        }
        Some(TypeKind::Map { key, value }) => {
            BamlValueWithFlags::Map(conditions, target, BamlMap::new())
        }
        Some(TypeKind::Optional { inner }) => {
            BamlValueWithFlags::Null(target, conditions)
        }
        _ => {
            BamlValueWithFlags::Null(target, conditions)
        }
    }
}

/// Convert TypeKind to TypeIR for targets
fn type_kind_to_type_ir(kind: &TypeKind, streaming: bool) -> TypeIR {
    match kind {
        TypeKind::Primitive(p) => match p {
            PrimitiveKind::String => primitive_type(TypeValue::String, streaming),
            PrimitiveKind::Int => primitive_type(TypeValue::Int, streaming),
            PrimitiveKind::Float => primitive_type(TypeValue::Float, streaming),
            PrimitiveKind::Bool => primitive_type(TypeValue::Bool, streaming),
            PrimitiveKind::Null => primitive_type(TypeValue::Null, streaming),
            PrimitiveKind::Media => primitive_type(TypeValue::String, streaming), // Fallback
        },
        TypeKind::Enum { name, .. } => enum_type(name, streaming),
        TypeKind::Literal(lit) => match lit {
            LiteralKind::String(s) => literal_type(LiteralValue::String(s.clone()), streaming),
            LiteralKind::Int(i) => literal_type(LiteralValue::Int(*i), streaming),
            LiteralKind::Bool(b) => literal_type(LiteralValue::Bool(*b), streaming),
        },
        TypeKind::Class { name, .. } => class_type(name, streaming),
        TypeKind::List { .. } => list_type(primitive_type(TypeValue::String, streaming), streaming),
        TypeKind::Map { .. } => map_type(
            primitive_type(TypeValue::String, streaming),
            primitive_type(TypeValue::String, streaming),
            streaming,
        ),
        TypeKind::Union { .. } => {
            // Just return string as representative
            primitive_type(TypeValue::String, streaming)
        }
        TypeKind::Optional { .. } => {
            TypeIR::union(vec![primitive_type(TypeValue::Null, streaming)])
        }
        TypeKind::Tuple { .. } => list_type(primitive_type(TypeValue::String, streaming), streaming),
        TypeKind::RecursiveAlias { name, .. } => {
            TypeIR::RecursiveTypeAlias {
                name: name.clone(),
                mode: StreamingMode::NonStreaming,
                meta: make_meta(streaming),
            }
        }
        TypeKind::Top => primitive_type(TypeValue::String, streaming),
    }
}

/// Remove accents from characters to enable fuzzy matching of unaccented input
/// against accented aliases/candidates.
pub fn remove_accents(s: &str) -> String {
    use unicode_normalization::UnicodeNormalization;

    // Handle ligatures separately since they're not combining marks
    let s = s
        .replace('ß', "ss")
        .replace('æ', "ae")
        .replace('Æ', "AE")
        .replace('ø', "o")
        .replace('Ø', "O")
        .replace('œ', "oe")
        .replace('Œ', "OE");

    s.nfkd()
        .filter(|c| !unicode_normalization::char::is_combining_mark(*c))
        .collect()
}

/// Match a string against enum values with fuzzy/substring matching
/// Returns the canonical value and any match flags, or None if no match found
fn match_enum_value(
    s: &str,
    values: &std::collections::HashMap<String, String>,
    fuzzy_map: &std::collections::HashMap<String, String>,
) -> Option<(String, Vec<Flag>)> {
    let mut flags = Vec::new();

    // 1. Try exact match in values map (rendered -> real)
    if let Some(real) = values.get(s) {
        return Some((real.clone(), flags));
    }

    // 2. Try lowercase match in fuzzy_map
    let s_lower = s.to_lowercase();
    if let Some(real) = fuzzy_map.get(&s_lower) {
        return Some((real.clone(), flags));
    }

    // 3. Try unaccented lowercase match
    let s_unaccented = remove_accents(s).to_lowercase();
    if let Some(real) = fuzzy_map.get(&s_unaccented) {
        return Some((real.clone(), flags));
    }

    // 4. Try normalized match (remove punctuation)
    let s_normalized = normalize_enum_value(s);
    if let Some(real) = fuzzy_map.get(&s_normalized) {
        return Some((real.clone(), flags));
    }

    // 5. Substring match with non-overlapping occurrence counting (like legacy match_string)
    // Find all matches, filter overlapping ones, then count occurrences per value

    // Pass 1: Case-sensitive substring match
    if let Some(result) = find_best_substring_match(s, values, &mut flags) {
        return Some(result);
    }

    // Pass 2: Strip punctuation and try case-sensitive
    let s_stripped = strip_punctuation(s);
    let values_stripped: std::collections::HashMap<String, String> = values
        .iter()
        .map(|(k, v)| (strip_punctuation(k), v.clone()))
        .collect();
    if let Some(result) = find_best_substring_match(&s_stripped, &values_stripped, &mut flags) {
        return Some(result);
    }

    // Pass 3: Case-insensitive substring match on stripped strings
    // Include fuzzy_map aliases
    let s_lower_stripped = s_stripped.to_lowercase();
    let mut all_candidates: std::collections::HashMap<String, String> = values
        .iter()
        .map(|(k, v)| (strip_punctuation(k).to_lowercase(), v.clone()))
        .collect();
    for (alias, real) in fuzzy_map {
        all_candidates.insert(strip_punctuation(alias).to_lowercase(), real.clone());
    }
    if let Some(result) = find_best_substring_match(&s_lower_stripped, &all_candidates, &mut flags) {
        return Some(result);
    }

    // No match found
    None
}

/// Find substring matches with non-overlapping filtering and occurrence counting
fn find_best_substring_match(
    s: &str,
    candidates: &std::collections::HashMap<String, String>,
    flags: &mut Vec<Flag>,
) -> Option<(String, Vec<Flag>)> {
    // Collect all matches with their positions: (start, end, real_value)
    let mut all_matches: Vec<(usize, usize, String)> = Vec::new();
    for (alias, real) in candidates {
        for (start, _) in s.match_indices(alias.as_str()) {
            let end = start + alias.len();
            all_matches.push((start, end, real.clone()));
        }
    }

    if all_matches.is_empty() {
        return None;
    }

    // Sort by position and length (longer matches first for same position)
    all_matches.sort_by(|a, b| {
        match a.0.cmp(&b.0) {
            std::cmp::Ordering::Equal => b.1.cmp(&a.1), // Longer first
            other => other,
        }
    });

    // Filter out overlapping matches (keep longer/earlier ones)
    let mut filtered_matches: Vec<(usize, usize, String)> = Vec::new();
    let mut last_end = 0;
    for m in all_matches {
        if m.0 >= last_end {
            last_end = m.1;
            filtered_matches.push(m);
        }
    }

    // Count occurrences per value
    let mut occurrence_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for (_, _, real) in filtered_matches {
        *occurrence_counts.entry(real).or_insert(0) += 1;
    }

    // Find value with most occurrences
    let max_count = *occurrence_counts.values().max().unwrap();
    let winners: Vec<_> = occurrence_counts
        .iter()
        .filter(|(_, &c)| c == max_count)
        .map(|(v, _)| v.clone())
        .collect();

    if winners.len() == 1 {
        flags.push(Flag::SubstringMatch(s.to_string()));
        return Some((winners.into_iter().next().unwrap(), std::mem::take(flags)));
    }

    // Multiple winners - ambiguous
    None
}

/// Strip punctuation from a string (keep alphanumeric, hyphens, underscores)
fn strip_punctuation(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
        .collect()
}

/// Normalize enum value for fuzzy matching (removes accents and punctuation)
fn normalize_enum_value(s: &str) -> String {
    let unaccented = remove_accents(s);
    unaccented
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect::<String>()
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_completion_flags() {
        let mut conditions = DeserializerConditions::new();
        apply_completion_flags(CompletionState::Incomplete, &mut conditions);
        assert!(conditions.flags.iter().any(|f| matches!(f, Flag::Incomplete)));

        let mut conditions = DeserializerConditions::new();
        apply_completion_flags(CompletionState::Pending, &mut conditions);
        assert!(conditions.flags.iter().any(|f| matches!(f, Flag::Pending)));
    }

    #[test]
    fn test_normalize_enum() {
        assert_eq!(normalize_enum_value("HELLO_WORLD"), "helloworld");
        assert_eq!(normalize_enum_value("Hello World"), "hello world");
        assert_eq!(normalize_enum_value("hello-world!"), "helloworld");
    }
}
