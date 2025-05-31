# BAML Bytecode VM

This crate implements a bytecode-based virtual machine for executing BAML programs.

## Architecture Overview

The VM follows the design outlined in the architecture decision document, with these key components:

### 1. **Bytecode Representation** (`bytecode.rs`)
- **Instructions**: Simple operations that do one thing each
- **Basic Blocks**: Groups of instructions with explicit control flow
- **Functions**: Collections of basic blocks with parameters
- **Program**: Collection of functions with an entry point

### 2. **Value System** (`value.rs`)
- **Value**: Runtime representation of data (null, bool, int, float, string, objects, arrays)
- **ColorlessValue**: Support for async operations with Pending/Done/Error states
- **PromiseId**: Unique identifiers for async operations

### 3. **Virtual Machine** (`vm.rs`)
- **ExecutionScope**: Manages local variables and program counter
- **VirtualMachine**: Executes bytecode programs
- **Call Stack**: Supports function calls and returns

### 4. **Compiler** (`compiler.rs`)
- Transforms AST into bytecode
- Generates SSA-style variable names (`_local.0`, `_local.1`, etc.)
- Handles control flow with basic blocks

## Example Usage

```rust
use baml_vm::{
    bytecode::Literal,
    compiler::{BinaryOp, Compiler, Expr, Stmt},
    VirtualMachine,
};

// Create a simple program
let statements = vec![
    Stmt::Let {
        name: "x".to_string(),
        value: Expr::Literal(Literal::Int(10)),
    },
    Stmt::Let {
        name: "y".to_string(),
        value: Expr::Literal(Literal::Int(20)),
    },
    Stmt::Let {
        name: "sum".to_string(),
        value: Expr::Binary {
            op: BinaryOp::Add,
            left: Box::new(Expr::Variable("x".to_string())),
            right: Box::new(Expr::Variable("y".to_string())),
        },
    },
    Stmt::Return(Some(Expr::Variable("sum".to_string()))),
];

// Compile to bytecode
let mut compiler = Compiler::new();
let program = compiler.compile_simple(statements)?;

// Execute
let mut vm = VirtualMachine::new(program)?;
let result = vm.execute()?;
```

## Bytecode Example

The above program compiles to bytecode like:

```
Function main:
  Block 0:
    LoadConst _local.0 = 10
    LoadVar x = _local.0
    LoadConst _local.1 = 20
    LoadVar y = _local.1
    Add _local.2 = x + y
    LoadVar sum = _local.2
    Return sum
```

## Current Features

- ✅ Basic arithmetic operations (add, sub, mul, div)
- ✅ Comparison operations (lt, gt, eq)
- ✅ Boolean operations (and, or, not)
- ✅ Control flow (if expressions, jumps)
- ✅ Variable assignment and loading
- ✅ Function structure (single function for now)
- ✅ Print statements

## Planned Features

Based on the design document:

- 🚧 Function calls
- 🚧 Async/await (colorless promises)
- 🚧 For/while loops
- 🚧 Exception handling
- 🚧 Streaming support
- 🚧 Debugger support
- 🚧 Source mapping

## Design Principles

1. **Simple Instructions**: Each bytecode instruction does exactly one thing
2. **SSA-like**: Temporary variables ensure values are immutable within blocks
3. **Explicit Control Flow**: Uses basic blocks and jumps rather than nested structures
4. **Colorless Async**: Async is handled at the value level, not the instruction level
5. **Extensible**: Easy to add new instructions and features

## Integration with BAML

Currently, the compiler uses simplified AST types for demonstration. To integrate with the actual BAML language:

1. Replace the demo AST types in `compiler.rs` with imports from `schema-ast`
2. Implement proper symbol table and type checking
3. Add support for BAML-specific features (classes, templates, etc.)

## Testing

Run tests with:
```bash
cargo test -p baml-vm
```

The tests demonstrate:
- Basic arithmetic compilation and execution
- If-expression control flow
- String operations 