//! Path representation for name resolution.
//!
//! Paths allow referencing items across module boundaries (future feature).
//! Today: All paths are single-segment (e.g., "User")
//! Future: Multi-segment paths (e.g., "`users::User`")

use baml_base::Name;

/// A path to an item (`foo::bar::Baz`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Path {
    /// Path segments (`["foo", "bar", "Baz"]`).
    pub segments: Vec<Name>,

    /// Path kind (absolute vs relative).
    pub kind: PathKind,
}

/// The kind of path resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PathKind {
    /// Relative path (`foo::bar`).
    /// Resolved relative to current scope.
    Plain,

    /// Absolute path (`::foo::bar`) (future feature).
    /// Resolved from project root.
    #[allow(dead_code)]
    Absolute,

    /// Super path (`super::foo`) (future feature).
    /// Resolved relative to parent module.
    #[allow(dead_code)]
    Super { count: u32 },
}

impl Path {
    /// Create a simple single-segment path.
    pub fn single(name: Name) -> Self {
        Path {
            segments: vec![name],
            kind: PathKind::Plain,
        }
    }

    /// Create a multi-segment path (future feature).
    #[allow(dead_code)]
    pub fn new(segments: Vec<Name>) -> Self {
        Path {
            segments,
            kind: PathKind::Plain,
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
