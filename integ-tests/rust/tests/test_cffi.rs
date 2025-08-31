//! CFFI (C Foreign Function Interface) integration tests
//!
//! Tests the shared dylib architecture that enables Rust to use the same
//! BAML runtime as Go, Python, TypeScript, and other languages.

use baml_integ_tests_rust::*;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

/// Test basic FFI library loading and version check
#[tokio::test]
async fn test_ffi_library_loading() {
    init_test_logging();

    // Test library initialization
    let version_result = baml_client_rust::ffi::get_library_version();
    assert!(
        version_result.is_ok(),
        "Failed to load BAML FFI library: {:?}",
        version_result.err()
    );

    let version = version_result.unwrap();
    assert!(!version.is_empty(), "Library version should not be empty");

    println!("BAML library version: {}", version);
}

/// Test FFI runtime creation and destruction
#[tokio::test]
async fn test_ffi_runtime_lifecycle() {
    init_test_logging();

    // Test runtime creation
    let env_vars = std::env::vars().collect::<std::collections::HashMap<String, String>>();
    let env_vars_json = serde_json::to_string(&env_vars).unwrap();
    let src_files_json =
        serde_json::to_string(&std::collections::HashMap::<String, String>::new()).unwrap();

    let runtime_result =
        baml_client_rust::ffi::create_baml_runtime(".", &src_files_json, &env_vars_json);

    assert!(
        runtime_result.is_ok(),
        "Failed to create BAML runtime via FFI: {:?}",
        runtime_result.err()
    );

    let runtime_ptr = runtime_result.unwrap();
    assert!(!runtime_ptr.is_null(), "Runtime pointer should not be null");

    // Test runtime cleanup
    let destroy_result = baml_client_rust::ffi::destroy_baml_runtime(runtime_ptr);
    assert!(
        destroy_result.is_ok(),
        "Failed to destroy BAML runtime: {:?}",
        destroy_result.err()
    );
}

/// Test concurrent FFI operations (thread safety)
#[tokio::test]
async fn test_ffi_thread_safety() {
    init_test_logging();

    const NUM_THREADS: usize = 10;
    const CALLS_PER_THREAD: usize = 5;

    let handles: Vec<_> = (0..NUM_THREADS)
        .map(|thread_id| {
            thread::spawn(move || {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    for call_id in 0..CALLS_PER_THREAD {
                        // Test library version call (read-only operation)
                        let version = baml_client_rust::ffi::get_library_version();
                        assert!(
                            version.is_ok(),
                            "Thread {} call {} failed: {:?}",
                            thread_id,
                            call_id,
                            version.err()
                        );

                        // Small delay to encourage race conditions if they exist
                        tokio::time::sleep(Duration::from_millis(1)).await;
                    }
                    thread_id
                })
            })
        })
        .collect();

    // Wait for all threads to complete
    for handle in handles {
        let thread_id = handle.join().expect("Thread panicked");
        println!("Thread {} completed successfully", thread_id);
    }
}

/// Test FFI error handling
#[tokio::test]
async fn test_ffi_error_handling() {
    init_test_logging();

    // Test invalid runtime creation (invalid JSON)
    let invalid_json = "{ invalid json }";
    let valid_json =
        serde_json::to_string(&std::collections::HashMap::<String, String>::new()).unwrap();

    let result = baml_client_rust::ffi::create_baml_runtime(".", invalid_json, &valid_json);

    // This should fail gracefully, not crash
    assert!(result.is_err(), "Expected error for invalid JSON input");

    // Test null pointer handling
    let destroy_result = baml_client_rust::ffi::destroy_baml_runtime(std::ptr::null());
    assert!(
        destroy_result.is_ok(),
        "Destroying null pointer should be safe"
    );
}

/// Test FFI library search paths
#[tokio::test]
async fn test_ffi_library_paths() {
    init_test_logging();

    // This test validates that the library can be found in various locations
    // The actual loading is tested by successful version retrieval
    let version = baml_client_rust::ffi::get_library_version();
    assert!(
        version.is_ok(),
        "Library should be findable in standard search paths"
    );

    // Test that multiple initializations work (should be idempotent)
    let version2 = baml_client_rust::ffi::get_library_version();
    assert!(
        version2.is_ok(),
        "Multiple library initializations should work"
    );

    assert_eq!(
        version.unwrap(),
        version2.unwrap(),
        "Version should be consistent"
    );
}

/// Test FFI callback mechanism (placeholder for future async callback tests)
#[tokio::test]
async fn test_ffi_callback_mechanism() {
    init_test_logging();

    // This test will be expanded once we have generated code to test actual function calls
    // For now, we test that we can create a client that uses the FFI interface

    let client = test_config::setup_test_client().expect("Failed to create FFI-based client");

    // The client should be using FFI internally
    assert!(
        !client.core_client().runtime_ptr().is_null(),
        "Client should have valid runtime pointer"
    );

    println!("FFI-based client created successfully");
}

/// Test FFI memory management
#[tokio::test]
async fn test_ffi_memory_management() {
    init_test_logging();

    // Test multiple client creations and drops to check for memory leaks
    const NUM_CLIENTS: usize = 50;

    for i in 0..NUM_CLIENTS {
        let client =
            test_config::setup_test_client().expect(&format!("Failed to create client {}", i));

        // Verify client is valid
        assert!(!client.core_client().runtime_ptr().is_null());

        // Client will be dropped here - test that this doesn't cause issues
    }

    println!("Created and dropped {} clients successfully", NUM_CLIENTS);

    // Test that we can still create clients after multiple drops
    let final_client = test_config::setup_test_client();
    assert!(
        final_client.is_ok(),
        "Should be able to create client after multiple drops"
    );
}

/// Test FFI function call interface (will be expanded after code generation)
#[tokio::test]
async fn test_ffi_function_call_interface() {
    init_test_logging();

    let client = test_config::setup_test_client().expect("Failed to create client");

    // Test invalid function call to verify error handling works through FFI
    let empty_context = BamlContext::new();
    let result = client
        .core_client()
        .call_function("NonExistentFunction", empty_context)
        .await;

    // Should get an error, not a crash
    assert!(
        result.is_err(),
        "Invalid function call should return error, not crash"
    );

    let error = result.unwrap_err();
    println!("Expected error for invalid function: {}", error);

    // Error should be structured (not just a generic FFI error)
    let error_str = error.to_string();
    assert!(
        error_str.contains("function")
            || error_str.contains("not found")
            || error_str.contains("NonExistentFunction")
            || error_str.contains("call"),
        "Error should be descriptive: {}",
        error_str
    );
}

/// Benchmark FFI call overhead
#[tokio::test]
#[ignore] // Mark as ignored for normal test runs, run with --ignored for performance testing
async fn benchmark_ffi_overhead() {
    init_test_logging();

    const NUM_CALLS: usize = 1000;

    let start = std::time::Instant::now();

    for _ in 0..NUM_CALLS {
        let _version =
            baml_client_rust::ffi::get_library_version().expect("Version call should succeed");
    }

    let duration = start.elapsed();
    let avg_call_time = duration / NUM_CALLS as u32;

    println!("FFI call benchmark:");
    println!("  Total time: {:?}", duration);
    println!("  Calls: {}", NUM_CALLS);
    println!("  Average per call: {:?}", avg_call_time);

    // Sanity check - FFI calls should be reasonably fast
    assert!(
        avg_call_time < Duration::from_millis(10),
        "FFI calls should be fast, got {:?} per call",
        avg_call_time
    );
}
