//! Resolved type representation — the output of type resolution.

use std::fmt;

use baml_base::Name;

/// Resolved type — the output of type resolution (Pass 2).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Ty {
    /// A class type — just the name, no expansion.
    Class(Name),
    /// An enum type.
    Enum(Name),
    /// An enum variant — Enum(name) . Variant(name).
    EnumVariant(Name, Name),
    /// A type alias — opaque name reference, NOT expanded.
    /// Expansion happens lazily at subtype-checking time.
    TypeAlias(Name),
    /// Primitive types.
    Primitive(PrimitiveType),
    /// T[]
    List(Box<Ty>),
    /// map<K, V>
    Map(Box<Ty>, Box<Ty>),
    /// A | B | C
    Union(Vec<Ty>),
    /// T?
    Optional(Box<Ty>),
    /// Literal string/int/bool as a type.
    ///
    /// Carries a `Freshness` flag modeled after TypeScript's fresh/regular
    /// literal types. Fresh literals (from expressions) widen to their base
    /// primitive at mutable binding sites. Regular literals (from type
    /// annotations or contextual typing) are preserved.
    Literal(LiteralValue, Freshness),
    /// Evolving list — created from empty array literal `[]` at mutable
    /// binding sites (via `make_evolving()`). Element type starts as `Never`
    /// and is refined by mutations (`.push()`, index assignment).
    ///
    /// Reading the variable in expression position produces the fixed
    /// `List(T)` type; the local entry stays `EvolvingList` so further
    /// mutations still work.
    ///
    /// Parallel to `Freshness` on literals: `make_evolving()` is the mirror
    /// of `widen_fresh()` — both called at `let` binding sites without
    /// type annotations.
    EvolvingList(Box<Ty>),
    /// Evolving map — created from empty map literal at mutable binding sites.
    /// Same semantics as `EvolvingList` but for maps.
    EvolvingMap(Box<Ty>, Box<Ty>),
    /// Function type: (params) -> return.
    Function {
        params: Vec<(Option<Name>, Ty)>,
        ret: Box<Ty>,
    },
    /// The bottom type — expression never produces a value.
    /// Assigned to `return`, `break`, `continue`, and blocks that always diverge.
    /// `Never` is a subtype of every type: `join(Never, T) = T`.
    Never,
    /// The void type — produced by statements and expressions that don't yield
    /// a useful value (e.g. `if` without `else`, bare function calls used as
    /// statements, `while` loops).
    ///
    /// `Void` is **not** a subtype of any other type. Consuming a `Void` value
    /// (assigning it, passing it as an argument, returning it) is a type error.
    /// In statement position the value is simply discarded.
    ///
    /// Analogous to TypeScript's fresh-object excess-property check pattern:
    /// the type is valid only when nobody reads the value.
    Void,
    /// Error recovery — the type is structurally unknown (e.g., name unresolved).
    Unknown,
    /// Error sentinel — a hard error was emitted for this expression.
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PrimitiveType {
    Int,
    Float,
    String,
    Bool,
    Null,
    Image,
    Audio,
    Video,
    Pdf,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum LiteralValue {
    String(std::string::String),
    Int(i64),
    Float(std::string::String),
    Bool(bool),
}

/// Freshness flag for literal types.
///
/// Modeled after TypeScript's fresh/regular literal type distinction.
/// - **Fresh**: produced by literal expressions (`1`, `"hello"`). Widens to
///   the base primitive at mutable binding sites (`let x = 1` → `int`).
/// - **Regular**: produced by type annotations (`let x: 1 = 1`) or contextual
///   typing. Preserved through mutable bindings.
///
/// Freshness is **ignored** by the subtype checker — `Literal(1, Fresh)` and
/// `Literal(1, Regular)` are structurally identical for assignability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Freshness {
    Fresh,
    Regular,
}

impl Ty {
    /// Widen fresh literal types to their base primitive.
    ///
    /// Called at mutable binding sites (`let` without annotation).
    /// Regular (non-fresh) literals pass through unchanged.
    pub fn widen_fresh(self) -> Ty {
        match self {
            Ty::Literal(lit, Freshness::Fresh) => Ty::Primitive(lit.base_primitive()),
            other => other,
        }
    }

    /// Promote empty containers to evolving containers.
    ///
    /// Called at mutable binding sites (`let` without annotation), right
    /// after `widen_fresh()`. This is the mirror of `widen_fresh()`:
    /// - `widen_fresh` *removes* literal specificity (1 → int)
    /// - `make_evolving` *adds* container mutability (List(Never) → EvolvingList(Never))
    ///
    /// Only converts `List(Never)` and `Map(Never, Never)` — non-empty
    /// container literals already have a known element type and don't need
    /// evolving semantics.
    pub fn make_evolving(self) -> Ty {
        match self {
            Ty::List(inner) if matches!(*inner, Ty::Never) => Ty::EvolvingList(inner),
            Ty::Map(k, v) if matches!(*k, Ty::Never) && matches!(*v, Ty::Never) => {
                Ty::EvolvingMap(k, v)
            }
            other => other,
        }
    }
}

impl LiteralValue {
    /// The base primitive type that this literal widens to.
    pub fn base_primitive(&self) -> PrimitiveType {
        match self {
            LiteralValue::String(_) => PrimitiveType::String,
            LiteralValue::Int(_) => PrimitiveType::Int,
            LiteralValue::Float(_) => PrimitiveType::Float,
            LiteralValue::Bool(_) => PrimitiveType::Bool,
        }
    }
}

// ── Display impls ────────────────────────────────────────────────────────────

impl fmt::Display for Ty {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Ty::Class(n) => write!(f, "{n}"),
            Ty::Enum(n) => write!(f, "enum {n}"),
            Ty::EnumVariant(e, v) => write!(f, "{e}.{v}"),
            Ty::TypeAlias(n) => write!(f, "type {n}"),
            Ty::Primitive(p) => write!(f, "{p}"),
            Ty::List(inner) => write!(f, "{inner}[]"),
            Ty::Map(k, v) => write!(f, "map<{k}, {v}>"),
            Ty::EvolvingList(inner) => {
                if matches!(**inner, Ty::Never) {
                    write!(f, "_[]")
                } else {
                    write!(f, "{inner}[] (evolving)")
                }
            }
            Ty::EvolvingMap(k, v) => {
                if matches!(**k, Ty::Never) && matches!(**v, Ty::Never) {
                    write!(f, "map<_, _>")
                } else {
                    write!(f, "map<{k}, {v}> (evolving)")
                }
            }
            Ty::Union(members) => {
                let parts: Vec<_> = members.iter().map(|m| m.to_string()).collect();
                write!(f, "{}", parts.join(" | "))
            }
            Ty::Optional(inner) => write!(f, "{inner}?"),
            Ty::Literal(lit, _freshness) => write!(f, "{lit}"),
            Ty::Function { params, ret } => {
                let ps: Vec<String> = params
                    .iter()
                    .map(|(name, ty)| {
                        name.as_ref()
                            .map(|n| format!("{n}: {ty}"))
                            .unwrap_or_else(|| ty.to_string())
                    })
                    .collect();
                write!(f, "({}) -> {ret}", ps.join(", "))
            }
            Ty::Never => write!(f, "never"),
            Ty::Void => write!(f, "void"),
            Ty::Unknown => write!(f, "unknown"),
            Ty::Error => write!(f, "!error"),
        }
    }
}

impl fmt::Display for PrimitiveType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PrimitiveType::Int => write!(f, "int"),
            PrimitiveType::Float => write!(f, "float"),
            PrimitiveType::String => write!(f, "string"),
            PrimitiveType::Bool => write!(f, "bool"),
            PrimitiveType::Null => write!(f, "null"),
            PrimitiveType::Image => write!(f, "image"),
            PrimitiveType::Audio => write!(f, "audio"),
            PrimitiveType::Video => write!(f, "video"),
            PrimitiveType::Pdf => write!(f, "pdf"),
        }
    }
}

impl fmt::Display for LiteralValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LiteralValue::String(s) => write!(f, "\"{s}\""),
            LiteralValue::Int(i) => write!(f, "{i}"),
            LiteralValue::Float(s) => write!(f, "{s}"),
            LiteralValue::Bool(b) => write!(f, "{b}"),
        }
    }
}
