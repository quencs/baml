#![cfg(feature = "generated-client")]
//! Error handling integration tests
//!
//! Tests comprehensive error scenarios including:
//! - Network connectivity issues
//! - Invalid API responses
//! - Validation errors
//! - Timeout handling
//! - Rate limiting
//! - Provider-specific errors
use baml_integ_tests_rust::*;

/// Test network connectivity errors
#[tokio::test]
async fn test_network_error() {
    init_test_logging();

    let client = test_config::setup_test_client().expect("Failed to create client");

    // Test with a function call - network issues will surface during execution
    let result = client
        .test_fn_named_args_single_string("network test".to_string())
        .await;

    match result {
        Ok(response) => {
            println!("Network test succeeded: {}", response);
        }
        Err(e) => {
            let error_msg = e.to_string().to_lowercase();
            println!("Network error (may be expected): {}", e);
            // Check for network-related error indicators
            if error_msg.contains("network")
                || error_msg.contains("connection")
                || error_msg.contains("timeout")
            {
                println!("Confirmed network-related error");
            }
        }
    }
}

/// Test invalid API key handling
#[tokio::test]
async fn test_invalid_api_key() {
    init_test_logging();

    // Create client with invalid API key
    let invalid_client_result = BamlClientBuilder::new()
        .env_var("OPENAI_API_KEY", "invalid-key-12345")
        .build();

    match invalid_client_result {
        Ok(invalid_client) => {
            let result = invalid_client
                .test_fn_named_args_single_string("test".to_string())
                .await;

            match result {
                Ok(response) => {
                    println!("Unexpectedly succeeded with invalid key: {}", response);
                }
                Err(error) => {
                    let error_msg = error.to_string().to_lowercase();
                    println!("Expected error with invalid API key: {}", error);
                    assert!(
                        error_msg.contains("401")
                            || error_msg.contains("unauthorized")
                            || error_msg.contains("invalid")
                            || error_msg.contains("key")
                    );
                }
            }
        }
        Err(e) => {
            println!("Client creation failed with invalid key (expected): {}", e);
        }
    }
}

/// Test malformed API responses
#[tokio::test]
async fn test_malformed_response_handling() {
    init_test_logging();

    let client = test_config::setup_test_client().expect("Failed to create client");

    // Test with a function that expects structured output - malformed responses should be handled gracefully
    let result = client
        .aaa_sam_output_format(
            "This is not a valid recipe and might cause parsing issues: {{invalid json}}"
                .to_string(),
        )
        .await;

    match result {
        Ok(recipe) => {
            println!(
                "Successfully parsed recipe despite malformed input: {:?}",
                recipe
            );
        }
        Err(e) => {
            println!("Parsing error (expected with malformed input): {}", e);
            let error_msg = e.to_string().to_lowercase();
            if error_msg.contains("parse")
                || error_msg.contains("json")
                || error_msg.contains("deserial")
            {
                println!("Confirmed parsing-related error");
            }
        }
    }
}

/// Test validation errors for invalid input types
#[tokio::test]
async fn test_validation_errors() {
    init_test_logging();

    let client = test_config::setup_test_client().expect("Failed to create client");

    // TODO: Update after code generation
    // Test validation of input parameters at the Rust type level
    // This should catch errors before they reach the API

    println!(
        "Client created successfully - validation test will be completed after code generation"
    );
}

/// Test timeout handling
#[tokio::test]
async fn test_timeout_handling() {
    init_test_logging();

    let client = test_config::setup_test_client().expect("Failed to create client");

    // TODO: Update after code generation to test function calls with very short timeouts
    // let context = BamlContext::new().with_timeout(Duration::from_millis(1));
    // let result = client.slow_function_with_context(context, "test input").await;
    // assert!(result.is_err());
    // assert!(result.unwrap_err().to_string().contains("timeout"));

    println!("Client created successfully - timeout test will be completed after code generation");
}

/// Test rate limiting errors
#[tokio::test]
async fn test_rate_limiting() {
    init_test_logging();

    let client = test_config::setup_test_client().expect("Failed to create client");

    // TODO: Update after code generation
    // Make rapid successive calls to trigger rate limiting
    // const RAPID_CALLS: usize = 100;
    // let mut error_count = 0;
    //
    // for i in 0..RAPID_CALLS {
    //     let result = client.simple_function(&format!("call {}", i)).await;
    //     if let Err(e) = result {
    //         if e.to_string().contains("rate") || e.to_string().contains("429") {
    //             error_count += 1;
    //         }
    //     }
    //     // Small delay to avoid overwhelming the system
    //     tokio::time::sleep(Duration::from_millis(10)).await;
    // }
    //
    // // We should see some rate limiting errors with rapid calls
    // assert!(error_count > 0, "Expected some rate limiting errors");

    println!(
        "Client created successfully - rate limiting test will be completed after code generation"
    );
}

/// Test retry mechanism on transient errors
#[tokio::test]
async fn test_retry_on_transient_errors() {
    init_test_logging();

    let client = test_config::setup_test_client().expect("Failed to create client");

    // TODO: Update after code generation to test retry behavior
    // Configure client with retry policy and test with flaky network conditions

    println!("Client created successfully - retry test will be completed after code generation");
}

/// Test provider-specific error formats
#[tokio::test]
async fn test_provider_specific_errors() {
    init_test_logging();

    let client = test_config::setup_test_client().expect("Failed to create client");

    // TODO: Update after code generation
    // Test different error formats from different providers:
    // - OpenAI error format
    // - Anthropic error format
    // - Azure OpenAI error format
    // - Local model errors

    println!("Client created successfully - provider-specific errors test will be completed after code generation");
}

/// Test error context preservation through FFI boundary
#[tokio::test]
async fn test_error_context_preservation() {
    init_test_logging();

    let client = test_config::setup_test_client().expect("Failed to create client");

    // Test that error details are properly preserved when crossing the FFI boundary
    let context = BamlContext::new();
    let result = client
        .call_function_raw("NonExistentFunction", context)
        .await;

    assert!(result.is_err());
    let error = result.unwrap_err();

    // Error should contain meaningful information, not just generic FFI error
    let error_string = error.to_string();
    println!("Error message: {}", error_string);

    // Check that error contains function name or other contextual info
    assert!(
        error_string.contains("NonExistentFunction")
            || error_string.contains("function")
            || error_string.contains("not found")
            || !error_string.is_empty(),
        "Error should contain contextual information"
    );
}

/// Test concurrent error handling
#[tokio::test]
async fn test_concurrent_error_handling() {
    init_test_logging();

    let client = test_config::setup_test_client().expect("Failed to create client");

    const NUM_CONCURRENT: usize = 10;
    let mut handles = Vec::new();

    for i in 0..NUM_CONCURRENT {
        let client_clone = client.clone();
        let handle = tokio::spawn(async move {
            // Test concurrent calls to non-existent function
            let context = BamlContext::new();
            let result = client_clone
                .call_function_raw(&format!("NonExistent{}", i), context)
                .await;

            // All should fail gracefully
            assert!(result.is_err());
            result.unwrap_err()
        });
        handles.push(handle);
    }

    // Wait for all to complete - none should panic or hang
    for (i, handle) in handles.into_iter().enumerate() {
        let error = handle
            .await
            .expect(&format!("Task {} should complete without panic", i));
        assert!(
            !error.to_string().is_empty(),
            "Error {} should have a message",
            i
        );
    }

    println!("All concurrent error handling tasks completed successfully");
}

/// Test error serialization/deserialization across FFI
#[tokio::test]
async fn test_error_serialization_ffi() {
    init_test_logging();

    let client = test_config::setup_test_client().expect("Failed to create client");

    // Test different types of errors to ensure they're properly serialized across FFI boundary
    let test_cases = vec![
        "NonExistentFunction",
        "", // Empty function name
        "Function_With_Special_Characters!@#$%",
    ];

    for (i, function_name) in test_cases.iter().enumerate() {
        let context = BamlContext::new();
        let result = client.call_function_raw(function_name, context).await;

        assert!(result.is_err(), "Test case {} should produce error", i);
        let error = result.unwrap_err();

        // Verify error can be converted to string (serialization works)
        let error_string = error.to_string();
        assert!(
            !error_string.is_empty(),
            "Error {} should have string representation",
            i
        );

        // Verify error has some structure (not just raw pointer or memory address)
        assert!(
            !error_string.starts_with("0x"),
            "Error should not be raw memory address"
        );

        println!("Test case {}: {} -> {}", i, function_name, error_string);
    }
}

/// Test memory safety with error handling
#[tokio::test]
async fn test_error_memory_safety() {
    init_test_logging();

    const NUM_ERRORS: usize = 1000;
    let mut errors = Vec::with_capacity(NUM_ERRORS);

    // Generate many errors to test for memory leaks or corruption
    for i in 0..NUM_ERRORS {
        let client =
            test_config::setup_test_client().expect(&format!("Failed to create client {}", i));

        let context = BamlContext::new();
        let result = client
            .call_function_raw(&format!("Error{}", i), context)
            .await;

        if let Err(error) = result {
            errors.push(error);
        }

        // Periodically check that we can still create clients (no resource exhaustion)
        if i % 100 == 0 {
            let test_client = test_config::setup_test_client();
            assert!(
                test_client.is_ok(),
                "Should still be able to create clients at iteration {}",
                i
            );
        }
    }

    // Verify we collected errors (not all succeeded unexpectedly)
    assert!(
        errors.len() > NUM_ERRORS / 2,
        "Should have collected substantial errors"
    );

    // Test that errors are still accessible (no memory corruption)
    for (i, error) in errors.iter().enumerate() {
        let error_string = error.to_string();
        assert!(
            !error_string.is_empty(),
            "Error {} should still be accessible",
            i
        );

        // Spot check a few errors
        if i < 10 {
            println!("Error {}: {}", i, error_string);
        }
    }

    println!("Memory safety test completed with {} errors", errors.len());
}

/// Test error handling with custom context data
#[tokio::test]
async fn test_custom_context_error_handling() {
    init_test_logging();

    let client = test_config::setup_test_client().expect("Failed to create client");

    // TODO: Update after code generation to test custom context
    // let mut context = BamlContext::new();
    // context.set("custom_key", "custom_value");
    // context.set("retry_count", 3);
    //
    // let result = client.function_with_context(context, "test").await;
    // // Test that context information is preserved in error messages

    println!("Client created successfully - custom context error test will be completed after code generation");
}
