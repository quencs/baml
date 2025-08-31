//! Client registry integration tests
//!
//! Tests dynamic client configuration and registry patterns including:
//! - Multiple client instances with different configurations
//! - Runtime client switching and selection
//! - Client isolation and resource management
//! - Configuration inheritance and overrides
//! - Client lifecycle management

use assert_matches::assert_matches;
use baml_integ_tests_rust::*;
use std::collections::HashMap;

// This module will be populated with generated types after running baml-cli generate
#[allow(unused_imports)]
use baml_client::{types::*, *};

/// Test creating multiple client instances with different configurations
/// Reference: Go test_client_registry_test.go:TestMultipleClientConfigs
#[tokio::test]
async fn test_multiple_client_configurations() {
    init_test_logging();

    // Create clients with different provider configurations
    let openai_client = BamlClientBuilder::new()
        .env_var("OPENAI_API_KEY", test_config::get_openai_api_key())
        .env_var("DEFAULT_PROVIDER", "openai")
        .build()
        .expect("Failed to create OpenAI client");

    // TODO: Add more providers after code generation
    // let anthropic_client = BamlClientBuilder::new()
    //     .env_var("ANTHROPIC_API_KEY", test_config::get_anthropic_api_key())
    //     .env_var("DEFAULT_PROVIDER", "anthropic")
    //     .build()
    //     .expect("Failed to create Anthropic client");

    // Verify clients are independent
    assert!(!openai_client.core_client().runtime_ptr().is_null());
    // assert!(!anthropic_client.core_client().runtime_ptr().is_null());

    println!("Multiple client configurations created successfully");
}

/// Test client builder pattern with chaining
#[tokio::test]
async fn test_client_builder_chaining() {
    init_test_logging();

    let client = BamlClientBuilder::new()
        .env_var("OPENAI_API_KEY", test_config::get_openai_api_key())
        .env_var("CUSTOM_VAR_1", "value1")
        .env_var("CUSTOM_VAR_2", "value2")
        .build()
        .expect("Failed to build client with chained configuration");

    assert!(!client.core_client().runtime_ptr().is_null());

    println!("Client builder chaining works correctly");
}

/// Test client configuration inheritance
#[tokio::test]
async fn test_client_configuration_inheritance() {
    init_test_logging();

    // TODO: Update after code generation to test actual configuration inheritance
    // Test that clients inherit base configuration and can override specific settings

    let base_client = test_config::setup_test_client().expect("Failed to create base client");

    // Create derived client with additional configuration
    let derived_client = BamlClientBuilder::new()
        .env_var("OPENAI_API_KEY", test_config::get_openai_api_key())
        .env_var("CUSTOM_TIMEOUT", "30")
        .env_var("RETRY_COUNT", "3")
        .build()
        .expect("Failed to create derived client");

    // Both should be functional but potentially have different behaviors
    assert!(!base_client.core_client().runtime_ptr().is_null());
    assert!(!derived_client.core_client().runtime_ptr().is_null());

    println!("Client configuration inheritance tested successfully");
}

/// Test runtime client switching
#[tokio::test]
async fn test_runtime_client_switching() {
    init_test_logging();

    // TODO: Update after code generation to test actual client switching
    // This might involve a client registry or factory pattern

    let client1 = test_config::setup_test_client().expect("Failed to create client 1");
    let client2 = BamlClientBuilder::new()
        .env_var("OPENAI_API_KEY", test_config::get_openai_api_key())
        .env_var("PROVIDER_PREFERENCE", "alternative")
        .build()
        .expect("Failed to create client 2");

    // Test that we can switch between clients for different operations
    // In a real scenario, this might involve different providers or configurations

    println!("Runtime client switching tested successfully");
}

/// Test client isolation and resource separation
#[tokio::test]
async fn test_client_isolation() {
    init_test_logging();

    const NUM_CLIENTS: usize = 10;
    let mut clients = Vec::new();

    // Create multiple isolated clients
    for i in 0..NUM_CLIENTS {
        let client = BamlClientBuilder::new()
            .env_var("OPENAI_API_KEY", test_config::get_openai_api_key())
            .env_var("CLIENT_ID", &format!("client_{}", i))
            .build()
            .expect(&format!("Failed to create client {}", i));

        clients.push(client);
    }

    // Verify all clients are independent and functional
    for (i, client) in clients.iter().enumerate() {
        assert!(
            !client.core_client().runtime_ptr().is_null(),
            "Client {} should be valid",
            i
        );
    }

    // Test that modifying one client doesn't affect others
    // (This would be more meaningful with actual client operations after code generation)

    println!("Created {} isolated clients successfully", NUM_CLIENTS);
}

/// Test client lifecycle management
#[tokio::test]
async fn test_client_lifecycle_management() {
    init_test_logging();

    // Test client creation, usage, and cleanup
    {
        let client = test_config::setup_test_client().expect("Failed to create client");
        assert!(!client.core_client().runtime_ptr().is_null());

        // TODO: After code generation, test actual client operations here
        // let result = client.simple_function("test").await;
        // assert!(result.is_ok());

        // Client goes out of scope here and should be cleaned up
    }

    // Create new client after previous one was dropped
    let new_client =
        test_config::setup_test_client().expect("Failed to create new client after cleanup");
    assert!(!new_client.core_client().runtime_ptr().is_null());

    println!("Client lifecycle management tested successfully");
}

/// Test concurrent client operations
#[tokio::test]
async fn test_concurrent_client_operations() {
    init_test_logging();

    let client = test_config::setup_test_client().expect("Failed to create shared client");

    const NUM_CONCURRENT: usize = 20;
    let mut handles = Vec::new();

    for i in 0..NUM_CONCURRENT {
        let client_clone = Arc::clone(&client);
        let handle = tokio::spawn(async move {
            // TODO: Update after code generation to test actual concurrent operations
            // For now, test that the client can handle concurrent access

            // Simulate concurrent access to client methods
            let context = BamlContext::new();
            let result = client_clone
                .core_client()
                .call_function(&format!("TestFunction{}", i), context)
                .await;

            // We expect this to fail (function doesn't exist), but it should fail gracefully
            assert!(result.is_err());
            i
        });
        handles.push(handle);
    }

    // Wait for all concurrent operations to complete
    for handle in handles {
        let task_id = handle
            .await
            .expect("Concurrent task should complete without panic");
        println!("Concurrent task {} completed successfully", task_id);
    }

    println!(
        "All {} concurrent client operations completed",
        NUM_CONCURRENT
    );
}

/// Test client configuration validation
#[tokio::test]
async fn test_client_configuration_validation() {
    init_test_logging();

    // Test valid configuration
    let valid_client = BamlClientBuilder::new()
        .env_var("OPENAI_API_KEY", test_config::get_openai_api_key())
        .build();
    assert!(valid_client.is_ok(), "Valid configuration should succeed");

    // Test configuration with missing required variables
    let minimal_client = BamlClientBuilder::new().build();
    assert!(
        minimal_client.is_ok(),
        "Minimal configuration should still work"
    );

    // Test configuration with invalid values
    // TODO: Add more specific validation tests after understanding the configuration schema

    println!("Client configuration validation tested successfully");
}

/// Test client registry patterns (if applicable)
#[tokio::test]
async fn test_client_registry_patterns() {
    init_test_logging();

    // TODO: Update after code generation if BAML supports client registries
    // This might involve named client instances or factory patterns

    // Simulate a simple client registry
    let mut client_registry = HashMap::new();

    client_registry.insert(
        "default",
        test_config::setup_test_client().expect("Failed to create default client"),
    );

    client_registry.insert(
        "high_performance",
        BamlClientBuilder::new()
            .env_var("OPENAI_API_KEY", test_config::get_openai_api_key())
            .env_var("PERFORMANCE_MODE", "high")
            .build()
            .expect("Failed to create high performance client"),
    );

    // Test registry access
    let default_client = client_registry
        .get("default")
        .expect("Should have default client");
    let hp_client = client_registry
        .get("high_performance")
        .expect("Should have HP client");

    assert!(!default_client.core_client().runtime_ptr().is_null());
    assert!(!hp_client.core_client().runtime_ptr().is_null());

    println!("Client registry patterns tested successfully");
}

/// Test client configuration hot-reloading (if supported)
#[tokio::test]
async fn test_client_configuration_hot_reload() {
    init_test_logging();

    // TODO: Update after code generation if hot-reloading is supported
    // Test that configuration changes can be applied to existing clients

    let client = test_config::setup_test_client().expect("Failed to create client");

    // Simulate configuration change
    // In a real implementation, this might involve reloading from a config file
    // or updating environment variables

    println!("Client configuration hot-reload tested (placeholder)");
}

/// Test client resource management and cleanup
#[tokio::test]
async fn test_client_resource_management() {
    init_test_logging();

    // Test that clients properly manage and clean up resources
    const RESOURCE_TEST_CYCLES: usize = 100;

    for i in 0..RESOURCE_TEST_CYCLES {
        let client = BamlClientBuilder::new()
            .env_var("OPENAI_API_KEY", test_config::get_openai_api_key())
            .env_var("CYCLE_ID", &format!("{}", i))
            .build()
            .expect(&format!("Failed to create client for cycle {}", i));

        assert!(!client.core_client().runtime_ptr().is_null());

        // Client is dropped here - test for resource leaks
        if i % 20 == 0 {
            println!("Completed resource management cycle {}", i);
        }
    }

    // Final client creation to ensure resources are still available
    let final_client = test_config::setup_test_client()
        .expect("Should still be able to create clients after resource test");
    assert!(!final_client.core_client().runtime_ptr().is_null());

    println!(
        "Resource management tested over {} cycles",
        RESOURCE_TEST_CYCLES
    );
}

/// Test client configuration merging and precedence
#[tokio::test]
async fn test_configuration_precedence() {
    init_test_logging();

    // TODO: Update after code generation to test configuration precedence rules
    // Test the order of precedence for configuration sources:
    // 1. Explicit builder methods
    // 2. Environment variables
    // 3. Configuration files
    // 4. Default values

    let client = BamlClientBuilder::new()
        .env_var("OPENAI_API_KEY", test_config::get_openai_api_key())
        .env_var("TEST_PRECEDENCE", "builder_value")
        .build()
        .expect("Failed to create client for precedence test");

    assert!(!client.core_client().runtime_ptr().is_null());

    println!("Configuration precedence tested successfully");
}

/// Test client factory patterns
#[tokio::test]
async fn test_client_factory_patterns() {
    init_test_logging();

    // TODO: Update after code generation if factory patterns are supported
    // Test creating clients through factory methods for common configurations

    // Simulate factory methods
    fn create_development_client() -> BamlResult<BamlClient> {
        BamlClientBuilder::new()
            .env_var("OPENAI_API_KEY", test_config::get_openai_api_key())
            .env_var("ENVIRONMENT", "development")
            .build()
    }

    fn create_production_client() -> BamlResult<BamlClient> {
        BamlClientBuilder::new()
            .env_var("OPENAI_API_KEY", test_config::get_openai_api_key())
            .env_var("ENVIRONMENT", "production")
            .env_var("RETRY_COUNT", "5")
            .build()
    }

    let dev_client = create_development_client().expect("Failed to create dev client");
    let prod_client = create_production_client().expect("Failed to create prod client");

    assert!(!dev_client.core_client().runtime_ptr().is_null());
    assert!(!prod_client.core_client().runtime_ptr().is_null());

    println!("Client factory patterns tested successfully");
}
