//! ExternalValue type for FFI boundary.
//!
//! `ExternalValue` is a self-contained value type that can be passed
//! across the FFI boundary without requiring heap access to inspect.
//! Primitives are inlined, heap objects use opaque `Handle`.

use crate::{Handle, Snapshot};

/// A value that can cross the FFI boundary.
///
/// Mirrors internal `Value` - primitives are inlined, heap objects use opaque `Handle`.
/// To inspect a heap object, resolve the handle via `BexEngine::get_object()`.
///
/// # Design
///
/// - Primitives (`Null`, `Int`, `Float`, `Bool`) are inlined (Copy, no heap access)
/// - All heap objects (strings, arrays, maps, instances, variants) use `Object(Handle)`
/// - Type information lives in the heap, not in ExternalValue
///
/// # Example
///
/// ```ignore
/// let result: ExternalValue = engine.call_function("get_user", &[]).await?;
///
/// match result {
///     ExternalValue::Int(n) => println!("Got int: {}", n),
///     ExternalValue::Object(handle) => {
///         // Resolve handle to inspect the object
///         match engine.get_object(&handle)? {
///             Object::String(s) => println!("Got string: {}", s),
///             Object::Array(arr) => println!("Got array of {} items", arr.len()),
///             Object::Instance(inst) => { /* access fields */ }
///             _ => {}
///         }
///     }
///     _ => {}
/// }
/// ```
#[derive(Clone, Debug, Default)]
pub enum ExternalValue {
    /// Null value.
    #[default]
    Null,

    /// 64-bit signed integer.
    Int(i64),

    /// 64-bit floating point.
    Float(f64),

    /// Boolean value.
    Bool(bool),

    /// Handle to any heap-allocated object (string, array, map, instance, variant, etc.).
    /// Resolve via `BexEngine::get_object()` to inspect.
    Object(Handle),

    /// Owned data to be allocated on the heap when passed to `call_function`.
    ///
    /// Use this variant when you want to pass complex data (strings, arrays, maps)
    /// to a function without needing to pre-allocate handles. The engine will
    /// allocate the snapshot data onto the heap when processing arguments.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use bex_external_types::{ExternalValue, Snapshot};
    ///
    /// // Pass a string argument
    /// let args = vec![
    ///     ExternalValue::Snapshot(Snapshot::String("hello".into())),
    ///     ExternalValue::Int(42),
    /// ];
    /// let result = engine.call_function("my_func", &args).await?;
    /// ```
    Snapshot(Snapshot),
}

impl ExternalValue {
    /// Check if this value is null.
    pub fn is_null(&self) -> bool {
        matches!(self, ExternalValue::Null)
    }

    /// Try to get as an integer.
    pub fn as_int(&self) -> Option<i64> {
        match self {
            ExternalValue::Int(i) => Some(*i),
            _ => None,
        }
    }

    /// Try to get as a float.
    pub fn as_float(&self) -> Option<f64> {
        match self {
            ExternalValue::Float(f) => Some(*f),
            _ => None,
        }
    }

    /// Try to get as a boolean.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            ExternalValue::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Try to get as an object handle.
    pub fn as_object(&self) -> Option<&Handle> {
        match self {
            ExternalValue::Object(handle) => Some(handle),
            _ => None,
        }
    }

    /// Get the type name for error messages.
    ///
    /// For `Object` variants, returns "object" - resolve the handle
    /// to get the specific type (string, array, map, instance, etc.).
    pub fn type_name(&self) -> &'static str {
        match self {
            ExternalValue::Null => "null",
            ExternalValue::Int(_) => "int",
            ExternalValue::Float(_) => "float",
            ExternalValue::Bool(_) => "bool",
            ExternalValue::Object(_) => "object",
            ExternalValue::Snapshot(s) => s.type_name(),
        }
    }

    /// Try to get as a snapshot reference.
    pub fn as_snapshot(&self) -> Option<&Snapshot> {
        match self {
            ExternalValue::Snapshot(s) => Some(s),
            _ => None,
        }
    }
}

impl From<i64> for ExternalValue {
    fn from(value: i64) -> Self {
        ExternalValue::Int(value)
    }
}

impl From<f64> for ExternalValue {
    fn from(value: f64) -> Self {
        ExternalValue::Float(value)
    }
}

impl From<bool> for ExternalValue {
    fn from(value: bool) -> Self {
        ExternalValue::Bool(value)
    }
}

impl From<Handle> for ExternalValue {
    fn from(value: Handle) -> Self {
        ExternalValue::Object(value)
    }
}

impl From<Snapshot> for ExternalValue {
    fn from(value: Snapshot) -> Self {
        ExternalValue::Snapshot(value)
    }
}

impl From<String> for ExternalValue {
    fn from(value: String) -> Self {
        ExternalValue::Snapshot(Snapshot::String(value))
    }
}

impl From<&str> for ExternalValue {
    fn from(value: &str) -> Self {
        ExternalValue::Snapshot(Snapshot::String(value.to_string()))
    }
}

impl PartialEq for ExternalValue {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (ExternalValue::Null, ExternalValue::Null) => true,
            (ExternalValue::Int(a), ExternalValue::Int(b)) => a == b,
            (ExternalValue::Float(a), ExternalValue::Float(b)) => a == b,
            (ExternalValue::Bool(a), ExternalValue::Bool(b)) => a == b,
            // Object handles compare by slab_key (identity)
            (ExternalValue::Object(a), ExternalValue::Object(b)) => a.slab_key() == b.slab_key(),
            // Snapshots compare by value
            (ExternalValue::Snapshot(a), ExternalValue::Snapshot(b)) => a == b,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use bex_vm_types::ObjectIndex;

    use super::*;

    #[test]
    fn test_external_value_primitives() {
        assert!(ExternalValue::Null.is_null());
        assert_eq!(ExternalValue::Int(42).as_int(), Some(42));
        assert_eq!(ExternalValue::Float(1.23).as_float(), Some(1.23));
        assert_eq!(ExternalValue::Bool(true).as_bool(), Some(true));
    }

    #[test]
    fn test_external_value_from() {
        let v: ExternalValue = 42i64.into();
        assert_eq!(v.as_int(), Some(42));

        let v: ExternalValue = 1.23f64.into();
        assert_eq!(v.as_float(), Some(1.23));

        let v: ExternalValue = true.into();
        assert_eq!(v.as_bool(), Some(true));
    }

    #[test]
    fn test_external_value_object() {
        let handle = Handle::new_detached(42, ObjectIndex::from_raw(100));
        let v = ExternalValue::Object(handle);

        assert!(v.as_object().is_some());
        assert_eq!(v.as_object().unwrap().slab_key(), 42);
    }

    #[test]
    fn test_external_value_type_name() {
        assert_eq!(ExternalValue::Null.type_name(), "null");
        assert_eq!(ExternalValue::Int(0).type_name(), "int");
        assert_eq!(ExternalValue::Float(0.0).type_name(), "float");
        assert_eq!(ExternalValue::Bool(false).type_name(), "bool");

        let handle = Handle::new_detached(0, ObjectIndex::from_raw(0));
        assert_eq!(ExternalValue::Object(handle).type_name(), "object");
    }

    #[test]
    fn test_external_value_equality() {
        assert_eq!(ExternalValue::Null, ExternalValue::Null);
        assert_eq!(ExternalValue::Int(42), ExternalValue::Int(42));
        assert_ne!(ExternalValue::Int(42), ExternalValue::Int(43));
        assert_ne!(ExternalValue::Int(42), ExternalValue::Float(42.0));
    }

    #[test]
    fn test_external_value_default() {
        let v: ExternalValue = Default::default();
        assert!(v.is_null());
    }
}
