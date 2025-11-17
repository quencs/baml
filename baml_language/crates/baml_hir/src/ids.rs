//! HIR item identifiers with Salsa interning.
//!
//! This module defines stable IDs for all top-level items in BAML.
//! IDs are interned via Salsa, providing:
//! - Stability across edits (content-based, not order-based)
//! - Compactness (u32 instead of full location data)
//! - Efficient comparison and hashing

use std::marker::PhantomData;

// Note: In Salsa 2022, interned types are their own IDs.
// The #[salsa::interned] macro in loc.rs creates these types directly.
// We re-export them here as type aliases for clarity.

/// Identifier for a function (LLM or expression).
/// This is the interned `FunctionLoc` from loc.rs.
pub use crate::loc::FunctionLoc as FunctionId;

/// Identifier for a class definition.
pub use crate::loc::ClassLoc as ClassId;

/// Identifier for an enum definition.
pub use crate::loc::EnumLoc as EnumId;

/// Identifier for a type alias.
pub use crate::loc::TypeAliasLoc as TypeAliasId;

/// Identifier for a client configuration.
pub use crate::loc::ClientLoc as ClientId;

/// Identifier for a test definition.
pub use crate::loc::TestLoc as TestId;

// Manual Debug implementations for Salsa interned types
// These types don't auto-derive Debug, so we provide simple implementations

impl std::fmt::Debug for FunctionId<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "FunctionId(..)")
    }
}

impl std::fmt::Debug for ClassId<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ClassId(..)")
    }
}

impl std::fmt::Debug for EnumId<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "EnumId(..)")
    }
}

impl std::fmt::Debug for TypeAliasId<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "TypeAliasId(..)")
    }
}

impl std::fmt::Debug for ClientId<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ClientId(..)")
    }
}

impl std::fmt::Debug for TestId<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "TestId(..)")
    }
}

/// Union type for any top-level item.
///
/// Note: Salsa interned types have a `'db` lifetime, so `ItemId` must also have one.
#[derive(Clone, Copy, PartialEq, Eq, Hash, salsa::Update)]
pub enum ItemId<'db> {
    Function(FunctionId<'db>),
    Class(ClassId<'db>),
    Enum(EnumId<'db>),
    TypeAlias(TypeAliasId<'db>),
    Client(ClientId<'db>),
    Test(TestId<'db>),
}

// Manual Debug impl since Salsa interned types don't auto-derive Debug
impl std::fmt::Debug for ItemId<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ItemId::Function(_) => write!(f, "Function(_)"),
            ItemId::Class(_) => write!(f, "Class(_)"),
            ItemId::Enum(_) => write!(f, "Enum(_)"),
            ItemId::TypeAlias(_) => write!(f, "TypeAlias(_)"),
            ItemId::Client(_) => write!(f, "Client(_)"),
            ItemId::Test(_) => write!(f, "Test(_)"),
        }
    }
}

/// Local ID within an arena (type-safe index).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LocalItemId<T> {
    index: u32,
    _phantom: PhantomData<T>,
}

impl<T> LocalItemId<T> {
    pub const fn new(index: u32) -> Self {
        LocalItemId {
            index,
            _phantom: PhantomData,
        }
    }

    pub const fn as_u32(self) -> u32 {
        self.index
    }

    pub const fn as_usize(self) -> usize {
        self.index as usize
    }
}

// Implement From for convenience
impl<T> From<u32> for LocalItemId<T> {
    fn from(index: u32) -> Self {
        LocalItemId::new(index)
    }
}

impl<T> From<usize> for LocalItemId<T> {
    #[allow(clippy::cast_possible_truncation)]
    fn from(index: usize) -> Self {
        LocalItemId::new(index as u32)
    }
}
