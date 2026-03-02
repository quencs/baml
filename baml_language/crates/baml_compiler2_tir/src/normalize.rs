//! Type normalization and subtyping.
//!
//! Converts surface `Ty` types to an internal `StructuralTy` where all type
//! aliases are resolved. Recursive aliases are represented using Mu types with
//! equirecursive (co-inductive) subtyping.

use std::collections::{HashMap, HashSet};

use baml_base::Name;

use crate::ty::{LiteralValue, PrimitiveType, Ty};

// ═══════════════════════════════════════════════════════════════════════════
// PUBLIC API
// ═══════════════════════════════════════════════════════════════════════════

/// Check if `sub` is a subtype of `sup`, resolving type aliases.
pub(crate) fn is_subtype_of(sub: &Ty, sup: &Ty, aliases: &HashMap<Name, Ty>) -> bool {
    let recursive = find_recursive_aliases(aliases);
    let sub_norm = normalize(sub, aliases, &recursive);
    let sup_norm = normalize(sup, aliases, &recursive);
    sub_norm.is_subtype_of(&sup_norm, &mut HashSet::new())
}

/// Find all recursive type aliases via DFS.
pub fn find_recursive_aliases(aliases: &HashMap<Name, Ty>) -> HashSet<Name> {
    let mut recursive = HashSet::new();
    for name in aliases.keys() {
        let mut visited = HashSet::new();
        let mut stack = HashSet::new();
        if has_cycle(name, aliases, &mut visited, &mut stack) {
            recursive.insert(name.clone());
        }
    }
    recursive
}

// ═══════════════════════════════════════════════════════════════════════════
// STRUCTURAL TYPE (private)
// ═══════════════════════════════════════════════════════════════════════════

/// Normalized structural type. All aliases resolved, recursion explicit.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum StructuralTy {
    // Primitives
    Int,
    Float,
    String,
    Bool,
    Null,
    Image,
    Audio,
    Video,
    Pdf,
    // Literal
    Literal(LiteralValue),
    // User-defined (resolved by name)
    Class(Name),
    Enum(Name),
    EnumVariant(Name, Name),
    // Constructors
    Optional(Box<StructuralTy>),
    List(Box<StructuralTy>),
    Map {
        key: Box<StructuralTy>,
        value: Box<StructuralTy>,
    },
    Union(Vec<StructuralTy>),
    Function {
        params: Vec<StructuralTy>,
        ret: Box<StructuralTy>,
    },
    // Recursion
    Mu {
        var: Name,
        body: Box<StructuralTy>,
    },
    TyVar(Name),
    // Special
    Never,
    Void,
    Unknown,
    Error,
}

impl StructuralTy {
    /// Equirecursive subtyping with co-inductive assumptions.
    fn is_subtype_of(
        &self,
        other: &StructuralTy,
        assumptions: &mut HashSet<(StructuralTy, StructuralTy)>,
    ) -> bool {
        // Co-inductive: if we've assumed this pair, it holds
        let pair = (self.clone(), other.clone());
        if assumptions.contains(&pair) {
            return true;
        }

        // Reflexivity
        if self == other {
            return true;
        }

        // Never is the bottom type — subtype of everything
        if matches!(self, StructuralTy::Never) {
            return true;
        }

        // Void is only compatible with itself (handled by reflexivity above)
        if matches!(self, StructuralTy::Void) || matches!(other, StructuralTy::Void) {
            return false;
        }

        // Error recovery: Unknown/Error are compatible with anything
        if matches!(self, StructuralTy::Unknown | StructuralTy::Error)
            || matches!(other, StructuralTy::Unknown | StructuralTy::Error)
        {
            return true;
        }

        assumptions.insert(pair.clone());

        let result = match (self, other) {
            // Mu unfolding
            (StructuralTy::Mu { var, body }, other) => {
                let unfolded = substitute(body, var, self);
                unfolded.is_subtype_of(other, assumptions)
            }
            (self_ty, StructuralTy::Mu { var, body }) => {
                let unfolded = substitute(body, var, other);
                self_ty.is_subtype_of(&unfolded, assumptions)
            }

            // TyVar (inside Mu bodies)
            (StructuralTy::TyVar(v1), StructuralTy::TyVar(v2)) => v1 == v2,

            // Null <: Optional<T>
            (StructuralTy::Null, StructuralTy::Optional(_)) => true,

            // T <: Optional<T>
            (inner, StructuralTy::Optional(opt_inner)) => {
                inner.is_subtype_of(opt_inner, assumptions)
            }

            // Optional<T> <: T | null
            (StructuralTy::Optional(inner), StructuralTy::Union(types)) => {
                types.contains(&StructuralTy::Null)
                    && types.iter().any(|t| inner.is_subtype_of(t, assumptions))
            }

            // T <: T | U
            (inner, StructuralTy::Union(types)) => {
                types.iter().any(|t| inner.is_subtype_of(t, assumptions))
            }

            // Union<T1, T2> <: U iff all Ti <: U
            (StructuralTy::Union(types), other) => {
                types.iter().all(|t| t.is_subtype_of(other, assumptions))
            }

            // List covariance
            (StructuralTy::List(inner1), StructuralTy::List(inner2)) => {
                inner1.is_subtype_of(inner2, assumptions)
            }

            // Map covariance in value, invariant in key
            (
                StructuralTy::Map { key: k1, value: v1 },
                StructuralTy::Map { key: k2, value: v2 },
            ) => {
                let keys_compatible = k1 == k2
                    || matches!(k1.as_ref(), StructuralTy::Unknown | StructuralTy::Error)
                    || matches!(k2.as_ref(), StructuralTy::Unknown | StructuralTy::Error);
                keys_compatible && v1.is_subtype_of(v2, assumptions)
            }

            // Int <: Float
            (StructuralTy::Int, StructuralTy::Float) => true,

            // Literal types are subtypes of their base types
            (StructuralTy::Literal(LiteralValue::Int(_)), StructuralTy::Int) => true,
            (StructuralTy::Literal(LiteralValue::Int(_)), StructuralTy::Float) => true,
            (StructuralTy::Literal(LiteralValue::Float(_)), StructuralTy::Float) => true,
            (StructuralTy::Literal(LiteralValue::String(_)), StructuralTy::String) => true,
            (StructuralTy::Literal(LiteralValue::Bool(_)), StructuralTy::Bool) => true,

            // EnumVariant(E, V) <: Enum(E)
            (StructuralTy::EnumVariant(e, _), StructuralTy::Enum(sup_e)) => e == sup_e,

            // Function subtyping: contravariant params, covariant return
            (
                StructuralTy::Function {
                    params: params1,
                    ret: ret1,
                },
                StructuralTy::Function {
                    params: params2,
                    ret: ret2,
                },
            ) => {
                if !ret1.is_subtype_of(ret2, assumptions) {
                    return false;
                }
                if params2.len() > params1.len() {
                    return false;
                }
                for (p1, p2) in params1.iter().zip(params2.iter()) {
                    if !p2.is_subtype_of(p1, assumptions) {
                        return false;
                    }
                }
                true
            }

            _ => false,
        };

        assumptions.remove(&pair);
        result
    }
}

/// Substitute `TyVar` with replacement in type.
fn substitute(ty: &StructuralTy, var: &Name, replacement: &StructuralTy) -> StructuralTy {
    match ty {
        StructuralTy::TyVar(v) if v == var => replacement.clone(),
        StructuralTy::Optional(inner) => {
            StructuralTy::Optional(Box::new(substitute(inner, var, replacement)))
        }
        StructuralTy::List(inner) => {
            StructuralTy::List(Box::new(substitute(inner, var, replacement)))
        }
        StructuralTy::Map { key, value } => StructuralTy::Map {
            key: Box::new(substitute(key, var, replacement)),
            value: Box::new(substitute(value, var, replacement)),
        },
        StructuralTy::Union(types) => StructuralTy::Union(
            types
                .iter()
                .map(|t| substitute(t, var, replacement))
                .collect(),
        ),
        StructuralTy::Function { params, ret } => StructuralTy::Function {
            params: params
                .iter()
                .map(|t| substitute(t, var, replacement))
                .collect(),
            ret: Box::new(substitute(ret, var, replacement)),
        },
        StructuralTy::Mu { var: v, body } if v != var => StructuralTy::Mu {
            var: v.clone(),
            body: Box::new(substitute(body, var, replacement)),
        },
        _ => ty.clone(),
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// NORMALIZATION (private)
// ═══════════════════════════════════════════════════════════════════════════

fn normalize(ty: &Ty, aliases: &HashMap<Name, Ty>, recursive: &HashSet<Name>) -> StructuralTy {
    let mut expanding = HashSet::new();
    normalize_impl(ty, aliases, recursive, &mut expanding)
}

fn normalize_impl(
    ty: &Ty,
    aliases: &HashMap<Name, Ty>,
    recursive: &HashSet<Name>,
    expanding: &mut HashSet<Name>,
) -> StructuralTy {
    match ty {
        Ty::Primitive(p) => match p {
            PrimitiveType::Int => StructuralTy::Int,
            PrimitiveType::Float => StructuralTy::Float,
            PrimitiveType::String => StructuralTy::String,
            PrimitiveType::Bool => StructuralTy::Bool,
            PrimitiveType::Null => StructuralTy::Null,
            PrimitiveType::Image => StructuralTy::Image,
            PrimitiveType::Audio => StructuralTy::Audio,
            PrimitiveType::Video => StructuralTy::Video,
            PrimitiveType::Pdf => StructuralTy::Pdf,
        },
        Ty::Never => StructuralTy::Never,
        Ty::Void => StructuralTy::Void,
        Ty::Unknown => StructuralTy::Unknown,
        Ty::Error => StructuralTy::Error,
        Ty::Literal(lit, _freshness) => StructuralTy::Literal(lit.clone()),
        Ty::Class(name) => StructuralTy::Class(name.clone()),
        Ty::Enum(name) => StructuralTy::Enum(name.clone()),
        Ty::EnumVariant(e, v) => StructuralTy::EnumVariant(e.clone(), v.clone()),

        Ty::TypeAlias(name) => {
            if expanding.contains(name) {
                return StructuralTy::TyVar(name.clone());
            }

            if let Some(alias_ty) = aliases.get(name) {
                if recursive.contains(name) {
                    expanding.insert(name.clone());
                    let body = normalize_impl(alias_ty, aliases, recursive, expanding);
                    expanding.remove(name);
                    StructuralTy::Mu {
                        var: name.clone(),
                        body: Box::new(body),
                    }
                } else {
                    normalize_impl(alias_ty, aliases, recursive, expanding)
                }
            } else {
                StructuralTy::Error
            }
        }

        Ty::Optional(inner) => StructuralTy::Optional(Box::new(normalize_impl(
            inner, aliases, recursive, expanding,
        ))),
        Ty::List(inner) | Ty::EvolvingList(inner) => StructuralTy::List(Box::new(normalize_impl(
            inner, aliases, recursive, expanding,
        ))),
        Ty::Map(key, value) | Ty::EvolvingMap(key, value) => StructuralTy::Map {
            key: Box::new(normalize_impl(key, aliases, recursive, expanding)),
            value: Box::new(normalize_impl(value, aliases, recursive, expanding)),
        },
        Ty::Union(types) => StructuralTy::Union(
            types
                .iter()
                .map(|t| normalize_impl(t, aliases, recursive, expanding))
                .collect(),
        ),
        Ty::Function { params, ret } => StructuralTy::Function {
            params: params
                .iter()
                .map(|(_, t)| normalize_impl(t, aliases, recursive, expanding))
                .collect(),
            ret: Box::new(normalize_impl(ret, aliases, recursive, expanding)),
        },
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// CYCLE DETECTION
// ═══════════════════════════════════════════════════════════════════════════

fn has_cycle(
    name: &Name,
    aliases: &HashMap<Name, Ty>,
    visited: &mut HashSet<Name>,
    stack: &mut HashSet<Name>,
) -> bool {
    if stack.contains(name) {
        return true;
    }
    if visited.contains(name) {
        return false;
    }
    visited.insert(name.clone());
    stack.insert(name.clone());
    let result = aliases
        .get(name)
        .is_some_and(|ty| ty_has_cycle(ty, aliases, visited, stack));
    stack.remove(name);
    result
}

fn ty_has_cycle(
    ty: &Ty,
    aliases: &HashMap<Name, Ty>,
    visited: &mut HashSet<Name>,
    stack: &mut HashSet<Name>,
) -> bool {
    match ty {
        Ty::TypeAlias(name) if aliases.contains_key(name) => {
            has_cycle(name, aliases, visited, stack)
        }
        Ty::Optional(inner) | Ty::List(inner) | Ty::EvolvingList(inner) => {
            ty_has_cycle(inner, aliases, visited, stack)
        }
        Ty::Map(key, value) | Ty::EvolvingMap(key, value) => {
            ty_has_cycle(key, aliases, visited, stack)
                || ty_has_cycle(value, aliases, visited, stack)
        }
        Ty::Union(types) => types
            .iter()
            .any(|t| ty_has_cycle(t, aliases, visited, stack)),
        Ty::Function { params, ret } => {
            params
                .iter()
                .any(|(_, t)| ty_has_cycle(t, aliases, visited, stack))
                || ty_has_cycle(ret, aliases, visited, stack)
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ty::Freshness;

    fn type_alias(name: &str) -> Ty {
        Ty::TypeAlias(Name::new(name))
    }

    #[test]
    fn test_simple_alias() {
        let mut aliases = HashMap::new();
        aliases.insert(Name::new("MyInt"), Ty::Primitive(PrimitiveType::Int));

        assert!(is_subtype_of(
            &type_alias("MyInt"),
            &Ty::Primitive(PrimitiveType::Int),
            &aliases
        ));
        assert!(is_subtype_of(
            &Ty::Primitive(PrimitiveType::Int),
            &type_alias("MyInt"),
            &aliases
        ));
    }

    #[test]
    fn test_transitive_alias() {
        let mut aliases = HashMap::new();
        aliases.insert(Name::new("MyInt"), Ty::Primitive(PrimitiveType::Int));
        aliases.insert(Name::new("AnotherInt"), type_alias("MyInt"));

        assert!(is_subtype_of(
            &type_alias("AnotherInt"),
            &Ty::Primitive(PrimitiveType::Int),
            &aliases
        ));
        assert!(is_subtype_of(
            &type_alias("AnotherInt"),
            &type_alias("MyInt"),
            &aliases
        ));
    }

    #[test]
    fn test_union_alias() {
        let mut aliases = HashMap::new();
        aliases.insert(
            Name::new("IntOrString"),
            Ty::Union(vec![
                Ty::Primitive(PrimitiveType::Int),
                Ty::Primitive(PrimitiveType::String),
            ]),
        );

        assert!(is_subtype_of(
            &Ty::Primitive(PrimitiveType::Int),
            &type_alias("IntOrString"),
            &aliases
        ));
        assert!(is_subtype_of(
            &Ty::Primitive(PrimitiveType::String),
            &type_alias("IntOrString"),
            &aliases
        ));
        assert!(!is_subtype_of(
            &Ty::Primitive(PrimitiveType::Bool),
            &type_alias("IntOrString"),
            &aliases
        ));
    }

    #[test]
    fn test_recursive_alias_detection() {
        let mut aliases = HashMap::new();
        aliases.insert(
            Name::new("List"),
            Ty::Union(vec![Ty::Primitive(PrimitiveType::Null), type_alias("List")]),
        );

        let recursive = find_recursive_aliases(&aliases);
        assert!(recursive.contains(&Name::new("List")));
    }

    #[test]
    fn test_non_recursive_not_marked() {
        let mut aliases = HashMap::new();
        aliases.insert(Name::new("MyInt"), Ty::Primitive(PrimitiveType::Int));

        let recursive = find_recursive_aliases(&aliases);
        assert!(!recursive.contains(&Name::new("MyInt")));
    }

    #[test]
    fn test_never_is_bottom() {
        let aliases = HashMap::new();

        assert!(is_subtype_of(
            &Ty::Never,
            &Ty::Primitive(PrimitiveType::Int),
            &aliases
        ));
        assert!(is_subtype_of(
            &Ty::Never,
            &Ty::Primitive(PrimitiveType::String),
            &aliases
        ));
        assert!(is_subtype_of(
            &Ty::Never,
            &Ty::Class(Name::new("Foo")),
            &aliases
        ));
        assert!(is_subtype_of(
            &Ty::Never,
            &Ty::Optional(Box::new(Ty::Primitive(PrimitiveType::Int))),
            &aliases
        ));
    }

    #[test]
    fn test_int_subtype_of_float() {
        let aliases = HashMap::new();
        assert!(is_subtype_of(
            &Ty::Primitive(PrimitiveType::Int),
            &Ty::Primitive(PrimitiveType::Float),
            &aliases
        ));
        assert!(!is_subtype_of(
            &Ty::Primitive(PrimitiveType::Float),
            &Ty::Primitive(PrimitiveType::Int),
            &aliases
        ));
    }

    #[test]
    fn test_literal_widens() {
        let aliases = HashMap::new();
        // Fresh and Regular should both be subtypes of their base primitive
        assert!(is_subtype_of(
            &Ty::Literal(LiteralValue::Int(42), Freshness::Fresh),
            &Ty::Primitive(PrimitiveType::Int),
            &aliases
        ));
        assert!(is_subtype_of(
            &Ty::Literal(LiteralValue::Int(42), Freshness::Regular),
            &Ty::Primitive(PrimitiveType::Float),
            &aliases
        ));
        assert!(is_subtype_of(
            &Ty::Literal(LiteralValue::String("hi".into()), Freshness::Fresh),
            &Ty::Primitive(PrimitiveType::String),
            &aliases
        ));
        assert!(!is_subtype_of(
            &Ty::Literal(LiteralValue::Int(42), Freshness::Fresh),
            &Ty::Primitive(PrimitiveType::String),
            &aliases
        ));
        // Freshness is ignored for subtyping: Fresh(1) <: Regular(1)
        assert!(is_subtype_of(
            &Ty::Literal(LiteralValue::Int(42), Freshness::Fresh),
            &Ty::Literal(LiteralValue::Int(42), Freshness::Regular),
            &aliases
        ));
    }

    #[test]
    fn test_enum_variant_subtype_of_enum() {
        let aliases = HashMap::new();
        assert!(is_subtype_of(
            &Ty::EnumVariant(Name::new("Color"), Name::new("Red")),
            &Ty::Enum(Name::new("Color")),
            &aliases
        ));
        assert!(!is_subtype_of(
            &Ty::EnumVariant(Name::new("Color"), Name::new("Red")),
            &Ty::Enum(Name::new("Shape")),
            &aliases
        ));
    }

    #[test]
    fn test_function_covariant_return() {
        let aliases = HashMap::new();
        let f1 = Ty::Function {
            params: vec![(None, Ty::Primitive(PrimitiveType::Int))],
            ret: Box::new(Ty::Primitive(PrimitiveType::Int)),
        };
        let f2 = Ty::Function {
            params: vec![(None, Ty::Primitive(PrimitiveType::Int))],
            ret: Box::new(Ty::Primitive(PrimitiveType::Float)),
        };
        assert!(is_subtype_of(&f1, &f2, &aliases));
        assert!(!is_subtype_of(&f2, &f1, &aliases));
    }

    #[test]
    fn test_function_contravariant_params() {
        let aliases = HashMap::new();
        let f1 = Ty::Function {
            params: vec![(None, Ty::Primitive(PrimitiveType::Float))],
            ret: Box::new(Ty::Primitive(PrimitiveType::String)),
        };
        let f2 = Ty::Function {
            params: vec![(None, Ty::Primitive(PrimitiveType::Int))],
            ret: Box::new(Ty::Primitive(PrimitiveType::String)),
        };
        assert!(is_subtype_of(&f1, &f2, &aliases));
        assert!(!is_subtype_of(&f2, &f1, &aliases));
    }

    #[test]
    fn test_optional_subtyping() {
        let aliases = HashMap::new();
        // int <: int?
        assert!(is_subtype_of(
            &Ty::Primitive(PrimitiveType::Int),
            &Ty::Optional(Box::new(Ty::Primitive(PrimitiveType::Int))),
            &aliases
        ));
        // null <: int?
        assert!(is_subtype_of(
            &Ty::Primitive(PrimitiveType::Null),
            &Ty::Optional(Box::new(Ty::Primitive(PrimitiveType::Int))),
            &aliases
        ));
        // string NOT <: int?
        assert!(!is_subtype_of(
            &Ty::Primitive(PrimitiveType::String),
            &Ty::Optional(Box::new(Ty::Primitive(PrimitiveType::Int))),
            &aliases
        ));
    }

    // ── Evolving container tests ────────────────────────────────────────────

    #[test]
    fn test_evolving_list_subtype_of_list() {
        let aliases = HashMap::new();
        // EvolvingList(int) <: List(int)
        assert!(is_subtype_of(
            &Ty::EvolvingList(Box::new(Ty::Primitive(PrimitiveType::Int))),
            &Ty::List(Box::new(Ty::Primitive(PrimitiveType::Int))),
            &aliases
        ));
        // List(int) <: EvolvingList(int)
        assert!(is_subtype_of(
            &Ty::List(Box::new(Ty::Primitive(PrimitiveType::Int))),
            &Ty::EvolvingList(Box::new(Ty::Primitive(PrimitiveType::Int))),
            &aliases
        ));
    }

    #[test]
    fn test_evolving_list_covariance() {
        let aliases = HashMap::new();
        // EvolvingList(int) <: List(float) (int <: float)
        assert!(is_subtype_of(
            &Ty::EvolvingList(Box::new(Ty::Primitive(PrimitiveType::Int))),
            &Ty::List(Box::new(Ty::Primitive(PrimitiveType::Float))),
            &aliases
        ));
        // EvolvingList(string) NOT <: List(int)
        assert!(!is_subtype_of(
            &Ty::EvolvingList(Box::new(Ty::Primitive(PrimitiveType::String))),
            &Ty::List(Box::new(Ty::Primitive(PrimitiveType::Int))),
            &aliases
        ));
    }

    #[test]
    fn test_evolving_list_never_is_bottom() {
        let aliases = HashMap::new();
        // EvolvingList(Never) <: List(int) — empty evolving is assignable anywhere
        assert!(is_subtype_of(
            &Ty::EvolvingList(Box::new(Ty::Never)),
            &Ty::List(Box::new(Ty::Primitive(PrimitiveType::Int))),
            &aliases
        ));
    }

    #[test]
    fn test_evolving_map_subtype_of_map() {
        let aliases = HashMap::new();
        // EvolvingMap(string, int) <: Map(string, int)
        assert!(is_subtype_of(
            &Ty::EvolvingMap(
                Box::new(Ty::Primitive(PrimitiveType::String)),
                Box::new(Ty::Primitive(PrimitiveType::Int)),
            ),
            &Ty::Map(
                Box::new(Ty::Primitive(PrimitiveType::String)),
                Box::new(Ty::Primitive(PrimitiveType::Int)),
            ),
            &aliases
        ));
    }

    #[test]
    fn test_make_evolving() {
        // List(Never) → EvolvingList(Never)
        assert_eq!(
            Ty::List(Box::new(Ty::Never)).make_evolving(),
            Ty::EvolvingList(Box::new(Ty::Never))
        );
        // Map(Never, Never) → EvolvingMap(Never, Never)
        assert_eq!(
            Ty::Map(Box::new(Ty::Never), Box::new(Ty::Never)).make_evolving(),
            Ty::EvolvingMap(Box::new(Ty::Never), Box::new(Ty::Never))
        );
        // Non-empty List passes through
        assert_eq!(
            Ty::List(Box::new(Ty::Primitive(PrimitiveType::Int))).make_evolving(),
            Ty::List(Box::new(Ty::Primitive(PrimitiveType::Int)))
        );
        // Non-container passes through
        assert_eq!(
            Ty::Primitive(PrimitiveType::Int).make_evolving(),
            Ty::Primitive(PrimitiveType::Int)
        );
    }

    #[test]
    fn test_evolving_display() {
        assert_eq!(Ty::EvolvingList(Box::new(Ty::Never)).to_string(), "_[]");
        assert_eq!(
            Ty::EvolvingList(Box::new(Ty::Primitive(PrimitiveType::Int))).to_string(),
            "int[] (evolving)"
        );
        assert_eq!(
            Ty::EvolvingMap(Box::new(Ty::Never), Box::new(Ty::Never)).to_string(),
            "map<_, _>"
        );
        assert_eq!(
            Ty::EvolvingMap(
                Box::new(Ty::Primitive(PrimitiveType::String)),
                Box::new(Ty::Primitive(PrimitiveType::Int))
            )
            .to_string(),
            "map<string, int> (evolving)"
        );
    }
}
