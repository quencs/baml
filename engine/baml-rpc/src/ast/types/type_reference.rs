/*
 * EvaluationContext is used to evaluate a function call with context-specific information.
 *
 * For example, client_registry and type_builder
 */

use serde::{Deserialize, Serialize};

use super::type_definition::TypeId;

/// FieldType represents the type of either a class field or a function arg.
#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, Eq, Hash)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum TypeReference {
    // Primitive types
    String,
    Int,
    Float,
    Bool,
    Null,
    Media(MediaTypeDefinition),
    Literal(LiteralTypeDefinition),

    // Container types
    List(Box<TypeReference>),
    Map {
        key: Box<TypeReference>,
        value: Box<TypeReference>,
    },
    Union {
        one_of: Vec<TypeReference>,
        selected_index: usize,
    },
    Tuple {
        items: Vec<TypeReference>,
    },

    // User-defined types
    Class {
        name: TypeId,
    },
    Enum {
        name: TypeId,
    },
    RecursiveTypeAlias {
        name: TypeId,
    },
    // Narrowing types
    Checked(
        Box<TypeReference>,
        /* Order matters */ Vec<CheckedType>,
    ),
    Asserted(
        Box<TypeReference>,
        /* Order matters */ Vec<AssertedType>,
    ),
}

type CheckedType = NarrowingType<String>;
type AssertedType = NarrowingType<Option<String>>;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct NarrowingType<T> {
    pub name: T,
    pub expressions: Expression,
}

#[derive(Debug, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaTypeDefinition {
    Image,
    Audio,
}

/// Subset of [`crate::BamlValue`] allowed for literal type definitions.
#[derive(serde::Serialize, serde::Deserialize, Debug, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub enum LiteralTypeDefinition {
    String(String),
    Int(i64),
    Bool(bool),
}

#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, Eq, Hash)]
#[serde(tag = "expression_type", content = "data", rename_all = "snake_case")]
pub enum Expression {
    Jinja(String),
}
