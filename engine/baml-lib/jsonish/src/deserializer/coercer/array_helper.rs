use std::any::Any;

use crate::deserializer::{
    deserialize_flags::Flag,
    types::{BamlValueWithFlags, HasFlags, HasType},
};
use anyhow::Result;
use baml_types::{ir_type::TypeGeneric, BamlValueWithMeta};

use super::{ParsingContext, ParsingError};

pub fn coerce_array_to_singular<M, T>(
    ctx: &ParsingContext,
    target: &TypeGeneric<T>,
    items: &[&crate::jsonish::Value],
    coercion: &dyn (Fn(&crate::jsonish::Value) -> Result<BamlValueWithMeta<M>, ParsingError>),
) -> Result<BamlValueWithMeta<M>, ParsingError>
where
    M: HasType<Meta = T> + HasFlags + Clone,
    M: HasFlags,
{
    let parsed = items.iter().map(|item| coercion(item)).collect::<Vec<_>>();

    let mut best = pick_best(ctx, target, &parsed);

    if let Ok(ref mut f) = best {
        f.meta_mut().flags_mut().add_flag(Flag::FirstMatch(
            0,
            parsed
                .iter()
                .map(|r| match r {
                    Ok(v) => {
                        // Convert to concrete type for flag storage
                        let concrete_v: BamlValueWithFlags = v
                            .clone()
                            .map_meta(|m| (m.flags().clone(), m.r#type().clone()));
                        Ok(concrete_v)
                    }
                    Err(e) => Err(e.clone()),
                })
                .collect(),
        ))
    }

    best
}

pub(super) fn pick_best<M, T>(
    ctx: &ParsingContext,
    target: &TypeGeneric<T>,
    res: &[Result<BamlValueWithMeta<M>, ParsingError>],
) -> Result<BamlValueWithMeta<M>, ParsingError>
where
    M: HasType<Meta = T> + HasFlags + Clone,
{
    let Some(first) = res.first() else {
        return Err(ctx.error_unexpected_empty_array(target));
    };
    if res.len() == 1 {
        return first.clone();
    }

    let res_index = (0..res.len())
        .map(|i| match res[i] {
            Ok(ref v) => (i, v.meta().flags().score()),
            Err(_) => (i, i32::MAX),
        })
        .collect::<Vec<_>>();

    // Pick the best one, but in case of picking "default" values like null or empty list, prefer picking the first one
    let mut all_valid_scores = res_index
        .iter()
        .filter_map(|&(i, score)| match res.get(i) {
            Some(Ok(r)) => Some((
                i,
                score,
                match r {
                    BamlValueWithMeta::List(items, meta) => {
                        items.is_empty()
                            && meta
                                .flags()
                                .flags
                                .iter()
                                .any(|f| matches!(f, Flag::SingleToArray))
                    }
                    _ => false,
                },
                r,
            )),
            _ => None,
        })
        .collect::<Vec<_>>();

    // Sort by (false, score, index)
    all_valid_scores.sort_by(
        |&(a, a_score, a_default, a_val), &(b, b_score, b_default, b_val)| {
            // TODO: This is a bit of a hack. We should likely use some is_subtype_of logic here
            // to ensure that we're accepting the "best" type.
            // E.g. if a is a subtype of b, we should prefer a over b. (empty list is a subtype of any list)
            if matches!(a_val, BamlValueWithMeta::List(..))
                && matches!(b_val, BamlValueWithMeta::List(..))
            {
                let a_is_single = a_val
                    .meta()
                    .flags()
                    .flags
                    .iter()
                    .any(|f| matches!(f, Flag::SingleToArray));
                let b_is_single = b_val
                    .meta()
                    .flags()
                    .flags
                    .iter()
                    .any(|f| matches!(f, Flag::SingleToArray));

                match (a_is_single, b_is_single) {
                    // Return B
                    (true, false) => return std::cmp::Ordering::Greater,
                    // Return A
                    (false, true) => return std::cmp::Ordering::Less,
                    _ => {
                        if let (
                            BamlValueWithMeta::List(items_a, _),
                            BamlValueWithMeta::List(items_b, _),
                        ) = (a_val, b_val)
                        {
                            let unparseables_a = a_val
                                .meta()
                                .flags()
                                .flags
                                .iter()
                                .filter(|f| matches!(f, Flag::ArrayItemParseError(..)))
                                .count();
                            let unparseables_b = b_val
                                .meta()
                                .flags()
                                .flags
                                .iter()
                                .filter(|f| matches!(f, Flag::ArrayItemParseError(..)))
                                .count();
                            match (unparseables_a, unparseables_b) {
                                // If A has no unparseables and B has unparseables and B is empty, prefer A
                                (0, b) if b > 0 && items_b.is_empty() => {
                                    return std::cmp::Ordering::Less
                                }
                                // If A has unparseables and B has no unparseables and A is empty, prefer B
                                (a, 0) if a > 0 && items_a.is_empty() => {
                                    return std::cmp::Ordering::Greater
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }

            // De-value default values when comparing
            if let (
                BamlValueWithMeta::Class(_, a_props, _),
                BamlValueWithMeta::Class(_, b_props, _),
            ) = (a_val, b_val)
            {
                // If matching on a union, and one of the choices is picking an object that only
                // had a single string coerced from JSON, prefer the other one
                // (since string cost is low, its better to pick the other one if possible)
                if matches!(target, TypeGeneric::Union(_, _)) {
                    let a_is_coerced_string = a_props.len() == 1
                        && a_props.iter().all(|(_, cond)| {
                            matches!(cond, BamlValueWithMeta::String(..))
                                && cond
                                    .meta()
                                    .flags()
                                    .flags
                                    .iter()
                                    .any(|f| matches!(f, Flag::ImpliedKey(..)))
                        });

                    let b_is_coerced_string = b_props.len() == 1
                        && b_props.iter().all(|(_, cond)| {
                            matches!(cond, BamlValueWithMeta::String(..))
                                && cond
                                    .meta()
                                    .flags()
                                    .flags
                                    .iter()
                                    .any(|f| matches!(f, Flag::ImpliedKey(..)))
                        });

                    match (a_is_coerced_string, b_is_coerced_string) {
                        // Return B
                        (true, false) => return std::cmp::Ordering::Greater,
                        // Return A
                        (false, true) => return std::cmp::Ordering::Less,
                        _ => {}
                    }
                }

                let a_is_default = a_props.iter().all(|(k, cond)| {
                    cond.meta().flags().flags.iter().any(|f| {
                        matches!(
                            f,
                            Flag::OptionalDefaultFromNoValue | Flag::DefaultFromNoValue
                        )
                    })
                });
                let b_is_default = b_props.iter().all(|(k, cond)| {
                    cond.meta().flags().flags.iter().any(|f| {
                        matches!(
                            f,
                            Flag::OptionalDefaultFromNoValue | Flag::DefaultFromNoValue
                        )
                    })
                });

                match (a_is_default, b_is_default) {
                    // Return B
                    (true, false) => return std::cmp::Ordering::Greater,
                    // Return A
                    (false, true) => return std::cmp::Ordering::Less,
                    _ => {}
                }
            }

            // Devalue strings that were cast from objects.
            let a_is_composite = matches!(
                a_val,
                BamlValueWithMeta::Class(..)
                    | BamlValueWithMeta::List(..)
                    | BamlValueWithMeta::Map(..)
            );
            let b_is_composite = matches!(
                b_val,
                BamlValueWithMeta::Class(..)
                    | BamlValueWithMeta::List(..)
                    | BamlValueWithMeta::Map(..)
            );

            if !a_is_composite && b_is_composite {
                if a_val
                    .meta()
                    .flags()
                    .flags
                    .iter()
                    .any(|f| matches!(f, Flag::JsonToString(..) | Flag::FirstMatch(_, _)))
                {
                    return std::cmp::Ordering::Greater;
                }
            }

            if a_is_composite && !b_is_composite {
                if b_val
                    .meta()
                    .flags()
                    .flags
                    .iter()
                    .any(|f| matches!(f, Flag::JsonToString(..) | Flag::FirstMatch(_, _)))
                {
                    return std::cmp::Ordering::Less;
                }
            }

            match a_default.cmp(&b_default) {
                std::cmp::Ordering::Equal => match a_score.cmp(&b_score) {
                    std::cmp::Ordering::Equal => a.cmp(&b),
                    std::cmp::Ordering::Less => std::cmp::Ordering::Less,
                    std::cmp::Ordering::Greater => std::cmp::Ordering::Greater,
                },
                std::cmp::Ordering::Less => std::cmp::Ordering::Less,
                std::cmp::Ordering::Greater => std::cmp::Ordering::Greater,
            }
        },
    );

    log::trace!(
        "Picking {} from {:?} items. Picked({:?}):\n{}",
        target,
        res_index,
        first,
        res.as_ref()
            .iter()
            .enumerate()
            .map(|(idx, r)| match r {
                Ok(r) => format!("{idx} {r:#}"),
                Err(e) => format!("{idx} {e:#}"),
            })
            .collect::<Vec<_>>()
            .join("\n")
    );

    // Take the best one
    match all_valid_scores.first() {
        Some(&(i, _, _, v)) => {
            let mut v = v.clone();
            if res.len() > 1 {
                v.meta_mut()
                    .flags_mut()
                    .add_flag(if matches!(target, TypeGeneric::Union(_, _)) {
                        Flag::UnionMatch(
                            i,
                            res.iter()
                                .map(|r| match r {
                                    Ok(val) => {
                                        // Convert to concrete type for flag storage
                                        let concrete_v: BamlValueWithFlags = val
                                            .clone()
                                            .map_meta(|m| (m.flags().clone(), m.r#type().clone()));
                                        Ok(concrete_v)
                                    }
                                    Err(e) => Err(e.clone()),
                                })
                                .collect(),
                        )
                    } else {
                        Flag::FirstMatch(
                            i,
                            res.iter()
                                .map(|r| match r {
                                    Ok(val) => {
                                        // Convert to concrete type for flag storage
                                        let concrete_v: BamlValueWithFlags = val
                                            .clone()
                                            .map_meta(|m| (m.flags().clone(), m.r#type().clone()));
                                        Ok(concrete_v)
                                    }
                                    Err(e) => Err(e.clone()),
                                })
                                .collect(),
                        )
                    });
            }
            Ok(v.to_owned())
        }
        None => {
            if !res.is_empty() {
                let errors = res.iter().filter_map(|r| match r {
                    Ok(_) => None,
                    Err(e) => Some(e),
                });
                Err(ctx.error_merge_multiple(
                    &format!("Failed to find any {} in {} items", target, res.len()),
                    errors,
                ))
            } else {
                Err(ctx.error_internal("Index out of bounds"))
            }
        }
    }
}
