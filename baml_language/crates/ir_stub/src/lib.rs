//! Placeholder IR types and runtime types.
//!
//! This crate provides:
//! - Stub types for schema-ast, IR, and TypedIR dependencies
//! - Runtime value types (BamlValue, BamlMedia, etc.)
//! - Type IR for representing BAML types at runtime

use std::fmt;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

// ============================================================================
// BamlMap - Type alias for ordered map
// ============================================================================

/// Ordered map from string keys to BamlValue.
pub type BamlMap = IndexMap<String, BamlValue>;

// ============================================================================
// BamlValue - Runtime value type
// ============================================================================

/// A BAML runtime value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum BamlValue {
    /// String value.
    String(String),
    /// Integer value.
    Int(i64),
    /// Float value.
    Float(f64),
    /// Boolean value.
    Bool(bool),
    /// Null value.
    Null,
    /// List of values.
    List(Vec<BamlValue>),
    /// Map of string keys to values.
    Map(BamlMap),
    /// Media content (image, audio, etc.).
    Media(BamlMedia),
    /// Enum value with type name and variant.
    Enum(String, String),
    /// Class instance with type name and fields.
    Class(String, BamlMap),
}

impl BamlValue {
    /// Get the string value if this is a String.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            BamlValue::String(s) => Some(s),
            _ => None,
        }
    }

    /// Get the integer value if this is an Int.
    pub fn as_int(&self) -> Option<i64> {
        match self {
            BamlValue::Int(i) => Some(*i),
            _ => None,
        }
    }

    /// Get the float value if this is a Float.
    pub fn as_float(&self) -> Option<f64> {
        match self {
            BamlValue::Float(f) => Some(*f),
            _ => None,
        }
    }

    /// Get the boolean value if this is a Bool.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            BamlValue::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Check if this is null.
    pub fn is_null(&self) -> bool {
        matches!(self, BamlValue::Null)
    }

    /// Get the list if this is a List.
    pub fn as_list(&self) -> Option<&Vec<BamlValue>> {
        match self {
            BamlValue::List(l) => Some(l),
            _ => None,
        }
    }

    /// Get the map if this is a Map.
    pub fn as_map(&self) -> Option<&BamlMap> {
        match self {
            BamlValue::Map(m) => Some(m),
            _ => None,
        }
    }

    /// Get the type name of this value.
    pub fn type_name(&self) -> &'static str {
        match self {
            BamlValue::String(_) => "string",
            BamlValue::Int(_) => "int",
            BamlValue::Float(_) => "float",
            BamlValue::Bool(_) => "bool",
            BamlValue::Null => "null",
            BamlValue::List(_) => "list",
            BamlValue::Map(_) => "map",
            BamlValue::Media(_) => "media",
            BamlValue::Enum(_, _) => "enum",
            BamlValue::Class(_, _) => "class",
        }
    }
}

impl From<&str> for BamlValue {
    fn from(s: &str) -> Self {
        BamlValue::String(s.to_string())
    }
}

impl From<String> for BamlValue {
    fn from(s: String) -> Self {
        BamlValue::String(s)
    }
}

impl From<i64> for BamlValue {
    fn from(i: i64) -> Self {
        BamlValue::Int(i)
    }
}

impl From<f64> for BamlValue {
    fn from(f: f64) -> Self {
        BamlValue::Float(f)
    }
}

impl From<bool> for BamlValue {
    fn from(b: bool) -> Self {
        BamlValue::Bool(b)
    }
}

impl fmt::Display for BamlValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BamlValue::String(s) => write!(f, "\"{}\"", s),
            BamlValue::Int(i) => write!(f, "{}", i),
            BamlValue::Float(fl) => write!(f, "{}", fl),
            BamlValue::Bool(b) => write!(f, "{}", b),
            BamlValue::Null => write!(f, "null"),
            BamlValue::List(l) => {
                write!(f, "[")?;
                for (i, v) in l.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", v)?;
                }
                write!(f, "]")
            }
            BamlValue::Map(m) => {
                write!(f, "{{")?;
                for (i, (k, v)) in m.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "\"{}\": {}", k, v)?;
                }
                write!(f, "}}")
            }
            BamlValue::Media(m) => write!(f, "<media:{}>", m.media_type),
            BamlValue::Enum(t, v) => write!(f, "{}::{}", t, v),
            BamlValue::Class(t, _) => write!(f, "{} {{ ... }}", t),
        }
    }
}

/// BamlValue with additional metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BamlValueWithMeta<T> {
    /// The value.
    pub value: BamlValue,
    /// Additional metadata.
    pub meta: T,
}

// ============================================================================
// Media types
// ============================================================================

/// Media type enumeration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BamlMediaType {
    /// Image media.
    Image,
    /// Audio media.
    Audio,
    /// Video media.
    Video,
    /// PDF document.
    Pdf,
}

impl fmt::Display for BamlMediaType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BamlMediaType::Image => write!(f, "image"),
            BamlMediaType::Audio => write!(f, "audio"),
            BamlMediaType::Video => write!(f, "video"),
            BamlMediaType::Pdf => write!(f, "pdf"),
        }
    }
}

/// URL-based media content.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BamlMediaUrl {
    /// The URL of the media.
    pub url: String,
    /// Optional MIME type.
    pub media_type: Option<String>,
}

/// Base64-encoded media content.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BamlMediaBase64 {
    /// Base64-encoded data.
    pub base64: String,
    /// MIME type of the media.
    pub media_type: String,
}

/// File-based media content.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BamlMediaFile {
    /// Path to the file.
    pub path: String,
    /// Optional MIME type.
    pub media_type: Option<String>,
}

/// Media content - can be URL, base64, or file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum BamlMediaContent {
    /// URL-based media.
    Url(BamlMediaUrl),
    /// Base64-encoded media.
    Base64(BamlMediaBase64),
    /// File-based media.
    File(BamlMediaFile),
}

/// Media value with type and content.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BamlMedia {
    /// The type of media (image, audio, etc.).
    pub media_type: BamlMediaType,
    /// The media content.
    pub content: BamlMediaContent,
}

impl BamlMedia {
    /// Create a URL-based media value.
    pub fn url(media_type: BamlMediaType, url: impl Into<String>) -> Self {
        Self {
            media_type,
            content: BamlMediaContent::Url(BamlMediaUrl {
                url: url.into(),
                media_type: None,
            }),
        }
    }

    /// Create a base64-encoded media value.
    pub fn base64(media_type: BamlMediaType, base64: impl Into<String>, mime_type: impl Into<String>) -> Self {
        Self {
            media_type,
            content: BamlMediaContent::Base64(BamlMediaBase64 {
                base64: base64.into(),
                media_type: mime_type.into(),
            }),
        }
    }

    /// Create a file-based media value.
    pub fn file(media_type: BamlMediaType, path: impl Into<String>) -> Self {
        Self {
            media_type,
            content: BamlMediaContent::File(BamlMediaFile {
                path: path.into(),
                media_type: None,
            }),
        }
    }
}

// ============================================================================
// Completion state (for streaming)
// ============================================================================

/// Completion state for streaming values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CompletionState {
    /// The value is complete.
    Complete,
    /// The value is still being streamed.
    Incomplete,
}

impl Default for CompletionState {
    fn default() -> Self {
        CompletionState::Complete
    }
}

// ============================================================================
// Constraints
// ============================================================================

/// Level of a constraint (assert vs check).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ConstraintLevel {
    /// Assert - failure is an error.
    Assert,
    /// Check - failure is a warning.
    Check,
}

/// A constraint on a type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Constraint {
    /// The constraint level.
    pub level: ConstraintLevel,
    /// The constraint expression (as a string for now).
    pub expression: String,
    /// Optional name for the constraint.
    pub name: Option<String>,
}

/// Result of checking a response constraint.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseCheck {
    /// Name of the check.
    pub name: String,
    /// Expression that was evaluated.
    pub expression: String,
    /// Whether the check passed.
    pub passed: bool,
    /// Optional message.
    pub message: Option<String>,
}

// ============================================================================
// Jinja Expression
// ============================================================================

/// A Jinja expression for dynamic evaluation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JinjaExpression(pub String);

impl JinjaExpression {
    /// Create a new Jinja expression.
    pub fn new(expression: impl Into<String>) -> Self {
        Self(expression.into())
    }

    /// Get the expression string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

// ============================================================================
// StreamingMode
// ============================================================================

/// Streaming mode for type definitions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum StreamingMode {
    /// Non-streaming mode (default).
    #[default]
    NonStreaming,
    /// Streaming mode - fields may be partial.
    Streaming,
}

// ============================================================================
// Type IR - Intermediate representation for types
// ============================================================================

/// Primitive type values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TypeValue {
    /// String type.
    String,
    /// Integer type.
    Int,
    /// Float type.
    Float,
    /// Boolean type.
    Bool,
    /// Null type.
    Null,
    /// Media type with specific media kind.
    Media(MediaTypeValue),
}

/// Media type value for primitives.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MediaTypeValue {
    /// Image media.
    Image,
    /// Audio media.
    Audio,
    /// Video media.
    Video,
    /// PDF document.
    Pdf,
}

/// Literal value for literal types.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LiteralValue {
    /// String literal.
    String(String),
    /// Integer literal.
    Int(i64),
    /// Boolean literal.
    Bool(bool),
    /// Float literal (stored as string for precision).
    Float(String),
}

impl Eq for LiteralValue {}

impl std::hash::Hash for LiteralValue {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match self {
            LiteralValue::String(s) => s.hash(state),
            LiteralValue::Int(i) => i.hash(state),
            LiteralValue::Bool(b) => b.hash(state),
            LiteralValue::Float(f) => f.hash(state),
        }
    }
}

/// Metadata for a type.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TypeMeta {
    /// Constraints on the type.
    pub constraints: Vec<Constraint>,
    /// Whether this type is dynamic.
    pub is_dynamic: bool,
}

/// Type intermediate representation.
///
/// This represents BAML types at runtime for coercion and validation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TypeIR {
    /// Primitive type.
    Primitive(TypeValue, TypeMeta),
    /// Literal type (exact value required).
    Literal(LiteralValue, TypeMeta),
    /// Optional type (nullable).
    Optional(Box<TypeIR>, TypeMeta),
    /// List type.
    List(Box<TypeIR>, TypeMeta),
    /// Map type (key must be string).
    Map(Box<TypeIR>, Box<TypeIR>, TypeMeta),
    /// Union type (one of multiple types).
    Union(Vec<TypeIR>, TypeMeta),
    /// Class type (by name).
    Class(String, TypeMeta),
    /// Enum type (by name).
    Enum(String, TypeMeta),
    /// Type alias.
    Alias(String, Box<TypeIR>, TypeMeta),
    /// Type with additional constraints.
    Constrained {
        /// Base type.
        base: Box<TypeIR>,
        /// Constraints to apply.
        constraints: Vec<Constraint>,
        /// Additional metadata.
        meta: TypeMeta,
    },
}

impl TypeIR {
    /// Create a string type.
    pub fn string() -> Self {
        TypeIR::Primitive(TypeValue::String, TypeMeta::default())
    }

    /// Create an int type.
    pub fn int() -> Self {
        TypeIR::Primitive(TypeValue::Int, TypeMeta::default())
    }

    /// Create a float type.
    pub fn float() -> Self {
        TypeIR::Primitive(TypeValue::Float, TypeMeta::default())
    }

    /// Create a bool type.
    pub fn bool() -> Self {
        TypeIR::Primitive(TypeValue::Bool, TypeMeta::default())
    }

    /// Create a null type.
    pub fn null() -> Self {
        TypeIR::Primitive(TypeValue::Null, TypeMeta::default())
    }

    /// Create an optional type.
    pub fn optional(inner: TypeIR) -> Self {
        TypeIR::Optional(Box::new(inner), TypeMeta::default())
    }

    /// Create a list type.
    pub fn list(element: TypeIR) -> Self {
        TypeIR::List(Box::new(element), TypeMeta::default())
    }

    /// Create a map type.
    pub fn map(key: TypeIR, value: TypeIR) -> Self {
        TypeIR::Map(Box::new(key), Box::new(value), TypeMeta::default())
    }

    /// Create a union type.
    pub fn union(variants: Vec<TypeIR>) -> Self {
        TypeIR::Union(variants, TypeMeta::default())
    }

    /// Create a class type.
    pub fn class(name: impl Into<String>) -> Self {
        TypeIR::Class(name.into(), TypeMeta::default())
    }

    /// Create an enum type.
    pub fn r#enum(name: impl Into<String>) -> Self {
        TypeIR::Enum(name.into(), TypeMeta::default())
    }

    /// Get the metadata for this type.
    pub fn meta(&self) -> &TypeMeta {
        match self {
            TypeIR::Primitive(_, m) => m,
            TypeIR::Literal(_, m) => m,
            TypeIR::Optional(_, m) => m,
            TypeIR::List(_, m) => m,
            TypeIR::Map(_, _, m) => m,
            TypeIR::Union(_, m) => m,
            TypeIR::Class(_, m) => m,
            TypeIR::Enum(_, m) => m,
            TypeIR::Alias(_, _, m) => m,
            TypeIR::Constrained { meta, .. } => meta,
        }
    }

    /// Check if this is a string type.
    pub fn is_string(&self) -> bool {
        matches!(self, TypeIR::Primitive(TypeValue::String, _))
    }

    /// Check if this is an int type.
    pub fn is_int(&self) -> bool {
        matches!(self, TypeIR::Primitive(TypeValue::Int, _))
    }

    /// Check if this is a float type.
    pub fn is_float(&self) -> bool {
        matches!(self, TypeIR::Primitive(TypeValue::Float, _))
    }

    /// Check if this is a bool type.
    pub fn is_bool(&self) -> bool {
        matches!(self, TypeIR::Primitive(TypeValue::Bool, _))
    }

    /// Check if this is a null type.
    pub fn is_null(&self) -> bool {
        matches!(self, TypeIR::Primitive(TypeValue::Null, _))
    }
}

// ============================================================================
// Legacy stub types (for compatibility)
// ============================================================================

/// Placeholder for type reference.
/// Will be replaced by actual HIR/TIR type reference.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeRef {
    /// Type name for display/debugging
    pub name: String,
}

impl TypeRef {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }

    pub fn string() -> Self {
        Self::new("string")
    }

    pub fn int() -> Self {
        Self::new("int")
    }

    pub fn float() -> Self {
        Self::new("float")
    }

    pub fn bool() -> Self {
        Self::new("bool")
    }
}

/// Placeholder for function definition.
/// Will be replaced by actual HIR/TIR function definition.
#[derive(Debug, Clone)]
pub struct FunctionDef {
    pub name: String,
    pub params: Vec<ParamDef>,
    pub output_type: TypeRef,
    pub client_spec: ClientSpec,
    pub prompt_template: PromptTemplate,
}

/// Function parameter definition.
#[derive(Debug, Clone)]
pub struct ParamDef {
    pub name: String,
    pub param_type: TypeRef,
}

/// Placeholder for client specification.
/// Will be replaced by actual client configuration from HIR/TIR.
#[derive(Debug, Clone)]
pub struct ClientSpec {
    pub client_name: String,
}

impl ClientSpec {
    pub fn new(client_name: impl Into<String>) -> Self {
        Self {
            client_name: client_name.into(),
        }
    }
}

/// Placeholder for prompt template.
/// Will be replaced by actual prompt template from HIR/TIR.
#[derive(Debug, Clone)]
pub struct PromptTemplate {
    pub template: String,
}

impl PromptTemplate {
    pub fn new(template: impl Into<String>) -> Self {
        Self {
            template: template.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_baml_value_string() {
        let v = BamlValue::from("hello");
        assert_eq!(v.as_str(), Some("hello"));
    }

    #[test]
    fn test_baml_value_int() {
        let v = BamlValue::from(42i64);
        assert_eq!(v.as_int(), Some(42));
    }

    #[test]
    fn test_baml_value_map() {
        let mut map = BamlMap::new();
        map.insert("key".to_string(), BamlValue::from("value"));
        let v = BamlValue::Map(map);

        if let BamlValue::Map(m) = v {
            assert_eq!(m.get("key").and_then(|v| v.as_str()), Some("value"));
        } else {
            panic!("Expected Map");
        }
    }

    #[test]
    fn test_type_ir_primitives() {
        let string_type = TypeIR::string();
        assert!(matches!(string_type, TypeIR::Primitive(TypeValue::String, _)));

        let int_type = TypeIR::int();
        assert!(matches!(int_type, TypeIR::Primitive(TypeValue::Int, _)));
    }

    #[test]
    fn test_type_ir_optional() {
        let optional = TypeIR::optional(TypeIR::string());
        assert!(matches!(optional, TypeIR::Optional(_, _)));
    }

    #[test]
    fn test_type_ir_list() {
        let list = TypeIR::list(TypeIR::int());
        assert!(matches!(list, TypeIR::List(_, _)));
    }

    #[test]
    fn test_media_url() {
        let media = BamlMedia::url(BamlMediaType::Image, "https://example.com/image.png");
        assert_eq!(media.media_type, BamlMediaType::Image);
        assert!(matches!(media.content, BamlMediaContent::Url(_)));
    }

    #[test]
    fn test_completion_state() {
        assert_eq!(CompletionState::default(), CompletionState::Complete);
    }

    #[test]
    fn test_streaming_mode() {
        assert_eq!(StreamingMode::default(), StreamingMode::NonStreaming);
    }
}
