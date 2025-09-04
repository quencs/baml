# How the BAML CFFI Layer Works

This document explains how the `baml_cffi` crate functions as a bridge between the Rust-based BAML runtime and language clients, based on investigation and validation of the codebase.

## Architecture Overview

The `baml_cffi` crate serves as a **C Foreign Function Interface (FFI)** that provides a language-agnostic way to access the BAML runtime:

```
┌─────────────────────────┐
│   BAML Source Files     │  (.baml files)
│   (Functions, Types,    │
│    Clients, etc.)       │
└─────────┬───────────────┘
          │
          ▼
┌─────────────────────────┐
│     BAML Compiler       │  (Rust - baml-lib, baml-runtime)
│   - Parses .baml files  │
│   - Generates AST/IR    │
│   - Validates syntax    │
└─────────┬───────────────┘
          │
          ▼
┌─────────────────────────┐
│    baml_cffi (Rust)     │  ◄── This is the bridge
│   - C FFI Interface     │
│   - Wraps baml-runtime  │
│   - Protobuf messages   │
└─────────┬───────────────┘
          │
          ├─────────────────────────────────────────┐
          ▼                                         ▼
┌─────────────────────────┐               ┌─────────────────────────┐
│   Go Language Client    │               │  Rust Language Client   │
│   - Uses baml_cffi      │               │  - Uses baml_cffi       │
│   - CGO bindings        │               │  - Direct Rust calls    │
└─────────────────────────┘               └─────────────────────────┘
```

## Key Discovery: Interpreted Runtime Model

Our investigation revealed that BAML uses an **interpreted runtime model**, not a compiled one.

### Evidence from Code Analysis

#### 1. Runtime Creation from Source Files

In `/src/ffi/runtime.rs:52`:

```rust
let runtime = BamlRuntime::from_file_content(root_path_str, &src_files, env_vars)
```

The runtime is created from **file content at runtime**, not pre-compiled code.

#### 2. Source Files as Strings

In the generated client files (e.g., `/integ-tests/typescript/baml_client/inlinedbaml.ts`):

```typescript
const fileMap = {
  "clients.baml": "retry_policy Bar {\n  max_retries 3\n  strategy {\n    type exponential_backoff\n  }\n}\n\nclient<llm> GPT4 {\n  provider openai\n  options {\n    model gpt-4o\n    api_key env.OPENAI_API_KEY\n  }\n}",
  "custom-task.baml": "class BookOrder {\n  orderId string @description(#\"\n    The ID of the book order\n  \"#)\n  // ... more BAML source code as strings
}
```

The actual BAML source code is embedded as **raw strings** in the generated clients, not compiled bytecode.

#### 3. Runtime Parsing Process

In `/baml-runtime/src/runtime/mod.rs:77-80`:

```rust
let mut schema = validate(&PathBuf::from(directory), contents.clone());
schema.diagnostics.to_result()?;

let ir = IntermediateRepr::from_parser_database(&schema.db, schema.configuration)?;
```

The runtime **parses BAML source files and builds an Intermediate Representation (IR) at runtime**.

## Validation Process

### Problem Encountered

Initially, `baml-client-rust` couldn't find the `baml_cffi` dependency:

```
error[E0432]: unresolved import `baml_cffi`
 --> language_client_rust/src/ffi.rs:7:9
  |
7 | pub use baml_cffi::{
  |         ^^^^^^^^^ use of unresolved module or unlinked crate `baml_cffi`
```

### Root Cause Analysis

The issue was in `/language_client_cffi/Cargo.toml`:

```toml
[lib]
crate-type = ["cdylib"]  # Only produces C dynamic library
```

This configuration only produces a C dynamic library (`.so`/`.dylib`/`.dll`) for other languages, but doesn't create a Rust library that other Rust crates can import.

### Solution Applied

We modified the crate configuration to produce both formats:

```toml
[lib]
crate-type = ["cdylib", "rlib"]  # Produces both C library and Rust library
```

- `cdylib`: For non-Rust languages (Go, Python, etc.) via C FFI
- `rlib`: For Rust-to-Rust dependencies

### Verification

After the fix, the build succeeded:

```bash
$ cargo build -p baml-client-rust
   Compiling baml_cffi v0.205.0 (/Users/.../language_client_cffi)
   Compiling baml-client-rust v0.205.0 (/Users/.../language_client_rust)
    Finished `dev` profile [unoptimized + debuginfo] target(s)
```

## Core FFI Functions

The `baml_cffi` crate exposes these key C functions:

- **Runtime Management**: `create_baml_runtime()`, `destroy_baml_runtime()`
- **Function Execution**: `call_function_from_c()`, `call_function_stream_from_c()`
- **Parsing**: `call_function_parse_from_c()`
- **Callbacks**: `register_callbacks()` for streaming and progress updates
- **Utilities**: `version()`, `invoke_runtime_cli()`

## Project-Level Isolation

Each BAML project generates its own `cdylib`:

- **Project Foo**: → `libbaml_cffi_foo.so` (contains Foo's runtime + functions)
- **Project Bar**: → `libbaml_cffi_bar.so` (contains Bar's runtime + functions)

Each project maintains complete isolation with its own:
- Function registry
- Client configurations  
- Type schemas
- Runtime environment

## What Changes When You Modify BAML Logic

When you modify a BAML function (e.g., changing a prompt template):

1. **Source Code Changes**: Generated client files (`inlinedbaml.ts`) get new BAML source strings
2. **Same Binary Interface**: The `libbaml_cffi.so` C FFI interface remains unchanged
3. **Runtime Re-interpretation**: Next runtime startup parses new source and builds new IR
4. **Different Behavior**: Same function signature, but different execution logic

## Why This Architecture

This interpreted model enables BAML's key features:

- **Dynamic Types**: `@@dynamic` classes can be defined at runtime
- **Runtime Schema Validation**: Types and constraints are validated during execution
- **Hot Reloading**: Source changes don't require recompiling the runtime binary
- **Cross-Language Consistency**: All language clients use the same tested FFI interface
- **Flexible Deployment**: Logic changes only require updating source strings

## Implications for Development

- **Logic Changes**: Only require regenerating client files, not rebuilding the runtime
- **Type Changes**: Validated at runtime during function execution
- **Debugging**: Source code is available for runtime error reporting
- **Performance**: Parse/validate overhead on runtime initialization, but cached thereafter