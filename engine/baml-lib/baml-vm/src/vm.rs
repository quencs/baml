//! Virtual Machine implementation
//! 
//! This module implements the VM execution engine, including:
//! - ExecutionScope for managing local variables and program counter
//! - VirtualMachine for executing bytecode programs
//! - Basic async support infrastructure

use crate::{
    bytecode::{BasicBlock, BlockId, Function, Instruction, Literal, Program, VarId},
    value::{ColorlessValue, PromiseId, Value},
    Result, VmError,
};
use indexmap::IndexMap;
use std::collections::HashMap;
use std::sync::Arc;

/// Execution scope as defined in the design document
/// Manages local variables and tracks the current execution position
#[derive(Debug, Clone)]
pub struct ExecutionScope {
    /// Local variables in the current scope
    pub locals: HashMap<VarId, ColorlessValue>,
    
    /// Current basic block being executed
    pub current_block: BlockId,
    
    /// Program counter within the current block
    pub program_counter: usize,
    
    /// Function name for debugging
    pub function_name: String,
}

impl ExecutionScope {
    pub fn new(function_name: String, entry_block: BlockId) -> Self {
        Self {
            locals: HashMap::new(),
            current_block: entry_block,
            program_counter: 0,
            function_name,
        }
    }
    
    /// Get a local variable value
    pub fn get_local(&self, var: &VarId) -> Result<&ColorlessValue> {
        self.locals.get(var)
            .ok_or_else(|| VmError::VariableNotFound(var.to_string()))
    }
    
    /// Set a local variable value
    pub fn set_local(&mut self, var: VarId, value: ColorlessValue) {
        self.locals.insert(var, value);
    }
    
    /// Get a resolved value (not pending)
    pub fn get_value(&self, var: &VarId) -> Result<&Value> {
        match self.get_local(var)? {
            ColorlessValue::Done(value) => Ok(value),
            ColorlessValue::Pending(id) => Err(VmError::RuntimeError(
                format!("Value {} is still pending (promise {})", var, id.0)
            )),
            ColorlessValue::Error(err) => Err(VmError::RuntimeError(
                format!("Value {} has error: {}", var, err)
            )),
        }
    }
}

/// Call stack frame
#[derive(Debug, Clone)]
struct CallFrame {
    scope: ExecutionScope,
    function: String,
    return_var: Option<VarId>,
}

/// Virtual Machine state
pub struct VirtualMachine {
    /// The program being executed
    program: Program,
    
    /// Call stack
    call_stack: Vec<CallFrame>,
    
    /// Current execution scope (top of call stack)
    current_scope: ExecutionScope,
    
    /// Promise counter for generating unique IDs
    promise_counter: u64,
    
    /// Pending promises (for future async implementation)
    pending_promises: HashMap<PromiseId, ColorlessValue>,
}

impl VirtualMachine {
    pub fn new(program: Program) -> Result<Self> {
        // Find the entry function
        let entry_fn = program.find_function(&program.entry_function)
            .ok_or_else(|| VmError::FunctionNotFound(program.entry_function.clone()))?;
        
        let current_scope = ExecutionScope::new(
            entry_fn.name.clone(),
            entry_fn.entry_block,
        );
        
        Ok(Self {
            program,
            call_stack: Vec::new(),
            current_scope,
            promise_counter: 0,
            pending_promises: HashMap::new(),
        })
    }
    
    /// Execute the program
    pub fn execute(&mut self) -> Result<Option<Value>> {
        loop {
            if let Some(value) = self.step()? {
                return Ok(Some(value));
            }
        }
    }
    
    /// Execute a single instruction
    pub fn step(&mut self) -> Result<Option<Value>> {
        let function = self.get_current_function()?;
        let block = self.get_current_block(&function)?;
        
        if self.current_scope.program_counter >= block.instructions.len() {
            return Err(VmError::RuntimeError(
                "Program counter out of bounds".to_string()
            ));
        }
        
        let instruction = &block.instructions[self.current_scope.program_counter].clone();
        println!("Executing instruction: {}", instruction);
        self.current_scope.program_counter += 1;
        
        self.execute_instruction(instruction)
    }
    
    /// Execute a single instruction
    fn execute_instruction(&mut self, instruction: &Instruction) -> Result<Option<Value>> {
        match instruction {
            Instruction::LoadConst { dest, value } => {
                let val = match value {
                    Literal::Null => Value::Null,
                    Literal::Bool(b) => Value::Bool(*b),
                    Literal::Int(i) => Value::Int(*i),
                    Literal::Float(f) => Value::Float(*f),
                    Literal::String(s) => Value::String(s.clone()),
                };
                self.current_scope.set_local(dest.clone(), ColorlessValue::Done(val));
                Ok(None)
            }
            
            Instruction::LoadVar { dest, source } => {
                let value = self.current_scope.get_local(source)?.clone();
                self.current_scope.set_local(dest.clone(), value);
                Ok(None)
            }
            
            Instruction::Add { dest, left, right } => {
                let left_val = self.current_scope.get_value(left)?;
                let right_val = self.current_scope.get_value(right)?;
                let result = left_val.add(right_val)?;
                self.current_scope.set_local(dest.clone(), ColorlessValue::Done(result));
                Ok(None)
            }
            
            Instruction::Sub { dest, left, right } => {
                let left_val = self.current_scope.get_value(left)?;
                let right_val = self.current_scope.get_value(right)?;
                let result = left_val.sub(right_val)?;
                self.current_scope.set_local(dest.clone(), ColorlessValue::Done(result));
                Ok(None)
            }
            
            Instruction::Mul { dest, left, right } => {
                let left_val = self.current_scope.get_value(left)?;
                let right_val = self.current_scope.get_value(right)?;
                let result = left_val.mul(right_val)?;
                self.current_scope.set_local(dest.clone(), ColorlessValue::Done(result));
                Ok(None)
            }
            
            Instruction::Div { dest, left, right } => {
                let left_val = self.current_scope.get_value(left)?;
                let right_val = self.current_scope.get_value(right)?;
                let result = left_val.div(right_val)?;
                self.current_scope.set_local(dest.clone(), ColorlessValue::Done(result));
                Ok(None)
            }
            
            Instruction::Lt { dest, left, right } => {
                let left_val = self.current_scope.get_value(left)?;
                let right_val = self.current_scope.get_value(right)?;
                let result = left_val.lt(right_val)?;
                self.current_scope.set_local(dest.clone(), ColorlessValue::Done(result));
                Ok(None)
            }
            
            Instruction::Eq { dest, left, right } => {
                let left_val = self.current_scope.get_value(left)?;
                let right_val = self.current_scope.get_value(right)?;
                let result = left_val.eq(right_val)?;
                self.current_scope.set_local(dest.clone(), ColorlessValue::Done(result));
                Ok(None)
            }
            
            Instruction::Gt { dest, left, right } => {
                let left_val = self.current_scope.get_value(left)?;
                let right_val = self.current_scope.get_value(right)?;
                let result = right_val.lt(left_val)?; // a > b is b < a
                self.current_scope.set_local(dest.clone(), ColorlessValue::Done(result));
                Ok(None)
            }
            
            Instruction::And { dest, left, right } => {
                let left_val = self.current_scope.get_value(left)?;
                let right_val = self.current_scope.get_value(right)?;
                let result = Value::Bool(left_val.to_bool()? && right_val.to_bool()?);
                self.current_scope.set_local(dest.clone(), ColorlessValue::Done(result));
                Ok(None)
            }
            
            Instruction::Or { dest, left, right } => {
                let left_val = self.current_scope.get_value(left)?;
                let right_val = self.current_scope.get_value(right)?;
                let result = Value::Bool(left_val.to_bool()? || right_val.to_bool()?);
                self.current_scope.set_local(dest.clone(), ColorlessValue::Done(result));
                Ok(None)
            }
            
            Instruction::Not { dest, operand } => {
                let val = self.current_scope.get_value(operand)?;
                let result = Value::Bool(!val.to_bool()?);
                self.current_scope.set_local(dest.clone(), ColorlessValue::Done(result));
                Ok(None)
            }
            
            Instruction::StoreVar { dest, source } => {
                let value = self.current_scope.get_local(source)?.clone();
                self.current_scope.set_local(dest.clone(), value);
                Ok(None)
            }
            
            Instruction::Jump { target } => {
                self.current_scope.current_block = *target;
                self.current_scope.program_counter = 0;
                Ok(None)
            }
            
            Instruction::JumpIf { condition, target } => {
                let cond_val = self.current_scope.get_value(condition)?;
                if cond_val.to_bool()? {
                    self.current_scope.current_block = *target;
                    self.current_scope.program_counter = 0;
                }
                Ok(None)
            }
            
            Instruction::JumpIfNot { condition, target } => {
                let cond_val = self.current_scope.get_value(condition)?;
                if !cond_val.to_bool()? {
                    self.current_scope.current_block = *target;
                    self.current_scope.program_counter = 0;
                }
                Ok(None)
            }
            
            Instruction::Print { value } => {
                let val = self.current_scope.get_value(value)?;
                println!("{}", val);
                Ok(None)
            }
            
            Instruction::Return { value } => {
                let return_value = match value {
                    Some(var) => Some(self.current_scope.get_value(var)?.clone()),
                    None => None,
                };
                
                // Pop call stack
                if let Some(frame) = self.call_stack.pop() {
                    self.current_scope = frame.scope;
                    if let (Some(return_var), Some(val)) = (frame.return_var, return_value) {
                        self.current_scope.set_local(return_var, ColorlessValue::Done(val));
                    }
                    Ok(None)
                } else {
                    // Returning from main function
                    Ok(return_value)
                }
            }
            
            // TODO: Implement these
            Instruction::Call { .. } => {
                Err(VmError::RuntimeError("Function calls not yet implemented".to_string()))
            }
            
            Instruction::Await { .. } => {
                Err(VmError::RuntimeError("Await not yet implemented".to_string()))
            }
            
            _ => Err(VmError::InvalidInstruction(format!("{:?}", instruction))),
        }
    }
    
    fn get_current_function(&self) -> Result<&Function> {
        self.program.find_function(&self.current_scope.function_name)
            .ok_or_else(|| VmError::FunctionNotFound(self.current_scope.function_name.clone()))
    }
    
    fn get_current_block<'a>(&self, function: &'a Function) -> Result<&'a BasicBlock> {
        function.blocks.iter()
            .find(|b| b.id == self.current_scope.current_block)
            .ok_or_else(|| VmError::RuntimeError(
                format!("Block {:?} not found", self.current_scope.current_block)
            ))
    }
} 