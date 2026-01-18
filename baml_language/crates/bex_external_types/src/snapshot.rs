//! Snapshot type for deep-copied value trees.
//!
//! `Snapshot` is a fully-owned value tree with no heap references.
//! Use when you need to traverse and convert the entire object graph,
//! such as for FFI conversion to Python/JS objects.
//!
//! Unlike `ExternalValue` which uses `Handle` for heap objects,
//! `Snapshot` contains owned copies of all data.

use indexmap::IndexMap;

/// A deep-copied value tree with no heap references.
///
/// Use `BexEngine::snapshot(&handle)` to create a Snapshot from a Handle.
/// This recursively copies the entire object graph into owned Rust types.
///
/// # When to use Snapshot vs ExternalValue
///
/// - **ExternalValue**: When you want to keep data in the heap and access lazily.
///   Good for passing handles across FFI without copying.
///
/// - **Snapshot**: When you need to convert the entire value to another format
///   (Python objects, JSON, etc.). Since you're traversing anyway, might as
///   well have owned data.
///
/// # Example
///
/// ```ignore
/// // Get handle from function call
/// let result = engine.call_function("get_user", &[]).await?;
///
/// if let ExternalValue::Object(handle) = result {
///     // Deep copy for Python conversion
///     let snapshot = engine.snapshot(&handle)?;
///     let py_obj = snapshot_to_python(py, snapshot)?;
/// }
/// ```
#[derive(Clone, Debug, PartialEq, Default)]
pub enum Snapshot {
    /// Null value.
    #[default]
    Null,

    /// 64-bit signed integer.
    Int(i64),

    /// 64-bit floating point.
    Float(f64),

    /// Boolean value.
    Bool(bool),

    /// Owned string.
    String(String),

    /// Owned array of snapshots.
    Array(Vec<Snapshot>),

    /// Owned map with string keys.
    Map(IndexMap<String, Snapshot>),

    /// Class instance with class name and field values.
    Instance {
        class_name: String,
        fields: IndexMap<String, Snapshot>,
    },

    /// Enum variant with enum name and variant name.
    Variant {
        enum_name: String,
        variant_name: String,
    },
}

impl Snapshot {
    /// Check if this value is null.
    pub fn is_null(&self) -> bool {
        matches!(self, Snapshot::Null)
    }

    /// Try to get as an integer.
    pub fn as_int(&self) -> Option<i64> {
        match self {
            Snapshot::Int(i) => Some(*i),
            _ => None,
        }
    }

    /// Try to get as a float.
    pub fn as_float(&self) -> Option<f64> {
        match self {
            Snapshot::Float(f) => Some(*f),
            _ => None,
        }
    }

    /// Try to get as a boolean.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Snapshot::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Try to get as a string reference.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Snapshot::String(s) => Some(s),
            _ => None,
        }
    }

    /// Try to get as an array reference.
    pub fn as_array(&self) -> Option<&[Snapshot]> {
        match self {
            Snapshot::Array(arr) => Some(arr),
            _ => None,
        }
    }

    /// Try to get as a map reference.
    pub fn as_map(&self) -> Option<&IndexMap<String, Snapshot>> {
        match self {
            Snapshot::Map(map) => Some(map),
            _ => None,
        }
    }

    /// Get the type name for error messages.
    pub fn type_name(&self) -> &'static str {
        match self {
            Snapshot::Null => "null",
            Snapshot::Int(_) => "int",
            Snapshot::Float(_) => "float",
            Snapshot::Bool(_) => "bool",
            Snapshot::String(_) => "string",
            Snapshot::Array(_) => "array",
            Snapshot::Map(_) => "map",
            Snapshot::Instance { .. } => "instance",
            Snapshot::Variant { .. } => "variant",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snapshot_primitives() {
        assert!(Snapshot::Null.is_null());
        assert_eq!(Snapshot::Int(42).as_int(), Some(42));
        assert_eq!(Snapshot::Float(1.23).as_float(), Some(1.23));
        assert_eq!(Snapshot::Bool(true).as_bool(), Some(true));
        assert_eq!(Snapshot::String("hello".into()).as_str(), Some("hello"));
    }

    #[test]
    fn test_snapshot_array() {
        let arr = Snapshot::Array(vec![
            Snapshot::Int(1),
            Snapshot::Int(2),
            Snapshot::String("three".into()),
        ]);

        let items = arr.as_array().unwrap();
        assert_eq!(items.len(), 3);
        assert_eq!(items[0].as_int(), Some(1));
        assert_eq!(items[2].as_str(), Some("three"));
    }

    #[test]
    fn test_snapshot_map() {
        let mut map = IndexMap::new();
        map.insert("name".to_string(), Snapshot::String("Alice".into()));
        map.insert("age".to_string(), Snapshot::Int(30));

        let snapshot = Snapshot::Map(map);
        let m = snapshot.as_map().unwrap();

        assert_eq!(m.get("name").and_then(|v| v.as_str()), Some("Alice"));
        assert_eq!(m.get("age").and_then(|v| v.as_int()), Some(30));
    }

    #[test]
    fn test_snapshot_instance() {
        let mut fields = IndexMap::new();
        fields.insert("x".to_string(), Snapshot::Int(10));
        fields.insert("y".to_string(), Snapshot::Int(20));

        let snapshot = Snapshot::Instance {
            class_name: "Point".to_string(),
            fields,
        };

        assert_eq!(snapshot.type_name(), "instance");
    }

    #[test]
    fn test_snapshot_variant() {
        let snapshot = Snapshot::Variant {
            enum_name: "Color".to_string(),
            variant_name: "Red".to_string(),
        };

        assert_eq!(snapshot.type_name(), "variant");
    }

    #[test]
    fn test_snapshot_equality() {
        assert_eq!(Snapshot::Null, Snapshot::Null);
        assert_eq!(Snapshot::Int(42), Snapshot::Int(42));
        assert_eq!(
            Snapshot::String("hello".into()),
            Snapshot::String("hello".into())
        );
        assert_ne!(Snapshot::Int(42), Snapshot::Float(42.0));
    }
}
