//! Type simplification pass: `Ty → Ty`.
//!
//! Canonicalizes unions (flatten, dedup, optional→nullable),
//! absorbs subtypes and redundant nulls, and recursively
//! simplifies container inner types.
//!
//! This is a pure transformation on `baml_type::Ty` — it doesn't depend
//! on which compiler produced the type. When compiler2 lands with its own
//! `Ty → baml_type::Ty` conversion, this same simplifier works unchanged.

use crate::Ty;
use baml_base::TyAttr;

/// Simplify a type for codegen consumption.
///
/// Canonicalizes unions (flatten, dedup, optional→nullable),
/// absorbs subtypes and redundant nulls, and recursively
/// simplifies container inner types.
pub fn simplify(ty: &Ty) -> Ty {
    match ty {
        // Optional(T) → simplify(Union([T, Null]))
        Ty::Optional(inner, attr) => {
            let as_union = Ty::Union(
                vec![inner.as_ref().clone(), Ty::null()],
                attr.clone(),
            );
            simplify(&as_union)
        }

        Ty::Union(members, _attr) => simplify_union(members),

        // Recurse into containers
        Ty::List(inner, attr) => Ty::List(Box::new(simplify(inner)), attr.clone()),
        Ty::Map { key, value, attr } => Ty::Map {
            key: Box::new(simplify(key)),
            value: Box::new(simplify(value)),
            attr: attr.clone(),
        },

        // Everything else passes through unchanged
        _ => ty.clone(),
    }
}

/// Core union simplification.
fn simplify_union(members: &[Ty]) -> Ty {
    // 1. Recursively simplify each member
    let simplified: Vec<Ty> = members.iter().map(|m| simplify(m)).collect();

    // 2. Flatten nested unions and convert Optional members
    let mut flat = Vec::new();
    flatten_into(&simplified, &mut flat);

    // 3. Deduplicate by structural equality (ignoring TyAttr via strip)
    dedup(&mut flat);

    // 4. Separate null from non-null variants
    let has_null = flat.iter().any(is_null);
    let mut variants: Vec<Ty> = flat.into_iter().filter(|t| !is_null(t)).collect();

    // 5. Absorb subtypes: if variant X appears inside another variant Y's union, remove X
    absorb_subtypes(&mut variants);

    // 6. Null absorption: if null is top-level AND inside a variant's inner union, absorb it
    let null_absorbed = has_null && any_variant_contains_null(&variants);
    let has_null = has_null && !null_absorbed;

    // 7. Reconstruct
    match variants.len() {
        0 => Ty::null(),
        1 if !has_null => variants.into_iter().next().unwrap(),
        1 => Ty::Union(
            {
                let mut v = variants;
                v.push(Ty::null());
                v
            },
            TyAttr::default(),
        ),
        _ => {
            if has_null {
                variants.push(Ty::null());
            }
            Ty::Union(variants, TyAttr::default())
        }
    }
}

/// Flatten nested unions and convert Optional→[T, Null] into `out`.
fn flatten_into(types: &[Ty], out: &mut Vec<Ty>) {
    for ty in types {
        match ty {
            Ty::Union(inner, _) => flatten_into(inner, out),
            Ty::Optional(inner, _) => {
                flatten_into(&[inner.as_ref().clone()], out);
                out.push(Ty::null());
            }
            other => out.push(other.clone()),
        }
    }
}

/// Remove duplicates preserving order. Uses structural equality
/// ignoring TyAttr (so `int` and `int` with different attrs dedup).
fn dedup(types: &mut Vec<Ty>) {
    let mut seen = Vec::new();
    types.retain(|t| {
        let stripped = strip_attr(t);
        if seen.contains(&stripped) {
            false
        } else {
            seen.push(stripped);
            true
        }
    });
}

/// Strip TyAttr from a type for comparison purposes.
fn strip_attr(ty: &Ty) -> Ty {
    ty.clone().with_attr(TyAttr::default())
}

fn is_null(ty: &Ty) -> bool {
    matches!(ty, Ty::Null { .. })
}

/// Check if `candidate` (without meta) appears as a member of any union variant.
fn is_subtype_of_variant(candidate: &Ty, target: &Ty) -> bool {
    if let Ty::Union(inner, _) = target {
        let candidate_stripped = strip_attr(candidate);
        inner.iter().any(|t| strip_attr(t) == candidate_stripped)
    } else {
        false
    }
}

/// Remove variants that are subtypes of other variants.
/// When X appears inside union Y, X is absorbed (removed).
fn absorb_subtypes(variants: &mut Vec<Ty>) {
    let to_remove: Vec<usize> = variants
        .iter()
        .enumerate()
        .filter_map(|(i, candidate)| {
            let dominated = variants.iter().enumerate().any(|(j, target)| {
                i != j && is_subtype_of_variant(candidate, target)
            });
            if dominated { Some(i) } else { None }
        })
        .collect();

    // Remove in reverse order to preserve indices
    for i in to_remove.into_iter().rev() {
        variants.remove(i);
    }
}

/// Check if any variant is a union that contains null internally.
fn any_variant_contains_null(variants: &[Ty]) -> bool {
    variants.iter().any(|v| {
        if let Ty::Union(inner, _) = v {
            inner.iter().any(is_null)
        } else {
            false
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TypeName;

    #[test]
    fn simplify_null() {
        assert_eq!(simplify(&Ty::null()), Ty::null());
    }

    #[test]
    fn simplify_int() {
        assert_eq!(simplify(&Ty::int()), Ty::int());
    }

    #[test]
    fn simplify_optional_int() {
        // int? → int | null
        let input = Ty::optional(Ty::int());
        let expected = Ty::union([Ty::int(), Ty::null()]);
        assert_eq!(simplify(&input), expected);
    }

    #[test]
    fn simplify_nested_unions() {
        // ((int | null) | string) → int | string | null
        let inner = Ty::union([Ty::int(), Ty::null()]);
        let outer = Ty::union([inner, Ty::string()]);
        let expected = Ty::union([Ty::int(), Ty::string(), Ty::null()]);
        assert_eq!(simplify(&outer), expected);
    }

    #[test]
    fn simplify_repeated_variants() {
        // int | int | string | string → int | string
        let input = Ty::union([Ty::int(), Ty::int(), Ty::string(), Ty::string()]);
        let expected = Ty::union([Ty::int(), Ty::string()]);
        assert_eq!(simplify(&input), expected);
    }

    #[test]
    fn simplify_nested_with_repeats() {
        // int | (int | null) | string → int | string | null
        let inner = Ty::union([Ty::int(), Ty::null()]);
        let input = Ty::union([Ty::int(), inner, Ty::string()]);
        let expected = Ty::union([Ty::int(), Ty::string(), Ty::null()]);
        assert_eq!(simplify(&input), expected);
    }

    #[test]
    fn simplify_subtype_absorption() {
        // (A | B) | B → (A | B)
        // Where A=int, B=string: Union(Union(int, string), string) → Union(int, string)
        let inner = Ty::union([Ty::int(), Ty::string()]);
        let input = Ty::union([inner, Ty::string()]);
        // string is absorbed because it appears inside the inner union
        let expected = Ty::union([Ty::int(), Ty::string()]);
        assert_eq!(simplify(&input), expected);
    }

    #[test]
    fn simplify_null_absorption() {
        // (int | null) | null → int | null
        let inner = Ty::union([Ty::int(), Ty::null()]);
        let input = Ty::union([inner, Ty::null()]);
        let expected = Ty::union([Ty::int(), Ty::null()]);
        assert_eq!(simplify(&input), expected);
    }

    #[test]
    fn simplify_list_inner() {
        // (int | int)[] → int[]
        let inner_union = Ty::union([Ty::int(), Ty::int()]);
        let input = Ty::list(inner_union);
        let expected = Ty::list(Ty::int());
        assert_eq!(simplify(&input), expected);
    }

    #[test]
    fn simplify_map_inner() {
        // map<string, int | int> → map<string, int>
        let value_union = Ty::union([Ty::int(), Ty::int()]);
        let input = Ty::Map {
            key: Box::new(Ty::string()),
            value: Box::new(value_union),
            attr: TyAttr::default(),
        };
        let expected = Ty::Map {
            key: Box::new(Ty::string()),
            value: Box::new(Ty::int()),
            attr: TyAttr::default(),
        };
        assert_eq!(simplify(&input), expected);
    }

    #[test]
    fn simplify_single_member_union() {
        // Union([int]) → int
        let input = Ty::union([Ty::int()]);
        assert_eq!(simplify(&input), Ty::int());
    }

    #[test]
    fn simplify_all_null_union() {
        // Union([null, null]) → null
        let input = Ty::union([Ty::null(), Ty::null()]);
        assert_eq!(simplify(&input), Ty::null());
    }

    #[test]
    fn simplify_class_passthrough() {
        let input = Ty::class("MyClass");
        assert_eq!(simplify(&input), Ty::class("MyClass"));
    }

    #[test]
    fn simplify_deeply_nested_optional() {
        // ((int?)?) → int | null
        let input = Ty::optional(Ty::optional(Ty::int()));
        let expected = Ty::union([Ty::int(), Ty::null()]);
        assert_eq!(simplify(&input), expected);
    }

    #[test]
    fn simplify_type_alias_passthrough() {
        // Recursive TypeAlias passes through unchanged
        let input = Ty::TypeAlias(TypeName::local("Json".into()), TyAttr::default());
        assert_eq!(simplify(&input), input);
    }

    #[test]
    fn simplify_optional_type_alias() {
        // Json? → Json | null (TypeAlias survives in the union)
        let alias = Ty::TypeAlias(TypeName::local("Json".into()), TyAttr::default());
        let input = Ty::optional(alias.clone());
        let expected = Ty::union([alias, Ty::null()]);
        assert_eq!(simplify(&input), expected);
    }

    #[test]
    fn simplify_union_with_type_alias() {
        // Json | int → Json | int (alias is not flattened)
        let alias = Ty::TypeAlias(TypeName::local("Json".into()), TyAttr::default());
        let input = Ty::union([alias.clone(), Ty::int()]);
        let expected = Ty::union([alias, Ty::int()]);
        assert_eq!(simplify(&input), expected);
    }

    #[test]
    fn simplify_self_referencing_class_in_list() {
        // Node[] passes through (class ref inside list)
        let input = Ty::list(Ty::class("Node"));
        assert_eq!(simplify(&input), Ty::list(Ty::class("Node")));
    }

    #[test]
    fn simplify_optional_self_referencing_class() {
        // Node? → Node | null
        let input = Ty::optional(Ty::class("Node"));
        let expected = Ty::union([Ty::class("Node"), Ty::null()]);
        assert_eq!(simplify(&input), expected);
    }
}
