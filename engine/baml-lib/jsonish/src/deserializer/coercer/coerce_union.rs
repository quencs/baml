use anyhow::Result;
use baml_types::{BamlValueWithMeta, FieldType};

use crate::deserializer::{coercer::array_helper, types::{HasFlags, HasType}};

use super::{ParsingContext, ParsingError, TypeCoercer};

pub(super) fn coerce_union<M>(
    ctx: &ParsingContext,
    union_target: &FieldType,
    value: Option<&crate::jsonish::Value>,
) -> Result<BamlValueWithMeta<M>, ParsingError>
where
    M: HasType<Type = FieldType> + HasFlags,
{
    assert!(matches!(union_target, FieldType::Union(_, _)));

    log::debug!(
        "scope: {scope} :: coercing to: {name} (current: {current})",
        name = union_target.to_string(),
        scope = ctx.display_scope(),
        current = value.map(|v| v.r#type()).unwrap_or("<null>".into())
    );

    let options = match union_target {
        FieldType::Union(options, _) => options,
        _ => unreachable!("coerce_union"),
    };

    let parsed = options
        .iter_include_null()
        .iter()
        .map(|option| option.coerce(ctx, union_target, value))
        .collect::<Vec<_>>();

    array_helper::pick_best(ctx, union_target, &parsed)
}
