use super::{BamlMediaType, FieldType, TypeMetadata, TypeValue, UserDefinedObject};

impl FieldType {
    pub fn primitive(value: TypeValue) -> Self {
        FieldType::Primitive(value, TypeMetadata::default())
    }

    pub fn string() -> Self {
        FieldType::primitive(TypeValue::String)
    }

    pub fn literal(value: super::LiteralValue) -> Self {
        FieldType::Literal(value, TypeMetadata::default())
    }

    pub fn literal_string(value: String) -> Self {
        FieldType::literal(super::LiteralValue::String(value))
    }

    pub fn literal_int(value: i64) -> Self {
        FieldType::literal(super::LiteralValue::Int(value))
    }

    pub fn literal_bool(value: bool) -> Self {
        FieldType::literal(super::LiteralValue::Bool(value))
    }

    pub fn int() -> Self {
        FieldType::primitive(TypeValue::Int)
    }

    pub fn float() -> Self {
        FieldType::primitive(TypeValue::Float)
    }

    pub fn bool() -> Self {
        FieldType::primitive(TypeValue::Bool)
    }

    pub fn null() -> Self {
        FieldType::Null(TypeMetadata::default())
    }

    pub fn image() -> Self {
        FieldType::primitive(TypeValue::Media(BamlMediaType::Image))
    }

    pub fn audio() -> Self {
        FieldType::primitive(TypeValue::Media(BamlMediaType::Audio))
    }

    pub fn r#enum(name: &str) -> Self {
        FieldType::Enum(
            name.to_string(),
            UserDefinedObject::default(),
            TypeMetadata::default(),
        )
    }

    pub fn class(name: &str) -> Self {
        FieldType::Class(
            name.to_string(),
            UserDefinedObject::default(),
            TypeMetadata::default(),
        )
    }

    pub fn list(inner: FieldType) -> Self {
        FieldType::List(Box::new(inner.simplify()), TypeMetadata::default())
    }

    pub fn as_list(self) -> Self {
        FieldType::List(Box::new(self), TypeMetadata::default())
    }

    pub fn map(key: FieldType, value: FieldType) -> Self {
        FieldType::Map(
            Box::new(key.simplify()),
            Box::new(value.simplify()),
            TypeMetadata::default(),
        )
    }

    pub fn union(choices: Vec<FieldType>) -> Self {
        FieldType::Union(choices, TypeMetadata::default()).simplify()
    }

    pub fn tuple(choices: Vec<FieldType>) -> Self {
        FieldType::Tuple(
            choices.into_iter().map(|t| t.simplify()).collect(),
            TypeMetadata::default(),
        )
    }

    pub fn optional(inner: FieldType) -> Self {
        FieldType::Union(
            vec![inner, FieldType::Null(TypeMetadata::default())],
            TypeMetadata::default(),
        )
        .simplify()
    }

    pub fn as_optional(self) -> Self {
        FieldType::Union(
            vec![self, FieldType::Null(TypeMetadata::default())],
            TypeMetadata::default(),
        )
        .simplify()
    }
}
