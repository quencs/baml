
# BAML Go Client vs Rust Client CFFI Analysis

After analyzing both the Go and Rust language clients, I've identified significant differences in their CFFI implementation and several missing features in the Rust client.

## Overview

The Go client implements a **complete CFFI interface** while the Rust client has **basic CFFI bindings** with critical missing features.

---

Plan to make the same thing for Rust!

## Complete CFFI Function Coverage Analysis

### Go Client: ✅ **COMPLETE** (All 11 CFFI Functions)
1. `version()` ✅
2. `create_baml_runtime()` ✅
3. `destroy_baml_runtime()` ✅
4. `invoke_runtime_cli()` ✅
5. `register_callbacks()` ✅
6. `call_function_from_c()` ✅
7. `call_function_stream_from_c()` ✅
8. `call_function_parse_from_c()` ✅
9. `call_object_constructor()` ✅
10. `call_object_method()` ✅
11. `free_buffer()` ✅

### Rust Client: ❌ **INCOMPLETE** (Only 5/11 Functions)
1. `version()` ✅
2. `create_baml_runtime()` ✅
3. `destroy_baml_runtime()` ✅
4. `invoke_runtime_cli()` ❌ **MISSING**
5. `register_callbacks()` ❌ **MISSING**
6. `call_function_from_c()` ✅
7. `call_function_stream_from_c()` ✅
8. `call_function_parse_from_c()` ❌ **MISSING**
9. `call_object_constructor()` ❌ **MISSING**
10. `call_object_method()` ❌ **MISSING**
11. `free_buffer()` ❌ **MISSING**

## Missing Implementation Details

### **1. Callback System** ❌
**Go Implementation:**
```go
// Complete async callback system
extern void trigger_callback(uint32_t id, int is_done, const int8_t *content, int length);
extern void error_callback(uint32_t id, int is_done, const int8_t *content, int length);
extern void on_tick_callback(uint32_t id);

func RegisterCallbacks(callbackFn, errorFn, onTickFn unsafe.Pointer) error
```

**Rust Gap:** No callback registration system at all

### **2. Parse-Only Execution** ❌
**Go Implementation:**
```go
func (r *BamlRuntime) CallFunctionParse(ctx context.Context, functionName string, encoded_args []byte) (any, error)
```

**Rust Gap:** Missing `call_function_parse_from_c` binding

### **3. Object System** ❌
**Go Implementation:**
```go
struct Buffer call_object_constructor(const char *encoded_args, uintptr_t length);
struct Buffer call_object_method(const void *runtime, const char *encoded_args, uintptr_t length);
void free_buffer(struct Buffer buf);
```

**Rust Gap:** No object construction/method system

### **4. CLI Integration** ❌
**Go Implementation:**
```go
func InvokeRuntimeCli(args []string) int
```

**Rust Gap:** No CLI integration

### **5. Protocol Buffer CFFI** ❌
**Go Implementation:**
```go
import "github.com/boundaryml/baml/engine/language_client_go/pkg/cffi"

// Complete type system integration
func (c SimpleArrays) Encode() (*cffi.CFFIValueHolder, error)
func (c *SimpleArrays) Decode(holder *cffi.CFFIValueClass, typeMap baml.TypeMap)
```

**Rust Gap:** No protocol buffer CFFI equivalent

## Priority Implementation Order

### **🔥 Critical (Blocking Basic Usage)**
1. **`register_callbacks()`** - Essential for async operations
2. **`call_function_parse_from_c()`** - Parse-only execution mode
3. **Callback system integration** - Required for non-blocking calls

### **⚡ High Priority (Advanced Features)**
4. **`call_object_constructor()` & `call_object_method()`** - Object system
5. **`free_buffer()`** - Memory management
6. **Protocol buffer CFFI** - Type safety & compatibility

### **📋 Medium Priority (Developer Experience)**
7. **`invoke_runtime_cli()`** - CLI integration
8. **TypeBuilder system** - Dynamic type construction
9. **Streaming types** - Parallel type system

## Conclusion

The Rust client implements **only 45% (5/11)** of the CFFI functions that the Go client has. The missing features represent critical functionality gaps that prevent feature parity.

## Core Runtime APIs

### **BamlRuntime** - Main runtime instance
- `CreateRuntime(root_path string, src_files map[string]string, env_vars map[string]string) (BamlRuntime, error)`
- `(r *BamlRuntime) CallFunction(ctx context.Context, functionName string, encoded_args []byte, onTick OnTickCallbackData) (*ResultCallback, error)`
- `(r *BamlRuntime) CallFunctionStream(ctx context.Context, functionName string, encoded_args []byte, onTick OnTickCallbackData) (<-chan ResultCallback, error)`
- `(r *BamlRuntime) CallFunctionParse(ctx context.Context, functionName string, encoded_args []byte) (any, error)`

### **ClientRegistry** - Client management
- `NewClientRegistry() *ClientRegistry`

### **Utility Functions**
- `InvokeRuntimeCli(args []string) int`
- `SetTypeMap(t serde.TypeMap)`

## Data Encoding/Decoding APIs

### **Core Encoding Functions**
- `EncodeClass(name func() *cffi.CFFITypeName, fields map[string]any, dynamicFields *map[string]any) (*cffi.CFFIValueHolder, error)`
- `EncodeEnum(name func() *cffi.CFFITypeName, value string, is_dynamic bool) (*cffi.CFFIValueHolder, error)`
- `EncodeUnion(name func() *cffI.CFFITypeName, variant string, value any) (*cffi.CFFIValueHolder, error)`

### **Decoding Functions**
- `Decode(holder *cffi.CFFIValueHolder) reflect.Value`
- `DecodeStreamingState[T any](holder *cffi.CFFIValueHolder, decodeFunc func(inner *cffi.CFFIValueHolder) T) shared.StreamState[T]`
- `DecodeChecked[T any](holder *cffi.CFFIValueHolder, decodeFunc func(inner *cffi.CFFIValueHolder) T) shared.Checked[T]`

### **Type Casting Functions**
- `CastChecked[T any](value any, castFunc func(inner any) T) shared.Checked[T]`
- `CastStreamState[T any](value any, castFunc func(inner any) T) shared.StreamState[T]`

## Data Structures & Types

### **Core Types**
- `BamlFunctionArguments` - Function call arguments structure
- `ResultCallback` - Function result wrapper
- `BamlError`, `BamlClientError`, `BamlClientHttpError` - Error types
- `TypeMap` - Type mapping interface
- `Checked[T]` - Generic checked result wrapper
- `StreamState[T]` - Generic streaming state wrapper

### **Streaming Constants**
- `StreamStatePending`
- `StreamStateIncomplete` 
- `StreamStateComplete`

## Media & Content APIs

### **Media Types**
- `Image`, `Audio`, `PDF`, `Video` interfaces
- `media` interface for generic media handling

### **HTTP APIs**
- `HTTPRequest`, `HTTPResponse`, `HTTPBody` interfaces
- `SSEResponse` interface for Server-Sent Events

## LLM & Function Execution APIs

### **LLM Call Tracking**
- `LLMCall`, `LLMStreamCall` interfaces
- `Usage` interface for token usage tracking
- `Timing`, `StreamTiming` interfaces for performance metrics
- `FunctionLog` interface for function execution logging

### **Collectors & Observability**
- `Collector` interface for collecting execution data
- `OnTickCallbackData` interface for streaming callbacks

## Type Building & Schema APIs

### **Type Builders**
- `TypeBuilder` interface for dynamic type construction
- `ClassBuilder`, `EnumBuilder`, `UnionBuilder` interfaces
- `ClassPropertyBuilder`, `EnumValueBuilder` interfaces
- `Type` interface for type definitions

### **LLM Renderable**
- `llmRenderable` interface for LLM-compatible objects

## Callback & Async APIs

### **Callback Management**
- `OnTickCallbackData` interface for streaming callbacks
- `TickCallback` function type for tick events
- `FunctionSignal` interface for function signals

## Usage Examples from Generated Tests

The generated tests show these APIs being used for:

1. **Function Execution**: `bamlRuntime.CallFunction()`, `bamlRuntime.CallFunctionStream()`
2. **Type Management**: `baml.SetTypeMap(typeMap)`
3. **Runtime Creation**: `baml.CreateRuntime("./baml_src", getBamlFiles(), getEnvVars(nil))`
4. **Argument Encoding**: `args.Encode()` on `BamlFunctionArguments`
5. **Streaming**: Channel-based streaming with `CallFunctionStream`
6. **Error Handling**: Using `ResultCallback.Error` for error propagation

This comprehensive API surface enables the Go client to handle all aspects of BAML function execution, from basic calls to advanced streaming, type management, and observability features.