//! Container abstraction for item locations.
//!
//! A container represents where an item is defined:
//! - Currently: Always a file
//! - Future: Could be a module or block scope
//!
//! This abstraction future-proofs the design for modules without requiring
//! breaking changes when they're added.

use baml_base::FileId;

/// Container that holds items.
///
/// This abstraction allows items to be located in:
/// - Files (current implementation)
/// - Modules (future feature)
/// - Block scopes (future feature)
///
/// Note: Cannot be Copy because of Box<BlockId>.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ContainerId {
    /// Item defined directly in a file (current behavior).
    File(FileId),

    /// Item defined in a module (future feature).
    /// Currently unused, but costs nothing to have.
    #[allow(dead_code)]
    Module(ModuleId),

    /// Item defined in a block expression (future feature).
    /// For block-scoped definitions.
    /// Boxed to break the recursion cycle.
    #[allow(dead_code)]
    Block(Box<BlockId>),
}

impl ContainerId {
    /// Constructor for current file-based system.
    pub const fn file(file: FileId) -> Self {
        ContainerId::File(file)
    }

    /// Get the file this container belongs to (if known).
    pub fn file_id(self) -> Option<FileId> {
        match self {
            ContainerId::File(f) => Some(f),
            // Future: Walk up module tree to find file
            _ => None,
        }
    }
}

/// Module identifier (future feature).
///
/// Even though we don't use modules yet, defining this now costs nothing
/// and makes the future transition trivial.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ModuleId {
    /// The project this module belongs to.
    pub project: ProjectId,

    /// The module's index within its `DefMap`.
    pub local_id: LocalModuleId,
}

impl ModuleId {
    /// The root module (equivalent to current "global scope").
    pub const ROOT: ModuleId = ModuleId {
        project: ProjectId(0),
        local_id: LocalModuleId(0),
    };
}

/// Local module ID within a project.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LocalModuleId(pub u32);

/// Project identifier (future feature for multi-crate support).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ProjectId(pub u32);

/// Block scope identifier (future feature).
///
/// Note: Cannot be Copy because `ContainerId` contains Box<BlockId>.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BlockId {
    /// The function or item containing this block.
    pub parent: ContainerId,

    /// Local ID within the parent.
    pub local_id: u32,
}
