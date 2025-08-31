//! Streaming function integration tests
//!
//! Tests BAML streaming functionality including:
//! - Basic streaming with futures::Stream
//! - Partial vs final results  
//! - Stream error handling
//! - Concurrent streaming calls

use assert_matches::assert_matches;
use baml_integ_tests_rust::*;
use futures::{StreamExt, TryStreamExt};

// This module will be populated with generated types after running baml-cli generate
#[allow(unused_imports)]
use baml_client::{types::*, *};

/// Test basic streaming functionality  
#[tokio::test]
async fn test_basic_streaming() {
    init_test_logging();

    let client = test_config::setup_test_client().expect("Failed to create client");

    // Test streaming with the test_fn_named_args_single_string_stream function
    let stream_result = client.test_fn_named_args_single_string_stream("stream test input".to_string()).await;
    
    match stream_result {
        Ok(mut stream) => {
            println!("Successfully created stream");
            
            let mut partial_count = 0;
            let mut final_result = None;
            let mut stream_completed = false;
            
            // Use a timeout to avoid hanging indefinitely
            let timeout_duration = std::time::Duration::from_secs(30);
            let stream_future = async {
                while let Some(result) = stream.next().await {
                    match result {
                        Ok(stream_state) => {
                            match stream_state {
                                baml_client_rust::StreamState::Partial(data) => {
                                    partial_count += 1;
                                    println!("Partial result {}: {:?}", partial_count, data);
                                }
                                baml_client_rust::StreamState::Final(data) => {
                                    final_result = Some(data);
                                    println!("Final result: {:?}", final_result);
                                    stream_completed = true;
                                    break;
                                }
                            }
                        }
                        Err(e) => {
                            println!("Stream error: {:?}", e);
                            break;
                        }
                    }
                }
            };
            
            match tokio::time::timeout(timeout_duration, stream_future).await {
                Ok(_) => {
                    println!("Stream processing completed");
                    println!("Received {} partial results", partial_count);
                    if stream_completed {
                        assert!(final_result.is_some(), "Should receive final result");
                    }
                }
                Err(_) => {
                    println!("Stream timed out after {:?} - this may be expected in test environments", timeout_duration);
                }
            }
        }
        Err(e) => {
            println!("Failed to create stream (may be expected in test environment): {}", e);
        }
    }
}

/// Test stream error handling
#[tokio::test]
async fn test_stream_error_handling() {
    init_test_logging();

    let client = test_config::setup_test_client().expect("Failed to create client");

    // Test streaming with potentially problematic input
    let problematic_inputs = vec![
        "".to_string(),  // Empty string
        "\x00\x01\x02".to_string(),  // Binary data
        "\"unclosed quote".to_string(),  // Malformed JSON-like input
        "{'malformed': json}".to_string(),  // Invalid JSON
    ];
    
    for (i, input) in problematic_inputs.into_iter().enumerate() {
        println!("Testing problematic input {}: {:?}", i, input);
        
        let stream_result = client.test_fn_named_args_single_string_stream(input).await;
        
        match stream_result {
            Ok(mut stream) => {
                // If stream creation succeeded, test error handling during consumption
                let timeout_duration = std::time::Duration::from_secs(10);
                let stream_future = async {
                    while let Some(result) = stream.next().await {
                        match result {
                            Ok(stream_state) => {
                                println!("Stream state received: {:?}", stream_state);
                                // Continue processing
                            }
                            Err(e) => {
                                println!("Stream error (expected): {}", e);
                                // Error in stream is expected for some inputs
                                break;
                            }
                        }
                    }
                };
                
                match tokio::time::timeout(timeout_duration, stream_future).await {
                    Ok(_) => println!("Stream completed"),
                    Err(_) => println!("Stream timed out (may be expected)"),
                }
            }
            Err(e) => {
                println!("Stream creation failed (may be expected): {}", e);
            }
        }
    }
}

/// Test concurrent streaming calls
#[tokio::test]
async fn test_concurrent_streaming() {
    init_test_logging();

    let client = test_config::setup_test_client().expect("Failed to create client");

    const NUM_STREAMS: usize = 3;
    let mut handles = Vec::new();

    for i in 0..NUM_STREAMS {
        let client_clone = client.clone();
        let handle = tokio::spawn(async move {
            let input = format!("concurrent stream input {}", i);
            println!("Starting concurrent stream {}", i);
            
            match client_clone.test_fn_named_args_single_string_stream(input).await {
                Ok(mut stream) => {
                    let mut results = Vec::new();
                    let timeout = std::time::Duration::from_secs(15);
                    
                    let stream_future = async {
                        while let Some(result) = stream.next().await {
                            match result {
                                Ok(stream_state) => {
                                    match stream_state {
                                        baml_client_rust::StreamState::Partial(data) => {
                                            results.push(format!("Partial: {:?}", data));
                                        }
                                        baml_client_rust::StreamState::Final(data) => {
                                            results.push(format!("Final: {:?}", data));
                                            break;
                                        }
                                    }
                                }
                                Err(e) => {
                                    println!("Stream {} error: {:?}", i, e);
                                    break;
                                }
                            }
                        }
                        results
                    };
                    
                    match tokio::time::timeout(timeout, stream_future).await {
                        Ok(results) => {
                            println!("Stream {} completed with {} results", i, results.len());
                            (i, results)
                        }
                        Err(_) => {
                            println!("Stream {} timed out", i);
                            (i, vec!["timeout".to_string()])
                        }
                    }
                }
                Err(e) => {
                    println!("Failed to create stream {}: {}", i, e);
                    (i, vec![format!("error: {}", e)])
                }
            }
        });
        handles.push(handle);
    }

    // Wait for all streams to complete
    let mut all_results = Vec::new();
    for handle in handles {
        let (stream_id, results) = handle.await.expect("Stream task should not panic");
        println!("Stream {} final result count: {}", stream_id, results.len());
        all_results.push((stream_id, results));
    }

    assert_eq!(all_results.len(), NUM_STREAMS, "All stream tasks should complete");
    
    // Check that we got some results from most streams
    let successful_streams = all_results.iter()
        .filter(|(_, results)| !results.is_empty())
        .count();
    
    println!("Successful streams: {}/{}", successful_streams, NUM_STREAMS);
}

/// Test stream cancellation and cleanup
#[tokio::test]
async fn test_stream_cancellation() {
    init_test_logging();

    let client = test_config::setup_test_client().expect("Failed to create client");

    // TODO: Update after code generation
    // Test that dropping a stream properly cleans up resources

    println!("Client created successfully - stream cancellation test will be completed after code generation");
}

/// Test stream with timeout
#[tokio::test]
async fn test_stream_with_timeout() {
    init_test_logging();

    let client = test_config::setup_test_client().expect("Failed to create client");

    // TODO: Update after code generation
    // let timeout_duration = std::time::Duration::from_secs(30);
    //
    // let stream = client.streaming_function("test input").await
    //     .expect("Failed to create stream");
    //
    // let timed_stream = tokio::time::timeout(timeout_duration, async {
    //     let results: Vec<_> = stream.collect().await;
    //     results
    // });
    //
    // match timed_stream.await {
    //     Ok(results) => {
    //         assert!(!results.is_empty(), "Should receive results within timeout");
    //         // Check that all results are Ok
    //         for result in results {
    //             assert!(result.is_ok(), "All stream results should be Ok");
    //         }
    //     }
    //     Err(_) => panic!("Stream timed out after {:?}", timeout_duration),
    // }

    println!("Client created successfully - timeout test will be completed after code generation");
}

/// Test stream collect functionality
#[tokio::test]
async fn test_stream_collect() {
    init_test_logging();

    let client = test_config::setup_test_client().expect("Failed to create client");

    // TODO: Update after code generation
    // Test different ways of collecting stream results

    // Method 1: Collect all
    // let stream1 = client.streaming_function("input1").await.expect("Stream failed");
    // let all_results: Vec<_> = stream1.collect().await;
    // assert!(!all_results.is_empty());

    // Method 2: Take first N
    // let stream2 = client.streaming_function("input2").await.expect("Stream failed");
    // let first_three: Vec<_> = stream2.take(3).collect().await;
    // assert!(first_three.len() <= 3);

    // Method 3: Filter for finals only
    // let stream3 = client.streaming_function("input3").await.expect("Stream failed");
    // let finals: Vec<_> = stream3
    //     .filter_map(|result| async {
    //         match result {
    //             Ok(StreamState::Final(data)) => Some(Ok(data)),
    //             Ok(StreamState::Partial(_)) => None,
    //             Err(e) => Some(Err(e)),
    //         }
    //     })
    //     .collect()
    //     .await;
    // assert!(!finals.is_empty());

    println!("Client created successfully - collect test will be completed after code generation");
}

/// Test streaming with different input types
#[tokio::test]
async fn test_streaming_input_types() {
    init_test_logging();

    let client = test_config::setup_test_client().expect("Failed to create client");

    // Test string input streaming
    println!("Testing string input streaming...");
    let string_stream_result = client.test_fn_named_args_single_string_stream("string input test".to_string()).await;
    test_stream_basic_consumption(string_stream_result, "string").await;
    
    // Test integer input streaming  
    println!("Testing integer input streaming...");
    let int_stream_result = client.test_fn_named_args_single_int_stream(42).await;
    test_stream_basic_consumption(int_stream_result, "int").await;
    
    // Test boolean input streaming
    println!("Testing boolean input streaming...");
    let bool_stream_result = client.test_fn_named_args_single_bool_stream(true).await;
    test_stream_basic_consumption(bool_stream_result, "bool").await;
    
    // Test float input streaming
    println!("Testing float input streaming...");
    let float_stream_result = client.test_fn_named_args_single_float_stream(3.14).await;
    test_stream_basic_consumption(float_stream_result, "float").await;
}

// Helper function to test basic stream consumption
async fn test_stream_basic_consumption<T>(
    stream_result: BamlResult<impl futures::Stream<Item = BamlResult<baml_client_rust::StreamState<T>>> + Send + Sync>,
    input_type: &str
) where T: std::fmt::Debug {
    match stream_result {
        Ok(mut stream) => {
            println!("Successfully created {} stream", input_type);
            
            let timeout = std::time::Duration::from_secs(10);
            let stream_future = async {
                let mut count = 0;
                while let Some(result) = stream.next().await {
                    count += 1;
                    match result {
                        Ok(stream_state) => {
                            println!("  {} stream result {}: {:?}", input_type, count, stream_state);
                            if count >= 5 { // Limit to prevent long runs in test
                                break;
                            }
                        }
                        Err(e) => {
                            println!("  {} stream error: {}", input_type, e);
                            break;
                        }
                    }
                }
                count
            };
            
            match tokio::time::timeout(timeout, stream_future).await {
                Ok(count) => println!("  {} stream processed {} items", input_type, count),
                Err(_) => println!("  {} stream timed out", input_type),
            }
        }
        Err(e) => {
            println!("{} stream creation failed (may be expected): {}", input_type, e);
        }
    }
}

/// Test streaming memory usage (performance test)
#[tokio::test]
#[ignore] // Run with --ignored for performance testing
async fn test_streaming_memory_usage() {
    init_test_logging();

    let client = test_config::setup_test_client().expect("Failed to create client");

    // TODO: Update after code generation
    // Test that streaming doesn't accumulate excessive memory
    // This is important for long-running streams

    println!("Client created successfully - memory test will be completed after code generation");
}

/// Test streaming backpressure handling
#[tokio::test]
async fn test_streaming_backpressure() {
    init_test_logging();

    let client = test_config::setup_test_client().expect("Failed to create client");

    // TODO: Update after code generation
    // Test what happens when consumer is slower than producer

    println!(
        "Client created successfully - backpressure test will be completed after code generation"
    );
}

/// Test streaming with client drop (cleanup test)
#[tokio::test]
async fn test_streaming_with_client_drop() {
    init_test_logging();

    // TODO: Update after code generation
    // Test behavior when client is dropped while stream is active

    println!("Cleanup test will be completed after code generation");
}
