use anyhow::Result;
use baml_types::{BamlValueWithMeta, FieldType};

use crate::deserializer::{types::{HasFlags, HasType}, coercer::{ParsingContext, ParsingError, TypeCoercer}};

pub(super) fn coerce_alias<M>(
    ctx: &ParsingContext,
    alias: &FieldType,
    value: Option<&crate::jsonish::Value>,
) -> Result<BamlValueWithMeta<M>, ParsingError>
where
    M: HasType<Type = FieldType> + HasFlags,
{
    // For recursive type aliases, we need to find the target type and coerce to that
    let FieldType::RecursiveTypeAlias { name, .. } = alias else {
        return Err(ctx.error_internal("coerce_alias called on non-alias type"));
    };
    
    // Find the target type for this alias
    match ctx.of.find_recursive_alias_target(name) {
        Ok(target_type) => {
            // Coerce to the target type
            target_type.coerce(ctx, target_type, value)
        }
        Err(e) => Err(ctx.error_internal(e.to_string())),
    }
}
