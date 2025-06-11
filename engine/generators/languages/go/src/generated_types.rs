use crate::package::CurrentRenderPackage;
use crate::r#type::{SerializeType, TypeGo};

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

    #[derive(askama::Template)]
    #[template(path = "class.go.j2", escape = "none", ext = "txt")]
    pub struct ClassGo<'a> {
        pub name: String,
        pub docstring: Option<String>,
        pub fields: Vec<FieldGo<'a>>,
        pub dynamic: bool,
        pub pkg: &'a CurrentRenderPackage,
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
    pub struct FieldGo<'a> {
        pub docstring: Option<String>,
        pub name: String,
        pub r#type: TypeGo,
        pub pkg: &'a CurrentRenderPackage,
    }
    impl std::fmt::Debug for FieldGo<'_> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(
                f,
                "FieldGo {{docstring: {:?}, name: {}, type: {:?}, pkg: <<Mutex>> }}",
                self.docstring, self.name, self.r#type
            )
        }
    }

    mod helpers {
        use crate::{package::Package, r#type::TypeGo};

        pub fn stream_variants(t: &TypeGo) -> Vec<TypeGo> {
            let mut variants = vec![t.clone()];

            let stream_pkg = Package::stream_types();

            // add stream types for user defined types (classes and unions)
            // enums have no "stream" variants
            match t {
                TypeGo::Class {
                    name,
                    meta,
                    dynamic,
                    package: _unused,
                } => {
                    variants.push(TypeGo::Class {
                        name: name.clone(),
                        package: stream_pkg.clone(),
                        meta: meta.clone(),
                        dynamic: *dynamic,
                    });
                }
                TypeGo::Union {
                    name,
                    meta,
                    package: _unused,
                } => {
                    variants.push(TypeGo::Union {
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
    #[derive(askama::Template)]
    #[template(path = "enums.go.j2", escape = "none")]
    pub struct EnumGo {
        pub name: String,
        pub docstring: Option<String>,
        pub values: Vec<(String, Option<String>)>,
        pub dynamic: bool,
    }
}

mod union {
    use super::*;

    #[derive(askama::Template)]
    #[template(path = "unions.go.j2", escape = "none")]
    pub struct UnionGo<'a> {
        pub name: String,
        pub cffi_name: String,
        pub docstring: Option<String>,
        pub variants: Vec<VariantGo>,
        pub pkg: &'a CurrentRenderPackage,
    }

    #[derive(Clone)]
    pub struct VariantGo {
        pub name: String,
        pub cffi_name: String,
        pub type_: TypeGo,
    }

    /// A union in Go that is used for stream state.
    ///
    /// ```askama
    /// {% if let Some(docstring) = docstring -%}
    /// {{ crate::utils::prefix_lines(docstring, "/// ") }}
    /// {%- endif %}
    /// type Generic__{{ name }} struct[{% for v in variants -%}Type__{{ v.name }}, {%- endfor %}]
    /// ```
    #[derive(askama::Template)]
    #[template(in_doc = true, escape = "none", ext = "txt")]
    struct StreamUnionGo<'a> {
        name: String,
        docstring: Option<String>,
        variants: Vec<VariantGo>,
        pkg: &'a CurrentRenderPackage,
    }

    impl<'a> From<&'a UnionGo<'a>> for StreamUnionGo<'a> {
        fn from(value: &'a UnionGo) -> Self {
            Self {
                name: value.name.clone(),
                docstring: value.docstring.clone(),
                variants: value.variants.clone(),
                pkg: value.pkg,
            }
        }
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
    /// type {{ name }} = {{ type_.serialize_type(pkg) }}
    /// ```
    #[derive(askama::Template)]
    #[template(in_doc = true, escape = "none", ext = "txt")]
    pub struct TypeAliasGo<'a> {
        pub name: String,
        pub type_: TypeGo,
        pub docstring: Option<String>,
        pub pkg: &'a CurrentRenderPackage,
    }
}

pub(crate) fn render_type_aliases(
    aliases: &[TypeAliasGo],
    pkg: &CurrentRenderPackage,
) -> Result<String, askama::Error> {
    use askama::Template;
    GoTypes {
        items: aliases,
        pkg,
    }
    .render()
}

/// A list of types in Go.
///
/// ```askama
/// package types
///
/// import (
/// 	baml "github.com/boundaryml/baml/engine/language_client_go/pkg"
/// )
///
/// type Checked[T any] baml.Checked[T]
/// ```
///
#[derive(askama::Template)]
#[template(in_doc = true, escape = "none", ext = "txt")]
pub struct GoTypesUtils<'ir> {
    pkg: &'ir CurrentRenderPackage,
}

pub(crate) fn render_go_types_utils(pkg: &CurrentRenderPackage) -> Result<String, askama::Error> {
    use askama::Template;

    GoTypesUtils { pkg }.render()
}

/// A list of types in Go.
///
/// ```askama
/// package types
///
/// import (
/// 	"encoding/json"
/// 	"fmt"
///
/// 	flatbuffers "github.com/google/flatbuffers/go"
/// 	baml "github.com/boundaryml/baml/engine/language_client_go/pkg"
/// 	"github.com/boundaryml/baml/engine/language_client_go/pkg/cffi"
/// )
///
/// {% for item in items -%}
/// {{ item.render()? }}
/// {% endfor %}
///
/// ```
///
#[derive(askama::Template)]
#[template(in_doc = true, escape = "none", ext = "txt")]
pub struct GoTypes<'ir, T: askama::Template> {
    items: &'ir [T],
    pkg: &'ir CurrentRenderPackage,
}

pub(crate) fn render_go_types<T: askama::Template>(
    items: &[T],
    pkg: &CurrentRenderPackage,
) -> Result<String, askama::Error> {
    use askama::Template;

    GoTypes { items, pkg }.render()
}

const STREAM_STATE_GO: &str = r#"
type StreamStateType string

const (
	StreamStatePending    StreamStateType = "Pending"
	StreamStateIncomplete StreamStateType = "Incomplete"
	StreamStateComplete   StreamStateType = "Complete"
)

// Values returns all allowed values for the AliasedEnum type.
func (StreamStateType) Values() []StreamStateType {
	return []StreamStateType{
		StreamStatePending,
		StreamStateIncomplete,
		StreamStateComplete,
	}
}

// IsValid checks whether the given AliasedEnum value is valid.
func (e StreamStateType) IsValid() bool {

	for _, v := range e.Values() {
		if e == v {
			return true
		}
	}
	return false

}

// MarshalJSON customizes JSON marshaling for AliasedEnum.
func (e StreamStateType) MarshalJSON() ([]byte, error) {
	if !e.IsValid() {
		return nil, fmt.Errorf("invalid StreamStateType: %q", e)
	}
	return json.Marshal(string(e))
}

// UnmarshalJSON customizes JSON unmarshaling for AliasedEnum.
func (e *StreamStateType) UnmarshalJSON(data []byte) error {
	var s string
	if err := json.Unmarshal(data, &s); err != nil {
		return err
	}
	*e = StreamStateType(s)
	if !e.IsValid() {
		return fmt.Errorf("invalid StreamStateType: %q", s)
	}
	return nil
}


type StreamState[T any] struct {
    Value T `json:"value"`
    State StreamStateType `json:"state"`    
}
"#;

/// A list of types in Go.
///
/// ```askama
/// package stream_types
///
/// import (
/// 	"encoding/json"
/// 	"fmt"
///
/// 	flatbuffers "github.com/google/flatbuffers/go"
/// 	baml "github.com/boundaryml/baml/engine/language_client_go/pkg"
/// 	"github.com/boundaryml/baml/engine/language_client_go/pkg/cffi"
/// )
///
/// {{ STREAM_STATE_GO }}
///
/// ```
///
#[derive(askama::Template)]
#[template(in_doc = true, escape = "none", ext = "txt")]
pub struct GoStreamTypesUtils<'ir> {
    pkg: &'ir CurrentRenderPackage,
}

pub(crate) fn render_go_stream_types_utils(
    pkg: &CurrentRenderPackage,
) -> Result<String, askama::Error> {
    use askama::Template;

    GoStreamTypesUtils { pkg }.render()
}
/// A list of types in Go.
///
/// ```askama
/// package stream_types
///
/// import (
/// 	"encoding/json"
/// 	"fmt"
///
/// 	flatbuffers "github.com/google/flatbuffers/go"
/// 	baml "github.com/boundaryml/baml/engine/language_client_go/pkg"
/// 	"github.com/boundaryml/baml/engine/language_client_go/pkg/cffi"
/// )
///
/// {% for item in items -%}
/// {{ item.render()? }}
/// {%- endfor %}
/// ```
///
#[derive(askama::Template)]
#[template(in_doc = true, escape = "none", ext = "txt")]
pub(crate) struct GoStreamTypes<'ir, T: askama::Template> {
    items: &'ir [T],
    pkg: &'ir CurrentRenderPackage,
}

pub(crate) fn render_go_stream_types<T: askama::Template>(
    items: &[T],
    pkg: &CurrentRenderPackage,
) -> Result<String, askama::Error> {
    use askama::Template;

    GoStreamTypes { items, pkg }.render()
}

pub use class::{ClassGo, FieldGo};
pub use enums::EnumGo;
pub use union::{UnionGo, VariantGo};
pub use type_aliases::TypeAliasGo;
