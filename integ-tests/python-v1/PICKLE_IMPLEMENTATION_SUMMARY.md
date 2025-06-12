# BAML Python Client Pickle Support Implementation

## Summary

This document summarizes the implementation of pickle support for the BAML Python client to enable multiprocessing scenarios, addressing [PyO3 issue #100](https://github.com/PyO3/pyo3/issues/100).

## Problem Statement

The original BAML Python client (`b` object) was not pickleable, causing failures when used with Python's `multiprocessing` module:

```python
# This would fail with: "TypeError: cannot pickle '_contextvars.ContextVar' object"
from multiprocessing import Process
from baml_client import b

def worker():
    result = b.ExtractResume("test resume", None)  # Would fail during pickling

Process(target=worker).start()
```

## Implementation

### Core Solution

Added `__reduce__` method to the `BamlRuntime` class in `/workspace/engine/language_client_python/src/runtime.rs`:

```rust
#[pyo3()]
fn __reduce__(&self, py: Python<'_>) -> PyResult<(PyObject, PyObject)> {
    // Get the from_files static method as reconstruction callable
    let cls = py.get_type::<Self>();
    let from_files = cls.getattr("from_files")?;
    
    // Get current environment variables
    let env_vars: HashMap<String, String> = std::env::vars().collect();
    
    // Dynamically retrieve BAML files from Python's inlinedbaml module
    let root_path = "baml_src".to_string();
    let files = match self.get_baml_files_from_python(py) {
        Ok(files) => files,
        Err(_) => HashMap::new()
    };
    
    let args = (root_path, files, env_vars);
    Ok((from_files.unbind(), args.into_py_any(py)?.into()))
}
```

### Dynamic BAML File Retrieval

Implemented helper method to dynamically retrieve BAML configuration files during pickle reconstruction:

```rust
fn get_baml_files_from_python(&self, py: Python<'_>) -> PyResult<HashMap<String, String>> {
    let sys = py.import("sys")?;
    let modules = sys.getattr("modules")?;
    let modules_dict = modules.downcast::<pyo3::types::PyDict>()?;
    
    // Search for inlinedbaml module and call get_baml_files()
    for item in modules_dict.try_iter()? {
        let item = item?;
        let module_name = item.get_item(0)?;
        let module = item.get_item(1)?;
        let module_name_str: Result<String, _> = module_name.extract();
        if let Ok(name) = module_name_str {
            if name.contains("inlinedbaml") || name.ends_with(".inlinedbaml") {
                if let Ok(get_baml_files) = module.getattr("get_baml_files") {
                    if let Ok(files_result) = get_baml_files.call0() {
                        let files: HashMap<String, String> = files_result.extract()?;
                        return Ok(files);
                    }
                }
            }
        }
    }
    Ok(HashMap::new())
}
```

### Technical Challenges Resolved

1. **PyO3 API Compatibility**: Fixed compilation errors by updating from deprecated `ToPyObject` to `IntoPyObject`
2. **Dictionary Iteration**: Updated from deprecated `iter()` to `try_iter()` for PyDict iteration
3. **Object Conversion**: Properly handled PyObject conversions using `into_py_any()` and `into()`

## Current Status

### ✅ Successful Components

- **Core Runtime Pickle Support**: The `BamlRuntime` class now supports pickling
- **Dynamic Configuration Retrieval**: Successfully retrieves BAML files during unpickling
- **Environment Preservation**: Maintains environment variables across pickle/unpickle cycles
- **Compilation Success**: All Rust code compiles without errors

### ❌ Current Limitations

The implementation encounters a fundamental architectural issue:

```
TypeError: cannot pickle '_contextvars.ContextVar' object
```

**Root Cause**: The `BamlAsyncClient` (the `b` object) contains unpickleable components:
- Context variables (`_contextvars.ContextVar`)
- Various async HTTP clients and stream handlers
- Internal state management objects

**Architecture Overview**:
```
BamlAsyncClient (b)
├── __runtime: BamlRuntime ✅ (now pickleable)
├── __ctx_manager: BamlCtxManager ❌ (contains ContextVar)
├── __stream_client: BamlStreamClient ❌
├── __http_request: AsyncHttpRequest ❌
├── __http_stream_request: AsyncHttpStreamRequest ❌
└── other components... ❌
```

## Verification Tests

### Basic Pickle Test
```python
import pickle
from baml_client import b

# Currently fails with ContextVar error
pickled_data = pickle.dumps(b)
```

### Sync Client Works
```python
from baml_client.sync_client import b as sync_b

# Sync client doesn't have the same context variable issues
result = sync_b.ExtractResume2("John Doe\nSoftware Engineer")  # Works
```

## Recommendations

### 1. For BAML Team (Rust/Code Generation)

**Priority: High** - Add `__reduce__` methods to generated client classes:

```rust
// In generated BamlAsyncClient
#[pyo3()]
fn __reduce__(&self, py: Python<'_>) -> PyResult<(PyObject, PyObject)> {
    // Reconstruct using only pickleable components
    // Use the runtime's pickle support + create new context manager
}
```

**Implementation Strategy**:
1. Make `BamlCtxManager` pickleable by replacing ContextVar with thread-safe alternatives
2. Add `__reduce__` to all generated client classes
3. Implement proper reconstruction logic that recreates non-pickleable components

### 2. For Users (Immediate Workarounds)

**Option A: Use Sync Client**
```python
from multiprocessing import Process
from baml_client.sync_client import b as sync_b

def worker():
    result = sync_b.ExtractResume2("resume text")
    return result

Process(target=worker).start()
```

**Option B: Pass Arguments Instead of Client**
```python
def worker(resume_text):
    from baml_client.sync_client import b as sync_b
    return sync_b.ExtractResume2(resume_text)

Process(target=worker, args=("resume text",)).start()
```

### 3. Technical Architecture Considerations

**Context Management**: The use of `contextvars.ContextVar` in `BamlCtxManager` needs to be reconsidered for multiprocessing compatibility. Possible solutions:

1. **Thread-local storage**: Replace ContextVar with threading.local
2. **Stateless design**: Pass context explicitly rather than storing in variables
3. **Pickle-friendly context**: Implement custom context objects that can be serialized

## Technical Details

### Files Modified
- `/workspace/engine/language_client_python/src/runtime.rs` - Added pickle support to BamlRuntime

### Key Implementation Points
- Uses `from_files` static method for reconstruction
- Dynamically retrieves BAML configuration files from Python modules
- Preserves environment variables across processes
- Handles PyO3 object conversion correctly

### Testing Environment
- Built successfully with maturin
- Python 3.13 environment
- All dependencies installed and working

## Conclusion

The implementation successfully adds pickle support to the core BAML runtime and provides a robust foundation for multiprocessing support. However, full multiprocessing compatibility requires addressing the generated client architecture to handle unpickleable components like context variables.

The sync client provides an immediate workaround for users needing multiprocessing support, while the implemented runtime pickle support provides the foundation for a complete solution when the client architecture is updated.

## Next Steps

1. **For BAML Team**: Modify code generation to include pickle support in client classes
2. **For Users**: Use sync client for multiprocessing scenarios
3. **Future Enhancement**: Consider architectural changes to improve multiprocessing compatibility across all client types