use super::{BamlMediaType, FieldType, TypeMetadataIR, TypeValue, UnionType};

impl FieldType {
    pub fn string() -> Self {
        FieldType::Primitive(TypeValue::String, TypeMetadataIR::default())
    }

    pub fn literal_string(value: String) -> Self {
        FieldType::Literal(
            super::LiteralValue::String(value),
            TypeMetadataIR::default(),
        )
    }

    pub fn literal_int(value: i64) -> Self {
        FieldType::Literal(super::LiteralValue::Int(value), TypeMetadataIR::default())
    }

    pub fn literal_bool(value: bool) -> Self {
        FieldType::Literal(super::LiteralValue::Bool(value), TypeMetadataIR::default())
    }

    pub fn int() -> Self {
        FieldType::Primitive(TypeValue::Int, TypeMetadataIR::default())
    }

    pub fn float() -> Self {
        FieldType::Primitive(TypeValue::Float, TypeMetadataIR::default())
    }

    pub fn bool() -> Self {
        FieldType::Primitive(TypeValue::Bool, TypeMetadataIR::default())
    }

    pub fn null() -> Self {
        FieldType::Primitive(TypeValue::Null, TypeMetadataIR::default())
    }

    pub fn image() -> Self {
        FieldType::Primitive(
            TypeValue::Media(BamlMediaType::Image),
            TypeMetadataIR::default(),
        )
    }

    pub fn audio() -> Self {
        FieldType::Primitive(
            TypeValue::Media(BamlMediaType::Audio),
            TypeMetadataIR::default(),
        )
    }

    pub fn r#enum(name: &str) -> Self {
        FieldType::Enum(name.to_string(), TypeMetadataIR::default())
    }

    pub fn class(name: &str) -> Self {
        FieldType::Class(name.to_string(), TypeMetadataIR::default())
    }

    pub fn list(inner: FieldType) -> Self {
        FieldType::List(Box::new(inner), TypeMetadataIR::default())
    }

    pub fn as_list(self) -> Self {
        FieldType::List(Box::new(self), TypeMetadataIR::default())
    }

    pub fn map(key: FieldType, value: FieldType) -> Self {
        FieldType::Map(Box::new(key), Box::new(value), TypeMetadataIR::default())
    }

    pub fn union(choices: Vec<FieldType>) -> Self {
        FieldType::Union(UnionType::new(choices), TypeMetadataIR::default()).simplify()
    }

    pub fn tuple(choices: Vec<FieldType>) -> Self {
        FieldType::Tuple(choices, TypeMetadataIR::default())
    }

    pub fn optional(inner: FieldType) -> Self {
        FieldType::Union(
            UnionType::new(vec![
                inner,
                FieldType::Primitive(TypeValue::Null, TypeMetadataIR::default()),
            ]),
            TypeMetadataIR::default(),
        )
        .simplify()
    }

    pub fn as_optional(self) -> Self {
        FieldType::Union(
            UnionType::new(vec![
                self,
                FieldType::Primitive(TypeValue::Null, TypeMetadataIR::default()),
            ]),
            TypeMetadataIR::default(),
        )
        .simplify()
    }
}
