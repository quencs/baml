use crate::BamlMediaType;
use crate::Constraint;
use crate::ConstraintLevel;
use indexmap::IndexSet;
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

/// A union type may never hold more than 1 null
/// A view into a union type that classifies its variants
#[derive(Debug)]
pub enum UnionTypeView<'a> {
    /// A union containing only the null type
    Null,
    /// A union containing exactly one non-null type and the null type
    Optional(&'a FieldType),
    /// A union containing multiple non-null types with no optional variants
    OneOf(Vec<&'a FieldType>),
    /// A union containing multiple types where at least one is optional
    OneOfOptional(Vec<&'a FieldType>),
}

pub static NULL_TYPE: once_cell::sync::Lazy<FieldType> =
    once_cell::sync::Lazy::new(|| FieldType::Primitive(TypeValue::Null, TypeMetadataIR::default()));

impl<'a> UnionTypeView<'a> {
    /// A helper-function for the `FieldType::flatten`.
    /// See `FieldType::flatten` for context.
    fn flatten(&self) -> Vec<FieldType> {
        match self {
            UnionTypeView::Null => vec![(*NULL_TYPE).clone()],
            UnionTypeView::Optional(field_type) => field_type
                .flatten()
                .into_iter()
                .chain(std::iter::once((*NULL_TYPE).clone()))
                .collect(),
            UnionTypeView::OneOf(field_types) => {
                field_types.iter().flat_map(|t| t.flatten()).collect()
            }
            UnionTypeView::OneOfOptional(field_types) => field_types
                .iter()
                .flat_map(|t| t.flatten())
                .chain(std::iter::once((*NULL_TYPE).clone()))
                .collect(),
        }
    }
}

impl UnionType {
    // disallow construction so people have to use:
    // FieldType::union(vec![...]) which does a simplify() default
    pub(crate) fn new(types: Vec<FieldType>) -> Self {
        if types.len() <= 1 {
            panic!(
                "FATAL, please report this bug: Union type must have at least 2 types. Got {:?}",
                types
            );
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
        let non_null_types = self
            .types
            .iter()
            .filter(|t| !t.is_null())
            .collect::<Vec<_>>();
        println!(
            "non_null_types_len: {}, self.types.len: {}",
            non_null_types.len(),
            self.types.len(),
        );
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

    // Hello
    pub fn view_as_iter(&self, include_null: bool) -> (Vec<&FieldType>, bool) {
        match self.view() {
            UnionTypeView::Null => (
                if include_null {
                    vec![&NULL_TYPE]
                } else {
                    vec![]
                },
                true,
            ),
            UnionTypeView::Optional(field_type) => {
                if include_null {
                    (vec![field_type, &NULL_TYPE], true)
                } else {
                    (vec![field_type], true)
                }
            }
            UnionTypeView::OneOf(items) => (items, false),
            UnionTypeView::OneOfOptional(items) => {
                if include_null {
                    (
                        items
                            .into_iter()
                            .chain(std::iter::once(&(*NULL_TYPE)))
                            .collect(),
                        true,
                    )
                } else {
                    (items, true)
                }
            }
        }
    }
}

/// FieldType represents the type of either a class field or a function arg.
#[derive(serde::Serialize, Debug, Clone, PartialEq, Eq, Hash)]
pub enum FieldType {
    Primitive(TypeValue, TypeMetadataIR),
    Enum(String, TypeMetadataIR),
    Literal(LiteralValue, TypeMetadataIR),
    Class(String, TypeMetadataIR),
    List(Box<FieldType>, TypeMetadataIR),
    Map(Box<FieldType>, Box<FieldType>, TypeMetadataIR),
    RecursiveTypeAlias(String, TypeMetadataIR),
    Tuple(Vec<FieldType>, TypeMetadataIR),
    Arrow(Box<Arrow>, TypeMetadataIR),
    Union(UnionType, TypeMetadataIR),
}

#[derive(serde::Serialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct TypeMetadataIR {
    pub constraints: Vec<Constraint>,
    pub streaming_behavior: StreamingBehavior,
}

impl Default for TypeMetadataIR {
    fn default() -> Self {
        TypeMetadataIR {
            constraints: Vec::new(),
            streaming_behavior: StreamingBehavior::default(),
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

#[derive(serde::Serialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct Arrow {
    pub param_types: Vec<FieldType>,
    pub return_type: FieldType,
}

impl std::fmt::Display for FieldType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut metadata_display_fmt = String::new();

        for constraint in &self.meta().constraints {
            // " @check( the_name, {{..}} )"
            let constraint_level = match constraint.level {
                ConstraintLevel::Assert => "assert",
                ConstraintLevel::Check => "check",
            };
            let constraint_name = match &constraint.label {
                None => "".to_string(),
                Some(label) => format!("{}, ", label),
            };
            metadata_display_fmt.push_str(&format!(
                " @{constraint_level}({constraint_name}, {{{{..}}}} )"
            ));
        }
        let StreamingBehavior {
            done,
            needed,
            state,
        } = self.meta().streaming_behavior;
        if done {
            metadata_display_fmt.push_str(" @stream.done")
        }
        if needed {
            metadata_display_fmt.push_str(" @stream.not_null")
        }
        if state {
            metadata_display_fmt.push_str(" @stream.with_state")
        }

        let _res = match self {
            FieldType::Enum(name, _)
            | FieldType::Class(name, _)
            | FieldType::RecursiveTypeAlias(name, _) => write!(f, "{name}"),
            FieldType::Primitive(t, _) => write!(f, "{t}"),
            FieldType::Literal(v, _) => write!(f, "{v}"),
            FieldType::Union(choices, _) => {
                let view = choices.view();
                let res = match view {
                    UnionTypeView::Null => "null".to_string(),
                    UnionTypeView::Optional(field_type) => format!("{}?", field_type.to_string()),
                    UnionTypeView::OneOf(field_types) => field_types
                        .iter()
                        .map(|t| t.to_string())
                        .collect::<Vec<_>>()
                        .join(" | "),
                    UnionTypeView::OneOfOptional(field_types) => {
                        let not_null_choices_str = field_types
                            .iter()
                            .map(|t| t.to_string())
                            .collect::<Vec<_>>()
                            .join(" | ");
                        format!("({})?", not_null_choices_str)
                    }
                };
                write!(f, "{res}")
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
        }?;

        write!(f, "{}", metadata_display_fmt)
    }
}

impl FieldType {
    /// Consolidate all `Null` types appear in a (potentially deeply) nested
    /// Union, and remove the tree structure of nested unions.
    ///
    /// e.g. (( ((int | null) | int) | (map<string,string> | null ))) =>
    ///         int | int | map<string,string> | null
    pub fn flatten(&self) -> Vec<FieldType> {
        match self {
            FieldType::Union(inner, _) => inner.view().flatten(),
            _ => vec![self.clone()],
        }
    }

    pub fn set_meta(&mut self, meta: TypeMetadataIR) {
        match self {
            FieldType::Class(_, type_metadata_ir) => *type_metadata_ir = meta,
            FieldType::Arrow(_, type_metadata_ir) => *type_metadata_ir = meta,
            FieldType::Primitive(_, type_metadata_ir) => *type_metadata_ir = meta,
            FieldType::Enum(_, type_metadata_ir) => *type_metadata_ir = meta,
            FieldType::Literal(_, type_metadata_ir) => *type_metadata_ir = meta,
            FieldType::List(_, type_metadata_ir) => *type_metadata_ir = meta,
            FieldType::Map(_, _, type_metadata_ir) => *type_metadata_ir = meta,
            FieldType::RecursiveTypeAlias(_, type_metadata_ir) => *type_metadata_ir = meta,
            FieldType::Tuple(_, type_metadata_ir) => *type_metadata_ir = meta,
            FieldType::Union(_, type_metadata_ir) => *type_metadata_ir = meta,
        }
    }

    pub fn meta(&self) -> &TypeMetadataIR {
        match self {
            FieldType::Class(_, type_metadata_ir) => type_metadata_ir,
            FieldType::Arrow(_, type_metadata_ir) => type_metadata_ir,
            FieldType::Primitive(_, type_metadata_ir) => type_metadata_ir,
            FieldType::Enum(_, type_metadata_ir) => type_metadata_ir,
            FieldType::Literal(_, type_metadata_ir) => type_metadata_ir,
            FieldType::List(_, type_metadata_ir) => type_metadata_ir,
            FieldType::Map(_, _, type_metadata_ir) => type_metadata_ir,
            FieldType::RecursiveTypeAlias(_, type_metadata_ir) => type_metadata_ir,
            FieldType::Tuple(_, type_metadata_ir) => type_metadata_ir,
            FieldType::Union(_, type_metadata_ir) => type_metadata_ir,
        }
    }

    /// For types that are unions, replace the variants list with
    /// a simplified (flattened) variants list.
    pub fn simplify(&self) -> FieldType {
        println!("[self] {self:?}");
        match self {
            FieldType::Union(inner, _) => {
                let view = inner.view();
                println!("[view] {view:?}");
                let flattened = view.flatten();
                println!("[flattened] {flattened:?}");
                let unique = flattened.into_iter().unique().collect::<Vec<_>>();
                let has_null = unique.contains(&NULL_TYPE);
                // if the union contains null, we'll detect that here.
                let mut variants = unique
                    .into_iter()
                    .filter(|t| t != &(*NULL_TYPE))
                    .collect::<Vec<_>>();

                let simplified = match variants.len() {
                    0 => return FieldType::null(),
                    1 => {
                        if has_null {
                            // Return an optional of a single variant.
                            FieldType::Union(
                                UnionType {
                                    types: vec![variants[0].clone(), (*NULL_TYPE).clone()],
                                },
                                TypeMetadataIR::default(),
                            )
                        } else {
                            // Return the single variant.
                            variants[0].clone()
                        }
                    }
                    _ => {
                        if has_null {
                            variants.push((*NULL_TYPE).clone());
                        }
                        FieldType::Union(UnionType { types: variants }, TypeMetadataIR::default())
                    }
                };

                simplified
            }
            _ => self.clone(),
        }
    }

    pub fn is_primitive(&self) -> bool {
        match self {
            FieldType::Primitive(_, _) => true,
            FieldType::List(t, _) => t.is_primitive(),
            _ => false,
        }
    }

    pub fn is_optional(&self) -> bool {
        match self {
            FieldType::Primitive(TypeValue::Null, _) => true,
            FieldType::Union(choices, _) => choices.is_optional(),
            _ => false,
        }
    }

    pub fn is_null(&self) -> bool {
        match self {
            FieldType::Primitive(TypeValue::Null, _) => true,
            _ => false,
        }
    }

    // TODO: remove Optionality, so that callers don't need to worry about it.
    pub fn streaming_behavior(&self) -> Option<&StreamingBehavior> {
        Some(&self.meta().streaming_behavior)
    }

    /// The immediate (non transitive) dependencies of a given type?
    pub fn dependencies(&self) -> HashSet<String> {
        let mut deps = HashSet::new();
        let mut queue = vec![self];
        while let Some(current) = queue.pop() {
            match current {
                FieldType::Class(name, _) => {
                    deps.insert(name.clone());
                }
                FieldType::Enum(name, _) => {
                    deps.insert(name.clone());
                }
                FieldType::List(inner, _) => {
                    queue.push(inner);
                }
                FieldType::Map(field_type, field_type1, _) => {
                    queue.push(field_type);
                    queue.push(field_type1);
                }
                FieldType::Union(inner, _) => match inner.view() {
                    UnionTypeView::Null => {}
                    UnionTypeView::Optional(field_type) => queue.push(field_type),
                    UnionTypeView::OneOf(field_types) => queue.extend(field_types.into_iter()),
                    UnionTypeView::OneOfOptional(field_types) => {
                        queue.extend(field_types.into_iter())
                    }
                },
                FieldType::Tuple(inner, _) => {
                    queue.extend(inner.iter());
                }
                FieldType::Arrow(arrow, _) => {
                    queue.extend(arrow.param_types.iter());
                    queue.push(&arrow.return_type);
                }
                FieldType::RecursiveTypeAlias(name, _) => {
                    deps.insert(name.clone());
                }
                FieldType::Primitive(_, _) | FieldType::Literal(_, _) => {}
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
    fn find_union_types(&self) -> IndexSet<FieldType> {
        // TODO: its pretty hard to get type aliases here
        let value = self.simplify();
        match &value {
            FieldType::Union(_, _) => IndexSet::from_iter([value]),
            FieldType::List(inner, _) => inner.find_union_types(),
            FieldType::Map(field_type, field_type1, _) => {
                let mut set = field_type.find_union_types();
                set.extend(field_type1.find_union_types());
                set
            }
            FieldType::Primitive(_, _)
            | FieldType::Enum(_, _)
            | FieldType::Literal(_, _)
            | FieldType::Class(_, _)
            | FieldType::RecursiveTypeAlias(_, _)
            | FieldType::Arrow(_, _) => IndexSet::new(),
            FieldType::Tuple(inner, _) => inner.iter().flat_map(|t| t.find_union_types()).collect(),
        }
    }

    fn to_union_name(&self) -> String {
        match self {
            FieldType::Primitive(type_value, _) => type_value.to_string(),
            FieldType::Enum(name, _) => name.to_string(),
            FieldType::Literal(literal_value, _) => match literal_value {
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
            FieldType::Class(name, _) => name.to_string(),
            FieldType::List(field_type, _) => {
                format!("List__{}", field_type.to_union_name())
            }
            FieldType::Map(field_type, field_type1, _) => {
                format!(
                    "Map__{}_{}",
                    field_type.to_union_name(),
                    field_type1.to_union_name()
                )
            }
            FieldType::Union(field_types, _) => match field_types.view() {
                UnionTypeView::Null => "null".to_string(),
                UnionTypeView::Optional(field_type) => field_type.to_union_name(),
                UnionTypeView::OneOf(field_types) | UnionTypeView::OneOfOptional(field_types) => {
                    format!(
                        "Union__{}",
                        field_types
                            .iter()
                            .map(|t| t.to_union_name())
                            .collect::<Vec<_>>()
                            .join("__")
                    )
                }
            },
            FieldType::Tuple(field_types, _) => format!(
                "Tuple__{}",
                field_types
                    .iter()
                    .map(|v| v.to_union_name())
                    .collect::<Vec<_>>()
                    .join("__")
                    .to_string()
            ),
            FieldType::RecursiveTypeAlias(name, _) => name.to_string(),
            FieldType::Arrow(_, _) => "function".to_string(),
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
mod tests {
    use super::*;

    #[test]
    fn simplify_base_case() {
        let null = (*NULL_TYPE).clone();
        assert_eq!(null.simplify(), null);
    }

    #[test]
    fn simplify_int() {
        let int = FieldType::int();
        assert_eq!(int.simplify(), int);
    }

    #[test]
    fn simplify_optional_int() {
        let optional_int = FieldType::optional(FieldType::int());
        assert_eq!(optional_int.simplify(), optional_int);
    }

    #[test]
    fn const_null_equals_synthetic_null() {
        let optional_int = FieldType::optional(FieldType::int());
        let union = FieldType::Union(
            UnionType::new(vec![FieldType::int(), (*NULL_TYPE).clone()]),
            TypeMetadataIR::default(),
        );
        assert_eq!(optional_int, union)
    }

    #[test]
    fn simplify_nested_unions() {
        // ((int | null) | string)
        let inner_union = FieldType::union(vec![FieldType::int(), FieldType::null()]);
        let outer_union = FieldType::union(vec![inner_union, FieldType::string()]);
        // union(union(int, null), string)
        assert_eq!(
            outer_union.simplify(),
            FieldType::union(vec![
                FieldType::int(),
                FieldType::string(),
                FieldType::null()
            ])
        );
    }

    #[test]
    fn simplify_repeated_variants() {
        let union = FieldType::union(vec![
            FieldType::int(),
            FieldType::int(),
            FieldType::string(),
            FieldType::string(),
        ]);
        assert_eq!(
            union.simplify(),
            FieldType::union(vec![FieldType::int(), FieldType::string()])
        );
    }

    #[test]
    fn simplify_nested_with_repeats() {
        let inner_union = FieldType::union(vec![FieldType::int(), FieldType::null()]);
        let union = FieldType::union(vec![FieldType::int(), inner_union, FieldType::string()]);
        assert_eq!(
            union.simplify(),
            FieldType::union(vec![
                FieldType::int(),
                FieldType::string(),
                FieldType::null()
            ])
        );
    }

    #[test]
    fn flatten_base_case() {
        let null = (*NULL_TYPE).clone();
        assert_eq!(null.flatten(), vec![null])
    }

    #[test]
    fn flatten_int() {
        let int = FieldType::int();
        assert_eq!(int.flatten(), vec![int])
    }

    #[test]
    fn flatten_optional_int() {
        let optional_int = FieldType::optional(FieldType::int());
        assert_eq!(
            optional_int.flatten(),
            vec![FieldType::int(), FieldType::null()]
        )
    }
}
