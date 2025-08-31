//! Simple smoke test for BAML Rust client
//!
//! This binary provides a basic test to verify that the BAML client
//! can be initialized and basic operations work.

use baml_integ_tests_rust::{init_test_logging, test_config};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_test_logging();

    println!("🦀 BAML Rust Client Smoke Test");
    println!("==============================");

    // Test 1: Basic client initialization
    println!("\n1. Testing client initialization...");
    match test_config::setup_test_client() {
        Ok(_client) => println!("   ✅ Client initialized successfully"),
        Err(e) => {
            println!("   ❌ Client initialization failed: {}", e);
            return Err(e.into());
        }
    }

    // Test 2: BAML library version check
    println!("\n2. Checking BAML library version...");
    match baml_client_rust::ffi::get_library_version() {
        Ok(version) => println!("   ✅ BAML library version: {}", version),
        Err(e) => {
            println!("   ❌ Failed to get library version: {}", e);
            return Err(e.into());
        }
    }

    // Test 3: Environment variable check
    println!("\n3. Checking environment configuration...");
    let api_key = test_config::get_openai_api_key();
    if api_key == "test-key" {
        println!("   ⚠️  Using default test API key (set OPENAI_API_KEY for real tests)");
    } else {
        println!("   ✅ Found OPENAI_API_KEY in environment");
    }

    println!("\n🎉 All smoke tests passed!");
    println!("\nNext steps:");
    println!("  • Generate BAML client code: cd .. && baml-cli generate");
    println!("  • Run integration tests: cargo test");

    Ok(())
}
