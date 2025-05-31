//! Value representation in the VM
//! 
//! This module defines how values are represented at runtime, including
//! support for colorless promises (async values).

use crate::VmError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

/// Runtime values in the VM
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    
    // For object/record support (future)
    Object(HashMap<String, Value>),
    
    // For array support (future)
    Array(Vec<Value>),
    
    // For function references (future)
    Function(String),
    
    // Promise handle for async operations
    Promise(PromiseId),
}

/// A unique identifier for a promise
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PromiseId(pub u64);

/// Colorless value representation for async support
/// Based on the design document's ColorlessValue enum
#[derive(Debug, Clone)]
pub enum ColorlessValue {
    /// Value is still being computed
    Pending(PromiseId),
    
    /// Value has been successfully computed
    Done(Value),
    
    /// An error occurred during computation
    Error(Arc<VmError>),
}

impl Value {
    /// Convert to a boolean value
    pub fn to_bool(&self) -> Result<bool, VmError> {
        match self {
            Value::Bool(b) => Ok(*b),
            Value::Null => Ok(false),
            Value::Int(0) => Ok(false),
            Value::Int(_) => Ok(true),
            Value::Float(f) if *f == 0.0 => Ok(false),
            Value::Float(_) => Ok(true),
            Value::String(s) => Ok(!s.is_empty()),
            _ => Err(VmError::TypeError(format!("Cannot convert {:?} to bool", self))),
        }
    }
    
    /// Convert to an integer value
    pub fn to_int(&self) -> Result<i64, VmError> {
        match self {
            Value::Int(i) => Ok(*i),
            Value::Float(f) => Ok(*f as i64),
            Value::Bool(true) => Ok(1),
            Value::Bool(false) => Ok(0),
            _ => Err(VmError::TypeError(format!("Cannot convert {:?} to int", self))),
        }
    }
    
    /// Convert to a float value
    pub fn to_float(&self) -> Result<f64, VmError> {
        match self {
            Value::Float(f) => Ok(*f),
            Value::Int(i) => Ok(*i as f64),
            _ => Err(VmError::TypeError(format!("Cannot convert {:?} to float", self))),
        }
    }
    
    /// Convert to a string value
    pub fn to_string_value(&self) -> String {
        match self {
            Value::Null => "null".to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Int(i) => i.to_string(),
            Value::Float(f) => f.to_string(),
            Value::String(s) => s.clone(),
            Value::Object(_) => "[object]".to_string(),
            Value::Array(_) => "[array]".to_string(),
            Value::Function(name) => format!("[function: {}]", name),
            Value::Promise(id) => format!("[promise: {}]", id.0),
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string_value())
    }
}

/// Helper functions for arithmetic operations
impl Value {
    pub fn add(&self, other: &Value) -> Result<Value, VmError> {
        match (self, other) {
            (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a + b)),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a + b)),
            (Value::Int(a), Value::Float(b)) => Ok(Value::Float(*a as f64 + b)),
            (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a + *b as f64)),
            (Value::String(a), Value::String(b)) => Ok(Value::String(format!("{}{}", a, b))),
            _ => Err(VmError::TypeError(format!("Cannot add {:?} and {:?}", self, other))),
        }
    }
    
    pub fn sub(&self, other: &Value) -> Result<Value, VmError> {
        match (self, other) {
            (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a - b)),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a - b)),
            (Value::Int(a), Value::Float(b)) => Ok(Value::Float(*a as f64 - b)),
            (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a - *b as f64)),
            _ => Err(VmError::TypeError(format!("Cannot subtract {:?} and {:?}", self, other))),
        }
    }
    
    pub fn mul(&self, other: &Value) -> Result<Value, VmError> {
        match (self, other) {
            (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a * b)),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a * b)),
            (Value::Int(a), Value::Float(b)) => Ok(Value::Float(*a as f64 * b)),
            (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a * *b as f64)),
            _ => Err(VmError::TypeError(format!("Cannot multiply {:?} and {:?}", self, other))),
        }
    }
    
    pub fn div(&self, other: &Value) -> Result<Value, VmError> {
        match (self, other) {
            (_, Value::Int(0)) => Err(VmError::RuntimeError("Division by zero".to_string())),
            (_, Value::Float(f)) if *f == 0.0 => Err(VmError::RuntimeError("Division by zero".to_string())),
            (Value::Int(a), Value::Int(b)) => Ok(Value::Float(*a as f64 / *b as f64)),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a / b)),
            (Value::Int(a), Value::Float(b)) => Ok(Value::Float(*a as f64 / b)),
            (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a / *b as f64)),
            _ => Err(VmError::TypeError(format!("Cannot divide {:?} by {:?}", self, other))),
        }
    }
    
    pub fn lt(&self, other: &Value) -> Result<Value, VmError> {
        match (self, other) {
            (Value::Int(a), Value::Int(b)) => Ok(Value::Bool(a < b)),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Bool(a < b)),
            (Value::Int(a), Value::Float(b)) => Ok(Value::Bool((*a as f64) < *b)),
            (Value::Float(a), Value::Int(b)) => Ok(Value::Bool(*a < (*b as f64))),
            (Value::String(a), Value::String(b)) => Ok(Value::Bool(a < b)),
            _ => Err(VmError::TypeError(format!("Cannot compare {:?} < {:?}", self, other))),
        }
    }
    
    pub fn eq(&self, other: &Value) -> Result<Value, VmError> {
        Ok(Value::Bool(self == other))
    }
} 