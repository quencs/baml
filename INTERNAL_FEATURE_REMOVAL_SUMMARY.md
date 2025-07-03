# Summary: Removal of `internal` Feature Flag from baml-runtime

## Overview
This document summarizes the changes made to remove the `--cfg internal` feature flag from baml-runtime and make all "internal" functionality always available.

## Changes Made

### 1. baml-runtime Cargo.toml
- **File**: `engine/baml-runtime/Cargo.toml`
- **Change**: Removed the `internal = []` feature from the `[features]` section
- **Before**: 
  ```toml
  [features]
  defaults = ["skip-integ-tests"]
  internal = []
  skip-integ-tests = []
  ```
- **After**:
  ```toml
  [features]
  defaults = ["skip-integ-tests"]
  skip-integ-tests = []
  ```

### 2. baml-runtime lib.rs
- **File**: `engine/baml-runtime/src/lib.rs`
- **Changes**:
  - Removed conditional compilation for `internal` module - now always public
  - Removed conditional compilation for `InternalRuntimeInterface` export - now always public
  - Removed conditional compilation for `ChatMessagePart` and `RenderedPrompt` exports - now always public
  - Removed conditional compilation for `internal()` method - now always public
  - Added test to verify internal functionality is always available

### 3. Test Files
- **Files**: 
  - `engine/baml-runtime/tests/test_runtime.rs`
  - `engine/baml-runtime/tests/test_log_collector.rs`
- **Changes**: 
  - Removed `#[cfg(feature = "internal")]` conditional compilation
  - Updated comment instructions to remove `--features "internal"` flag
  - Tests now run without requiring the internal feature flag

### 4. Dependent Crates
- **Files**:
  - `engine/language_client_typescript/Cargo.toml`
  - `engine/language_client_ruby/ext/ruby_ffi/Cargo.toml`
  - `engine/cli/Cargo.toml`
- **Changes**: Removed `features = ["internal"]` from baml-runtime dependencies

### 5. Documentation and Scripts
- **Files**:
  - `CONTRIBUTING.md`
  - `engine/baml-runtime/tests/test_cli.rs`
  - `engine/baml-schema-wasm/src/runtime_wasm/mod.rs`
  - `tools/build`
- **Changes**: Updated comments and build instructions to remove references to the internal feature flag

## API Changes

### Public API Expansion
The following items are now **always** available in the public API:

1. **`internal` module**: Previously conditionally compiled, now always public
2. **`InternalRuntimeInterface` trait**: Now always exported publicly
3. **`ChatMessagePart` and `RenderedPrompt` types**: Now always exported publicly
4. **`BamlRuntime::internal()` method**: Now always available

### Breaking Changes
- **None**: This is a pure expansion of the public API. All existing code that worked before will continue to work.
- **Benefit**: Code that previously required the `internal` feature flag now works without it.

## Testing
- Added a test (`test_internal_always_available`) to verify that internal functionality is always accessible
- All existing tests continue to work without requiring the internal feature flag

## Migration Guide

### For Users
- **Before**: Had to use `--features "internal"` to access internal functionality
- **After**: Internal functionality is always available, no feature flags needed

### For Developers
- **Before**: `cargo test --features internal`
- **After**: `cargo test`

### For Dependents
- **Before**: `baml-runtime = { path = "../baml-runtime", features = ["internal"] }`
- **After**: `baml-runtime = { path = "../baml-runtime" }`

## Benefits
1. **Simplified build process**: No need to remember to include the internal feature flag
2. **Consistent API**: Internal functionality is always available
3. **Reduced complexity**: Fewer conditional compilation directives
4. **Better developer experience**: No feature flag confusion

## Validation
- All conditional compilation directives related to the internal feature have been removed
- Dependencies updated to not require the internal feature
- Documentation and build scripts updated
- Test added to verify internal functionality is always available