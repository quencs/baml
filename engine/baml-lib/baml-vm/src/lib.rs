//! BAML Bytecode VM
//! 
//! This crate implements a bytecode-based virtual machine for executing BAML programs.
//! The VM follows the design outlined in the architecture document, supporting features like:
//! - Basic operations and control flow
//! - Async/await (colorless promises)
//! - Streaming
//! - Debugging support

pub mod bytecode;
pub mod compiler;
pub mod value;
pub mod vm;

pub use bytecode::{Instruction, Program};
pub use compiler::Compiler;
pub use value::Value;
pub use vm::{ExecutionScope, VirtualMachine};

#[derive(Debug, thiserror::Error)]
pub enum VmError {
    #[error("Compilation error: {0}")]
    CompilationError(String),
    
    #[error("Runtime error: {0}")]
    RuntimeError(String),
    
    #[error("Type error: {0}")]
    TypeError(String),
    
    #[error("Variable not found: {0}")]
    VariableNotFound(String),
    
    #[error("Function not found: {0}")]
    FunctionNotFound(String),
    
    #[error("Invalid instruction: {0}")]
    InvalidInstruction(String),
}

pub type Result<T> = std::result::Result<T, VmError>; 