//! GC integration tests for handle-as-GC-root behavior.
//!
//! These tests verify that handles returned from `call_function` properly
//! protect their referenced objects from garbage collection.

mod common;

use std::collections::HashMap;

use bex_engine::{BexEngine, ExternalValue, Snapshot};
use common::compile_for_engine;

/// Test that a handle prevents the referenced object from being collected.
#[tokio::test]
async fn test_handle_prevents_gc_collection() {
    let source = r#"
        function return_string() -> string {
            "hello world"
        }
    "#;

    let snapshot = compile_for_engine(source);
    let engine = BexEngine::new(snapshot, HashMap::new()).unwrap();

    // Get a handle to a string object
    let result = engine.call_function("return_string", &[]).await.unwrap();
    assert!(
        matches!(result, ExternalValue::Object(_)),
        "Expected Object handle, got {result:?}"
    );

    // Trigger GC
    let _stats = engine.collect_garbage().await;

    // Handle should still be valid - convert to snapshot
    let snapshot = engine.to_snapshot(result).unwrap();
    assert_eq!(snapshot, Snapshot::String("hello world".to_string()));
}

/// Test that handles to arrays preserve the entire structure.
#[tokio::test]
async fn test_array_preserved_through_gc() {
    let source = r#"
        function return_array() -> string[] {
            let items = ["a", "b", "c", "d", "e"]
            items
        }
    "#;

    let snapshot = compile_for_engine(source);
    let engine = BexEngine::new(snapshot, HashMap::new()).unwrap();

    // Get a handle to the array
    let result = engine.call_function("return_array", &[]).await.unwrap();
    assert!(
        matches!(result, ExternalValue::Object(_)),
        "Expected Object handle, got {result:?}"
    );

    // Trigger GC
    let _stats = engine.collect_garbage().await;

    // Array and all its elements should be preserved
    let snapshot = engine.to_snapshot(result).unwrap();
    match snapshot {
        Snapshot::Array(arr) => {
            assert_eq!(arr.len(), 5);
            assert_eq!(arr[0], Snapshot::String("a".to_string()));
            assert_eq!(arr[4], Snapshot::String("e".to_string()));
        }
        other => panic!("Expected array, got: {other:?}"),
    }
}

/// Test primitive return values (should be Snapshot, not Handle).
#[tokio::test]
async fn test_primitive_returns_are_snapshots() {
    let source = r#"
        function return_int() -> int {
            42
        }
        function return_null() -> null {
            null
        }
        function return_bool() -> bool {
            true
        }
    "#;

    let snapshot = compile_for_engine(source);
    let engine = BexEngine::new(snapshot, HashMap::new()).unwrap();

    // Int should be Snapshot
    let result = engine.call_function("return_int", &[]).await.unwrap();
    assert!(matches!(result, ExternalValue::Snapshot(Snapshot::Int(42))));

    // Null should be Snapshot
    let result = engine.call_function("return_null", &[]).await.unwrap();
    assert!(matches!(result, ExternalValue::Snapshot(Snapshot::Null)));

    // Bool should be Snapshot
    let result = engine.call_function("return_bool", &[]).await.unwrap();
    assert!(matches!(
        result,
        ExternalValue::Snapshot(Snapshot::Bool(true))
    ));
}
