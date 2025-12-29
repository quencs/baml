//! Unresolved type references in the HIR.
//!
//! These are type references before name resolution.
//! `TypeRef` -> Ty happens during THIR construction.

use baml_base::Name;

use crate::path::Path;

/// A type reference before name resolution.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TypeRef {
    /// Named type (with path for future module support).
    /// Examples:
    ///   `Path::single("User`") -> User
    ///   `Path::new`(`["users", "User"]`) -> `users::User` (future)
    Path(Path),

    /// Primitive types (no resolution needed).
    Int,
    Float,
    String,
    Bool,
    Null,
    Image,
    Audio,
    Video,
    Pdf,

    /// Type constructors.
    Optional(Box<TypeRef>),
    List(Box<TypeRef>),
    Map {
        key: Box<TypeRef>,
        value: Box<TypeRef>,
    },
    Union(Vec<TypeRef>),

    /// Literal types in unions.
    StringLiteral(String),
    IntLiteral(i64),
    /// Float literal stored as string to avoid f64's lack of Eq/Hash.
    FloatLiteral(String),
    /// Boolean literal for pattern matching (true/false as types).
    BoolLiteral(bool),

    /// Future: Generic type application.
    /// Example: Result<User, string>
    #[allow(dead_code)]
    Generic {
        base: Box<TypeRef>,
        args: Vec<TypeRef>,
    },

    /// Future: Type parameter reference.
    /// Example: T in `function<T>(x: T) -> T`
    #[allow(dead_code)]
    TypeParam(Name),

    /// Error sentinel.
    Error,

    /// Unknown/inferred.
    Unknown,
}

impl TypeRef {
    /// Create a simple named type reference.
    pub fn named(name: Name) -> Self {
        TypeRef::Path(Path::single(name))
    }

    /// Create an optional type.
    pub fn optional(inner: TypeRef) -> Self {
        TypeRef::Optional(Box::new(inner))
    }

    /// Create a list type.
    pub fn list(inner: TypeRef) -> Self {
        TypeRef::List(Box::new(inner))
    }

    /// Create a union type.
    pub fn union(types: Vec<TypeRef>) -> Self {
        TypeRef::Union(types)
    }

    /// Create a `TypeRef` from an AST `TypeExpr` node.
    ///
    /// This properly handles complex types including:
    /// - Primitives: int, string, bool, etc.
    /// - Named types: User, `MyClass`
    /// - Optional types: string?
    /// - List types: string[]
    /// - Union types: Success | Failure
    /// - String literal types: "user" | "assistant"
    ///
    /// NOTE: Type parsing occurs here, which is somewhat brittle for edge cases
    /// like `int??` or `int[][]`. See canary TODO for future improvements.
    pub fn from_ast(type_expr: &baml_syntax::ast::TypeExpr) -> Self {
        let parts = type_expr.parts();

        // If multiple parts, this is a union type
        if parts.len() > 1 {
            let members: Vec<TypeRef> = parts.iter().map(|p| Self::from_type_text(p)).collect();
            return TypeRef::Union(members);
        }

        // Single type (possibly with modifiers like ? or [])
        parts
            .first()
            .map(|p| Self::from_type_text(p))
            .unwrap_or(TypeRef::Unknown)
    }

    /// Create a `TypeRef` from a single type text (not a union).
    ///
    /// This handles:
    /// - String literal types: `"foo"` or `'bar'`
    /// - Array types: `int[]`
    /// - Optional types: `int?`
    /// - Generic types: `map<string, int>`
    /// - Boolean literal types: `true` or `false`
    /// - Integer literal types: `42`
    /// - Primitive types: `int`, `string`, etc.
    /// - Named types: `User`, `MyClass`
    pub(crate) fn from_type_text(text: &str) -> Self {
        // Check for string literal types like "user" or "assistant"
        if text.starts_with('"') && text.ends_with('"') {
            let inner = &text[1..text.len() - 1];
            return TypeRef::StringLiteral(inner.to_string());
        }

        // Check for array type (e.g., "int[]")
        if let Some(inner_text) = text.strip_suffix("[]") {
            let inner = Self::from_type_text(inner_text);
            return TypeRef::List(Box::new(inner));
        }

        // Check for optional type (e.g., "int?")
        if let Some(inner_text) = text.strip_suffix('?') {
            let inner = Self::from_type_text(inner_text);
            return TypeRef::Optional(Box::new(inner));
        }

        // Check for generic types like map<K, V>
        if let Some(result) = Self::try_parse_generic(text) {
            return result;
        }

        // Check for boolean literal types
        if text == "true" {
            return TypeRef::BoolLiteral(true);
        }
        if text == "false" {
            return TypeRef::BoolLiteral(false);
        }

        // Check for integer literal types (for exhaustiveness like 200 | 201)
        if let Ok(int_val) = text.parse::<i64>() {
            return TypeRef::IntLiteral(int_val);
        }

        // Detect numeric literals that failed parsing above:
        // - Integer overflow (e.g., "9...9" > i64::MAX)
        // - Float literals (e.g., "3.14")
        //
        // Without this check, these would fall through to `from_type_name` and
        // incorrectly become named types, causing confusing "unknown type" errors.
        //
        // TODO: Add spans to TypeRef to emit proper diagnostics instead of just Error.
        // See: https://github.com/BoundaryML/baml/pull/2838/files/1e6d23cc70e4825bfca302069caee658c7a0f437#r2634900737
        if text.starts_with(|c: char| c.is_ascii_digit()) {
            return TypeRef::Error;
        }

        Self::from_type_name(text)
    }

    /// Try to parse a generic type like `map<K, V>`.
    ///
    /// Returns `Some(TypeRef)` if successful, `None` if not a generic type.
    fn try_parse_generic(text: &str) -> Option<Self> {
        // Find the opening angle bracket
        let open_bracket = text.find('<')?;

        // Must end with '>'
        if !text.ends_with('>') {
            return None;
        }

        let base_name = text[..open_bracket].trim();
        let args_text = &text[open_bracket + 1..text.len() - 1];

        // Parse the type arguments, respecting nested angle brackets
        let args = Self::split_generic_args(args_text);

        match base_name.to_lowercase().as_str() {
            "map" => {
                if args.len() == 2 {
                    let key = Self::from_type_text(args[0].trim());
                    let value = Self::from_type_text(args[1].trim());
                    Some(TypeRef::Map {
                        key: Box::new(key),
                        value: Box::new(value),
                    })
                } else {
                    // Wrong number of type arguments for map
                    Some(TypeRef::Error)
                }
            }
            // Future: handle other generic types here (e.g., Result<T, E>)
            _ => {
                // Unknown generic type - treat as a named type for now
                // This preserves the original behavior
                Some(TypeRef::Path(Path::single(Name::new(text))))
            }
        }
    }

    /// Split generic type arguments by comma, respecting nested angle brackets.
    ///
    /// For example: `"string, map<int, bool>"` -> `["string", "map<int, bool>"]`
    fn split_generic_args(text: &str) -> Vec<&str> {
        let mut args = Vec::new();
        let mut depth = 0;
        let mut start = 0;

        for (i, c) in text.char_indices() {
            match c {
                '<' => depth += 1,
                '>' => depth -= 1,
                ',' if depth == 0 => {
                    args.push(&text[start..i]);
                    start = i + 1;
                }
                _ => {}
            }
        }

        // Don't forget the last argument
        if start < text.len() {
            args.push(&text[start..]);
        }

        args
    }

    /// Create a `TypeRef` from a type name string.
    fn from_type_name(name: &str) -> Self {
        match name.to_lowercase().as_str() {
            "int" => TypeRef::Int,
            "float" => TypeRef::Float,
            "string" => TypeRef::String,
            "bool" => TypeRef::Bool,
            "null" => TypeRef::Null,
            "image" => TypeRef::Image,
            "audio" => TypeRef::Audio,
            "video" => TypeRef::Video,
            "pdf" => TypeRef::Pdf,
            _ => TypeRef::Path(Path::single(Name::new(name))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_literal() {
        assert_eq!(
            TypeRef::from_type_text(r#""user""#),
            TypeRef::StringLiteral("user".to_string())
        );
    }

    #[test]
    fn test_optional_string_literal() {
        // Regression test: ensure "literal"? correctly produces Optional(StringLiteral)
        // The string literal check requires BOTH starts_with('"') AND ends_with('"').
        // For `"user"?`, ends_with('"') is false, so we fall through to optional check.
        assert_eq!(
            TypeRef::from_type_text(r#""user"?"#),
            TypeRef::Optional(Box::new(TypeRef::StringLiteral("user".to_string())))
        );
    }

    #[test]
    fn test_array_of_string_literal() {
        assert_eq!(
            TypeRef::from_type_text(r#""user"[]"#),
            TypeRef::List(Box::new(TypeRef::StringLiteral("user".to_string())))
        );
    }

    #[test]
    fn test_optional_array_of_string_literal() {
        // "user"[]? -> Optional(List(StringLiteral("user")))
        assert_eq!(
            TypeRef::from_type_text(r#""user"[]?"#),
            TypeRef::Optional(Box::new(TypeRef::List(Box::new(TypeRef::StringLiteral(
                "user".to_string()
            )))))
        );
    }

    #[test]
    fn test_optional_int_literal() {
        assert_eq!(
            TypeRef::from_type_text("200?"),
            TypeRef::Optional(Box::new(TypeRef::IntLiteral(200)))
        );
    }

    #[test]
    fn test_optional_bool_literal() {
        assert_eq!(
            TypeRef::from_type_text("true?"),
            TypeRef::Optional(Box::new(TypeRef::BoolLiteral(true)))
        );
    }

    #[test]
    fn test_primitives() {
        assert_eq!(TypeRef::from_type_text("int"), TypeRef::Int);
        assert_eq!(TypeRef::from_type_text("string"), TypeRef::String);
        assert_eq!(TypeRef::from_type_text("bool"), TypeRef::Bool);
    }

    #[test]
    fn test_optional_primitive() {
        assert_eq!(
            TypeRef::from_type_text("int?"),
            TypeRef::Optional(Box::new(TypeRef::Int))
        );
    }

    #[test]
    fn test_array_of_primitive() {
        assert_eq!(
            TypeRef::from_type_text("int[]"),
            TypeRef::List(Box::new(TypeRef::Int))
        );
    }

    #[test]
    fn test_map_simple() {
        assert_eq!(
            TypeRef::from_type_text("map<string, int>"),
            TypeRef::Map {
                key: Box::new(TypeRef::String),
                value: Box::new(TypeRef::Int),
            }
        );
    }

    #[test]
    fn test_map_with_bool_value() {
        assert_eq!(
            TypeRef::from_type_text("map<string, bool>"),
            TypeRef::Map {
                key: Box::new(TypeRef::String),
                value: Box::new(TypeRef::Bool),
            }
        );
    }

    #[test]
    fn test_map_nested() {
        // map<string, map<int, bool>>
        assert_eq!(
            TypeRef::from_type_text("map<string, map<int, bool>>"),
            TypeRef::Map {
                key: Box::new(TypeRef::String),
                value: Box::new(TypeRef::Map {
                    key: Box::new(TypeRef::Int),
                    value: Box::new(TypeRef::Bool),
                }),
            }
        );
    }

    #[test]
    fn test_map_optional() {
        // map<string, int>?
        assert_eq!(
            TypeRef::from_type_text("map<string, int>?"),
            TypeRef::Optional(Box::new(TypeRef::Map {
                key: Box::new(TypeRef::String),
                value: Box::new(TypeRef::Int),
            }))
        );
    }

    #[test]
    fn test_map_array() {
        // map<string, int>[]
        assert_eq!(
            TypeRef::from_type_text("map<string, int>[]"),
            TypeRef::List(Box::new(TypeRef::Map {
                key: Box::new(TypeRef::String),
                value: Box::new(TypeRef::Int),
            }))
        );
    }

    #[test]
    fn test_split_generic_args() {
        assert_eq!(
            TypeRef::split_generic_args("string, int"),
            vec!["string", " int"]
        );
        assert_eq!(
            TypeRef::split_generic_args("string, map<int, bool>"),
            vec!["string", " map<int, bool>"]
        );
    }
}
