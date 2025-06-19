use anyhow::Result;
use baml_types::{ir_type::TypeGeneric, BamlValueWithMeta, CompletionState, FieldType};

use crate::deserializer::{
    deserialize_flags::{DeserializerConditions, Flag},
    types::{HasFlags, HasType},
};

use super::{ParsingContext, ParsingError, TypeCoercer};

pub(super) fn coerce_array<M, T>(
    ctx: &ParsingContext,
    list_target: &TypeGeneric<T>,
    value: Option<&crate::jsonish::Value>,
) -> Result<BamlValueWithMeta<M>, ParsingError>
where
    M: HasType<Meta = T> + HasFlags + Clone,
    TypeGeneric<T>: std::fmt::Display,
{
    assert!(matches!(list_target, TypeGeneric::List(_, _)));

    log::debug!(
        "scope: {scope} :: coercing to: {name} (current: {current})",
        name = list_target.to_string(),
        scope = ctx.display_scope(),
        current = value.map(|v| v.r#type()).unwrap_or("<null>".into())
    );

    let inner = match list_target {
        TypeGeneric::List(inner, _) => inner,
        _ => unreachable!("coerce_array"),
    };

    let mut items = vec![];
    let mut flags = DeserializerConditions::new();

    match &value {
        Some(crate::jsonish::Value::Array(arr, completion_state)) => {
            if *completion_state == CompletionState::Incomplete {
                flags.add_flag(Flag::Incomplete);
            }
            for (i, item) in arr.iter().enumerate() {
                match inner.coerce(&ctx.enter_scope(&format!("{i}")), inner, Some(item)) {
                    Ok(v) => items.push(v),
                    // TODO(vbv): document why we penalize in proportion to how deep into an array a parse error is
                    Err(e) => flags.add_flag(Flag::ArrayItemParseError(i, e)),
                }
            }
        }
        Some(v) => {
            flags.add_flag(Flag::SingleToArray);
            match inner.coerce(&ctx.enter_scope("<implied>"), inner, Some(v)) {
                Ok(v) => items.push(v),
                Err(e) => flags.add_flag(Flag::ArrayItemParseError(0, e)),
            }
        }
        None => {}
    };

    let mut meta = M::default();
    *meta.type_mut() = list_target.clone();
    meta.flags_mut().flags.extend(flags.flags);

    Ok(BamlValueWithMeta::List(items, meta))
}
