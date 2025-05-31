//! Compiler from AST to bytecode
//! 
//! This module will compile the BAML AST into bytecode for execution by the VM.
//! Currently this is a stub showing the pattern - it will need to be connected
//! to the actual AST types from schema-ast.

use crate::{
    bytecode::{BasicBlock, BlockId, Function, Instruction, Literal, Program, VarId},
    Result, VmError,
};
use std::collections::HashMap;

/// Simple AST types for demonstration
/// TODO: Replace with actual AST types from schema-ast
#[derive(Debug, Clone)]
pub enum Expr {
    Literal(Literal),
    Variable(String),
    Binary {
        op: BinaryOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Call {
        function: String,
        args: Vec<Expr>,
    },
    If {
        condition: Box<Expr>,
        then_branch: Box<Expr>,
        else_branch: Option<Box<Expr>>,
    },
}

#[derive(Debug, Clone)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Lt,
    Gt,
    Eq,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Let {
        name: String,
        value: Expr,
    },
    Expression(Expr),
    Return(Option<Expr>),
    Print(Expr),
}

/// Compiler state
pub struct Compiler {
    /// Counter for generating unique variable names
    var_counter: usize,
    
    /// Counter for generating unique block IDs
    block_counter: usize,
    
    /// Current function being compiled
    current_function: Option<Function>,
    
    /// Current basic block being built
    current_block: Option<BasicBlock>,
}

impl Compiler {
    pub fn new() -> Self {
        Self {
            var_counter: 0,
            block_counter: 0,
            current_function: None,
            current_block: None,
        }
    }
    
    /// Generate a fresh variable name
    fn fresh_var(&mut self) -> VarId {
        let var = VarId(format!("_local.{}", self.var_counter));
        self.var_counter += 1;
        var
    }
    
    /// Generate a fresh block ID
    fn fresh_block(&mut self) -> BlockId {
        let id = BlockId(self.block_counter);
        self.block_counter += 1;
        id
    }
    
    /// Add an instruction to the current block
    fn emit(&mut self, instruction: Instruction) {
        if let Some(ref mut block) = self.current_block {
            block.instructions.push(instruction);
        }
    }
    
    /// Start a new basic block
    fn start_block(&mut self, id: BlockId, parameters: Vec<VarId>) {
        if let Some(block) = self.current_block.take() {
            if let Some(ref mut function) = self.current_function {
                function.blocks.push(block);
            }
        }
        
        self.current_block = Some(BasicBlock {
            id,
            instructions: Vec::new(),
            parameters,
        });
    }
    
    /// Compile an expression, returning the variable containing the result
    fn compile_expr(&mut self, expr: &Expr) -> Result<VarId> {
        match expr {
            Expr::Literal(lit) => {
                let dest = self.fresh_var();
                self.emit(Instruction::LoadConst {
                    dest: dest.clone(),
                    value: lit.clone(),
                });
                Ok(dest)
            }
            
            Expr::Variable(name) => {
                // For now, just use the variable name directly
                // In a real compiler, we'd have a symbol table
                Ok(VarId(name.clone()))
            }
            
            Expr::Binary { op, left, right } => {
                let left_var = self.compile_expr(left)?;
                let right_var = self.compile_expr(right)?;
                let dest = self.fresh_var();
                
                let instruction = match op {
                    BinaryOp::Add => Instruction::Add {
                        dest: dest.clone(),
                        left: left_var,
                        right: right_var,
                    },
                    BinaryOp::Sub => Instruction::Sub {
                        dest: dest.clone(),
                        left: left_var,
                        right: right_var,
                    },
                    BinaryOp::Mul => Instruction::Mul {
                        dest: dest.clone(),
                        left: left_var,
                        right: right_var,
                    },
                    BinaryOp::Div => Instruction::Div {
                        dest: dest.clone(),
                        left: left_var,
                        right: right_var,
                    },
                    BinaryOp::Lt => Instruction::Lt {
                        dest: dest.clone(),
                        left: left_var,
                        right: right_var,
                    },
                    BinaryOp::Eq => Instruction::Eq {
                        dest: dest.clone(),
                        left: left_var,
                        right: right_var,
                    },
                    _ => return Err(VmError::CompilationError(
                        format!("Unsupported binary operator: {:?}", op)
                    )),
                };
                
                self.emit(instruction);
                Ok(dest)
            }
            
            Expr::Call { function, args } => {
                let mut arg_vars = Vec::new();
                for arg in args {
                    arg_vars.push(self.compile_expr(arg)?);
                }
                
                let dest = self.fresh_var();
                self.emit(Instruction::Call {
                    dest: dest.clone(),
                    function: function.clone(),
                    args: arg_vars,
                });
                Ok(dest)
            }
            
            Expr::If { condition, then_branch, else_branch } => {
                // Compile condition
                let cond_var = self.compile_expr(condition)?;
                
                // Create blocks for branches
                let then_block = self.fresh_block();
                let else_block = self.fresh_block();
                let merge_block = self.fresh_block();
                let result_var = self.fresh_var();
                
                // Jump to appropriate branch
                self.emit(Instruction::JumpIf {
                    condition: cond_var,
                    target: then_block,
                });
                self.emit(Instruction::Jump { target: else_block });
                
                // Compile then branch
                self.start_block(then_block, vec![]);
                let then_result = self.compile_expr(then_branch)?;
                self.emit(Instruction::LoadVar {
                    dest: result_var.clone(),
                    source: then_result,
                });
                self.emit(Instruction::Jump { target: merge_block });
                
                // Compile else branch
                self.start_block(else_block, vec![]);
                if let Some(else_expr) = else_branch {
                    let else_result = self.compile_expr(else_expr)?;
                    self.emit(Instruction::LoadVar {
                        dest: result_var.clone(),
                        source: else_result,
                    });
                } else {
                    // No else branch, use null
                    self.emit(Instruction::LoadConst {
                        dest: result_var.clone(),
                        value: Literal::Null,
                    });
                }
                self.emit(Instruction::Jump { target: merge_block });
                
                // Continue at merge block
                self.start_block(merge_block, vec![]);
                Ok(result_var)
            }
        }
    }
    
    /// Compile a statement
    fn compile_stmt(&mut self, stmt: &Stmt) -> Result<()> {
        match stmt {
            Stmt::Let { name, value } => {
                let value_var = self.compile_expr(value)?;
                self.emit(Instruction::LoadVar {
                    dest: VarId(name.clone()),
                    source: value_var,
                });
                Ok(())
            }
            
            Stmt::Expression(expr) => {
                self.compile_expr(expr)?;
                Ok(())
            }
            
            Stmt::Return(expr) => {
                let value = match expr {
                    Some(e) => Some(self.compile_expr(e)?),
                    None => None,
                };
                self.emit(Instruction::Return { value });
                Ok(())
            }
            
            Stmt::Print(expr) => {
                let value = self.compile_expr(expr)?;
                self.emit(Instruction::Print { value });
                Ok(())
            }
        }
    }
    
    /// Compile a simple program (for testing)
    pub fn compile_simple(&mut self, statements: Vec<Stmt>) -> Result<Program> {
        let first_block = self.fresh_block();
        // Create main function
        self.current_function = Some(Function {
            name: "main".to_string(),
            parameters: vec![],
            blocks: vec![],
            entry_block: first_block,
        });
        
        // Start the entry block
        self.start_block(first_block, vec![]);
        
        // Compile all statements
        for stmt in statements {
            self.compile_stmt(&stmt)?;
        }
        
        // Add implicit return if needed
        if let Some(ref block) = self.current_block {
            if block.instructions.is_empty() || 
               !matches!(block.instructions.last(), Some(Instruction::Return { .. })) {
                self.emit(Instruction::Return { value: None });
            }
        }
        
        // Finish current block
        if let Some(block) = self.current_block.take() {
            if let Some(ref mut function) = self.current_function {
                function.blocks.push(block);
            }
        }
        
        // Create program
        let mut program = Program::new("main".to_string());
        if let Some(function) = self.current_function.take() {
            program.add_function(function);
        }
        
        Ok(program)
    }
} 