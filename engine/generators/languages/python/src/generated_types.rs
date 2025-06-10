use crate::module::CurrentRenderModule;
use crate::r#type::{SerializeType, TypePython};

mod filters {
    // This filter does not have extra arguments
    pub fn exported_name(s: &str, _: &dyn askama::Values) -> askama::Result<String> {
        // make first letter uppercase
        let first_letter = s.chars().next().unwrap().to_uppercase();
        let rest = s[1..].to_string();
        Ok(format!("{}{}", first_letter, rest))
    }
}

mod class {
    use super::*;

    /// A class in Python.
    ///
    /// ```askama
    /// class {{ name }}(BaseModel):
    ///   {%- if let Some(docstring) = docstring %}
    ///   {{ docstring }}
    ///   {%- endif %}
    ///   {%- if dynamic %}
    ///   model_config = ConfigDict(extra='allow')
    ///   {%- endif %}
    ///   {%- for field in fields %}
    ///   {{ field.render()? }}
    ///   {%- endfor %}
    /// ```
    #[derive(askama::Template)]
    #[template(in_doc = true, escape = "none", ext = "txt")]
    pub struct ClassPython<'a> {
        pub name: String,
        pub docstring: Option<String>,
        pub fields: Vec<FieldPython<'a>>,
        pub dynamic: bool,
        pub module: &'a CurrentRenderModule,
        // pub is_pydantic_2: bool, // TODO: add this.
    }

    /// A field in a class.
    ///
    /// ```askama
    /// {% if let Some(docstring) = docstring -%}
    /// {{ crate::utils::prefix_lines(docstring, "/// ") }}
    /// {%- endif %}
    /// {{ name|exported_name }} {{ type.serialize_type(pkg) }} `json:"{{ name }}"`
    /// ```
    #[derive(askama::Template, Clone)]
    #[template(in_doc = true, escape = "none", ext = "txt")]
    pub struct FieldPython<'a> {
        pub docstring: Option<String>,
        pub name: String,
        pub r#type: TypePython,
        pub module: &'a CurrentRenderModule,
    }
    impl std::fmt::Debug for FieldPython<'_> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(
                f,
                "FieldPython {{docstring: {:?}, name: {}, type: {:?}, module: <<Mutex>> }}",
                self.docstring, self.name, self.r#type
            )
        }
    }

    mod helpers {
        use crate::{module::Module, r#type::TypePython};

        pub fn stream_variants(t: &TypePython) -> Vec<TypePython> {
            let mut variants = vec![t.clone()];

            let stream_module = Module::new("baml_client.stream_types");

            // add stream types for user defined types (classes and unions)
            // enums have no "stream" variants
            match t {
                TypePython::Class {
                    name,
                    meta,
                    dynamic,
                    package: _unused,
                } => {
                    variants.push(TypePython::Class {
                        name: name.clone(),
                        package: stream_pkg.clone(),
                        meta: meta.clone(),
                        dynamic: *dynamic,
                    });
                }
                TypePython::Union {
                    name,
                    meta,
                    package: _unused,
                } => {
                    variants.push(TypePython::Union {
                        name: name.clone(),
                        package: stream_pkg.clone(),
                        meta: meta.clone(),
                    });
                }
                _ => {}
            }

            // add optional variants
            let optional_variants = variants
                .iter()
                .filter(|v| !v.meta().is_optional())
                .map(|v| {
                    let mut t = v.clone();
                    t.meta_mut().make_optional();
                    t
                })
                .collect::<Vec<_>>();
            variants.extend(optional_variants);

            // add stream state variants for each variant
            let stream_variants = variants
                .iter()
                .map(|v| {
                    let mut t = v.clone();
                    t.meta_mut().set_stream_state();
                    t
                })
                .collect::<Vec<_>>();

            variants.extend(stream_variants);
            variants
        }
    }
}

mod enums {
    /// An enum in Python.
    ///
    /// ```askama
    /// class {{ name }}(str, Enum):
    ///   {%- if let Some(docstring) = docstring %}
    ///   {{ docstring }}
    ///   {%- endif %}
    ///   {%- for (value, m_docstring) in values %}
    ///   {{ value }} = "{{ value }}"
    ///   {%- if let Some(docstring) = m_docstring %}
    ///   {{ docstring }}
    ///   {%- endif %}
    ///   {%- endfor %}
    #[derive(askama::Template)]
    #[template(in_doc = true, escape = "none", ext = "txt")]
    pub struct EnumGo {
        pub name: String,
        pub docstring: Option<String>,
        pub values: Vec<(String, Option<String>)>,
        pub dynamic: bool,
    }
}

mod type_aliases {
    use super::*;

    /// A type alias in Go.
    ///
    /// ```askama
    /// {% if let Some(docstring) = docstring -%}
    /// {{ crate::utils::prefix_lines(docstring, "/// ") }}
    /// {%- endif %}
    /// type {{ name }} = {{ type_.serialize_type(module) }}
    ///
    /// {# DONT DO THIS FOR NOW it seems to work?
    /// {% match type_ -%}
    /// {% when TypeGo::Union { .. } -%}
    /// func (u *{{ name }}) Decode(holder *cffi.CFFIValueUnionVariant) {
    ///     decodedUnion := {{ type_.zero_value(module) }}
    ///     decodedUnion.Decode(holder)
    ///     *u = {{ name }}{decodedUnion}
    /// }
    /// {%- else -%}
    /// {% endmatch %}
    /// #}
    /// ```
    #[derive(askama::Template)]
    #[template(in_doc = true, escape = "none", ext = "txt")]
    pub struct TypeAliasPython<'a> {
        pub name: String,
        pub type_: TypePython,
        pub docstring: Option<String>,
        pub module: &'a CurrentRenderModule,
    }
}

pub(crate) fn render_type_aliases(
    aliases: &[TypeAliasPython],
    module: &CurrentRenderModule,
) -> Result<String, askama::Error> {
    use askama::Template;
    PythonTypes {
        items: aliases,
        module,
    }
    .render()
}

/// A fragment of the types.py file.
///
/// ```askama
/// T = TypeVar('T')
/// CheckName = TypeVar('CheckName', bound=str)
///
/// class Check(BaseModel):
///     name: str
///     expression: str
///     status: str
///
/// class Checked(BaseModel, Generic[T,CheckName]):
///     value: T
///     checks: Dict[CheckName, Check]
///
/// def get_checks(checks: Dict[CheckName, Check]) -> List[Check]:
///     return list(checks.values())
/// def all_succeeded(checks: Dict[CheckName, Check]) -> bool:
///     return all(check.status == "succeeded" for check in get_checks(checks))
/// ```
///
#[derive(askama::Template)]
#[template(in_doc = true, escape = "none", ext = "txt")]
pub struct PythonTypesUtils<'ir> {
    module: &'ir CurrentRenderModule,
}

pub(crate) fn render_python_types_utils(
    module: &CurrentRenderModule,
) -> Result<String, askama::Error> {
    use askama::Template;

    PythonTypesUtils { module }.render()
}

/// A list of types in Python.
///
/// ```askama
/// from enum import Enum
///
/// from pydantic import BaseModel, ConfigDict
///
/// from typing_extensions import TypeAlias, Literal
/// from typing import Dict, Generic, List, Optional, TypeVar, Union
///
/// {% for item in items -%}
/// {{ item.render()? }}
/// {% endfor %}
///
/// ```
///
#[derive(askama::Template)]
#[template(in_doc = true, escape = "none", ext = "txt")]
pub struct PythonTypes<'ir, T: askama::Template> {
    items: &'ir [T],
    module: &'ir CurrentRenderModule,
}

pub(crate) fn render_python_types<T: askama::Template>(
    items: &[T],
    module: &CurrentRenderModule,
) -> Result<String, askama::Error> {
    use askama::Template;

    PythonTypes { items, module }.render()
}

/// A template fragment for the header of partial_types.py.
///
/// ```askama
/// import baml_py
/// from enum import Enum
///
/// from pydantic import BaseModel, ConfigDict
///
/// from typing_extensions import TypeAlias, Literal
/// from typing import Dict, Generic, List, Optional, TypeVar, Union
///
/// from . import types
/// from .types import Checked, Check
/// T = TypeVar('T')
/// class StreamState(BaseModel, Generic[T]):
///     value: T
///     state: Literal["Pending", "Incomplete", "Complete"]
///
/// ```
///
#[derive(askama::Template)]
#[template(in_doc = true, escape = "none", ext = "txt")]
pub struct PythonStreamTypesUtils<'ir> {
    module: &'ir CurrentRenderModule,
}

pub(crate) fn render_python_stream_types_utils(
    module: &CurrentRenderModule,
) -> Result<String, askama::Error> {
    use askama::Template;

    PythonStreamTypesUtils { module }.render()
}
/// A list of types in Python.
///
/// ```askama
///
/// {% for item in items -%}
/// {{ item.render()? }}
/// {%- endfor %}
/// ```
///
#[derive(askama::Template)]
#[template(in_doc = true, escape = "none", ext = "txt")]
pub(crate) struct PythonStreamTypes<'ir, T: askama::Template> {
    items: &'ir [T],
    module: &'ir CurrentRenderModule,
}

pub(crate) fn render_python_stream_types<T: askama::Template>(
    items: &[T],
    module: &CurrentRenderModule,
) -> Result<String, askama::Error> {
    use askama::Template;

    PythonStreamTypes { items, module }.render()
}

pub use class::{ClassPython, FieldGo};
pub use enums::EnumGo;
pub use type_aliases::TypeAliasPython;
pub use union::UnionGo;
