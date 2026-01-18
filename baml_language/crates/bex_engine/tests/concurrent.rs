//! Tests for concurrent execution with multiple VMs sharing the heap.
//!
//! These tests verify that `BexEngine` can safely execute multiple function
//! calls concurrently via `tokio::spawn`, with each call getting its own VM
//! and TLAB.

mod common;

use std::{
    collections::HashMap,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

use baml_tests::vm::Value;
use bex_engine::BexEngine;
use common::{compile_for_engine, value_from_resolved};

#[tokio::test]
async fn test_concurrent_calls_no_race() {
    // Create a simple BAML program with a function that does some allocation
    let source = r#"
        function test_function() -> int {
            let a = 10 + 1
            let b = a * 2
            b
        }
    "#;

    let snapshot = compile_for_engine(source);
    let engine =
        Arc::new(BexEngine::new(snapshot, HashMap::new()).expect("Failed to create engine"));

    // Spawn 10 concurrent calls
    let mut handles = vec![];
    for _ in 0..10 {
        let engine = Arc::clone(&engine);
        handles.push(tokio::spawn(async move {
            // Each call is independent
            engine.call_function("test_function", &[]).await
        }));
    }

    // All should complete successfully
    for (i, handle) in handles.into_iter().enumerate() {
        let result = handle.await.expect("task panicked");
        assert!(result.is_ok(), "concurrent call {i} failed: {result:?}");

        // Verify the result
        let value = result.unwrap();
        let actual = value_from_resolved(&value);
        let expected = Value::Int(22); // (10 + 1) * 2
        assert_eq!(actual, expected, "Result mismatch for call {i}");
    }
}

#[tokio::test]
async fn test_concurrent_allocations_no_overlap() {
    // Create a BAML program that allocates many objects
    let source = r#"
        function allocate_many() -> string[] {
            let items = ["a", "b", "c", "d", "e"]
            items
        }
    "#;

    let snapshot = compile_for_engine(source);
    let engine =
        Arc::new(BexEngine::new(snapshot, HashMap::new()).expect("Failed to create engine"));

    // Track allocations from each concurrent call
    let allocation_count = Arc::new(AtomicUsize::new(0));

    let mut handles = vec![];
    for _ in 0..5 {
        let engine = Arc::clone(&engine);
        let count = Arc::clone(&allocation_count);
        handles.push(tokio::spawn(async move {
            // Function that allocates many objects
            let result = engine.call_function("allocate_many", &[]).await?;

            count.fetch_add(1, Ordering::SeqCst);
            Ok::<_, bex_engine::EngineError>(result)
        }));
    }

    for handle in handles {
        let result = handle.await.expect("task panicked");
        assert!(result.is_ok(), "call failed: {result:?}");

        // Verify the result is correct
        let value = result.unwrap();
        let actual = value_from_resolved(&value);
        let expected = Value::array(vec![
            Value::string("a"),
            Value::string("b"),
            Value::string("c"),
            Value::string("d"),
            Value::string("e"),
        ]);
        assert_eq!(actual, expected);
    }

    assert_eq!(allocation_count.load(Ordering::SeqCst), 5);
}

#[tokio::test]
async fn test_heap_stats_during_concurrent_execution() {
    // Create a simple BAML program
    let source = r#"
        function test_function() -> int {
            42
        }
    "#;

    let snapshot = compile_for_engine(source);
    let engine =
        Arc::new(BexEngine::new(snapshot, HashMap::new()).expect("Failed to create engine"));

    let initial_stats = engine.heap_stats();

    // Run concurrent calls
    let mut handles = vec![];
    for _ in 0..3 {
        let engine = Arc::clone(&engine);
        handles.push(tokio::spawn(async move {
            engine.call_function("test_function", &[]).await
        }));
    }

    for handle in handles {
        handle.await.expect("task panicked").expect("call failed");
    }

    let final_stats = engine.heap_stats();

    // Should have allocated TLAB chunks for concurrent VMs
    // Note: Each VM gets its own TLAB, so we expect at least 3 chunks
    assert!(
        final_stats.tlab_chunks >= initial_stats.tlab_chunks,
        "Expected TLAB chunks to be allocated (initial: {}, final: {})",
        initial_stats.tlab_chunks,
        final_stats.tlab_chunks
    );
}

#[tokio::test]
async fn test_concurrent_string_allocations() {
    // Test that string allocations don't overlap between concurrent calls
    // Each function allocates different strings
    let source = r#"
        function create_string_a() -> string {
            "string_a"
        }

        function create_string_b() -> string {
            "string_b"
        }

        function create_string_c() -> string {
            "string_c"
        }
    "#;

    let snapshot = compile_for_engine(source);
    let engine =
        Arc::new(BexEngine::new(snapshot, HashMap::new()).expect("Failed to create engine"));

    // Spawn many concurrent calls that allocate different strings
    let mut handles = vec![];
    for func_name in ["create_string_a", "create_string_b", "create_string_c"]
        .iter()
        .cycle()
        .take(20)
    {
        let engine = Arc::clone(&engine);
        let func = (*func_name).to_string();
        handles.push(tokio::spawn(async move {
            let result = engine.call_function(&func, &[]).await?;
            Ok::<_, bex_engine::EngineError>((func, result))
        }));
    }

    // Collect all results
    for handle in handles {
        let (func_name, value) = handle.await.expect("task panicked").expect("call failed");
        let actual = value_from_resolved(&value);

        // Extract expected suffix from function name
        let suffix = func_name.strip_prefix("create_string_").unwrap();
        let expected = Value::string(&format!("string_{suffix}"));
        assert_eq!(actual, expected, "String mismatch for {func_name}");
    }
}

#[tokio::test]
async fn test_concurrent_array_allocations() {
    // Test concurrent array allocations with different functions
    let source = r#"
        function create_array_5() -> int[] {
            [0, 1, 2, 3, 4]
        }

        function create_array_10() -> int[] {
            [0, 1, 2, 3, 4, 5, 6, 7, 8, 9]
        }

        function create_array_15() -> int[] {
            [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14]
        }
    "#;

    let snapshot = compile_for_engine(source);
    let engine =
        Arc::new(BexEngine::new(snapshot, HashMap::new()).expect("Failed to create engine"));

    // Spawn concurrent calls with different array sizes
    let mut handles = vec![];
    for (func_name, size) in [
        ("create_array_5", 5),
        ("create_array_10", 10),
        ("create_array_15", 15),
    ] {
        let engine = Arc::clone(&engine);
        handles.push(tokio::spawn(async move {
            let result = engine.call_function(func_name, &[]).await?;
            Ok::<_, bex_engine::EngineError>((size, result))
        }));
    }

    // Verify all arrays are correct
    for handle in handles {
        let (size, value) = handle.await.expect("task panicked").expect("call failed");
        let actual = value_from_resolved(&value);

        // Build expected array [0, 1, 2, ..., size-1]
        let expected = Value::array((0..size).map(Value::Int).collect());
        assert_eq!(actual, expected, "Array mismatch for size {size}");
    }
}
