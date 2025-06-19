use std::vec;

use anyhow::Result;
use baml_types::{ir_type::TypeGeneric, BamlValueWithMeta, LiteralValue};
use internal_baml_jinja::CompletionOptions;

use crate::{
    deserializer::{
        coercer::{coerce_primitive::coerce_bool, match_string::match_string, TypeCoercer},
        deserialize_flags::{DeserializerConditions, Flag},
        types::{HasFlags, HasType},
    },
    jsonish,
};

use super::{coerce_primitive::coerce_int, ParsingContext, ParsingError};

impl<T, M> TypeCoercer<T, M> for LiteralValue
where
    M: HasType<Meta = T> + HasFlags,
    T: Clone + std::fmt::Display,
    TypeGeneric<T>: std::fmt::Display,
{
    fn coerce(
        &self,
        ctx: &ParsingContext,
        target: &TypeGeneric<T>,
        value: Option<&jsonish::Value>,
    ) -> Result<BamlValueWithMeta<M>, ParsingError> {
        log::debug!(
            "scope: {scope} :: coercing to: {name:?} (current: {current})",
            name = self,
            scope = ctx.display_scope(),
            current = value.map(|v| v.r#type()).unwrap_or("<null>".into())
        );

        // Get rid of nulls.
        let value = match value {
            None | Some(jsonish::Value::Null) => {
                return Err(ctx.error_unexpected_null(target));
            }
            Some(v) => v,
        };

        // If we get an object with a single key-value pair, try to extract the value
        if let jsonish::Value::Object(obj, completion_state) = value {
            if obj.len() == 1 {
                let (key, inner_value) = obj.iter().next().unwrap();
                // only extract value if it's a primitive (not an object or array, hoping to god its fixed)
                match inner_value {
                    jsonish::Value::Number(_, _)
                    | jsonish::Value::Boolean(_)
                    | jsonish::Value::String(_, _) => {
                        let mut result: BamlValueWithMeta<M> =
                            self.coerce(ctx, target, Some(&inner_value))?;
                        result
                            .meta_mut()
                            .flags_mut()
                            .add_flag(Flag::ObjectToPrimitive(jsonish::Value::Object(
                                obj.clone(),
                                completion_state.clone(),
                            )));
                        return Ok(result);
                    }
                    _ => {}
                }
            }
        }

        match self {
            LiteralValue::Int(literal_int) => {
                let coerced_int = coerce_int(ctx, target, Some(value))?;
                match &coerced_int {
                    BamlValueWithMeta::Int(int_val, _) => {
                        if int_val == literal_int {
                            Ok(coerced_int)
                        } else {
                            Err(ctx.error_unexpected_type(target, &value))
                        }
                    }
                    _ => unreachable!("coerce_int returned a non-integer value"),
                }
            }

            LiteralValue::Bool(literal_bool) => {
                let coerced_bool = coerce_bool(ctx, target, Some(value))?;
                match &coerced_bool {
                    BamlValueWithMeta::Bool(bool_val, _) => {
                        if bool_val == literal_bool {
                            Ok(coerced_bool)
                        } else {
                            Err(ctx.error_unexpected_type(target, &value))
                        }
                    }
                    _ => unreachable!("coerce_bool returned a non-boolean value"),
                }
            }

            LiteralValue::String(literal_str) => {
                // second element is the list of aliases.
                let candidates = vec![(literal_str.as_str(), vec![literal_str.clone()])];

                let literal_match = match_string(ctx, target, Some(value), &candidates)?;

                Ok(literal_match)
            }
        }
    }
}
