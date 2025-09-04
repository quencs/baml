# Rust Generator Feature Parity Implementation Plan

## Overview

This plan implements the missing advanced features to bring the BAML Rust generator to parity with the Go generator. Currently, the Rust generator generates only 4 basic files compared to Go's 11+ files with 3 directories, missing critical features like CFFI integration, streaming types, and dynamic type building.

## Current State Analysis

### What Works Today
- Basic type generation (classes, enums, unions) in `types.rs:generators/languages/rust/src/_templates/`
- Simple function clients in `client.rs:generators/languages/rust/src/_templates/client.rs.j2`
- JSON serialization via `ToBamlValue/FromBamlValue:language_client_rust/src/types.rs:11-20`
- Basic streaming wrapper `StreamState<T>`

### Critical Missing Components
1. **CFFI Type Encoding/Decoding** - No protocol buffer integration for FFI calls
2. **Streaming Types System** - No parallel streaming type hierarchy for real-time parsing
3. **TypeBuilder System** - No runtime type construction capabilities  
4. **Function Variants** - Missing parse-only and streaming-specific function types
5. **Advanced File Structure** - Only 4 files vs Go's 11 files + 3 directories

### Key Discoveries
- Go uses protocol buffers via `language_client_go/pkg/cffi/cffi.pb.go` for CFFI communication
- Streaming types use nullable pointers (`*string`) vs required fields (`string`) for partial parsing
- TypeBuilder system uses CFFI raw objects with method dispatch via `CallMethod()`
- Function variants support parse-only, streaming, and combined modes

## Desired End State

After completion, the Rust generator will:
- Generate equivalent file structure to Go (11+ files with modular directories)
- Support full CFFI integration with Encode/Decode methods on all types
- Provide streaming types hierarchy for real-time partial parsing
- Enable dynamic type construction via TypeBuilder system
- Offer complete function variants (standard, streaming, parse, parse-streaming)

### Verification Criteria
- Generated Rust client passes all Go generator integration tests
- CFFI protocol buffer communication works correctly
- Streaming types support partial parsing during LLM response streaming
- TypeBuilder enables runtime type construction and manipulation
- Performance matches Go client for equivalent operations

## What We're NOT Doing

- Changing existing Rust client API that's already working
- Modifying the core BAML compiler or runtime
- Breaking backwards compatibility with current Rust client usage
- Implementing features not present in Go generator
- Optimizing beyond Go generator performance characteristics

## Implementation Approach

The plan uses incremental implementation with each phase building on previous work. We follow the proven Go generator patterns while adapting to Rust idioms (Option<T> instead of *T, Result<T,E> for error handling, Drop trait for memory management).

## Phase 1: CFFI Protocol Buffer Integration

### Overview
Add protocol buffer-based CFFI communication to all generated types, enabling binary protocol communication with the BAML runtime.

### Changes Required

#### 1. Protocol Buffer Dependencies
**File**: `generators/languages/rust/Cargo.toml`
**Changes**: Add prost dependencies for protocol buffer support

```toml
[dependencies]
prost = "0.12"
prost-types = "0.12"
baml-cffi = { path = "../../language_client_cffi" }
```

#### 2. CFFI Trait Implementation Template
**File**: `generators/languages/rust/src/_templates/cffi_traits.rs.j2`
**Changes**: New template for CFFI encode/decode methods

```rust
use baml_cffi::baml::cffi::{CffiValueHolder, CffiTypeName, CffiTypeNamespace};

impl CffiEncode for {{ class_name }} {
    fn encode(&self) -> Result<CffiValueHolder, BamlError> {
        let mut fields = HashMap::new();
        {% for field in fields -%}
        fields.insert("{{ field.name }}".to_string(), self.{{ field.name }}.encode()?);
        {% endfor %}
        
        Ok(CffiValueHolder {
            type_name: Some(CffiTypeName {
                namespace: CffiTypeNamespace::Types as i32,
                name: "{{ class_name }}".to_string(),
            }),
            value: Some(cffi_value::Value::ClassValue(CffiValueClass { fields })),
        })
    }
}

impl CffiDecode for {{ class_name }} {
    fn decode(holder: CffiValueHolder) -> Result<Self, BamlError> {
        // Type validation and field decoding
        match holder.value {
            Some(cffi_value::Value::ClassValue(class)) => {
                Ok({{ class_name }} {
                    {% for field in fields -%}
                    {{ field.name }}: {{ field.type_rust }}.decode(class.fields.get("{{ field.name }}"))?,
                    {% endfor %}
                })
            }
            _ => Err(BamlError::TypeError("Expected class value".to_string()))
        }
    }
}
```

#### 3. Generator Integration
**File**: `generators/languages/rust/src/lib.rs`
**Changes**: Integrate CFFI template rendering at lines 174-216

```rust
pub fn render_types_rs(package: &CurrentRenderPackage) -> Result<String, anyhow::Error> {
    let mut content = String::new();
    
    // Existing type rendering
    content.push_str(&RustTypes { items: &package.types() }.render()?);
    
    // Add CFFI trait implementations
    for class in &package.ir.walk_classes() {
        content.push_str(&CffiTraits { 
            class_name: &class.name(),
            fields: &class.fields() 
        }.render()?);
    }
    
    Ok(content)
}
```

### Success Criteria

#### Automated Verification
- [ ] All generated types compile without errors: `cargo build --package generators-rust`
- [ ] CFFI protocol buffer types are correctly generated: `cargo test cffi_integration`
- [ ] Round-trip encoding/decoding tests pass: `cargo test encode_decode_roundtrip`

#### Manual Verification
- [ ] Generated types include both JSON and CFFI serialization methods
- [ ] CFFI encode/decode maintains data integrity
- [ ] Protocol buffer messages are compatible with Go client protocol

---

## Phase 2: Streaming Types Hierarchy

### Overview
Create parallel streaming type hierarchy using `Option<T>` instead of required fields, enabling partial parsing during LLM response streaming.

### Changes Required

#### 1. Streaming Type Generator
**File**: `generators/languages/rust/src/ir_to_rust/stream_types.rs`
**Changes**: New module for streaming type conversion

```rust
use crate::ir_to_rust::TypeRust;

pub fn ir_class_to_rust_stream(class: &IRClass, pkg: &CurrentRenderPackage) -> ClassRustStream {
    ClassRustStream {
        name: class.name().to_string(),
        fields: class.fields().iter().map(|f| FieldRustStream {
            name: f.name().to_string(),
            type_rust: wrap_in_option(ir_to_rust_type(&f.field_type(), pkg)),
        }).collect(),
        namespace: "stream_types".to_string(),
    }
}

fn wrap_in_option(type_rust: TypeRust) -> TypeRust {
    TypeRust {
        type_str: format!("Option<{}>", type_rust.type_str),
        // ... rest of metadata
    }
}
```

#### 2. Streaming Type Templates
**File**: `generators/languages/rust/src/_templates/stream_struct.rs.j2`
**Changes**: Template for streaming variant structures

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct {{ class_name }} {
    {% for field in fields -%}
    pub {{ field.name }}: Option<{{ field.type_rust }}>,
    {% endfor %}
}

impl CffiEncode for {{ class_name }} {
    fn encode(&self) -> Result<CffiValueHolder, BamlError> {
        // Similar to regular encode but handle Option fields
        let mut fields = HashMap::new();
        {% for field in fields -%}
        if let Some(ref value) = self.{{ field.name }} {
            fields.insert("{{ field.name }}".to_string(), value.encode()?);
        }
        {% endfor %}
        
        Ok(CffiValueHolder {
            type_name: Some(CffiTypeName {
                namespace: CffiTypeNamespace::StreamTypes as i32,
                name: "{{ class_name }}".to_string(),
            }),
            value: Some(cffi_value::Value::ClassValue(CffiValueClass { fields })),
        })
    }
}
```

#### 3. Stream Types Module Generation
**File**: `generators/languages/rust/src/lib.rs`
**Changes**: Add stream_types.rs file generation at lines 52-67

```rust
pub fn generate_sdk_files(ir: IntermediateRepr, package_name: &str) -> Result<Vec<crate::File>, anyhow::Error> {
    let mut files = vec![
        // Existing files
        crate::File {
            path: "src/lib.rs".to_string(),
            content: render_lib_rs(&package)?,
        },
        // New streaming types file
        crate::File {
            path: "src/stream_types.rs".to_string(),
            content: render_stream_types_rs(&package)?,
        },
    ];
    Ok(files)
}
```

### Success Criteria

#### Automated Verification
- [ ] Streaming types compile correctly: `cargo build`
- [ ] All streaming types are `Option<T>` wrapped: `cargo test streaming_optional_fields`
- [ ] CFFI namespace correctly set to `StreamTypes`: `cargo test cffi_stream_namespace`

#### Manual Verification
- [ ] Streaming types support partial parsing during LLM responses
- [ ] Optional fields handle null values gracefully
- [ ] Memory usage is reasonable for partial structures

---

## Phase 3: Multiple Function Variants

### Overview
Generate parse-only, streaming, and combination function variants matching Go client functionality.

### Changes Required

#### 1. Function Variant Templates
**File**: `generators/languages/rust/src/_templates/function_parse.rs.j2`
**Changes**: Parse-only function template

```rust
impl {{ client_name }} {
    pub async fn {{ fn.name }}_parse(&self, input: &str) -> Result<{{ fn.return_type.serialize_type() }}, BamlError> {
        let result = unsafe {
            language_client_cffi::call_function_parse_from_c(
                self.runtime.inner,
                c_str!("{{ fn.name }}"),
                input.as_ptr() as *const i8,
                input.len(),
                0,
            )
        };
        
        // Parse and decode result
        let decoded = decode_cffi_result(result)?;
        {{ fn.return_type.serialize_type() }}::decode(decoded)
    }
}
```

#### 2. Streaming Function Template  
**File**: `generators/languages/rust/src/_templates/function_stream.rs.j2`
**Changes**: Streaming function with channel-like interface

```rust
use tokio::sync::mpsc;

pub struct StreamValue<TStream, TFinal> {
    pub is_final: bool,
    pub error: Option<BamlError>,
    pub stream_data: Option<TStream>,
    pub final_data: Option<TFinal>,
}

impl {{ client_name }} {
    pub async fn {{ fn.name }}_stream(
        &self, 
        {{ fn.inputs | map(attribute="name") | map("snake_case") | join(", ") }}: {{ fn.inputs | map(attribute="type_rust") | join(", ") }}
    ) -> Result<mpsc::Receiver<StreamValue<stream_types::{{ fn.return_type.serialize_type() }}, {{ fn.return_type.serialize_type() }}>>, BamlError> {
        let (tx, rx) = mpsc::channel(100);
        
        // Setup streaming call with callback
        let callback = move |result: CffiValueHolder| {
            // Decode and send via channel
            let stream_val = if is_final_result(&result) {
                StreamValue {
                    is_final: true,
                    final_data: Some({{ fn.return_type.serialize_type() }}::decode(result)?),
                    ..Default::default()
                }
            } else {
                StreamValue {
                    is_final: false, 
                    stream_data: Some(stream_types::{{ fn.return_type.serialize_type() }}::decode(result)?),
                    ..Default::default()
                }
            };
            tx.send(stream_val).await.ok();
        };
        
        unsafe {
            language_client_cffi::call_function_stream_from_c(/* args with callback */);
        }
        
        Ok(rx)
    }
}
```

#### 3. Multi-file Client Generation
**File**: `generators/languages/rust/src/lib.rs`
**Changes**: Generate separate files for different function types

```rust
pub fn generate_sdk_files(ir: IntermediateRepr, package_name: &str) -> Result<Vec<crate::File>, anyhow::Error> {
    let files = vec![
        crate::File {
            path: "src/client.rs".to_string(),
            content: render_client_rs(&package)?,
        },
        crate::File {
            path: "src/client_stream.rs".to_string(), 
            content: render_client_stream_rs(&package)?,
        },
        crate::File {
            path: "src/client_parse.rs".to_string(),
            content: render_client_parse_rs(&package)?,
        },
    ];
    Ok(files)
}
```

### Success Criteria

#### Automated Verification
- [ ] All function variants compile: `cargo build`
- [ ] Parse functions work with text input: `cargo test parse_functions`
- [ ] Streaming functions return async channels: `cargo test streaming_functions`
- [ ] Integration with existing CFFI functions: `cargo test cffi_integration`

#### Manual Verification
- [ ] Parse functions correctly handle pre-parsed text input
- [ ] Streaming functions provide real-time partial results
- [ ] Error handling works across all function variants

---

## Phase 4: TypeBuilder System

### Overview
Implement dynamic type construction system using CFFI raw objects and method dispatch.

### Changes Required

#### 1. TypeBuilder Trait System
**File**: `generators/languages/rust/src/_templates/type_builder.rs.j2`
**Changes**: Core TypeBuilder traits and implementations

```rust
use baml_cffi::BamlRawObject;

pub trait TypeBuilderTrait {
    fn string_type(&self) -> Type;
    fn int_type(&self) -> Type;
    fn add_enum(&self, name: &str) -> Result<EnumBuilder, BamlError>;
    fn add_class(&self, name: &str) -> Result<ClassBuilder, BamlError>;
}

pub struct TypeBuilder {
    inner: BamlRawObject,
}

impl TypeBuilder {
    pub fn new(runtime: &BamlRuntime) -> Result<Self, BamlError> {
        let raw_object = unsafe {
            language_client_cffi::create_type_builder(runtime.inner)
        };
        
        Ok(TypeBuilder {
            inner: BamlRawObject::new(raw_object)?,
        })
    }
}

impl TypeBuilderTrait for TypeBuilder {
    fn add_enum(&self, name: &str) -> Result<EnumBuilder, BamlError> {
        let args = serde_json::json!({ "name": name });
        let result = self.inner.call_method("add_enum", &args)?;
        Ok(EnumBuilder::from_raw_object(result)?)
    }
    
    fn add_class(&self, name: &str) -> Result<ClassBuilder, BamlError> {
        let args = serde_json::json!({ "name": name });
        let result = self.inner.call_method("add_class", &args)?;
        Ok(ClassBuilder::from_raw_object(result)?)
    }
}
```

#### 2. Builder Interfaces
**File**: `generators/languages/rust/src/_templates/enum_builder.rs.j2`
**Changes**: EnumBuilder and ClassBuilder implementations

```rust
pub trait EnumBuilderTrait {
    fn add_value(&self, value: &str) -> Result<EnumValueBuilder, BamlError>;
    fn list_values(&self) -> Result<Vec<EnumValueBuilder>, BamlError>;
    fn get_type(&self) -> Result<Type, BamlError>;
}

pub struct EnumBuilder {
    inner: BamlRawObject,
}

impl EnumBuilderTrait for EnumBuilder {
    fn add_value(&self, value: &str) -> Result<EnumValueBuilder, BamlError> {
        let args = serde_json::json!({ "value": value });
        let result = self.inner.call_method("add_value", &args)?;
        Ok(EnumValueBuilder::from_raw_object(result)?)
    }
}

pub trait ClassBuilderTrait {
    fn add_property(&self, name: &str, type_def: Type) -> Result<ClassPropertyBuilder, BamlError>;
    fn list_properties(&self) -> Result<Vec<ClassPropertyBuilder>, BamlError>;
    fn get_type(&self) -> Result<Type, BamlError>;
}

pub struct ClassBuilder {
    inner: BamlRawObject,
}

impl ClassBuilderTrait for ClassBuilder {
    fn add_property(&self, name: &str, type_def: Type) -> Result<ClassPropertyBuilder, BamlError> {
        let args = serde_json::json!({ 
            "name": name,
            "type": type_def.to_cffi_type()
        });
        let result = self.inner.call_method("add_property", &args)?;
        Ok(ClassPropertyBuilder::from_raw_object(result)?)
    }
}
```

#### 3. Integration with Function Calls
**File**: `generators/languages/rust/src/_templates/client.rs.j2`
**Changes**: Add TypeBuilder support to function calls

```rust
pub struct BamlContext {
    pub type_builder: Option<TypeBuilder>,
    pub client_registry: Option<ClientRegistry>,
    pub env_vars: HashMap<String, String>,
}

impl {{ client_name }} {
    pub async fn {{ fn.name }}_with_context(
        &self,
        context: BamlContext,
        {{ fn.inputs | map(attribute="name") | map("snake_case") | join(", ") }}: {{ fn.inputs | map(attribute="type_rust") | join(", ") }}
    ) -> Result<{{ fn.return_type.serialize_type() }}, BamlError> {
        let args = BamlFunctionArguments {
            kwargs: {
                let mut map = HashMap::new();
                {% for input in fn.inputs -%}
                map.insert("{{ input.name }}".to_string(), {{ input.name | snake_case }}.to_baml_value()?);
                {% endfor %}
                map
            },
            type_builder: context.type_builder.map(|tb| tb.inner),
            client_registry: context.client_registry,
            env_vars: context.env_vars,
            collectors: None,
        };
        
        // Rest of function call implementation
    }
}
```

### Success Criteria

#### Automated Verification
- [ ] TypeBuilder system compiles: `cargo build`
- [ ] Dynamic enum/class creation works: `cargo test type_builder_creation`
- [ ] CFFI method dispatch functions correctly: `cargo test cffi_method_calls`
- [ ] Memory management works without leaks: `cargo test --release type_builder_memory`

#### Manual Verification
- [ ] TypeBuilder can create complex types at runtime
- [ ] Generated types integrate with BAML function calls
- [ ] Error handling provides useful messages for type construction failures

---

## Phase 5: Complete File Structure Parity

### Overview
Restructure generated files to match Go's modular approach with separate directories and comprehensive file coverage.

### Changes Required

#### 1. Directory Structure Reorganization
**File**: `generators/languages/rust/src/lib.rs`
**Changes**: Generate Go-equivalent file structure

```rust
pub fn generate_sdk_files(ir: IntermediateRepr, package_name: &str) -> Result<Vec<crate::File>, anyhow::Error> {
    let files = vec![
        // Main module files
        crate::File { path: "src/lib.rs".to_string(), content: render_lib_rs(&package)? },
        crate::File { path: "src/client.rs".to_string(), content: render_client_rs(&package)? },
        crate::File { path: "src/runtime.rs".to_string(), content: render_runtime_rs(&package)? },
        crate::File { path: "src/source_map.rs".to_string(), content: render_source_map_rs(&package)? },
        
        // Types directory
        crate::File { path: "src/types/mod.rs".to_string(), content: render_types_mod_rs(&package)? },
        crate::File { path: "src/types/classes.rs".to_string(), content: render_classes_rs(&package)? },
        crate::File { path: "src/types/enums.rs".to_string(), content: render_enums_rs(&package)? },
        crate::File { path: "src/types/unions.rs".to_string(), content: render_unions_rs(&package)? },
        
        // Streaming types directory  
        crate::File { path: "src/stream_types/mod.rs".to_string(), content: render_stream_types_mod_rs(&package)? },
        crate::File { path: "src/stream_types/classes.rs".to_string(), content: render_stream_classes_rs(&package)? },
        crate::File { path: "src/stream_types/unions.rs".to_string(), content: render_stream_unions_rs(&package)? },
        
        // TypeBuilder directory
        crate::File { path: "src/type_builder/mod.rs".to_string(), content: render_type_builder_mod_rs(&package)? },
        crate::File { path: "src/type_builder/type_builder.rs".to_string(), content: render_type_builder_rs(&package)? },
        crate::File { path: "src/type_builder/enum_builder.rs".to_string(), content: render_enum_builder_rs(&package)? },
        crate::File { path: "src/type_builder/class_builder.rs".to_string(), content: render_class_builder_rs(&package)? },
    ];
    
    Ok(files)
}
```

#### 2. Module System Integration
**File**: `generators/languages/rust/src/_templates/lib.rs.j2`  
**Changes**: Updated lib.rs with proper module exports

```rust
//! BAML client generated code
//! 
//! This crate provides type-safe Rust bindings for BAML functions and types.

pub mod client;
pub mod runtime;
pub mod source_map;

pub mod types {
    //! Standard BAML types for complete data structures
    pub mod classes;
    pub mod enums; 
    pub mod unions;
    
    pub use classes::*;
    pub use enums::*;
    pub use unions::*;
}

pub mod stream_types {
    //! Streaming variants of BAML types for partial parsing
    pub mod classes;
    pub mod unions;
    
    pub use classes::*;
    pub use unions::*;
}

pub mod type_builder {
    //! Dynamic type construction at runtime
    pub mod type_builder;
    pub mod enum_builder;
    pub mod class_builder;
    
    pub use type_builder::*;
    pub use enum_builder::*;
    pub use class_builder::*;
}

// Re-export main interfaces
pub use client::*;
pub use runtime::*;
```

#### 3. Advanced Features Integration
**File**: `generators/languages/rust/src/_templates/runtime.rs.j2`
**Changes**: Runtime management and configuration

```rust
use std::collections::HashMap;
use language_client_cffi::{BamlRuntime as CffiRuntime, BamlRuntimeBuilder};

pub struct BamlRuntime {
    pub(crate) inner: CffiRuntime,
}

impl BamlRuntime {
    pub fn new() -> Result<Self, BamlError> {
        let source_files = include_str!("../baml_src/inlined.baml");
        let runtime = BamlRuntimeBuilder::from_source_code(source_files)?.build()?;
        
        Ok(BamlRuntime { inner: runtime })
    }
    
    pub fn with_env_vars(env_vars: HashMap<String, String>) -> Result<Self, BamlError> {
        let source_files = include_str!("../baml_src/inlined.baml");
        let runtime = BamlRuntimeBuilder::from_source_code(source_files)?
            .with_env_vars(env_vars)
            .build()?;
            
        Ok(BamlRuntime { inner: runtime })
    }
    
    pub fn create_type_builder(&self) -> Result<crate::type_builder::TypeBuilder, BamlError> {
        crate::type_builder::TypeBuilder::new(self)
    }
}

impl Drop for BamlRuntime {
    fn drop(&mut self) {
        // CFFI runtime cleanup handled by language_client_cffi
    }
}
```

### Success Criteria

#### Automated Verification  
- [ ] All generated files compile correctly: `cargo build`
- [ ] Module system works without circular dependencies: `cargo check`
- [ ] Public API exports are consistent: `cargo test api_exports`
- [ ] Documentation builds successfully: `cargo doc`

#### Manual Verification
- [ ] Generated file structure matches Go generator layout
- [ ] Module organization is logical and easy to navigate  
- [ ] All advanced features work together correctly
- [ ] API feels idiomatic and consistent with Rust conventions

---

## Testing Strategy

### Unit Tests
- CFFI encode/decode roundtrip tests for all generated types
- Streaming type partial parsing validation
- TypeBuilder dynamic construction testing
- Function variant execution testing

### Integration Tests  
- Cross-language compatibility tests with Go client
- Real BAML function execution with all variants
- Performance benchmarks against Go client
- Memory safety and leak detection

### Manual Testing Steps
1. Generate Rust client from existing BAML schema with complex types
2. Verify CFFI protocol communication with runtime
3. Test streaming function calls with real LLM responses
4. Create dynamic types using TypeBuilder and use in function calls
5. Compare performance and functionality with Go client

## Performance Considerations

The implementation prioritizes correctness and API compatibility over performance optimizations. Key considerations:

- Protocol buffer serialization overhead for CFFI communication
- Memory usage of streaming types with Option<T> wrappers  
- Async runtime overhead for streaming function channels
- TypeBuilder CFFI method call latency

## Migration Notes

This implementation maintains backwards compatibility with existing Rust client usage:
- Existing `ToBamlValue/FromBamlValue` methods continue to work
- Current function call patterns remain supported
- Generated type names and structure unchanged for basic usage
- New features are additive and opt-in

## Important Guidelines

1. **Follow Go Patterns**: Use Go generator as reference implementation for all features
2. **Rust Idioms**: Adapt patterns to be idiomatic Rust (Option<T>, Result<T,E>, Drop trait)  
3. **Incremental Implementation**: Each phase builds on previous work and can be tested independently
4. **Comprehensive Testing**: All features must have both automated and manual verification
5. **Backwards Compatibility**: Existing Rust client users should not be affected