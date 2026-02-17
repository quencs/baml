//! Integration tests for LLM orchestration plan building.
//!
//! These tests verify that `baml.llm.build_plan` correctly expands client trees
//! (primitive, fallback, round-robin) into flat lists of `OrchestrationStep`s,
//! and that `baml.llm.wrap_with_retry` applies the correct retry logic with
//! exponential backoff delays.

mod common;

use bex_engine::{BexEngine, BexExternalValue};
use sys_native::SysOpsExt;

/// Helper: compile source, create engine, call function, return result.
async fn run(source: &str, entry: &str) -> Result<BexExternalValue, bex_engine::EngineError> {
    let snapshot = common::compile_for_engine(source);
    let engine =
        BexEngine::new(snapshot, sys_types::SysOps::native()).expect("Failed to create engine");
    engine.call_function(entry, vec![]).await
}

// ============================================================================
// build_plan: plan shape (number of steps)
// ============================================================================

/// A primitive client produces a single-step plan.
#[tokio::test]
async fn plan_primitive_has_one_step() {
    let source = r##"
client<llm> A {
    provider openai
    options { model "gpt-4" }
}

function F(x: string) -> string {
    client A
    prompt #"{{ x }}"#
}

function check_plan() -> int {
    let c = baml.llm.get_client("F");
    let plan = baml.llm.build_plan(c);
    plan.length()
}
"##;

    let result = run(source, "check_plan").await.expect("test failed");
    assert_eq!(result, BexExternalValue::Int(1));
}

/// A fallback client with two sub-clients produces two steps.
#[tokio::test]
async fn plan_fallback_has_two_steps() {
    let source = r##"
client<llm> A {
    provider openai
    options { model "gpt-4" }
}

client<llm> B {
    provider openai
    options { model "gpt-3.5-turbo" }
}

client<llm> FB {
    provider fallback
    options { strategy [A, B] }
}

function F(x: string) -> string {
    client FB
    prompt #"{{ x }}"#
}

function check_plan() -> int {
    let c = baml.llm.get_client("F");
    let plan = baml.llm.build_plan(c);
    plan.length()
}
"##;

    let result = run(source, "check_plan").await.expect("test failed");
    assert_eq!(result, BexExternalValue::Int(2));
}

/// A fallback client with three sub-clients produces three steps.
#[tokio::test]
async fn plan_fallback_three_clients() {
    let source = r##"
client<llm> A {
    provider openai
    options { model "gpt-4" }
}

client<llm> B {
    provider openai
    options { model "gpt-3.5-turbo" }
}

client<llm> C {
    provider openai
    options { model "gpt-4o" }
}

client<llm> FB {
    provider fallback
    options { strategy [A, B, C] }
}

function F(x: string) -> string {
    client FB
    prompt #"{{ x }}"#
}

function check_plan() -> int {
    let c = baml.llm.get_client("F");
    let plan = baml.llm.build_plan(c);
    plan.length()
}
"##;

    let result = run(source, "check_plan").await.expect("test failed");
    assert_eq!(result, BexExternalValue::Int(3));
}

/// A round-robin client with two sub-clients produces a single step
/// (it picks one sub-client per invocation).
#[tokio::test]
async fn plan_round_robin_has_one_step() {
    let source = r##"
client<llm> A {
    provider openai
    options { model "gpt-4" }
}

client<llm> B {
    provider openai
    options { model "gpt-3.5-turbo" }
}

client<llm> RR {
    provider round-robin
    options { strategy [A, B] }
}

function F(x: string) -> string {
    client RR
    prompt #"{{ x }}"#
}

function check_plan() -> int {
    let c = baml.llm.get_client("F");
    let plan = baml.llm.build_plan(c);
    plan.length()
}
"##;

    let result = run(source, "check_plan").await.expect("test failed");
    assert_eq!(result, BexExternalValue::Int(1));
}

// ============================================================================
// build_plan: retry expands steps
// ============================================================================

/// A primitive client with retry(max=2) produces 3 steps (1 original + 2 retries).
#[tokio::test]
async fn plan_primitive_with_retry_expands() {
    let source = r##"
retry_policy Retry2 {
    max_retries 2
    initial_delay_ms 100
    multiplier 2
    max_delay_ms 500
}

client<llm> A {
    provider openai
    retry_policy Retry2
    options { model "gpt-4" }
}

function F(x: string) -> string {
    client A
    prompt #"{{ x }}"#
}

function check_plan() -> int {
    let c = baml.llm.get_client("F");
    let plan = baml.llm.build_plan(c);
    plan.length()
}
"##;

    let result = run(source, "check_plan").await.expect("test failed");
    assert_eq!(result, BexExternalValue::Int(3));
}

/// A fallback[A, B] with retry(max=1) produces 4 steps:
/// attempt 0: [A, B], attempt 1: [A, B]
#[tokio::test]
async fn plan_fallback_with_retry_multiplies() {
    let source = r##"
retry_policy Retry1 {
    max_retries 1
    initial_delay_ms 200
}

client<llm> A {
    provider openai
    options { model "gpt-4" }
}

client<llm> B {
    provider openai
    options { model "gpt-3.5-turbo" }
}

client<llm> FB {
    provider fallback
    retry_policy Retry1
    options { strategy [A, B] }
}

function F(x: string) -> string {
    client FB
    prompt #"{{ x }}"#
}

function check_plan() -> int {
    let c = baml.llm.get_client("F");
    let plan = baml.llm.build_plan(c);
    plan.length()
}
"##;

    let result = run(source, "check_plan").await.expect("test failed");
    // 2 sub-clients × (1 original + 1 retry) = 4 steps
    assert_eq!(result, BexExternalValue::Int(4));
}

// ============================================================================
// build_plan: delay values
// ============================================================================

/// First step always has `delay_ms=0`, retry steps have exponential backoff.
#[tokio::test]
async fn plan_delays_exponential_backoff() {
    let source = r##"
retry_policy ExpBackoff {
    max_retries 3
    initial_delay_ms 100
    multiplier 2
    max_delay_ms 1000
}

client<llm> A {
    provider openai
    retry_policy ExpBackoff
    options { model "gpt-4" }
}

function F(x: string) -> string {
    client A
    prompt #"{{ x }}"#
}

// Return array of delay_ms values for each step in the plan
function check_plan() -> int[] {
    let c = baml.llm.get_client("F");
    let plan = baml.llm.build_plan(c);
    let delays: int[] = [];
    for (let step in plan) {
        delays.push(step.delay_ms);
    }
    delays
}
"##;

    let result = run(source, "check_plan").await.expect("test failed");
    // 4 steps: original(0) + retry1(100) + retry2(200) + retry3(400)
    assert_eq!(
        result,
        BexExternalValue::Array {
            element_type: bex_engine::Ty::Int,
            items: vec![
                BexExternalValue::Int(0),
                BexExternalValue::Int(100),
                BexExternalValue::Int(200),
                BexExternalValue::Int(400),
            ],
        }
    );
}

/// Delays are capped at `max_delay_ms`.
#[tokio::test]
async fn plan_delays_capped_at_max() {
    let source = r##"
retry_policy CappedBackoff {
    max_retries 4
    initial_delay_ms 100
    multiplier 3
    max_delay_ms 500
}

client<llm> A {
    provider openai
    retry_policy CappedBackoff
    options { model "gpt-4" }
}

function F(x: string) -> string {
    client A
    prompt #"{{ x }}"#
}

function check_plan() -> int[] {
    let c = baml.llm.get_client("F");
    let plan = baml.llm.build_plan(c);
    let delays: int[] = [];
    for (let step in plan) {
        delays.push(step.delay_ms);
    }
    delays
}
"##;

    let result = run(source, "check_plan").await.expect("test failed");
    // 5 steps: 0, 100, 300, 500(capped), 500(capped)
    assert_eq!(
        result,
        BexExternalValue::Array {
            element_type: bex_engine::Ty::Int,
            items: vec![
                BexExternalValue::Int(0),
                BexExternalValue::Int(100),
                BexExternalValue::Int(300),
                BexExternalValue::Int(500),
                BexExternalValue::Int(500),
            ],
        }
    );
}

/// Fallback with retry: delays apply uniformly to all sub-client steps in a retry attempt.
#[tokio::test]
async fn plan_fallback_retry_delays() {
    let source = r##"
retry_policy R {
    max_retries 1
    initial_delay_ms 200
}

client<llm> A {
    provider openai
    options { model "gpt-4" }
}

client<llm> B {
    provider openai
    options { model "gpt-3.5-turbo" }
}

client<llm> FB {
    provider fallback
    retry_policy R
    options { strategy [A, B] }
}

function F(x: string) -> string {
    client FB
    prompt #"{{ x }}"#
}

function check_plan() -> int[] {
    let c = baml.llm.get_client("F");
    let plan = baml.llm.build_plan(c);
    let delays: int[] = [];
    for (let step in plan) {
        delays.push(step.delay_ms);
    }
    delays
}
"##;

    let result = run(source, "check_plan").await.expect("test failed");
    // attempt 0: [A(0), B(0)], attempt 1: [A(200), B(200)]
    assert_eq!(
        result,
        BexExternalValue::Array {
            element_type: bex_engine::Ty::Int,
            items: vec![
                BexExternalValue::Int(0),
                BexExternalValue::Int(0),
                BexExternalValue::Int(200),
                BexExternalValue::Int(200),
            ],
        }
    );
}

// ============================================================================
// build_plan: no retry (null retry policy)
// ============================================================================

/// A client without `retry_policy` produces steps with all delays = 0.
#[tokio::test]
async fn plan_no_retry_all_zero_delays() {
    let source = r##"
client<llm> A {
    provider openai
    options { model "gpt-4" }
}

client<llm> B {
    provider openai
    options { model "gpt-3.5-turbo" }
}

client<llm> FB {
    provider fallback
    options { strategy [A, B] }
}

function F(x: string) -> string {
    client FB
    prompt #"{{ x }}"#
}

function check_plan() -> int[] {
    let c = baml.llm.get_client("F");
    let plan = baml.llm.build_plan(c);
    let delays: int[] = [];
    for (let step in plan) {
        delays.push(step.delay_ms);
    }
    delays
}
"##;

    let result = run(source, "check_plan").await.expect("test failed");
    assert_eq!(
        result,
        BexExternalValue::Array {
            element_type: bex_engine::Ty::Int,
            items: vec![BexExternalValue::Int(0), BexExternalValue::Int(0),],
        }
    );
}

// ============================================================================
// build_plan: nested composition
// ============================================================================

/// `Fallback[RoundRobin[A, B], C]` produces 2 steps: one from RR (picks one) + C.
#[tokio::test]
async fn plan_nested_fallback_round_robin() {
    let source = r##"
client<llm> A {
    provider openai
    options { model "gpt-4" }
}

client<llm> B {
    provider openai
    options { model "gpt-3.5-turbo" }
}

client<llm> RR {
    provider round-robin
    options { strategy [A, B] }
}

client<llm> C {
    provider openai
    options { model "gpt-4o" }
}

client<llm> FB {
    provider fallback
    options { strategy [RR, C] }
}

function F(x: string) -> string {
    client FB
    prompt #"{{ x }}"#
}

function check_plan() -> int {
    let c = baml.llm.get_client("F");
    let plan = baml.llm.build_plan(c);
    plan.length()
}
"##;

    let result = run(source, "check_plan").await.expect("test failed");
    // RR picks 1 sub-client, then C = 2 steps total
    assert_eq!(result, BexExternalValue::Int(2));
}

/// Nested retry: inner client has retry, outer fallback does not.
/// Fallback[A(retry=1), B] = A, A(retry), B = 3 steps.
#[tokio::test]
async fn plan_nested_inner_retry() {
    let source = r##"
retry_policy InnerRetry {
    max_retries 1
    initial_delay_ms 50
}

client<llm> A {
    provider openai
    retry_policy InnerRetry
    options { model "gpt-4" }
}

client<llm> B {
    provider openai
    options { model "gpt-3.5-turbo" }
}

client<llm> FB {
    provider fallback
    options { strategy [A, B] }
}

function F(x: string) -> string {
    client FB
    prompt #"{{ x }}"#
}

function check_len() -> int {
    let c = baml.llm.get_client("F");
    let plan = baml.llm.build_plan(c);
    plan.length()
}

function check_delays() -> int[] {
    let c = baml.llm.get_client("F");
    let plan = baml.llm.build_plan(c);
    let delays: int[] = [];
    for (let step in plan) {
        delays.push(step.delay_ms);
    }
    delays
}
"##;

    let snapshot = common::compile_for_engine(source);
    let engine =
        BexEngine::new(snapshot, sys_types::SysOps::native()).expect("Failed to create engine");

    // A(original) + A(retry) + B = 3 steps
    let len = engine
        .call_function("check_len", vec![])
        .await
        .expect("test_len failed");
    assert_eq!(len, BexExternalValue::Int(3));

    // Delays: A(0), A(50), B(0)
    let delays = engine
        .call_function("check_delays", vec![])
        .await
        .expect("test_delays failed");
    assert_eq!(
        delays,
        BexExternalValue::Array {
            element_type: bex_engine::Ty::Int,
            items: vec![
                BexExternalValue::Int(0),
                BexExternalValue::Int(50),
                BexExternalValue::Int(0),
            ],
        }
    );
}

// ============================================================================
// build_plan: client names in steps
// ============================================================================

/// Verify that the correct primitive clients appear in the plan.
#[tokio::test]
async fn plan_step_client_names() {
    let source = r##"
client<llm> Primary {
    provider openai
    options { model "gpt-4" }
}

client<llm> Backup {
    provider openai
    options { model "gpt-3.5-turbo" }
}

client<llm> FB {
    provider fallback
    options { strategy [Primary, Backup] }
}

function F(x: string) -> string {
    client FB
    prompt #"{{ x }}"#
}

function check_plan() -> string[] {
    let c = baml.llm.get_client("F");
    let plan = baml.llm.build_plan(c);
    let names: string[] = [];
    for (let step in plan) {
        names.push(step.primitive_client.name);
    }
    names
}
"##;

    let result = run(source, "check_plan").await.expect("test failed");
    assert_eq!(
        result,
        BexExternalValue::Array {
            element_type: bex_engine::Ty::String,
            items: vec![
                BexExternalValue::String("Primary".to_string()),
                BexExternalValue::String("Backup".to_string()),
            ],
        }
    );
}

/// Retry duplicates client names: [A, A, A] for retry=2.
#[tokio::test]
async fn plan_retry_duplicates_client_names() {
    let source = r##"
retry_policy R {
    max_retries 2
}

client<llm> A {
    provider openai
    retry_policy R
    options { model "gpt-4" }
}

function F(x: string) -> string {
    client A
    prompt #"{{ x }}"#
}

function check_plan() -> string[] {
    let c = baml.llm.get_client("F");
    let plan = baml.llm.build_plan(c);
    let names: string[] = [];
    for (let step in plan) {
        names.push(step.primitive_client.name);
    }
    names
}
"##;

    let result = run(source, "check_plan").await.expect("test failed");
    assert_eq!(
        result,
        BexExternalValue::Array {
            element_type: bex_engine::Ty::String,
            items: vec![
                BexExternalValue::String("A".to_string()),
                BexExternalValue::String("A".to_string()),
                BexExternalValue::String("A".to_string()),
            ],
        }
    );
}
