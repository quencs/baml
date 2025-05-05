use crate::BamlMediaType;
use baml_derive::BamlHash;
use indexmap::IndexSet;
use itertools::any;
use itertools::Itertools;
use std::collections::HashSet;

mod builder;

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, Eq, Hash)]
pub enum TypeValue {
    String,
    Int,
    Float,
    Bool,
    // Char,
    Media(BamlMediaType),
}

impl std::str::FromStr for TypeValue {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "string" => TypeValue::String,
            "int" => TypeValue::Int,
            "float" => TypeValue::Float,
            "bool" => TypeValue::Bool,
            "image" => TypeValue::Media(BamlMediaType::Image),
            "audio" => TypeValue::Media(BamlMediaType::Audio),
            _ => return Err(()),
        })
    }
}

impl std::fmt::Display for TypeValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TypeValue::String => write!(f, "string"),
            TypeValue::Int => write!(f, "int"),
            TypeValue::Float => write!(f, "float"),
            TypeValue::Bool => write!(f, "bool"),
            TypeValue::Media(BamlMediaType::Image) => write!(f, "image"),
            TypeValue::Media(BamlMediaType::Audio) => write!(f, "audio"),
        }
    }
}

/// Subset of [`crate::BamlValue`] allowed for literal type definitions.
#[derive(serde::Serialize, Debug, Clone, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub enum LiteralValue {
    String(String),
    Int(i64),
    Bool(bool),
}

impl LiteralValue {
    pub fn literal_base_type(&self) -> FieldType {
        match self {
            Self::String(_) => FieldType::string(),
            Self::Int(_) => FieldType::int(),
            Self::Bool(_) => FieldType::bool(),
        }
    }
}

impl std::fmt::Display for LiteralValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LiteralValue::String(str) => write!(f, "\"{str}\""),
            LiteralValue::Int(int) => write!(f, "{int}"),
            LiteralValue::Bool(bool) => write!(f, "{bool}"),
        }
    }
}

pub trait Mergeable {
    fn merge(&self, other: &Self) -> Self;
}

pub trait Metadata:
    Clone + Eq + PartialEq + std::hash::Hash + Default + TypeValidator<anyhow::Error> + Mergeable
{
}

pub trait TypeValidator<E> {
    fn validate_string(&self, value: &str) -> Result<bool, E>;
    fn validate_int(&self, value: &i64) -> Result<bool, E>;
    fn validate_float(&self, value: &f64) -> Result<bool, E>;
    fn validate_bool(&self, value: &bool) -> Result<bool, E>;
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, Eq, Hash, Default)]
pub struct TypeMetadata {
    pub streaming_behavior: StreamingBehavior,
    pub constraints: Vec<crate::Constraint>,
}

impl Mergeable for TypeMetadata {
    fn merge(&self, other: &Self) -> Self {
        Self {
            streaming_behavior: self.streaming_behavior.merge(&other.streaming_behavior),
            constraints: self
                .constraints
                .iter()
                .chain(other.constraints.iter())
                .cloned()
                .collect(),
        }
    }
}

impl Metadata for TypeMetadata {}

impl TypeMetadata {
    fn validate_value<T: ?Sized + serde::Serialize>(
        &self,
        value: &T,
    ) -> Result<bool, anyhow::Error> {
        let results = self
            .constraints
            .iter()
            .filter(|c| c.level == crate::ConstraintLevel::Assert)
            .map(|c| c.expression.evaluate(value).map(|v| v.is_true()));

        // reduce the results to a single result
        let result = results.reduce(|acc, result| match (acc, result) {
            (Err(e), _) => Err(e),
            (_, Err(e)) => Err(e),
            (Ok(false), _) => Ok(false),
            (_, Ok(false)) => Ok(false),
            (Ok(true), Ok(true)) => Ok(true),
        });

        result.unwrap_or(Ok(true))
    }
}

impl TypeValidator<anyhow::Error> for TypeMetadata {
    fn validate_string(&self, value: &str) -> Result<bool, anyhow::Error> {
        self.validate_value(value)
    }

    fn validate_int(&self, value: &i64) -> Result<bool, anyhow::Error> {
        self.validate_value(value)
    }

    fn validate_float(&self, value: &f64) -> Result<bool, anyhow::Error> {
        self.validate_value(value)
    }

    fn validate_bool(&self, value: &bool) -> Result<bool, anyhow::Error> {
        self.validate_value(value)
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct UserDefinedObject {
    dynamic: bool,
}

impl Default for UserDefinedObject {
    fn default() -> Self {
        Self { dynamic: false }
    }
}

/// FieldType represents the type of either a class field or a function arg.
#[derive(BamlHash, Debug)]
pub enum FieldType {
    Null(TypeMetadata),
    Primitive(TypeValue, TypeMetadata),
    Literal(LiteralValue, TypeMetadata),
    Class(String, UserDefinedObject, TypeMetadata),
    Enum(String, UserDefinedObject, TypeMetadata),
    List(Box<FieldType>, TypeMetadata),
    Map(Box<FieldType>, Box<FieldType>, TypeMetadata),
    Union(Vec<FieldType>, TypeMetadata),
    Tuple(Vec<FieldType>, TypeMetadata),
    RecursiveTypeAlias(String, TypeMetadata),
    Arrow(Box<Arrow>, TypeMetadata),
}

fn validate_union<T: ?Sized, F, E>(
    options: &[FieldType],
    value: &T,
    validator: F,
) -> Result<bool, E>
where
    F: Fn(&FieldType, &T) -> Result<bool, E>,
{
    let options = options.iter().map(|t| validator(t, &value));
    // reduce the options to a single result
    let value = options.reduce(|acc, option| match (acc, option) {
        (Ok(true), Ok(_)) => Ok(true),
        (Ok(_), Ok(true)) => Ok(true),
        (Ok(false), Ok(_)) => Ok(false),
        (Err(e), _) => Err(e),
        (_, Err(e)) => Err(e),
    });

    value.unwrap_or(Ok(false))
}

impl TypeValidator<anyhow::Error> for FieldType {
    fn validate_string(&self, value: &str) -> Result<bool, anyhow::Error> {
        match self {
            FieldType::Primitive(TypeValue::String, m) => m.validate_string(value),
            FieldType::Literal(LiteralValue::String(lit), m) => {
                if value == lit {
                    m.validate_string(value)
                } else {
                    Ok(false)
                }
            }
            FieldType::Union(options, m) => {
                validate_union(options, value, |t, v| match t.validate_string(v) {
                    Ok(true) => m.validate_string(value),
                    Ok(false) => Ok(false),
                    Err(e) => Err(e),
                })
            }
            _ => Ok(false),
        }
    }

    fn validate_int(&self, value: &i64) -> Result<bool, anyhow::Error> {
        match self {
            FieldType::Primitive(TypeValue::Int, m) => m.validate_int(value),
            FieldType::Literal(LiteralValue::Int(lit), m) => {
                if *value == *lit {
                    m.validate_int(value)
                } else {
                    Ok(false)
                }
            }
            FieldType::Union(options, m) => {
                validate_union(options, value, |t, v| match t.validate_int(v) {
                    Ok(true) => m.validate_int(value),
                    Ok(false) => Ok(false),
                    Err(e) => Err(e),
                })
            }
            _ => Ok(false),
        }
    }

    fn validate_float(&self, value: &f64) -> Result<bool, anyhow::Error> {
        match self {
            FieldType::Primitive(TypeValue::Float, m) => m.validate_float(value),
            FieldType::Union(options, m) => {
                validate_union(options, value, |t, v| match t.validate_float(v) {
                    Ok(true) => m.validate_float(value),
                    Ok(false) => Ok(false),
                    Err(e) => Err(e),
                })
            }
            _ => Ok(false),
        }
    }

    fn validate_bool(&self, value: &bool) -> Result<bool, anyhow::Error> {
        match self {
            FieldType::Primitive(TypeValue::Bool, m) => m.validate_bool(value),
            FieldType::Literal(LiteralValue::Bool(lit), m) => {
                if *value == *lit {
                    m.validate_bool(value)
                } else {
                    Ok(false)
                }
            }
            FieldType::Union(options, m) => {
                validate_union(options, value, |t, v| match t.validate_bool(v) {
                    Ok(true) => m.validate_bool(value),
                    Ok(false) => Ok(false),
                    Err(e) => Err(e),
                })
            }
            _ => Ok(false),
        }
    }
}

pub trait HasFieldType {
    fn field_type<'a>(&'a self) -> &'a FieldType;
}

impl HasFieldType for FieldType {
    fn field_type<'a>(&'a self) -> &'a FieldType {
        self
    }
}

#[derive(BamlHash, Debug)]
pub struct Arrow {
    pub param_types: Vec<FieldType>,
    pub return_type: FieldType,
}

// Impl display for FieldType
impl std::fmt::Display for FieldType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FieldType::Null(_) => write!(f, "null"),
            FieldType::Enum(name, ..)
            | FieldType::Class(name, ..)
            | FieldType::RecursiveTypeAlias(name, _) => write!(f, "{name}"),
            FieldType::Primitive(t, _) => write!(f, "{t}"),
            FieldType::Literal(v, _) => write!(f, "{v}"),
            FieldType::Union(choices, _) => {
                write!(
                    f,
                    "({})",
                    choices
                        .iter()
                        .map(|t| t.to_string())
                        .collect::<Vec<_>>()
                        .join(" | ")
                )
            }
            FieldType::Tuple(choices, _) => {
                write!(
                    f,
                    "({})",
                    choices
                        .iter()
                        .map(|t| t.to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
            FieldType::Map(k, v, _) => write!(f, "map<{k}, {v}>"),
            FieldType::List(t, _) => write!(f, "{t}[]"),
            FieldType::Arrow(arrow, _) => write!(
                f,
                "({}) -> {}",
                arrow
                    .param_types
                    .iter()
                    .map(|t| t.to_string())
                    .collect::<Vec<_>>()
                    .join(", "),
                arrow.return_type.to_string()
            ),
        }
    }
}

impl PartialEq for FieldType {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Primitive(l0, l1), Self::Primitive(r0, r1)) => l0 == r0 && l1 == r1,
            (Self::Literal(l0, l1), Self::Literal(r0, r1)) => l0 == r0 && l1 == r1,
            (Self::Class(l0, l1, l2), Self::Class(r0, r1, r2)) => l0 == r0 && l1 == r1 && l2 == r2,
            (Self::Enum(l0, l1, l2), Self::Enum(r0, r1, r2)) => l0 == r0 && l1 == r1 && l2 == r2,
            (Self::List(l0, l1), Self::List(r0, r1)) => l0 == r0 && l1 == r1,
            (Self::Map(l0, l1, l2), Self::Map(r0, r1, r2)) => l0 == r0 && l1 == r1 && l2 == r2,
            (Self::Union(l0, l1), Self::Union(r0, r1)) => l0 == r0 && l1 == r1,
            (Self::Tuple(l0, l1), Self::Tuple(r0, r1)) => l0 == r0 && l1 == r1,
            (Self::RecursiveTypeAlias(l0, l1), Self::RecursiveTypeAlias(r0, r1)) => {
                l0 == r0 && l1 == r1
            }
            (Self::Arrow(l0, l1), Self::Arrow(r0, r1)) => l0 == r0 && l1 == r1,
            _ => false,
        }
    }
}
impl Eq for FieldType {}

impl Clone for FieldType {
    fn clone(&self) -> Self {
        match self {
            FieldType::Null(m) => FieldType::Null(m.clone()),
            FieldType::Union(inner, m) => FieldType::Union(inner.clone(), m.clone()),
            FieldType::Primitive(type_value, m) => {
                FieldType::Primitive(type_value.clone(), m.clone())
            }
            FieldType::Literal(literal_value, m) => {
                FieldType::Literal(literal_value.clone(), m.clone())
            }
            FieldType::Class(name, udp, m) => {
                FieldType::Class(name.clone(), udp.clone(), m.clone())
            }
            FieldType::Enum(name, udp, m) => FieldType::Enum(name.clone(), udp.clone(), m.clone()),
            FieldType::List(field_type, m) => FieldType::List(field_type.clone(), m.clone()),
            FieldType::Map(field_type, field_type1, m) => {
                FieldType::Map(field_type.clone(), field_type1.clone(), m.clone())
            }
            FieldType::Tuple(field_types, m) => FieldType::Tuple(field_types.clone(), m.clone()),
            FieldType::RecursiveTypeAlias(name, m) => {
                FieldType::RecursiveTypeAlias(name.clone(), m.clone())
            }
            FieldType::Arrow(arrow, m) => FieldType::Arrow(arrow.clone(), m.clone()),
        }
    }
}

impl Clone for Arrow {
    fn clone(&self) -> Self {
        Arrow {
            param_types: self.param_types.clone(),
            return_type: self.return_type.clone(),
        }
    }
}

impl PartialEq for Arrow {
    fn eq(&self, other: &Self) -> bool {
        self.param_types == other.param_types && self.return_type == other.return_type
    }
}
impl Eq for Arrow {}

impl FieldType {
    fn merge_metadata(&self, new_m: &TypeMetadata) -> FieldType {
        match self {
            FieldType::Null(m) => FieldType::Null(m.merge(new_m)),
            FieldType::Union(inner, m) => FieldType::Union(inner.clone(), m.merge(new_m)),
            FieldType::Primitive(type_value, m) => {
                FieldType::Primitive(type_value.clone(), m.merge(new_m))
            }
            FieldType::Literal(literal_value, m) => {
                FieldType::Literal(literal_value.clone(), m.merge(new_m))
            }
            FieldType::Class(name, udp, m) => {
                FieldType::Class(name.clone(), udp.clone(), m.merge(new_m))
            }
            FieldType::Enum(name, udp, m) => {
                FieldType::Enum(name.clone(), udp.clone(), m.merge(new_m))
            }
            FieldType::List(field_type, m) => FieldType::List(field_type.clone(), m.merge(new_m)),
            FieldType::Map(field_type, field_type1, m) => {
                FieldType::Map(field_type.clone(), field_type1.clone(), m.merge(new_m))
            }
            FieldType::Tuple(field_types, m) => {
                FieldType::Tuple(field_types.clone(), m.merge(new_m))
            }
            FieldType::RecursiveTypeAlias(name, m) => {
                FieldType::RecursiveTypeAlias(name.clone(), m.merge(new_m))
            }
            FieldType::Arrow(arrow, m) => FieldType::Arrow(arrow.clone(), m.merge(new_m)),
        }
    }

    fn flatten(&self) -> Vec<FieldType> {
        match self {
            FieldType::Union(inner, m) => inner
                .iter()
                .flat_map(|t| t.flatten())
                .into_iter()
                .map(|f| f.merge_metadata(m))
                .collect(),
            _ => vec![self.clone()],
        }
    }

    pub fn simplify(&self) -> FieldType {
        match self {
            FieldType::Union(inner, m) => {
                let flattened = inner.iter().flat_map(|t| t.flatten()).collect::<Vec<_>>();
                let unique = flattened.into_iter().unique().collect::<Vec<_>>();
                let has_null = any(unique.iter(), |f| matches!(f, FieldType::Null(_)));
                // if the union contains null, we'll detect that here.
                let unique_without_null = unique
                    .into_iter()
                    .filter(|t| t.is_null())
                    .collect::<Vec<_>>();

                let simplified = match unique_without_null.len() {
                    0 => return FieldType::Null(m.clone()),
                    1 => unique_without_null[0].merge_metadata(m),
                    _ => FieldType::Union(unique_without_null, m.clone()),
                };

                if has_null {
                    FieldType::Union(vec![simplified, FieldType::Null(m.clone())], m.clone())
                } else {
                    simplified
                }
            }
            _ => self.clone(),
        }
    }

    pub fn is_primitive(&self) -> bool {
        match self {
            FieldType::Primitive(..) => true,
            FieldType::List(t, ..) => t.is_primitive(),
            _ => false,
        }
    }

    pub fn is_optional(&self) -> bool {
        match self {
            FieldType::Null(_) => true,
            FieldType::Union(types, ..) => types.iter().any(FieldType::is_optional),
            _ => false,
        }
    }

    pub fn is_null(&self) -> bool {
        match self {
            FieldType::Null(_) => true,
            _ => false,
        }
    }

    pub fn dependencies(&self) -> HashSet<String> {
        let mut deps = HashSet::new();
        let mut queue = vec![self];
        let mut visited = HashSet::new();
        while let Some(current) = queue.pop() {
            if visited.contains(current) {
                continue;
            }
            visited.insert(current);
            match current {
                FieldType::Class(name, ..)
                | FieldType::Enum(name, ..)
                | FieldType::RecursiveTypeAlias(name, ..) => {
                    deps.insert(name.clone());
                }
                FieldType::List(inner, ..) => {
                    queue.push(inner);
                }
                FieldType::Map(field_type, field_type1, ..) => {
                    queue.push(field_type);
                    queue.push(field_type1);
                }
                FieldType::Union(inner, ..) => {
                    queue.extend(inner.iter());
                }
                FieldType::Tuple(inner, ..) => {
                    queue.extend(inner.iter());
                }
                FieldType::Arrow(arrow, ..) => {
                    queue.extend(arrow.param_types.iter());
                    queue.push(&arrow.return_type);
                }
                FieldType::Primitive(..) | FieldType::Literal(..) | FieldType::Null(..) => {}
            }
        }
        deps
    }
}
pub trait ToUnionName {
    fn to_union_name(&self) -> String;
    fn find_union_types(&self) -> IndexSet<FieldType>;
}

impl ToUnionName for FieldType {
    // Discover all unions
    fn find_union_types(&self) -> IndexSet<FieldType> {
        // TODO: its pretty hard to get type aliases here
        let value = self.simplify();
        match &value {
            FieldType::Null(..) => IndexSet::new(),
            FieldType::Union(..) => IndexSet::from_iter([value]),
            FieldType::List(inner, ..) => inner.find_union_types(),
            FieldType::Map(field_type, field_type1, ..) => {
                let mut set = field_type.find_union_types();
                set.extend(field_type1.find_union_types());
                set
            }
            FieldType::Primitive(..)
            | FieldType::Enum(..)
            | FieldType::Literal(..)
            | FieldType::Class(..)
            | FieldType::RecursiveTypeAlias(..)
            | FieldType::Arrow(..) => IndexSet::new(),
            FieldType::Tuple(inner, ..) => {
                inner.iter().flat_map(|t| t.find_union_types()).collect()
            }
        }
    }

    fn to_union_name(&self) -> String {
        match self {
            FieldType::Null(..) => "null".to_string(),
            FieldType::Primitive(type_value, ..) => type_value.to_string(),
            FieldType::Enum(name, ..) => name.to_string(),
            FieldType::Literal(literal_value, ..) => match literal_value {
                LiteralValue::String(value) => format!(
                    "string_{}",
                    value
                        .chars()
                        .map(|c| if c.is_alphanumeric() { c } else { '_' })
                        .collect::<String>()
                ),
                LiteralValue::Int(val) => format!("int_{}", val.to_string()),
                LiteralValue::Bool(val) => format!("bool_{}", val.to_string()),
            },
            FieldType::Class(name, ..) => name.to_string(),
            FieldType::List(field_type, ..) => {
                format!("List__{}", field_type.to_union_name())
            }
            FieldType::Map(field_type, field_type1, ..) => {
                format!(
                    "Map__{}_{}",
                    field_type.to_union_name(),
                    field_type1.to_union_name()
                )
            }
            FieldType::Union(field_types, ..) => format!(
                "Union__{}",
                field_types
                    .iter()
                    .map(|v| v.to_union_name())
                    .collect::<Vec<_>>()
                    .join("__")
                    .to_string()
            ),
            FieldType::Tuple(field_types, ..) => format!(
                "Tuple__{}",
                field_types
                    .iter()
                    .map(|v| v.to_union_name())
                    .collect::<Vec<_>>()
                    .join("__")
                    .to_string()
            ),
            FieldType::RecursiveTypeAlias(name, ..) => name.to_string(),
            FieldType::Arrow(..) => "function".to_string(),
        }
    }
}

/// Metadata on a type that determines how it behaves under streaming conditions.
#[derive(Clone, Debug, PartialEq, serde::Serialize, Eq, Hash)]
pub struct StreamingBehavior {
    /// A type with the `not_null` property will not be visible in a stream until
    /// we are certain that it is not null (as in the value has at least begun)
    pub needed: bool,

    /// A type with the `done` property will not be visible in a stream until
    /// we are certain that it is completely available (i.e. the parser did
    /// not finalize it through any early termination, enough tokens were available
    /// from the LLM response to be certain that it is done).
    pub done: bool,

    /// A type with the `state` property will be represented in client code as
    /// a struct: `{value: T, streaming_state: "incomplete" | "complete"}`.
    pub state: bool,
}

impl Mergeable for StreamingBehavior {
    fn merge(&self, other: &Self) -> Self {
        Self {
            done: self.done || other.done,
            state: self.state || other.state,
            needed: self.needed || other.needed,
        }
    }
}

impl StreamingBehavior {
    pub fn combine(&self, other: &StreamingBehavior) -> StreamingBehavior {
        StreamingBehavior {
            done: self.done || other.done,
            state: self.state || other.state,
            needed: self.needed || other.needed,
        }
    }
}

impl Default for StreamingBehavior {
    fn default() -> Self {
        StreamingBehavior {
            done: false,
            state: false,
            needed: false,
        }
    }
}

#[cfg(test)]
mod tests {}
