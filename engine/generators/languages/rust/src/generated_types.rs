use crate::{
    package::CurrentRenderPackage,
    r#type::{SerializeType, TypeRust},
};
use askama::Template;

mod filters {
    use crate::utils::to_snake_case;

    pub fn snake_case(s: &str, _args: &dyn askama::Values) -> askama::Result<String> {
        Ok(to_snake_case(s))
    }

    pub fn json_string_literal(s: &str, _args: &dyn askama::Values) -> askama::Result<String> {
        serde_json::to_string(s).map_err(|e| askama::Error::Custom(Box::new(e)))
    }
}

// Template structs for Askama-based code generation
mod class {
    use super::*;

    #[derive(askama::Template)]
    #[template(path = "struct.rs.j2", escape = "none")]
    pub struct ClassRust<'a> {
        pub name: String,
        pub docstring: Option<String>,
        pub fields: Vec<FieldRust<'a>>,
        pub dynamic: bool,
        pub pkg: &'a CurrentRenderPackage,
    }

    #[derive(Clone)]
    pub struct FieldRust<'a> {
        pub name: String,
        pub original_name: String,
        pub docstring: Option<String>,
        pub rust_type: TypeRust,
        pub pkg: &'a CurrentRenderPackage,
    }

    impl std::fmt::Debug for FieldRust<'_> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(
            f,
            "FieldRust {{name: {}, original_name: {}, rust_type: <<TypeRust>>, pkg: <<Mutex>> }}",
            self.name, self.original_name
        )
        }
    }
}

pub use class::*;

mod r#enum {
    use super::*;

    #[derive(askama::Template)]
    #[template(path = "enum.rs.j2", escape = "none")]
    pub struct EnumRust<'a> {
        pub name: String,
        pub docstring: Option<String>,
        pub values: Vec<String>,
        pub dynamic: bool,
        pub pkg: &'a CurrentRenderPackage,
    }
}

pub use r#enum::*;

mod union {
    use super::*;

    #[derive(askama::Template)]
    #[template(path = "union.rs.j2", escape = "none")]
    pub struct UnionRust<'a> {
        pub name: String,
        pub docstring: Option<String>,
        pub variants: Vec<UnionVariantRust>,
        pub pkg: &'a CurrentRenderPackage,
    }

    #[derive(Debug, Clone)]
    pub struct UnionVariantRust {
        pub name: String,
        pub docstring: Option<String>,
        pub rust_type: TypeRust,
        pub literal_value: Option<String>,
        pub literal_kind: Option<RustLiteralKind>,
    }
}

pub use union::*;

mod type_alias {
    use super::*;

    #[derive(askama::Template)]
    #[template(in_doc = true, escape = "none", ext = "txt")]
    /// ```askama
    /// {% if let Some(docstring) = docstring -%}
    /// /// {{ docstring }}
    /// {% endif -%}
    /// pub type {{ name }} = {{ type_.serialize_type(pkg) }};
    /// ```
    pub struct TypeAliasRust<'a> {
        pub name: String,
        pub type_: TypeRust,
        pub docstring: Option<String>,
        pub pkg: &'a CurrentRenderPackage,
    }
}

pub use type_alias::*;

// Backward compatibility structs for ir_to_rust modules
#[derive(Debug, Clone)]
pub struct RustClass {
    pub name: String,
    pub fields: Vec<RustField>,
}

#[derive(Debug, Clone)]
pub struct RustField {
    pub name: String,
    pub original_name: String,
    pub rust_type: String,
    pub optional: bool,
}

#[derive(Debug, Clone)]
pub struct RustEnum {
    pub name: String,
    pub values: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct RustUnion {
    pub name: String,
    pub variants: Vec<RustVariant>,
    pub docstring: Option<String>,
}

#[derive(Debug, Clone)]
pub enum RustLiteralKind {
    String,
    Int,
    Bool,
}

#[derive(Debug, Clone)]
pub struct RustVariant {
    pub name: String,
    pub rust_type: crate::r#type::TypeRust,
    pub docstring: Option<String>,
    pub literal_value: Option<String>,
    pub literal_kind: Option<RustLiteralKind>,
}

/// A list of types in Rust.
///
/// ```askama
/// {% for item in items -%}
/// {{ item.render()? }}
///
/// {% endfor %}
/// ```
#[derive(askama::Template)]
#[template(in_doc = true, escape = "none", ext = "txt")]
pub struct RustTypes<'ir, T: askama::Template> {
    items: &'ir [T],
}

pub(crate) fn render_rust_types<T: askama::Template>(
    items: &[T],
    _pkg: &CurrentRenderPackage,
) -> Result<String, askama::Error> {
    use askama::Template;

    RustTypes { items }.render()
}

// Convenience function for mixed type rendering
pub fn render_all_rust_types(
    classes: &[ClassRust],
    enums: &[EnumRust],
    unions: &[UnionRust],
    type_aliases: &[TypeAliasRust],
    pkg: &CurrentRenderPackage,
) -> Result<String, askama::Error> {
    let mut output = String::new();

    output.push_str("use serde::{Deserialize, Serialize};\n");
    output.push_str("use std::collections::HashMap;\n\n");

    output.push_str(
        r#"/// Represents the BAML `null` type in Rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct NullValue;

impl baml_client_rust::types::ToBamlValue for NullValue {
    fn to_baml_value(self) -> baml_client_rust::BamlResult<baml_client_rust::types::BamlValue> {
        Ok(baml_client_rust::types::BamlValue::Null)
    }
}

impl baml_client_rust::types::FromBamlValue for NullValue {
    fn from_baml_value(
        value: baml_client_rust::types::BamlValue,
    ) -> baml_client_rust::BamlResult<Self> {
        match value {
            baml_client_rust::types::BamlValue::Null => Ok(NullValue),
            other => Err(baml_client_rust::BamlError::deserialization(format!(
                "Expected null, got {:?}",
                other
            ))),
        }
    }
}

"#,
    );

    // Render classes
    if !classes.is_empty() {
        output.push_str(&render_rust_types(classes, pkg)?);
        output.push_str("\n");
    }

    // Render enums
    if !enums.is_empty() {
        output.push_str(&render_rust_types(enums, pkg)?);
        output.push_str("\n");
    }

    // Render unions
    if !unions.is_empty() {
        output.push_str(&render_rust_types(unions, pkg)?);
        output.push_str("\n");
    }

    // Render type aliases
    if !type_aliases.is_empty() {
        output.push_str(&render_rust_types(type_aliases, pkg)?);
        output.push_str("\n");
    }

    Ok(output)
}
