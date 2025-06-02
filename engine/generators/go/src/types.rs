use crate::r#type::{Package, SerializeType, TypeGo};

/// A list of classes in Go.
///
/// ```askama
/// {% for item in items -%}
/// {{ item.render()? }}
/// {%- endfor %}
/// ```
#[derive(askama::Template)]
#[template(in_doc = true, escape = "none", ext = "txt")]
struct ListTemplate<'a, T: askama::Template> {
    items: &'a [T],
}


mod class {
    use askama::Template;

    use super::*;

    /// A class in Go.
    ///
    /// ```askama
    /// {% if let Some(docstring) = docstring -%}
    /// {{ crate::utils::prefix_lines(docstring, "/// ") }}
    /// {%- endif %}
    /// type {{ name }} struct {
    ///     {% for field in fields -%}
    ///     {{ field.render()? }}
    ///     {%- endfor %}
    ///     {% if dynamic -%}
    ///     DynamicProperties map[string]any
    ///     {%- endif %}
    /// }
    /// ```
    #[derive(askama::Template)]
    #[template(in_doc = true, escape = "none", ext = "txt")]
    pub struct ClassGo<'a> {
        name: String,
        docstring: Option<String>,
        fields: Vec<FieldGo<'a>>,
        dynamic: bool,
        pkg: &'a Package,
    }

    /// A field in a class.
    ///
    /// ```askama
    /// {% if let Some(docstring) = docstring -%}
    /// {{ crate::utils::prefix_lines(docstring, "/// ") }}
    /// {%- endif %}
    /// {{ name }} {{ type.serialize_type(pkg) }}
    /// ```
    #[derive(askama::Template)]
    #[template(in_doc = true, escape = "none", ext = "txt")]
    struct FieldGo<'a> {
        docstring: Option<String>,
        name: String,
        r#type: TypeGo,
        pkg: &'a Package,
    }

    pub(super) fn render_classes(classes: &[ClassGo], _: &Package) -> Result<String, askama::Error> {
        ListTemplate {
            items: classes,
        }.render()
    }
}

mod enums {
    use askama::Template;

    use crate::r#type::Package;

    use super::ListTemplate;


    #[derive(askama::Template)]
    #[template(path = "enums.go.j2", escape = "none")]
    pub struct EnumGo {
        name: String,
        docstring: Option<String>,
        values: Vec<(String, Option<String>)>,
        dynamic: bool,
    }

    pub(super) fn render_enums(enums: &[EnumGo], _: &Package) -> Result<String, askama::Error> {
        ListTemplate {
            items: enums,
        }.render()
    }
}

mod union {
    use askama::Template;

    use super::*;

    #[derive(askama::Template)]
    #[template(path = "unions.go.j2", escape = "none")]
    pub(super) struct UnionGo<'a> {
        name: String,
        docstring: Option<String>,
        variants: Vec<(String, TypeGo)>,
        pkg: &'a Package,
    }

    mod filters {
        // This filter does not have extra arguments
        pub fn exported_name(s: &str, _: &dyn askama::Values) -> askama::Result<String> {
            // make first letter uppercase
            let first_letter = s.chars().next().unwrap().to_uppercase();
            let rest = s[1..].to_string();
            Ok(format!("{}{}", first_letter, rest))
        }
    }

    pub(super) fn render_unions(unions: &[UnionGo], _: &Package) -> Result<String, askama::Error> {
        ListTemplate {
            items: unions,
        }.render()
    }
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
/// type Checked[T any] baml.Checked[T]
///
/// {{ enums::render_enums(&enums, pkg)? }}
///
/// {{ class::render_classes(&classes, pkg)? }}
///
/// {{ union::render_unions(&unions, pkg)? }}
///
/// ```
///
#[derive(askama::Template)]
#[template(in_doc = true, escape = "none", ext = "txt")]
pub(crate) struct GoTypes<'ir> {
    classes: Vec<class::ClassGo<'ir>>,
    enums: Vec<enums::EnumGo>,
    unions: Vec<union::UnionGo<'ir>>,
    pkg: &'ir Package,
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
/// {{ class::render_classes(&classes, pkg)? }}
///
/// {{ union::render_unions(&unions, pkg)? }}
///
/// ```
///
#[derive(askama::Template)]
#[template(in_doc = true, escape = "none", ext = "txt")]
pub(crate) struct GoStreamTypes<'ir> {
    classes: Vec<class::ClassGo<'ir>>,
    unions: Vec<union::UnionGo<'ir>>,
    pkg: &'ir Package,
}
