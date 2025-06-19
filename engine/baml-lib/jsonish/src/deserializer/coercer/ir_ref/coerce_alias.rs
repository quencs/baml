use anyhow::Result;
use baml_types::{ir_type::TypeGeneric, BamlValueWithMeta, FieldType};

use crate::deserializer::{
    coercer::{ParsingContext, ParsingError, TypeCoercer},
    types::{HasFlags, HasType},
};

pub(super) fn coerce_alias<T, M>(
    ctx: &ParsingContext,
    alias: &TypeGeneric<T>,
    value: Option<&crate::jsonish::Value>,
) -> Result<BamlValueWithMeta<M>, ParsingError>
where
    M: HasType<Meta = T> + HasFlags + Clone,
{
    // For recursive type aliases, we need to find the target type and coerce to that
    let TypeGeneric::RecursiveTypeAlias { name, .. } = alias else {
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
