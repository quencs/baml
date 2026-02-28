//! `baml_compiler2_ast` — Concrete AST structs and CST → AST lowering.
//!
//! This crate isolates all CST messiness in one boundary layer. After
//! `lower_file` returns, the CST is never needed again — all structural
//! content is owned by the returned `Vec<Item>`.
//!
//! No Salsa dependency. Everything downstream works with owned data and
//! can be constructed directly in tests without parsing.

pub mod ast;
pub(crate) mod lower_cst;
pub(crate) mod lower_expr_body;
pub(crate) mod lower_type_expr;

pub use ast::*;
pub use lower_cst::lower_file;
