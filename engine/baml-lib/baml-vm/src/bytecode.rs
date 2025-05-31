//! Bytecode representation and instructions
//! 
//! This module defines the bytecode instructions and program structure for the VM.
//! Following the design document, each instruction is simple and performs one operation.

use serde::{Deserialize, Serialize};
use std::fmt;

/// A variable identifier in the bytecode
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct VarId(pub String);

impl fmt::Display for VarId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A basic block identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BlockId(pub usize);

impl fmt::Display for BlockId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "block_{}", self.0)
    }
}

/// Bytecode instructions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Instruction {
    // Constants and literals
    LoadConst { dest: VarId, value: Literal },
    
    // Variable operations
    LoadVar { dest: VarId, source: VarId },
    StoreVar { dest: VarId, source: VarId },
    
    // Arithmetic operations
    Add { dest: VarId, left: VarId, right: VarId },
    Sub { dest: VarId, left: VarId, right: VarId },
    Mul { dest: VarId, left: VarId, right: VarId },
    Div { dest: VarId, left: VarId, right: VarId },
    
    // Comparison operations
    Lt { dest: VarId, left: VarId, right: VarId },
    Gt { dest: VarId, left: VarId, right: VarId },
    Eq { dest: VarId, left: VarId, right: VarId },
    
    // Boolean operations
    And { dest: VarId, left: VarId, right: VarId },
    Or { dest: VarId, left: VarId, right: VarId },
    Not { dest: VarId, operand: VarId },
    
    // Control flow
    Jump { target: BlockId },
    JumpIf { condition: VarId, target: BlockId },
    JumpIfNot { condition: VarId, target: BlockId },
    
    // Function calls
    Call { dest: VarId, function: String, args: Vec<VarId> },
    Return { value: Option<VarId> },
    
    // Async operations (for colorless promises)
    Await { dest: VarId, promise: VarId },
    
    // Debug/utility
    Print { value: VarId },
}

impl fmt::Display for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Instruction::LoadConst { dest, value } => write!(f, "LoadConst({dest}, {value})"),
            Instruction::LoadVar { dest, source } => write!(f, "LoadVar({dest}, {source})"),
            Instruction::StoreVar { dest, source } => write!(f, "StoreVar({dest}, {source})"),
            Instruction::Add { dest, left, right } => write!(f, "Add({dest}, {left}, {right})"),
            Instruction::Sub { dest, left, right } => write!(f, "Sub({dest}, {left}, {right})"),
            Instruction::Mul { dest, left, right } => write!(f, "Mul({dest}, {left}, {right})"),
            Instruction::Div { dest, left, right } => write!(f, "Div({dest}, {left}, {right})"),
            Instruction::Lt { dest, left, right } => write!(f, "Lt({dest}, {left}, {right})"),
            Instruction::Gt { dest, left, right } => write!(f, "Gt({dest}, {left}, {right})"),
            Instruction::Eq { dest, left, right } => write!(f, "Eq({dest}, {left}, {right})"),
            Instruction::And { dest, left, right } => write!(f, "And({dest}, {left}, {right})"),
            Instruction::Or { dest, left, right } => write!(f, "Or({dest}, {left}, {right})"),
            Instruction::Not { dest, operand } => write!(f, "Not({dest}, {operand})"),
            Instruction::Jump { target } => write!(f, "Jump({target})"),
            Instruction::JumpIf { condition, target } => write!(f, "JumpIf({condition}, {target})"),
            Instruction::JumpIfNot { condition, target } => write!(f, "JumpIfNot({condition}, {target})"),
            Instruction::Call { dest, function, args } => write!(f, "Call({dest}, {function}, {args:?})"),
            Instruction::Return { value } => write!(f, "Return({value:?})"),
            Instruction::Await { dest, promise } => write!(f, "Await({dest}, {promise})"),
            Instruction::Print { value } => write!(f, "Print({value})"),
        }
    }
}

/// Literal values that can be loaded as constants
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Literal {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
}

impl fmt::Display for Literal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Literal::Null => write!(f, "Null"),
            Literal::Bool(b) => write!(f, "Bool({b})"),
            Literal::Int(i) => write!(f, "Int({i})"),
            Literal::Float(fl) => write!(f, "Float({fl})"),
            Literal::String(s) => write!(f, "String({s})"),
        }
    }
}



/// A basic block containing a sequence of instructions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BasicBlock {
    pub id: BlockId,
    pub instructions: Vec<Instruction>,
    pub parameters: Vec<VarId>, // Parameters passed when jumping to this block
}

/// A function in the bytecode program
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Function {
    pub name: String,
    pub parameters: Vec<VarId>,
    pub blocks: Vec<BasicBlock>,
    pub entry_block: BlockId,
}

/// A complete bytecode program
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Program {
    pub functions: Vec<Function>,
    pub entry_function: String, // Name of the main/entry function
}

impl Program {
    pub fn new(entry_function: String) -> Self {
        Self {
            functions: Vec::new(),
            entry_function,
        }
    }
    
    pub fn add_function(&mut self, function: Function) {
        self.functions.push(function);
    }
    
    pub fn find_function(&self, name: &str) -> Option<&Function> {
        self.functions.iter().find(|f| f.name == name)
    }
} 

impl fmt::Display for Program {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Program(entry_function: {})", self.entry_function)?;
        for function in &self.functions {
            write!(f, "\nFunction({})", function.name)?;
            for block in &function.blocks {
                write!(f, "\n  {}({}):", block.id, block.parameters.iter().map(|v| v.to_string()).collect::<Vec<String>>().join(", "))?;
                for instruction in &block.instructions {
                    write!(f, "\n    {}", instruction)?;
                }
            }
        }
        Ok(())
    }
}