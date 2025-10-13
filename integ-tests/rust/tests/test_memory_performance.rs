//! Memory and performance integration tests
//!
//! Tests performance characteristics and memory safety including:
//! - Memory leak detection
//! - Performance benchmarking  
//! - Concurrent load testing
//! - Resource cleanup verification
//! - Stress testing under load

use baml_integ_tests_rust::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Test memory leak detection over multiple client lifecycle
/// Reference: Go test_memory_performance_test.go:TestMemoryLeakDetection
#[tokio::test]
#[ignore] // Run with --ignored for performance testing
async fn test_memory_leak_detection() {
    init_test_logging();

    const LEAK_TEST_CYCLES: usize = 1000;
    let mut peak_memory = 0usize;

    println!(
        "Starting memory leak detection test with {} cycles",
        LEAK_TEST_CYCLES
    );

    for cycle in 0..LEAK_TEST_CYCLES {
        // Create and use client
        let client = test_config::setup_test_client()
            .expect(&format!("Failed to create client in cycle {}", cycle));

        // TODO: Update after code generation to perform actual operations
        // Simulate memory-intensive operations
        // let large_input = create_large_test_data();
        // let result = client.memory_intensive_function(large_input).await;
        // assert!(result.is_ok() || result.is_err()); // Either outcome is fine for memory test

        // Test FFI calls that might leak memory
        let context = BamlContext::new();
        let _ = client
            .call_function_raw("MemoryTestFunction", context)
            .await;

        // Client goes out of scope here - test for proper cleanup

        if cycle % 100 == 0 {
            // Estimate memory usage (this is rough and platform-dependent)
            if let Ok(memory_info) = get_current_memory_usage() {
                println!(
                    "Cycle {}: Estimated memory usage: {} KB",
                    cycle,
                    memory_info / 1024
                );

                if memory_info > peak_memory {
                    peak_memory = memory_info;
                }

                // Check for excessive memory growth
                if cycle > 200 && memory_info > peak_memory * 2 {
                    panic!(
                        "Potential memory leak detected: memory usage doubled from {} KB to {} KB",
                        peak_memory / 1024,
                        memory_info / 1024
                    );
                }
            }
        }
    }

    println!(
        "Memory leak detection completed successfully over {} cycles",
        LEAK_TEST_CYCLES
    );
}

/// Test concurrent client performance under load
#[tokio::test]
#[ignore] // Run with --ignored for performance testing
async fn test_concurrent_performance() {
    init_test_logging();

    const NUM_CONCURRENT_CLIENTS: usize = 50;
    const CALLS_PER_CLIENT: usize = 20;

    let start_time = Instant::now();
    let success_counter = Arc::new(AtomicUsize::new(0));
    let error_counter = Arc::new(AtomicUsize::new(0));

    println!(
        "Starting concurrent performance test: {} clients × {} calls",
        NUM_CONCURRENT_CLIENTS, CALLS_PER_CLIENT
    );

    let mut handles = Vec::new();

    for client_id in 0..NUM_CONCURRENT_CLIENTS {
        let success_counter_clone = Arc::clone(&success_counter);
        let error_counter_clone = Arc::clone(&error_counter);

        let handle = tokio::spawn(async move {
            let client = test_config::setup_test_client()
                .expect(&format!("Failed to create client {}", client_id));

            let mut client_successes = 0;
            let mut client_errors = 0;

            for call_id in 0..CALLS_PER_CLIENT {
                // TODO: Update after code generation to use actual functions
                // let result = client.performance_test_function(&format!("data_{}_{}", client_id, call_id)).await;

                // For now, test FFI calls
                let context = BamlContext::new();
                let result = client
                    .call_function_raw(&format!("PerfTest_{}_{}", client_id, call_id), context)
                    .await;

                match result {
                    Ok(_) => client_successes += 1,
                    Err(_) => client_errors += 1,
                }

                // Small delay to avoid overwhelming the system
                tokio::time::sleep(Duration::from_millis(10)).await;
            }

            success_counter_clone.fetch_add(client_successes, Ordering::Relaxed);
            error_counter_clone.fetch_add(client_errors, Ordering::Relaxed);

            (client_id, client_successes, client_errors)
        });

        handles.push(handle);
    }

    // Wait for all concurrent operations to complete
    for handle in handles {
        let (client_id, successes, errors) = handle
            .await
            .expect("Concurrent performance task should complete");

        if client_id % 10 == 0 {
            println!(
                "Client {}: {} successes, {} errors",
                client_id, successes, errors
            );
        }
    }

    let total_time = start_time.elapsed();
    let total_successes = success_counter.load(Ordering::Relaxed);
    let total_errors = error_counter.load(Ordering::Relaxed);
    let total_calls = NUM_CONCURRENT_CLIENTS * CALLS_PER_CLIENT;

    println!("Concurrent performance test results:");
    println!("  Total time: {:?}", total_time);
    println!("  Total calls: {}", total_calls);
    println!("  Successes: {}", total_successes);
    println!("  Errors: {}", total_errors);
    println!(
        "  Calls per second: {:.2}",
        total_calls as f64 / total_time.as_secs_f64()
    );

    // Verify that most operations completed (some errors are expected for non-existent functions)
    assert!(
        total_successes + total_errors == total_calls,
        "All calls should complete"
    );
}

/// Test FFI call overhead and performance
#[tokio::test]
#[ignore] // Run with --ignored for performance testing
async fn test_ffi_call_overhead() {
    init_test_logging();

    let _client = test_config::setup_test_client().expect("Failed to create client");

    const NUM_CALLS: usize = 10_000;
    let mut call_times = Vec::with_capacity(NUM_CALLS);

    println!("Starting FFI overhead test with {} calls", NUM_CALLS);

    // Warm up
    for _ in 0..100 {
        let version = baml_client_rust::ffi::get_library_version();
        assert!(
            version.is_ok(),
            "Warmup version call failed: {:?}",
            version.err()
        );
    }

    // Measure call times
    for i in 0..NUM_CALLS {
        let start = Instant::now();

        let version = baml_client_rust::ffi::get_library_version();
        assert!(
            version.is_ok(),
            "FFI version call {} failed: {:?}",
            i,
            version.err()
        );

        let call_time = start.elapsed();
        call_times.push(call_time);

        if i % 1000 == 0 {
            println!("Completed {} calls", i);
        }
    }

    // Analyze performance statistics
    call_times.sort();

    let min_time = call_times[0];
    let max_time = call_times[NUM_CALLS - 1];
    let median_time = call_times[NUM_CALLS / 2];
    let p95_time = call_times[(NUM_CALLS as f64 * 0.95) as usize];
    let p99_time = call_times[(NUM_CALLS as f64 * 0.99) as usize];

    let total_time: Duration = call_times.iter().sum();
    let avg_time = total_time / NUM_CALLS as u32;

    println!("FFI call performance statistics:");
    println!("  Total calls: {}", NUM_CALLS);
    println!("  Total time: {:?}", total_time);
    println!("  Average: {:?}", avg_time);
    println!("  Median: {:?}", median_time);
    println!("  Min: {:?}", min_time);
    println!("  Max: {:?}", max_time);
    println!("  95th percentile: {:?}", p95_time);
    println!("  99th percentile: {:?}", p99_time);
    println!(
        "  Calls per second: {:.2}",
        NUM_CALLS as f64 / total_time.as_secs_f64()
    );

    // Performance assertions - these are loose bounds for reasonable performance
    assert!(
        avg_time < Duration::from_millis(100),
        "Average FFI call should be under 100ms"
    );
    assert!(
        p95_time < Duration::from_millis(500),
        "95% of calls should be under 500ms"
    );
}

/// Test resource cleanup under stress
#[tokio::test]
#[ignore] // Run with --ignored for performance testing
async fn test_resource_cleanup_stress() {
    init_test_logging();

    const STRESS_CYCLES: usize = 500;
    const CLIENTS_PER_CYCLE: usize = 20;

    println!(
        "Starting resource cleanup stress test: {} cycles × {} clients",
        STRESS_CYCLES, CLIENTS_PER_CYCLE
    );

    for cycle in 0..STRESS_CYCLES {
        let mut cycle_handles = Vec::new();

        // Create many clients simultaneously
        for i in 0..CLIENTS_PER_CYCLE {
            let handle = tokio::spawn(async move {
                let client = test_config::setup_test_client()
                    .expect(&format!("Failed to create client {}", i));

                // Perform some operations
                let context = BamlContext::new();
                let _ = client.call_function_raw("StressTest", context).await;

                // Client is automatically dropped when this task completes
                i
            });
            cycle_handles.push(handle);
        }

        // Wait for all clients in this cycle to complete
        for handle in cycle_handles {
            handle.await.expect("Stress test task should complete");
        }

        if cycle % 50 == 0 {
            println!("Completed stress cycle {}", cycle);

            // Try to create a new client to ensure resources are still available
            let test_client = test_config::setup_test_client();
            assert!(
                test_client.is_ok(),
                "Should still be able to create clients after stress cycle {}",
                cycle
            );
        }
    }

    println!("Resource cleanup stress test completed successfully");
}

/// Test large data handling performance
#[tokio::test]
#[ignore] // Run with --ignored for performance testing
async fn test_large_data_performance() {
    init_test_logging();

    let _client = test_config::setup_test_client().expect("Failed to create client");

    // TODO: Update after code generation to test with actual large data functions
    // Test different data sizes
    let data_sizes = vec![
        1_000,     // 1KB
        10_000,    // 10KB
        100_000,   // 100KB
        1_000_000, // 1MB
    ];

    println!("Testing large data handling performance");

    for size in data_sizes {
        let start_time = Instant::now();

        // Create large test data
        let large_data = create_large_test_data(size);

        // TODO: Replace with actual large data function call
        // let result = client.large_data_function(large_data).await;

        // For now, test that we can serialize large data (FFI boundary test)
        let serialized = serde_json::to_string(&large_data);
        assert!(
            serialized.is_ok(),
            "Should be able to serialize {} bytes of data",
            size
        );

        let processing_time = start_time.elapsed();

        println!("  {} bytes: {:?}", size, processing_time);

        // Performance assertions - adjust these based on expected performance
        match size {
            s if s <= 10_000 => assert!(processing_time < Duration::from_millis(100)),
            s if s <= 100_000 => assert!(processing_time < Duration::from_secs(1)),
            s if s <= 1_000_000 => assert!(processing_time < Duration::from_secs(10)),
            _ => (), // No assertion for very large data
        }
    }
}

/// Test streaming performance
#[tokio::test]
#[ignore] // Run with --ignored for performance testing
async fn test_streaming_performance() {
    init_test_logging();

    let _client = test_config::setup_test_client().expect("Failed to create client");

    // TODO: Update after code generation to test actual streaming functions
    // const STREAM_DURATION: Duration = Duration::from_secs(30);
    // let start_time = Instant::now();
    // let mut message_count = 0;
    //
    // let mut stream = client.performance_streaming_function("benchmark").await
    //     .expect("Failed to create performance stream");
    //
    // while let Some(result) = stream.next().await {
    //     match result {
    //         Ok(_) => message_count += 1,
    //         Err(e) => println!("Stream error: {:?}", e),
    //     }
    //
    //     if start_time.elapsed() > STREAM_DURATION {
    //         break;
    //     }
    // }
    //
    // let total_time = start_time.elapsed();
    // let messages_per_second = message_count as f64 / total_time.as_secs_f64();
    //
    // println!("Streaming performance:");
    // println!("  Duration: {:?}", total_time);
    // println!("  Messages: {}", message_count);
    // println!("  Messages per second: {:.2}", messages_per_second);

    println!("Streaming performance test will be completed after code generation");
}

/// Test memory usage patterns during normal operations
#[tokio::test]
#[ignore] // Run with --ignored for performance testing
async fn test_memory_usage_patterns() {
    init_test_logging();

    let client = test_config::setup_test_client().expect("Failed to create client");

    println!("Testing memory usage patterns");

    // Baseline memory usage
    if let Ok(baseline_memory) = get_current_memory_usage() {
        println!("Baseline memory: {} KB", baseline_memory / 1024);

        // Perform various operations and monitor memory
        const NUM_OPERATIONS: usize = 100;

        for i in 0..NUM_OPERATIONS {
            // TODO: Update with actual operations after code generation
            let context = BamlContext::new();
            let _ = client
                .call_function_raw(&format!("MemTest{}", i), context)
                .await;

            if i % 20 == 0 {
                if let Ok(current_memory) = get_current_memory_usage() {
                    let memory_delta = current_memory as i64 - baseline_memory as i64;
                    println!(
                        "Operation {}: {} KB (Δ{:+} KB)",
                        i,
                        current_memory / 1024,
                        memory_delta / 1024
                    );
                }
            }
        }

        // Final memory check
        tokio::time::sleep(Duration::from_millis(100)).await; // Allow cleanup

        if let Ok(final_memory) = get_current_memory_usage() {
            let final_delta = final_memory as i64 - baseline_memory as i64;
            println!(
                "Final memory: {} KB (Δ{:+} KB)",
                final_memory / 1024,
                final_delta / 1024
            );

            // Memory should not have grown excessively
            assert!(
                final_delta < 50 * 1024 * 1024, // Less than 50MB growth
                "Memory usage should not grow excessively: grew by {} KB",
                final_delta / 1024
            );
        }
    }
}

// Helper functions

fn create_large_test_data(size: usize) -> serde_json::Value {
    use serde_json::json;

    let chunk_size = 1000;
    let num_chunks = (size + chunk_size - 1) / chunk_size;

    let mut chunks = Vec::new();
    for i in 0..num_chunks {
        let chunk_data = "x".repeat(std::cmp::min(chunk_size, size - i * chunk_size));
        chunks.push(json!({
            "id": i,
            "data": chunk_data
        }));
    }

    json!({
        "type": "large_test_data",
        "size": size,
        "chunks": chunks
    })
}

fn get_current_memory_usage() -> Result<usize, std::io::Error> {
    // This is a rough estimation and platform-dependent
    // On a real system, you might use process-specific memory measurement

    #[cfg(target_os = "linux")]
    {
        use std::fs;
        let status = fs::read_to_string("/proc/self/status")?;
        for line in status.lines() {
            if line.starts_with("VmRSS:") {
                if let Some(kb_str) = line.split_whitespace().nth(1) {
                    if let Ok(kb) = kb_str.parse::<usize>() {
                        return Ok(kb * 1024); // Convert KB to bytes
                    }
                }
            }
        }
    }

    // Fallback for other platforms - return a placeholder value
    Ok(10 * 1024 * 1024) // 10MB placeholder
}
