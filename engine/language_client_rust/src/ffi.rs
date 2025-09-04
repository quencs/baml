//! FFI bindings to the shared BAML runtime library
//!
//! This module re-exports the FFI functions and types from baml_cffi,
//! providing a clean interface for the Rust client.

// Re-export the FFI functions from baml_cffi
pub use baml_cffi::{
    register_callbacks, CallbackFn, OnTickCallbackFn,
    call_function_from_c, call_function_parse_from_c, call_function_stream_from_c,
    call_object_constructor, call_object_method, free_buffer, Buffer,
    create_baml_runtime, destroy_baml_runtime, invoke_runtime_cli, version,
};

// Re-export the protobuf types
pub use baml_cffi::baml;
