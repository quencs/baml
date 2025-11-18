//! Position-independent item storage.
//!
//! The `ItemTree` contains minimal representations of all items in a container.
//! It acts as an "invalidation barrier" - only changes to item signatures
//! cause the `ItemTree` to change, not edits to whitespace, comments, or bodies.

use crate::{
    ids::LocalItemId,
    loc::{ClassMarker, ClientMarker, EnumMarker, FunctionMarker, TestMarker, TypeAliasMarker},
    type_ref::TypeRef,
};
use baml_base::Name;
use la_arena::{Arena, Idx};
use std::collections::HashMap;
use std::ops::Index;

/// Position-independent item storage for a container.
///
/// This is the core HIR data structure. Items are stored in arenas
/// with stable indices that survive source code edits.
///
/// **Key property:** Items are indexed by name, not source position.
/// Adding an item in the middle of the file doesn't change the `LocalItemIds`
/// of other items because `LocalItemIds` are derived from names, not arena indices.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemTree {
    pub functions: Arena<Function>,
    pub classes: Arena<Class>,
    pub enums: Arena<Enum>,
    pub type_aliases: Arena<TypeAlias>,
    pub clients: Arena<Client>,
    pub tests: Arena<Test>,

    // Map from content-based LocalItemId to arena Idx for lookups
    pub(crate) function_map: HashMap<LocalItemId<FunctionMarker>, Idx<Function>>,
    pub(crate) class_map: HashMap<LocalItemId<ClassMarker>, Idx<Class>>,
    pub(crate) enum_map: HashMap<LocalItemId<EnumMarker>, Idx<Enum>>,
    pub(crate) type_alias_map: HashMap<LocalItemId<TypeAliasMarker>, Idx<TypeAlias>>,
    pub(crate) client_map: HashMap<LocalItemId<ClientMarker>, Idx<Client>>,
    pub(crate) test_map: HashMap<LocalItemId<TestMarker>, Idx<Test>>,
}

impl Default for ItemTree {
    fn default() -> Self {
        Self::new()
    }
}

impl ItemTree {
    /// Create a new empty `ItemTree`.
    pub fn new() -> Self {
        Self {
            functions: Arena::new(),
            classes: Arena::new(),
            enums: Arena::new(),
            type_aliases: Arena::new(),
            clients: Arena::new(),
            tests: Arena::new(),
            function_map: HashMap::new(),
            class_map: HashMap::new(),
            enum_map: HashMap::new(),
            type_alias_map: HashMap::new(),
            client_map: HashMap::new(),
            test_map: HashMap::new(),
        }
    }

    /// Add a function and return its local ID.
    /// `LocalItemId` is derived from the function's name for position-independence.
    pub fn alloc_function(&mut self, func: Function) -> LocalItemId<FunctionMarker> {
        let id = LocalItemId::from_name(&func.name);
        let arena_idx = self.functions.alloc(func);
        self.function_map.insert(id, arena_idx);
        id
    }

    /// Add a class and return its local ID.
    /// `LocalItemId` is derived from the class's name for position-independence.
    pub fn alloc_class(&mut self, class: Class) -> LocalItemId<ClassMarker> {
        let id = LocalItemId::from_name(&class.name);
        let arena_idx = self.classes.alloc(class);
        self.class_map.insert(id, arena_idx);
        id
    }

    /// Add an enum and return its local ID.
    /// `LocalItemId` is derived from the enum's name for position-independence.
    pub fn alloc_enum(&mut self, enum_def: Enum) -> LocalItemId<EnumMarker> {
        let id = LocalItemId::from_name(&enum_def.name);
        let arena_idx = self.enums.alloc(enum_def);
        self.enum_map.insert(id, arena_idx);
        id
    }

    /// Add a type alias and return its local ID.
    /// `LocalItemId` is derived from the type alias's name for position-independence.
    /// If there's a name collision, appends a counter to make it unique.
    pub fn alloc_type_alias(&mut self, mut alias: TypeAlias) -> LocalItemId<TypeAliasMarker> {
        let mut id = LocalItemId::from_name(&alias.name);

        // Handle name collisions by appending counter
        let mut counter = 0;
        while self.type_alias_map.contains_key(&id) {
            counter += 1;
            let collision_name = Name::new(format!("{}_{}", alias.name.as_str(), counter));
            id = LocalItemId::from_name(&collision_name);
            alias.name = collision_name;
        }

        let arena_idx = self.type_aliases.alloc(alias);
        self.type_alias_map.insert(id, arena_idx);
        id
    }

    /// Add a client and return its local ID.
    /// `LocalItemId` is derived from the client's name for position-independence.
    pub fn alloc_client(&mut self, client: Client) -> LocalItemId<ClientMarker> {
        let id = LocalItemId::from_name(&client.name);
        let arena_idx = self.clients.alloc(client);
        self.client_map.insert(id, arena_idx);
        id
    }

    /// Add a test and return its local ID.
    /// `LocalItemId` is derived from the test's name for position-independence.
    pub fn alloc_test(&mut self, test: Test) -> LocalItemId<TestMarker> {
        let id = LocalItemId::from_name(&test.name);
        let arena_idx = self.tests.alloc(test);
        self.test_map.insert(id, arena_idx);
        id
    }

    // Note: Use the Index implementations instead of getter methods.
    // Example: let func = &item_tree[func_id];
}

/// A function definition in the `ItemTree`.
///
/// This is the MINIMAL representation - ONLY the name.
/// Everything else (params, return type, body) is in separate queries for incrementality.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Function {
    pub name: Name,
}

/// A class definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Class {
    pub name: Name,
    pub fields: Vec<Field>,

    /// Block attributes (@@dynamic, @@alias, etc.).
    pub is_dynamic: bool,
    // Note: Generic parameters are queried separately via generic_params()
    // for incrementality - changes to generics don't invalidate ItemTree
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
    // Note: Generic parameters are queried separately via generic_params()
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
    // Note: Generic parameters are queried separately via generic_params()
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

//
// ──────────────────────────────────────────────────────── INDEX IMPLS ─────
//

/// Index `ItemTree` by `FunctionMarker` to get Function data.
impl Index<LocalItemId<FunctionMarker>> for ItemTree {
    type Output = Function;
    fn index(&self, index: LocalItemId<FunctionMarker>) -> &Self::Output {
        let arena_idx = self
            .function_map
            .get(&index)
            .expect("Function not found in ItemTree");
        &self.functions[*arena_idx]
    }
}

/// Index `ItemTree` by `ClassMarker` to get Class data.
impl Index<LocalItemId<ClassMarker>> for ItemTree {
    type Output = Class;
    fn index(&self, index: LocalItemId<ClassMarker>) -> &Self::Output {
        let arena_idx = self
            .class_map
            .get(&index)
            .expect("Class not found in ItemTree");
        &self.classes[*arena_idx]
    }
}

/// Index `ItemTree` by `EnumMarker` to get Enum data.
impl Index<LocalItemId<EnumMarker>> for ItemTree {
    type Output = Enum;
    fn index(&self, index: LocalItemId<EnumMarker>) -> &Self::Output {
        let arena_idx = self
            .enum_map
            .get(&index)
            .expect("Enum not found in ItemTree");
        &self.enums[*arena_idx]
    }
}

/// Index `ItemTree` by `TypeAliasMarker` to get `TypeAlias` data.
impl Index<LocalItemId<TypeAliasMarker>> for ItemTree {
    type Output = TypeAlias;
    fn index(&self, index: LocalItemId<TypeAliasMarker>) -> &Self::Output {
        let arena_idx = self
            .type_alias_map
            .get(&index)
            .expect("TypeAlias not found in ItemTree");
        &self.type_aliases[*arena_idx]
    }
}

/// Index `ItemTree` by `ClientMarker` to get Client data.
impl Index<LocalItemId<ClientMarker>> for ItemTree {
    type Output = Client;
    fn index(&self, index: LocalItemId<ClientMarker>) -> &Self::Output {
        let arena_idx = self
            .client_map
            .get(&index)
            .expect("Client not found in ItemTree");
        &self.clients[*arena_idx]
    }
}

/// Index `ItemTree` by `TestMarker` to get Test data.
impl Index<LocalItemId<TestMarker>> for ItemTree {
    type Output = Test;
    fn index(&self, index: LocalItemId<TestMarker>) -> &Self::Output {
        let arena_idx = self
            .test_map
            .get(&index)
            .expect("Test not found in ItemTree");
        &self.tests[*arena_idx]
    }
}
