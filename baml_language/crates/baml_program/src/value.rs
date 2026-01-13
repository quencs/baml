//! Runtime value types for BAML.
//!
//! These types are moved from ir_stub as part of the runtime consolidation.

use std::{fmt, path::PathBuf};

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::program::MediaKind;

// ============================================================================
// BamlMap - Type alias for ordered map
// ============================================================================

/// Ordered map from string keys to BamlValue.
pub type BamlMap = IndexMap<String, BamlValue>;

// ============================================================================
// BamlValue - Runtime value type
// ============================================================================

/// A BAML runtime value.
///
/// Used for function arguments, return values, test args.
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
            BamlValue::Media(m) => write!(f, "<media:{:?}>", m.media_type),
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
// Media types (simplified per plan doc)
// ============================================================================

/// Media value with type and content.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BamlMedia {
    /// The type of media (image, audio, etc.).
    pub media_type: MediaKind,
    /// The media content.
    pub content: MediaContent,
}

/// Media content - can be URL, base64, or file path.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MediaContent {
    /// URL-based media.
    Url(String),
    /// Base64-encoded media.
    Base64(String),
    /// File-based media.
    File(PathBuf),
}

impl BamlMedia {
    /// Create a URL-based media value.
    pub fn url(media_type: MediaKind, url: impl Into<String>) -> Self {
        Self {
            media_type,
            content: MediaContent::Url(url.into()),
        }
    }

    /// Create a base64-encoded media value.
    pub fn base64(media_type: MediaKind, base64: impl Into<String>) -> Self {
        Self {
            media_type,
            content: MediaContent::Base64(base64.into()),
        }
    }

    /// Create a file-based media value.
    pub fn file(media_type: MediaKind, path: impl Into<PathBuf>) -> Self {
        Self {
            media_type,
            content: MediaContent::File(path.into()),
        }
    }
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
// Completion State (for streaming)
// ============================================================================

/// Completion state for streaming values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum CompletionState {
    /// The value is complete.
    #[default]
    Complete,
    /// The value is still being streamed.
    Incomplete,
}
