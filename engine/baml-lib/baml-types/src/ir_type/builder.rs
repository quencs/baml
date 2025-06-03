use super::{BamlMediaType, StreamingMode, TypeGeneric, TypeValue};

impl<T: Default + std::fmt::Debug> TypeGeneric<T> {
    pub fn string() -> Self {
        TypeGeneric::Primitive(TypeValue::String, T::default())
    }

    pub fn literal_string(value: String) -> Self {
        TypeGeneric::Literal(super::LiteralValue::String(value), T::default())
    }

    pub fn literal_int(value: i64) -> Self {
        TypeGeneric::Literal(super::LiteralValue::Int(value), T::default())
    }

    pub fn literal_bool(value: bool) -> Self {
        TypeGeneric::Literal(super::LiteralValue::Bool(value), T::default())
    }

    pub fn int() -> Self {
        TypeGeneric::Primitive(TypeValue::Int, T::default())
    }

    pub fn float() -> Self {
        TypeGeneric::Primitive(TypeValue::Float, T::default())
    }

    pub fn bool() -> Self {
        TypeGeneric::Primitive(TypeValue::Bool, T::default())
    }

    pub fn null() -> Self {
        TypeGeneric::Primitive(TypeValue::Null, T::default())
    }

    pub fn image() -> Self {
        TypeGeneric::Primitive(TypeValue::Media(BamlMediaType::Image), T::default())
    }

    pub fn audio() -> Self {
        TypeGeneric::Primitive(TypeValue::Media(BamlMediaType::Audio), T::default())
    }

    pub fn r#enum(name: &str) -> Self {
        TypeGeneric::Enum {
            name: name.to_string(),
            dynamic: false,
            meta: T::default(),
        }
    }

    pub fn class(name: &str) -> Self {
        TypeGeneric::Class {
            name: name.to_string(),
            dynamic: false,
            mode: StreamingMode::NonStreaming,
            meta: T::default(),
        }
    }

    pub fn list(inner: Self) -> Self {
        TypeGeneric::List(Box::new(inner), T::default())
    }

    pub fn as_list(self) -> Self {
        TypeGeneric::List(Box::new(self), T::default())
    }

    pub fn map(key: TypeGeneric<T>, value: TypeGeneric<T>) -> Self {
        TypeGeneric::Map(Box::new(key), Box::new(value), T::default())
    }

    pub fn tuple(choices: Vec<TypeGeneric<T>>) -> Self {
        TypeGeneric::Tuple(choices, T::default())
    }

    pub fn optional(inner: TypeGeneric<T>) -> Self
    where
        T: Clone + Eq + std::hash::Hash + std::fmt::Debug + Default,
    {
        Self::union(vec![inner, TypeGeneric::null()])
    }

    pub fn as_optional(self) -> Self
    where
        T: Clone + Eq + std::hash::Hash + std::fmt::Debug + Default,
    {
        Self::optional(self)
    }
}