use crate::BamlMediaType;
use crate::Constraint;
use indexmap::IndexSet;
use itertools::Itertools;

mod builder;

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, Eq, Hash)]
pub enum TypeValue {
    String,
    Int,
    Float,
    Bool,
    // Char,
    Null,
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
            "null" => TypeValue::Null,
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
            TypeValue::Null => write!(f, "null"),
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

#[derive(serde::Serialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct UnionType {
    types: Vec<FieldType>,
}

pub enum UnionTypeView<'a> {
    Null, // Someone unioned null | null
    // A union type may never hold more than 1 null
    Optional(&'a FieldType),
    OneOf(Vec<&'a FieldType>),
    OneOfOptional(Vec<&'a FieldType>),
}

static NULL_TYPE: FieldType = FieldType::Primitive(TypeValue::Null);

impl<'a> UnionTypeView<'a> {
    fn flatten(&self) -> Vec<FieldType> {
        match self {
            UnionTypeView::Null => vec![NULL_TYPE.clone()],
            UnionTypeView::Optional(field_type) => field_type.flatten().into_iter().chain(
                vec![NULL_TYPE.clone()],
            ).collect(),
            UnionTypeView::OneOf(field_types) => field_types.iter().flat_map(|t| t.flatten()).collect(),
            UnionTypeView::OneOfOptional(field_types) => field_types.iter().flat_map(|t| t.flatten()).chain(
                vec![NULL_TYPE.clone()],
            ).collect(),
        }
    }
}

impl UnionType {
    // disallow construction so people have to use:
    // FieldType::union(vec![...]) which does a simplify() default
    pub(crate) fn new(types: Vec<FieldType>) -> Self {
        if types.len() <= 1 {
            panic!("FATAL, please report this bug: Union type must have at least 2 types. Got {:?}", types);
        }
        Self { types }
    }

    pub fn is_optional(&self) -> bool {
        self.types.iter().any(|t| t.is_optional())
    }

    pub fn add_type(&mut self, t: FieldType) {
        self.types.push(t);
    }

    pub fn view<'a>(&'a self) -> UnionTypeView<'a> {
        let non_null_types = self.types.iter().filter(|t| !t.is_null()).collect::<Vec<_>>();
        match non_null_types.len() {
            0 => UnionTypeView::Null,
            1 => UnionTypeView::Optional(&non_null_types[0]),
            _ => {
                if non_null_types.len() == self.types.len() {
                    UnionTypeView::OneOf(non_null_types)
                } else {
                    UnionTypeView::OneOfOptional(non_null_types)
                }
            }
        }
    }

    pub fn view_as_iter(&self, include_null: bool) -> (Vec<&FieldType>, bool) {
        match self.view() {
            UnionTypeView::Null => (if include_null { vec![&NULL_TYPE] } else { vec![] }, true),
            UnionTypeView::Optional(field_type) => if include_null {
                (vec![field_type, &NULL_TYPE], true)
            } else {
                (vec![field_type], true)
            },
            UnionTypeView::OneOf(items) => (items, false),
            UnionTypeView::OneOfOptional(items) => if include_null {
                (items.into_iter().chain(std::iter::once(&NULL_TYPE)).collect(), true)
            } else {
                (items, true)
            },
        }
    }
}


/// FieldType represents the type of either a class field or a function arg.
#[derive(serde::Serialize, Debug, Clone, PartialEq, Eq, Hash)]
pub enum FieldType {
    Primitive(TypeValue),
    Enum(String),
    Literal(LiteralValue),
    Class(String),
    List(Box<FieldType>),
    Map(Box<FieldType>, Box<FieldType>),
    RecursiveTypeAlias(String),
    Tuple(Vec<FieldType>),
    Arrow(Box<Arrow>),
    Union(UnionType),
    WithMetadata {
        base: Box<FieldType>,
        constraints: Vec<Constraint>,
        streaming_behavior: StreamingBehavior,
    },
}

pub trait HasFieldType {
    fn field_type<'a>(&'a self) -> &'a FieldType;
}

#[derive(serde::Serialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct Arrow {
    pub param_types: Vec<FieldType>,
    pub return_type: FieldType,
}

// Impl display for FieldType
impl std::fmt::Display for FieldType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FieldType::Enum(name)
            | FieldType::Class(name)
            | FieldType::RecursiveTypeAlias(name) => write!(f, "{name}"),
            FieldType::Primitive(t) => write!(f, "{t}"),
            FieldType::Literal(v) => write!(f, "{v}"),
            FieldType::Union(choices) => {
                let view = choices.view();
                let res = match view {
                    UnionTypeView::Null => "null".to_string(),
                    UnionTypeView::Optional(field_type) => format!("{}?", field_type.to_string()),
                    UnionTypeView::OneOf(field_types) => {
                        field_types
                            .iter()
                            .map(|t| t.to_string())
                            .collect::<Vec<_>>()
                            .join(" | ")
                    },
                    UnionTypeView::OneOfOptional(field_types) => {
                        let not_null_choices_str = field_types
                            .iter()
                            .map(|t| t.to_string())
                            .collect::<Vec<_>>()
                            .join(" | ");
                        format!("({})?", not_null_choices_str)
                    },
                };
                write!(f, "{res}")
            }
            FieldType::Tuple(choices) => {
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
            FieldType::Map(k, v) => write!(f, "map<{k}, {v}>"),
            FieldType::List(t) => write!(f, "{t}[]"),
            FieldType::Arrow(arrow) => write!(
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
            FieldType::WithMetadata { base, .. } => base.fmt(f),
        }
    }
}

impl FieldType {
    pub fn flatten(&self) -> Vec<FieldType> {
        match self {
            FieldType::Union(inner) => inner.view().flatten(),
            _ => vec![self.clone()],
        }
    }

    pub fn simplify(&self) -> FieldType {
        match self {
            FieldType::Union(inner) => {
                let flattened = inner.view().flatten();
                let unique = flattened.into_iter().unique().collect::<Vec<_>>();
                let has_null = unique.contains(&NULL_TYPE);
                // if the union contains null, we'll detect that here.
                let mut unique_without_null = unique
                    .into_iter()
                    .filter(|t| t != &NULL_TYPE)
                    .collect::<Vec<_>>();

                let simplified = match unique_without_null.len() {
                    0 => return FieldType::Primitive(TypeValue::Null),
                    1 => unique_without_null[0].clone(),
                    _ => {
                        if has_null {
                            unique_without_null.push(NULL_TYPE.clone());
                        }
                        FieldType::Union(UnionType::new(unique_without_null))
                    },
                };

                simplified
            }
            _ => self.clone(),
        }
    }

    pub fn is_primitive(&self) -> bool {
        match self {
            FieldType::Primitive(_) => true,
            FieldType::List(t) => t.is_primitive(),
            FieldType::WithMetadata { base, .. } => base.is_primitive(),
            _ => false,
        }
    }

    pub fn is_optional(&self) -> bool {
        match self {
            FieldType::Primitive(TypeValue::Null) => true,
            FieldType::Union(choices) => choices.is_optional(),
            FieldType::WithMetadata { base, .. } => base.is_optional(),
            _ => false,
        }
    }

    pub fn is_null(&self) -> bool {
        match self {
            FieldType::Primitive(TypeValue::Null) => true,
            FieldType::WithMetadata { base, .. } => base.is_null(),
            _ => false,
        }
    }

    pub fn streaming_behavior(&self) -> Option<&StreamingBehavior> {
        match self {
            FieldType::WithMetadata {
                streaming_behavior, ..
            } => Some(streaming_behavior),
            _ => None,
        }
    }
}

pub trait ToUnionName {
    fn to_union_name(&self) -> String;
    fn find_union_types(&self) -> IndexSet<FieldType>;
}

impl ToUnionName for FieldType {
    fn find_union_types(&self) -> IndexSet<FieldType> {
        // TODO: its pretty hard to get type aliases here
        let value = self.simplify();
        match &value {
            FieldType::Union(_) => IndexSet::from_iter([value]),
            FieldType::List(inner) => inner.find_union_types(),
            FieldType::Map(field_type, field_type1) => {
                let mut set = field_type.find_union_types();
                set.extend(field_type1.find_union_types());
                set
            }
            FieldType::Primitive(_)
            | FieldType::Enum(_)
            | FieldType::Literal(_)
            | FieldType::Class(_)
            | FieldType::RecursiveTypeAlias(_)
            | FieldType::Arrow(_) => IndexSet::new(),
            FieldType::Tuple(inner) => inner.iter().flat_map(|t| t.find_union_types()).collect(),
            FieldType::WithMetadata { base, .. } => base.find_union_types(),
        }
    }

    fn to_union_name(&self) -> String {
        match self {
            FieldType::Primitive(type_value) => type_value.to_string(),
            FieldType::Enum(name) => name.to_string(),
            FieldType::Literal(literal_value) => match literal_value {
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
            FieldType::Class(name) => name.to_string(),
            FieldType::List(field_type) => {
                format!("List__{}", field_type.to_union_name())
            }
            FieldType::Map(field_type, field_type1) => {
                format!(
                    "Map__{}_{}",
                    field_type.to_union_name(),
                    field_type1.to_union_name()
                )
            }
            FieldType::Union(field_types) => {
                match field_types.view() {
                    UnionTypeView::Null => "null".to_string(),
                    UnionTypeView::Optional(field_type) => {
                        field_type.to_union_name()
                    }
                    UnionTypeView::OneOf(field_types) | UnionTypeView::OneOfOptional(field_types) => {
                        format!("Union__{}", field_types.iter().map(|t| t.to_union_name()).collect::<Vec<_>>().join("__"))
                    }
                    
                }
            },
            FieldType::Tuple(field_types) => format!(
                "Tuple__{}",
                field_types
                    .iter()
                    .map(|v| v.to_union_name())
                    .collect::<Vec<_>>()
                    .join("__")
                    .to_string()
            ),
            FieldType::RecursiveTypeAlias(name) => name.to_string(),
            FieldType::WithMetadata { base, .. } => base.to_union_name(),
            FieldType::Arrow(_) => "function".to_string(),
        }
    }
}

/// Metadata on a type that determines how it behaves under streaming conditions.
#[derive(Clone, Debug, PartialEq, serde::Serialize, Eq, Hash)]
pub struct StreamingBehavior {
    /// A type with the `done` property will not be visible in a stream until
    /// we are certain that it is completely available (i.e. the parser did
    /// not finalize it through any early termination, enough tokens were available
    /// from the LLM response to be certain that it is done).
    pub done: bool,

    /// A type with the `state` property will be represented in client code as
    /// a struct: `{value: T, streaming_state: "incomplete" | "complete"}`.
    pub state: bool,
}

impl StreamingBehavior {
    pub fn combine(&self, other: &StreamingBehavior) -> StreamingBehavior {
        StreamingBehavior {
            done: self.done || other.done,
            state: self.state || other.state,
        }
    }
}

impl Default for StreamingBehavior {
    fn default() -> Self {
        StreamingBehavior {
            done: false,
            state: false,
        }
    }
}

#[cfg(test)]
mod tests {}
