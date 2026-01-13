//! BamlProgram - The serializable program representation.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// The serializable BAML program representation.
///
/// This is the unified artifact used by:
/// - FFI Runtime (Python/TS/Go apps)
/// - Playground (surgically updated on file changes)
/// - Code generation (type info only)
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct BamlProgram {
    /// Class definitions.
    pub classes: HashMap<String, ClassDef>,
    /// Enum definitions.
    pub enums: HashMap<String, EnumDef>,
    /// Function definitions.
    pub functions: HashMap<String, FunctionDef>,
    /// Client definitions.
    pub clients: HashMap<String, ClientDef>,
    /// Retry policy definitions.
    pub retry_policies: HashMap<String, RetryPolicyDef>,
    /// Test case definitions.
    pub tests: HashMap<String, TestCaseDef>,
    /// Whether any part has panic nodes (from semantic errors).
    pub has_panics: bool,
}

impl BamlProgram {
    pub fn new() -> Self {
        Self::default()
    }

    // Surgical update methods for playground incrementality

    pub fn update_class(&mut self, name: &str, def: ClassDef) {
        self.classes.insert(name.to_string(), def);
    }

    pub fn update_enum(&mut self, name: &str, def: EnumDef) {
        self.enums.insert(name.to_string(), def);
    }

    pub fn update_function(&mut self, name: &str, def: FunctionDef) {
        self.functions.insert(name.to_string(), def);
    }

    pub fn update_client(&mut self, name: &str, def: ClientDef) {
        self.clients.insert(name.to_string(), def);
    }

    pub fn update_test(&mut self, name: &str, def: TestCaseDef) {
        self.tests.insert(name.to_string(), def);
    }

    pub fn remove_class(&mut self, name: &str) {
        self.classes.remove(name);
    }

    pub fn remove_enum(&mut self, name: &str) {
        self.enums.remove(name);
    }

    pub fn remove_function(&mut self, name: &str) {
        self.functions.remove(name);
    }

    pub fn remove_client(&mut self, name: &str) {
        self.clients.remove(name);
    }

    pub fn remove_test(&mut self, name: &str) {
        self.tests.remove(name);
    }
}

/// Class definition.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ClassDef {
    pub name: String,
    pub fields: Vec<FieldDef>,
    pub description: Option<String>,
}

/// Field definition within a class.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FieldDef {
    pub name: String,
    pub field_type: Ty,
    pub description: Option<String>,
    pub alias: Option<String>,
}

/// Enum definition.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EnumDef {
    pub name: String,
    pub variants: Vec<EnumVariantDef>,
    pub description: Option<String>,
}

/// Enum variant definition.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EnumVariantDef {
    pub name: String,
    pub description: Option<String>,
    pub alias: Option<String>,
}

/// Function definition.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FunctionDef {
    pub name: String,
    pub params: Vec<ParamDef>,
    pub return_type: Ty,
    pub body: FunctionBody,
}

/// Function parameter definition.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ParamDef {
    pub name: String,
    pub param_type: Ty,
}

/// Function body - either LLM or expression.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum FunctionBody {
    /// Declarative LLM function.
    Llm {
        prompt_template: String,
        client: String,
    },
    /// Imperative expression function (compiled to bytecode).
    Expr { bytecode_index: usize },
    /// Missing body (error state).
    Missing,
}

/// Client definition.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ClientDef {
    pub name: String,
    pub provider: String,
    pub options: HashMap<String, serde_json::Value>,
    pub retry_policy: Option<String>,
}

/// Retry policy definition.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RetryPolicyDef {
    pub name: String,
    pub max_retries: u32,
    pub strategy: RetryStrategy,
}

/// Retry strategy.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum RetryStrategy {
    ExponentialBackoff {
        initial_delay_ms: u64,
        multiplier: f64,
        max_delay_ms: u64,
    },
    ConstantDelay {
        delay_ms: u64,
    },
}

/// Test case definition.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TestCaseDef {
    pub name: String,
    pub function: String,
    pub args: TestArgs,
}

/// Test arguments - literal values or expression.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TestArgs {
    /// Literal values.
    Literal(HashMap<String, crate::value::BamlValue>),
    /// Expression that evaluates to args (compiled to bytecode).
    Expression { bytecode_index: usize },
}

/// Type representation for BamlProgram.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Ty {
    // Primitives
    Int,
    Float,
    String,
    Bool,
    Null,

    // Media types
    Media(MediaKind),

    // Literal types
    Literal(LiteralValue),

    // Named types (references into BamlProgram.classes/enums)
    Class(String),
    Enum(String),

    // Container types
    Optional(Box<Ty>),
    List(Box<Ty>),
    Map { key: Box<Ty>, value: Box<Ty> },
    Union(Vec<Ty>),
}

/// Media type kinds.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum MediaKind {
    Image,
    Audio,
    Video,
    Pdf,
}

/// Literal value types.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum LiteralValue {
    String(String),
    Int(i64),
    Bool(bool),
}
