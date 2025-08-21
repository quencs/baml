//! Core BAML types for the Rust client
//! 
//! This module provides the type system used by BAML functions.

// No additional imports needed for basic type conversions

// Re-export BamlValue and BamlMap from baml-types to maintain compatibility
pub use baml_types::{BamlValue, BamlMap};

/// Convert a Rust value to a BAML value
pub trait ToBamlValue {
    /// Convert self to a BamlValue
    fn to_baml_value(self) -> crate::BamlResult<BamlValue>;
}

/// Convert a BAML value to a Rust type
pub trait FromBamlValue: Sized {
    /// Try to convert a BamlValue to Self
    fn from_baml_value(value: BamlValue) -> crate::BamlResult<Self>;
}

// Implementations for common types
impl ToBamlValue for String {
    fn to_baml_value(self) -> crate::BamlResult<BamlValue> {
        Ok(BamlValue::String(self))
    }
}

impl ToBamlValue for &str {
    fn to_baml_value(self) -> crate::BamlResult<BamlValue> {
        Ok(BamlValue::String(self.to_string()))
    }
}

impl FromBamlValue for String {
    fn from_baml_value(value: BamlValue) -> crate::BamlResult<Self> {
        match value {
            BamlValue::String(s) => Ok(s),
            _ => Err(crate::BamlError::deserialization(format!("Expected string, got {:?}", value))),
        }
    }
}

impl ToBamlValue for i32 {
    fn to_baml_value(self) -> crate::BamlResult<BamlValue> {
        Ok(BamlValue::Int(self as i64))
    }
}

impl FromBamlValue for i32 {
    fn from_baml_value(value: BamlValue) -> crate::BamlResult<Self> {
        match value {
            BamlValue::Int(i) => i.try_into().map_err(|_| crate::BamlError::deserialization("Integer overflow".to_string())),
            _ => Err(crate::BamlError::deserialization(format!("Expected int, got {:?}", value))),
        }
    }
}

impl ToBamlValue for i64 {
    fn to_baml_value(self) -> crate::BamlResult<BamlValue> {
        Ok(BamlValue::Int(self))
    }
}

impl FromBamlValue for i64 {
    fn from_baml_value(value: BamlValue) -> crate::BamlResult<Self> {
        match value {
            BamlValue::Int(i) => Ok(i),
            _ => Err(crate::BamlError::deserialization(format!("Expected int, got {:?}", value))),
        }
    }
}

impl ToBamlValue for f64 {
    fn to_baml_value(self) -> crate::BamlResult<BamlValue> {
        Ok(BamlValue::Float(self))
    }
}

impl FromBamlValue for f64 {
    fn from_baml_value(value: BamlValue) -> crate::BamlResult<Self> {
        match value {
            BamlValue::Float(f) => Ok(f),
            BamlValue::Int(i) => Ok(i as f64),
            _ => Err(crate::BamlError::deserialization(format!("Expected float, got {:?}", value))),
        }
    }
}

impl ToBamlValue for bool {
    fn to_baml_value(self) -> crate::BamlResult<BamlValue> {
        Ok(BamlValue::Bool(self))
    }
}

impl FromBamlValue for bool {
    fn from_baml_value(value: BamlValue) -> crate::BamlResult<Self> {
        match value {
            BamlValue::Bool(b) => Ok(b),
            _ => Err(crate::BamlError::deserialization(format!("Expected bool, got {:?}", value))),
        }
    }
}

impl<T: ToBamlValue> ToBamlValue for Vec<T> {
    fn to_baml_value(self) -> crate::BamlResult<BamlValue> {
        let values: Result<Vec<_>, _> = self.into_iter().map(|v| v.to_baml_value()).collect();
        Ok(BamlValue::List(values?))
    }
}

impl<T: FromBamlValue> FromBamlValue for Vec<T> {
    fn from_baml_value(value: BamlValue) -> crate::BamlResult<Self> {
        match value {
            BamlValue::List(list) => {
                list.into_iter()
                    .map(T::from_baml_value)
                    .collect::<Result<Vec<_>, _>>()
            }
            _ => Err(crate::BamlError::deserialization(format!("Expected list, got {:?}", value))),
        }
    }
}

impl<T: ToBamlValue> ToBamlValue for Option<T> {
    fn to_baml_value(self) -> crate::BamlResult<BamlValue> {
        match self {
            Some(value) => value.to_baml_value(),
            None => Ok(BamlValue::Null),
        }
    }
}

impl<T: FromBamlValue> FromBamlValue for Option<T> {
    fn from_baml_value(value: BamlValue) -> crate::BamlResult<Self> {
        match value {
            BamlValue::Null => Ok(None),
            other => Ok(Some(T::from_baml_value(other)?)),
        }
    }
}

impl ToBamlValue for BamlMap<String, BamlValue> {
    fn to_baml_value(self) -> crate::BamlResult<BamlValue> {
        Ok(BamlValue::Map(self))
    }
}

impl FromBamlValue for BamlMap<String, BamlValue> {
    fn from_baml_value(value: BamlValue) -> crate::BamlResult<Self> {
        match value {
            BamlValue::Map(map) => Ok(map),
            _ => Err(crate::BamlError::deserialization(format!("Expected map, got {:?}", value))),
        }
    }
}

// Stub implementations for BAML runtime components we're no longer using directly

/// Type builder for BAML types (stub implementation)
#[derive(Debug, Clone)]
pub struct TypeBuilder {
    // This is now just a placeholder - the real type building happens in the FFI layer
}

impl TypeBuilder {
    /// Create a new type builder
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for TypeBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Client registry for BAML clients (stub implementation)
#[derive(Debug, Clone)]
pub struct ClientRegistry {
    // This is now just a placeholder - the real client registry is in the FFI layer
}

impl ClientRegistry {
    /// Create a new client registry
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for ClientRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Collector for BAML tracing (stub implementation)
#[derive(Debug, Clone)]
pub struct Collector {
    // This is now just a placeholder - the real collector is in the FFI layer
}

impl Collector {
    /// Create a new collector
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for Collector {
    fn default() -> Self {
        Self::new()
    }
}

/// Runtime context manager (stub implementation)
#[derive(Debug, Clone)]
pub struct RuntimeContextManager {
    // This is now just a placeholder - the real context management is in the FFI layer
}

impl RuntimeContextManager {
    /// Create a new runtime context manager
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for RuntimeContextManager {
    fn default() -> Self {
        Self::new()
    }
}