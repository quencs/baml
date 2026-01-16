//! Unresolved type references in the HIR.
//!
//! These are type references before name resolution.
//! `TypeRef` -> Ty happens during THIR construction.

use baml_base::Name;
use rowan::{TextRange, TextSize};

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

    Media(baml_base::MediaKind),

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
    pub fn from_ast(type_expr: &baml_compiler_syntax::ast::TypeExpr) -> Self {
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

    /// Create a `TypeRef` from an AST `TypeExpr` node, preserving spans for error reporting.
    ///
    /// This is like `from_ast` but captures span information in `TypeRef::Path` variants
    /// so that TIR can report errors with precise source locations.
    pub fn from_ast_with_spans(
        type_expr: &baml_compiler_syntax::ast::TypeExpr,
        file_id: baml_base::FileId,
    ) -> Self {
        let parts_with_spans = type_expr.parts_with_spans();

        // If multiple parts, this is a union type
        if parts_with_spans.len() > 1 {
            let members: Vec<TypeRef> = parts_with_spans
                .iter()
                .map(|(text, range)| {
                    let span = baml_base::Span::new(file_id, *range);
                    Self::from_type_text_with_span(text, span)
                })
                .collect();
            return TypeRef::Union(members);
        }

        // Single type (possibly with modifiers like ? or [])
        parts_with_spans
            .first()
            .map(|(text, range)| {
                let span = baml_base::Span::new(file_id, *range);
                Self::from_type_text_with_span(text, span)
            })
            .unwrap_or(TypeRef::Unknown)
    }

    /// Create a `TypeRef` from type text with span information.
    ///
    /// The `base_offset` is the byte offset from the start of the span to the start of `text`.
    /// This allows computing precise sub-spans for inner types.
    fn from_type_text_with_span(text: &str, span: baml_base::Span) -> Self {
        Self::from_type_text_with_offset(text, span, 0)
    }

    /// Internal helper that tracks byte offset for precise span computation.
    fn from_type_text_with_offset(text: &str, span: baml_base::Span, base_offset: u32) -> Self {
        // Check for string literal types like "user" or "assistant"
        if text.starts_with('"') && text.ends_with('"') {
            let inner = &text[1..text.len() - 1];
            return TypeRef::StringLiteral(inner.to_string());
        }

        // Check for array type (e.g., "int[]")
        if let Some(inner_text) = text.strip_suffix("[]") {
            // Inner type has same offset, just shorter length
            let inner = Self::from_type_text_with_offset(inner_text, span, base_offset);
            return TypeRef::List(Box::new(inner));
        }

        // Check for optional type (e.g., "int?")
        if let Some(inner_text) = text.strip_suffix('?') {
            // Inner type has same offset, just shorter length
            let inner = Self::from_type_text_with_offset(inner_text, span, base_offset);
            return TypeRef::Optional(Box::new(inner));
        }

        // Check for parenthesized expressions (e.g., "(A | B)")
        // These are union types wrapped in parentheses
        // Note: We only parse as union if there are no commas at the top level
        // (commas indicate tuple types, which are not supported)
        if text.starts_with('(') && text.ends_with(')') {
            let inner = &text[1..text.len() - 1];
            // Check if this contains top-level commas (tuple syntax, not supported)
            if !Self::has_top_level_comma(inner) {
                // Split by | at top level (respecting nested parens)
                let parts_with_offsets = Self::split_union_parts_with_offsets(inner);
                if parts_with_offsets.len() > 1 {
                    let members: Vec<TypeRef> = parts_with_offsets
                        .iter()
                        .map(|(part, part_offset)| {
                            let trimmed = part.trim();
                            let trim_offset = part.len() - part.trim_start().len();
                            // +1 for opening paren
                            let inner_offset = base_offset + 1 + *part_offset as u32 + trim_offset as u32;
                            Self::from_type_text_with_offset(trimmed, span, inner_offset)
                        })
                        .collect();
                    return TypeRef::Union(members);
                } else if parts_with_offsets.len() == 1 {
                    let (part, part_offset) = &parts_with_offsets[0];
                    let trimmed = part.trim();
                    let trim_offset = part.len() - part.trim_start().len();
                    let inner_offset = base_offset + 1 + *part_offset as u32 + trim_offset as u32;
                    return Self::from_type_text_with_offset(trimmed, span, inner_offset);
                }
            }
            // If has commas, fall through to be treated as unknown type
        }

        // Check for boolean literal types
        if text == "true" {
            return TypeRef::BoolLiteral(true);
        }
        if text == "false" {
            return TypeRef::BoolLiteral(false);
        }

        // Check for integer literal types
        if let Ok(int_val) = text.parse::<i64>() {
            return TypeRef::IntLiteral(int_val);
        }

        // Check for map type (e.g., "map<string, int>")
        if let Some(rest) = text.strip_prefix("map<") {
            if let Some(inner) = rest.strip_suffix('>') {
                if let Some((key_text, key_offset, value_text, value_offset)) =
                    Self::split_generic_params_with_offsets(inner)
                {
                    let key_trimmed = key_text.trim();
                    let key_trim_offset = key_text.len() - key_text.trim_start().len();
                    // +4 for "map<"
                    let key_inner_offset = base_offset + 4 + key_offset as u32 + key_trim_offset as u32;
                    let key = Self::from_type_text_with_offset(key_trimmed, span, key_inner_offset);

                    let value_trimmed = value_text.trim();
                    let value_trim_offset = value_text.len() - value_text.trim_start().len();
                    let value_inner_offset = base_offset + 4 + value_offset as u32 + value_trim_offset as u32;
                    let value = Self::from_type_text_with_offset(value_trimmed, span, value_inner_offset);

                    return TypeRef::Map {
                        key: Box::new(key),
                        value: Box::new(value),
                    };
                }
            }
        }

        // Detect numeric literals that failed parsing above
        if text.starts_with(|c: char| c.is_ascii_digit()) {
            return TypeRef::Error;
        }

        // Create a precise span for this type name
        let precise_span = Self::sub_span(span, base_offset, text.len() as u32);
        Self::from_type_name_with_span(text, precise_span)
    }

    /// Compute a sub-span given an offset and length within the original span.
    fn sub_span(span: baml_base::Span, offset: u32, len: u32) -> baml_base::Span {
        let start = span.range.start() + TextSize::from(offset);
        let end = start + TextSize::from(len);
        baml_base::Span::new(span.file_id, TextRange::new(start, end))
    }

    /// Split a union type string by | at the top level (respecting parentheses).
    fn split_union_parts(text: &str) -> Vec<&str> {
        let mut parts = Vec::new();
        let mut depth: i32 = 0;
        let mut start = 0;

        for (i, c) in text.char_indices() {
            match c {
                '(' | '<' | '[' => depth += 1,
                ')' | '>' | ']' => depth = (depth - 1).max(0),
                '|' if depth == 0 => {
                    parts.push(&text[start..i]);
                    start = i + 1;
                }
                _ => {}
            }
        }

        // Add the last part
        if start < text.len() {
            parts.push(&text[start..]);
        }

        parts
    }

    /// Split a union type string by | at the top level, returning parts with their byte offsets.
    fn split_union_parts_with_offsets(text: &str) -> Vec<(&str, usize)> {
        let mut parts = Vec::new();
        let mut depth: i32 = 0;
        let mut start = 0;

        for (i, c) in text.char_indices() {
            match c {
                '(' | '<' | '[' => depth += 1,
                ')' | '>' | ']' => depth = (depth - 1).max(0),
                '|' if depth == 0 => {
                    parts.push((&text[start..i], start));
                    start = i + 1;
                }
                _ => {}
            }
        }

        // Add the last part
        if start < text.len() {
            parts.push((&text[start..], start));
        }

        parts
    }

    /// Split generic parameters (key, value) with their byte offsets.
    /// Returns (key_text, key_offset, value_text, value_offset).
    fn split_generic_params_with_offsets(s: &str) -> Option<(&str, usize, &str, usize)> {
        let mut depth = 0;
        for (i, c) in s.char_indices() {
            match c {
                '<' => depth += 1,
                '>' => depth -= 1,
                ',' if depth == 0 => {
                    let key_text = &s[..i];
                    let value_text = &s[i + 1..];
                    return Some((key_text, 0, value_text, i + 1));
                }
                _ => {}
            }
        }
        None
    }

    /// Check if the text has a comma at the top level (not inside parens/brackets).
    /// This is used to detect tuple syntax like `(int, string)` which is not supported.
    fn has_top_level_comma(text: &str) -> bool {
        let mut depth: i32 = 0;
        for c in text.chars() {
            match c {
                '(' | '<' | '[' => depth += 1,
                ')' | '>' | ']' => depth = (depth - 1).max(0),
                ',' if depth == 0 => return true,
                _ => {}
            }
        }
        false
    }

    /// Create a `TypeRef` from a type name with span information.
    fn from_type_name_with_span(name: &str, span: baml_base::Span) -> Self {
        match name.to_lowercase().as_str() {
            "int" => TypeRef::Int,
            "float" => TypeRef::Float,
            "string" => TypeRef::String,
            "bool" => TypeRef::Bool,
            "null" => TypeRef::Null,
            "image" => TypeRef::Media(baml_base::MediaKind::Image),
            "audio" => TypeRef::Media(baml_base::MediaKind::Audio),
            "video" => TypeRef::Media(baml_base::MediaKind::Video),
            "pdf" => TypeRef::Media(baml_base::MediaKind::Pdf),
            // Named type with span preserved
            _ => TypeRef::Path(Path::single_with_span(Name::new(name), span)),
        }
    }

    /// Create a `TypeRef` from a single type text (not a union).
    ///
    /// This handles:
    /// - String literal types: `"foo"` or `'bar'`
    /// - Array types: `int[]`
    /// - Optional types: `int?`
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

        // Check for parenthesized expressions (e.g., "(A | B)")
        // These are union types wrapped in parentheses
        // Note: We only parse as union if there are no commas at the top level
        // (commas indicate tuple types, which are not supported)
        if text.starts_with('(') && text.ends_with(')') {
            let inner = &text[1..text.len() - 1];
            // Check if this contains top-level commas (tuple syntax, not supported)
            if !Self::has_top_level_comma(inner) {
                // Split by | at top level (respecting nested parens)
                let parts = Self::split_union_parts(inner);
                if parts.len() > 1 {
                    let members: Vec<TypeRef> = parts
                        .iter()
                        .map(|p| Self::from_type_text(p.trim()))
                        .collect();
                    return TypeRef::Union(members);
                } else if parts.len() == 1 {
                    // Single element in parens, just unwrap it
                    return Self::from_type_text(parts[0].trim());
                }
            }
            // If has commas, fall through to be treated as unknown type
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

        // Check for map type (e.g., "map<string, int>")
        if let Some(rest) = text.strip_prefix("map<") {
            if let Some(inner) = rest.strip_suffix('>') {
                // Find the comma that separates key and value types
                // Need to handle nested generics like map<string, map<int, bool>>
                if let Some((key_text, value_text)) = Self::split_generic_params(inner) {
                    let key = Self::from_type_text(key_text.trim());
                    let value = Self::from_type_text(value_text.trim());
                    return TypeRef::Map {
                        key: Box::new(key),
                        value: Box::new(value),
                    };
                }
            }
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

    /// Split generic parameters at the top-level comma.
    /// Handles nested generics like `string, map<int, bool>`.
    fn split_generic_params(s: &str) -> Option<(&str, &str)> {
        let mut depth = 0;
        for (i, c) in s.char_indices() {
            match c {
                '<' => depth += 1,
                '>' => depth -= 1,
                ',' if depth == 0 => {
                    return Some((&s[..i], &s[i + 1..]));
                }
                _ => {}
            }
        }
        None
    }

    /// Create a `TypeRef` from a type name string.
    fn from_type_name(name: &str) -> Self {
        match name.to_lowercase().as_str() {
            "int" => TypeRef::Int,
            "float" => TypeRef::Float,
            "string" => TypeRef::String,
            "bool" => TypeRef::Bool,
            "null" => TypeRef::Null,
            "image" => TypeRef::Media(baml_base::MediaKind::Image),
            "audio" => TypeRef::Media(baml_base::MediaKind::Audio),
            "video" => TypeRef::Media(baml_base::MediaKind::Video),
            "pdf" => TypeRef::Media(baml_base::MediaKind::Pdf),
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
}
