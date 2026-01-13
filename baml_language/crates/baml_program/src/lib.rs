//! BAML Program - Serializable intermediate representation.
//!
//! This crate provides the data-only BamlProgram representation:
//! - Serializable program structure (classes, enums, functions, clients)
//! - Runtime value types (BamlValue, BamlMedia, etc.)
//! - Type representation (Ty)
//!
//! This crate has no compiler dependencies - it's purely data.
//! Compilation from salsa database is in `baml_program_compile`.
//! Execution logic is in `baml_executor`.

mod program;
mod value;

// Program structure types
pub use program::{
    BamlProgram, ClassDef, ClientDef, EnumDef, EnumVariantDef, FieldDef, FunctionBody, FunctionDef,
    LiteralValue, MediaKind, ParamDef, RetryPolicyDef, RetryStrategy, TestArgs, TestCaseDef, Ty,
};
// Runtime value types
pub use value::{
    BamlMap, BamlMedia, BamlValue, BamlValueWithMeta, CompletionState, JinjaExpression,
    MediaContent,
};
