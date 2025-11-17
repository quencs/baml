//! Position-independent item storage.
//!
//! The `ItemTree` contains minimal representations of all items in a container.
//! It acts as an "invalidation barrier" - only changes to item signatures
//! cause the `ItemTree` to change, not edits to whitespace, comments, or bodies.

use crate::{
    ids::LocalItemId,
    loc::{ClassMarker, ClientMarker, EnumMarker, FunctionMarker, TestMarker, TypeAliasMarker},
    path::Path,
    type_ref::TypeRef,
};
use baml_base::Name;
use std::ops::Index;

/// Position-independent item storage for a container.
///
/// This is the core HIR data structure. Items are stored in arenas
/// with stable indices that survive source code edits.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ItemTree {
    pub functions: Vec<Function>,
    pub classes: Vec<Class>,
    pub enums: Vec<Enum>,
    pub type_aliases: Vec<TypeAlias>,
    pub clients: Vec<Client>,
    pub tests: Vec<Test>,
}

impl ItemTree {
    /// Create a new empty `ItemTree`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a function and return its local ID.
    #[allow(clippy::cast_possible_truncation)]
    pub fn alloc_function(&mut self, func: Function) -> LocalItemId<FunctionMarker> {
        let id = self.functions.len();
        self.functions.push(func);
        LocalItemId::new(id as u32)
    }

    /// Add a class and return its local ID.
    #[allow(clippy::cast_possible_truncation)]
    pub fn alloc_class(&mut self, class: Class) -> LocalItemId<ClassMarker> {
        let id = self.classes.len();
        self.classes.push(class);
        LocalItemId::new(id as u32)
    }

    /// Add an enum and return its local ID.
    #[allow(clippy::cast_possible_truncation)]
    pub fn alloc_enum(&mut self, enum_def: Enum) -> LocalItemId<EnumMarker> {
        let id = self.enums.len();
        self.enums.push(enum_def);
        LocalItemId::new(id as u32)
    }

    /// Add a type alias and return its local ID.
    #[allow(clippy::cast_possible_truncation)]
    pub fn alloc_type_alias(&mut self, alias: TypeAlias) -> LocalItemId<TypeAliasMarker> {
        let id = self.type_aliases.len();
        self.type_aliases.push(alias);
        LocalItemId::new(id as u32)
    }

    /// Add a client and return its local ID.
    #[allow(clippy::cast_possible_truncation)]
    pub fn alloc_client(&mut self, client: Client) -> LocalItemId<ClientMarker> {
        let id = self.clients.len();
        self.clients.push(client);
        LocalItemId::new(id as u32)
    }

    /// Add a test and return its local ID.
    #[allow(clippy::cast_possible_truncation)]
    pub fn alloc_test(&mut self, test: Test) -> LocalItemId<TestMarker> {
        let id = self.tests.len();
        self.tests.push(test);
        LocalItemId::new(id as u32)
    }

    /// Get a function by local ID.
    pub fn function(&self, id: LocalItemId<FunctionMarker>) -> Option<&Function> {
        self.functions.get(id.as_usize())
    }

    /// Get a class by local ID.
    pub fn class(&self, id: LocalItemId<ClassMarker>) -> Option<&Class> {
        self.classes.get(id.as_usize())
    }

    /// Get an enum by local ID.
    pub fn enum_def(&self, id: LocalItemId<EnumMarker>) -> Option<&Enum> {
        self.enums.get(id.as_usize())
    }

    /// Get a type alias by local ID.
    pub fn type_alias(&self, id: LocalItemId<TypeAliasMarker>) -> Option<&TypeAlias> {
        self.type_aliases.get(id.as_usize())
    }

    /// Get a client by local ID.
    pub fn client(&self, id: LocalItemId<ClientMarker>) -> Option<&Client> {
        self.clients.get(id.as_usize())
    }

    /// Get a test by local ID.
    pub fn test(&self, id: LocalItemId<TestMarker>) -> Option<&Test> {
        self.tests.get(id.as_usize())
    }
}

/// A function definition in the `ItemTree`.
///
/// This is the minimal representation - just the signature.
/// Function bodies are stored separately for incrementality.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Function {
    pub name: Name,
    pub params: Vec<Param>,
    pub return_type: TypeRef,

    /// Unresolved client reference (name only).
    pub client_ref: Option<Name>,

    /// Future: Type parameters for generic functions.
    pub type_params: Vec<TypeParam>,
}

/// Function parameter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Param {
    pub name: Name,
    pub type_ref: TypeRef,
}

/// A class definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Class {
    pub name: Name,
    pub fields: Vec<Field>,

    /// Block attributes (@@dynamic, @@alias, etc.).
    pub is_dynamic: bool,

    /// Future: Type parameters for generic classes.
    pub type_params: Vec<TypeParam>,
}

/// Class field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    pub name: Name,
    pub type_ref: TypeRef,
}

/// An enum definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Enum {
    pub name: Name,
    pub variants: Vec<EnumVariant>,

    /// Future: Type parameters.
    pub type_params: Vec<TypeParam>,
}

/// Enum variant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnumVariant {
    pub name: Name,
}

/// Type alias definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeAlias {
    pub name: Name,
    pub type_ref: TypeRef,

    /// Future: Type parameters for generic aliases.
    pub type_params: Vec<TypeParam>,
}

/// Client configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Client {
    pub name: Name,
    pub provider: Name,
}

/// Test definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Test {
    pub name: Name,

    /// Unresolved function references.
    pub function_refs: Vec<Name>,
}

/// Type parameter (for generics, currently unused).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TypeParam {
    pub name: Name,

    /// Type parameter bounds (T: `SomeTrait`) (future).
    pub bounds: Vec<Path>,

    /// Default type (T = string) (future).
    pub default: Option<TypeRef>,
}

//
// ──────────────────────────────────────────────────────── INDEX IMPLS ─────
//

/// Index `ItemTree` by `FunctionMarker` to get Function data.
impl Index<LocalItemId<FunctionMarker>> for ItemTree {
    type Output = Function;
    fn index(&self, index: LocalItemId<FunctionMarker>) -> &Self::Output {
        &self.functions[index.as_usize()]
    }
}

/// Index `ItemTree` by `ClassMarker` to get Class data.
impl Index<LocalItemId<ClassMarker>> for ItemTree {
    type Output = Class;
    fn index(&self, index: LocalItemId<ClassMarker>) -> &Self::Output {
        &self.classes[index.as_usize()]
    }
}

/// Index `ItemTree` by `EnumMarker` to get Enum data.
impl Index<LocalItemId<EnumMarker>> for ItemTree {
    type Output = Enum;
    fn index(&self, index: LocalItemId<EnumMarker>) -> &Self::Output {
        &self.enums[index.as_usize()]
    }
}

/// Index `ItemTree` by `TypeAliasMarker` to get `TypeAlias` data.
impl Index<LocalItemId<TypeAliasMarker>> for ItemTree {
    type Output = TypeAlias;
    fn index(&self, index: LocalItemId<TypeAliasMarker>) -> &Self::Output {
        &self.type_aliases[index.as_usize()]
    }
}

/// Index `ItemTree` by `ClientMarker` to get Client data.
impl Index<LocalItemId<ClientMarker>> for ItemTree {
    type Output = Client;
    fn index(&self, index: LocalItemId<ClientMarker>) -> &Self::Output {
        &self.clients[index.as_usize()]
    }
}

/// Index `ItemTree` by `TestMarker` to get Test data.
impl Index<LocalItemId<TestMarker>> for ItemTree {
    type Output = Test;
    fn index(&self, index: LocalItemId<TestMarker>) -> &Self::Output {
        &self.tests[index.as_usize()]
    }
}
