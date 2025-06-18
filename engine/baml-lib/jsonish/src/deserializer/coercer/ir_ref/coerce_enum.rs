use crate::{
    deserializer::{
        coercer::{match_string::match_string, ParsingContext, ParsingError, TypeCoercer},
        deserialize_flags::{DeserializerConditions, Flag},
        types::{HasFlags, HasType},
    },
    jsonish,
};
use anyhow::Result;
use baml_types::{BamlValueWithMeta, FieldType};
use internal_baml_jinja::types::Enum;

impl<M> TypeCoercer<FieldType, M> for &Enum
where
    M: HasType<Type = FieldType> + HasFlags,
{
    fn coerce(
        &self,
        ctx: &ParsingContext,
        target: &FieldType,
        value: Option<&jsonish::Value>,
    ) -> Result<BamlValueWithMeta<M>, ParsingError> {
        log::debug!(
            "scope: {scope} :: coercing to: {name} (current: {current})",
            name = target.to_string(),
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

        // Get valid values for this enum
        let candidates = self
            .values
            .iter()
            .map(|(name, _description)| {
                (
                    name.real_name(),
                    vec![name.real_name().to_string(), name.rendered_name().to_string()],
                )
            })
            .collect::<Vec<_>>();

        // First try string matching
        let string_match_result = match_string(ctx, target, Some(value), &candidates);
        match string_match_result {
            Ok(matched_string) => {
                // Create enum value with proper metadata
                let mut meta = M::default();
                *meta.type_mut() = target.clone();
                
                // Copy flags from string match result
                if let BamlValueWithMeta::String(_, string_meta) = &matched_string {
                    meta.flags_mut().flags.extend(string_meta.flags().flags.clone());
                }

                Ok(BamlValueWithMeta::Enum(
                    self.name.real_name().to_string(),
                    matched_string.into_inner().0,  // Extract the string value
                    meta,
                ))
            }
            Err(e) => Err(e),
        }
    }
}
