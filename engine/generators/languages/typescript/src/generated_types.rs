use crate::package::CurrentRenderPackage;
use crate::r#type::{SerializeType, TypeTS};

mod filters {
    // This filter does not have extra arguments
    // pub fn exported_name(s: &str, _: &dyn askama::Values) -> askama::Result<String> {
    //     // make first letter uppercase
    //     let first_letter = s.chars().next().unwrap().to_uppercase();
    //     let rest = s[1..].to_string();
    //     Ok(format!("{}{}", first_letter, rest))
    // }
}

mod class {
    use super::*;

    /// A class in TS.
    ///
    /// ```askama
    /// {%- if let Some(docstring) = docstring %}
    /// {{docstring}}
    /// {%- endif %}
    /// export interface {{name}} {
    ///   {%- for field in fields %}
    ///   {{field.render()?}}
    ///   {%- endfor %}
    /// }
    ///
    /// ```
    #[derive(askama::Template)]
    #[template(in_doc = true, escape = "none", ext = "txt")]
    pub struct ClassTS<'a> {
        pub name: String,
        pub docstring: Option<String>,
        pub fields: Vec<FieldTS<'a>>,
        pub dynamic: bool,
        pub pkg: &'a CurrentRenderPackage,
    }

    /// A field in a class.
    ///
    /// ```askama
    /// {% if let Some(docstring) = docstring -%}
    /// {{ crate::utils::prefix_lines(docstring, "//") }}
    /// {%- endif %}
    /// {{name}}{% if type.meta().is_optional() %}?{% endif %}: {{type.serialize_type(pkg)}}
    /// ```
    #[derive(askama::Template, Clone)]
    #[template(in_doc = true, escape = "none", ext = "txt")]
    pub struct FieldTS<'a> {
        pub docstring: Option<String>,
        pub name: String,
        pub r#type: TypeTS,
        pub pkg: &'a CurrentRenderPackage,
    }
    impl std::fmt::Debug for FieldTS<'_> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(
                f,
                "FieldTS {{docstring: {:?}, name: {}, type: {:?}, pkg: <<Mutex>> }}",
                self.docstring, self.name, self.r#type
            )
        }
    }
}

/// A list of types in TS.
///
/// ```askama
/// /// import type { Image, Audio } from "@boundaryml/baml"
/// /**
/// * Recursively partial type that can be null.
/// *
/// * @deprecated Use types from the `partial_types` namespace instead, which provides type-safe partial implementations
/// * @template T The type to make recursively partial.
/// */
/// export type RecursivePartialNull<T> = T extends object
///     ? { [P in keyof T]?: RecursivePartialNull<T[P]> }
///     : T | null;

/// export interface Checked<T,CheckName extends string = string> {
///     value: T,
///     checks: Record<CheckName, Check>,
/// }
/// export interface Check {
///     name: string,
///     expr: string
///     status: "succeeded" | "failed"
/// }
/// export function all_succeeded<CheckName extends string>(checks: Record<CheckName, Check>): boolean {
///     return get_checks(checks).every(check => check.status === "succeeded")
/// }
/// export function get_checks<CheckName extends string>(checks: Record<CheckName, Check>): Check[] {
///     return Object.values(checks)
/// }
/// ```
///
#[derive(askama::Template)]
#[template(in_doc = true, escape = "none", ext = "txt")]
struct TsTypes<'ir, T: askama::Template> {
    items: &'ir [T],
}

pub(crate) fn render_ts_types<T: askama::Template>(
    items: &[T],
    _pkg: &CurrentRenderPackage,
) -> Result<String, askama::Error> {
    use askama::Template;

    TsTypes { items }.render()
}

pub use class::{ClassTS, FieldTS};
// pub use enums::EnumTS;
// pub use union::{UnionTS, VariantTS};
// pub use type_aliases::TypeAliasTS;
