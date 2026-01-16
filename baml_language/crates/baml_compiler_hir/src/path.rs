//! Path representation for name resolution.
//!
//! Paths allow referencing items across module boundaries (future feature).
//! Today: Most paths are single-segment (e.g., "User") and refer to user-defined
//! items in the current project. There are also some paths that begin with the
//! "baml" segment, a builtin pseudomodule.

use baml_base::{Name, Span};

/// A path to an item (`foo.bar.Baz`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Path {
    /// Path segments (`["foo", "bar", "Baz"]`).
    pub segments: Vec<Name>,

    /// Path kind (absolute vs relative).
    pub kind: PathKind,

    /// Optional span for error reporting.
    /// This is not used for equality/hashing since it's just for diagnostics.
    #[allow(dead_code)]
    pub span: Option<Span>,
}

/// The kind of path resolution.
///
/// Only one variant today. Maybe in the future we support absolute paths.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PathKind {
    /// Relative path (`foo.bar`).
    /// Resolved relative to current scope.
    Plain,
}

impl Path {
    /// Create a simple single-segment path.
    pub fn single(name: Name) -> Self {
        Path {
            segments: vec![name],
            kind: PathKind::Plain,
            span: None,
        }
    }

    /// Create a simple single-segment path with a span for error reporting.
    pub fn single_with_span(name: Name, span: Span) -> Self {
        Path {
            segments: vec![name],
            kind: PathKind::Plain,
            span: Some(span),
        }
    }

    /// Create a multi-segment path (future feature).
    pub fn new(segments: Vec<Name>) -> Self {
        Path {
            segments,
            kind: PathKind::Plain,
            span: None,
        }
    }

    /// Check if this is a simple name (no :: separators).
    pub fn is_simple(&self) -> bool {
        self.segments.len() == 1 && self.kind == PathKind::Plain
    }

    /// Get the final segment (the item name).
    pub fn last_segment(&self) -> Option<&Name> {
        self.segments.last()
    }

    /// Get the first segment.
    pub fn first_segment(&self) -> Option<&Name> {
        self.segments.first()
    }
}

impl From<Name> for Path {
    fn from(name: Name) -> Self {
        Path::single(name)
    }
}
