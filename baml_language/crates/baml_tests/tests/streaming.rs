//! Tests for BEP-009 streaming primitives and full streaming orchestration.

use baml_tests::engine::{self, IndexMap, OptLevel};
use bex_engine::BexExternalValue;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path},
};

// ============================================================================
// Helpers
// ============================================================================

/// Build an OpenAI-style SSE body for streaming chat completions.
fn openai_sse_body(chunks: &[&str]) -> String {
    let mut body = String::new();
    for chunk in chunks {
        body.push_str(&format!(
            "event: message\ndata: {{\"choices\":[{{\"delta\":{{\"content\":\"{chunk}\"}}}}]}}\n\n"
        ));
    }
    body.push_str(
        "event: message\ndata: {\"choices\":[{\"delta\":{},\"finish_reason\":\"stop\"}]}\n\n",
    );
    body.push_str("event: message\ndata: [DONE]\n\n");
    body
}

/// Start a mock server that serves an SSE endpoint at /v1/chat/completions.
async fn mock_openai_streaming(chunks: &[&str]) -> (MockServer, String) {
    let server = MockServer::start().await;
    let body = openai_sse_body(chunks);
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(body),
        )
        .mount(&server)
        .await;
    let uri = server.uri();
    (server, uri)
}

/// Start a mock server that serves a raw SSE endpoint at /sse.
async fn mock_sse_server(events: &[(&str, &str)]) -> (MockServer, String) {
    let server = MockServer::start().await;
    let mut body = String::new();
    for (event_type, data) in events {
        body.push_str(&format!("event: {event_type}\ndata: {data}\n\n"));
    }
    Mock::given(method("GET"))
        .and(path("/sse"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(body),
        )
        .mount(&server)
        .await;
    let uri = server.uri();
    (server, uri)
}

/// Build BAML source for a streaming LLM test with a mock server URL.
fn streaming_llm_source(base_url: &str) -> String {
    format!(
        r##"
        client<llm> TestClient {{
            provider openai
            options {{
                model "gpt-4o"
                api_key "test-key"
                base_url "{base_url}"
            }}
        }}

        function TestFunc(input: string) -> string {{
            client TestClient
            prompt #"Say hello to {{{{ input }}}}"#
        }}

        function main() -> string {{
            baml.llm.stream_llm_function("TestFunc", {{"input": "world"}})
        }}
    "##
    )
}

// ============================================================================
// SSE Primitive Tests
// ============================================================================

#[tokio::test]
async fn sse_fetch_and_next_returns_events() {
    let (_server, uri) = mock_sse_server(&[("message", "hello"), ("message", "world")]).await;

    let output = engine::run_test(
        &format!(
            r#"
            function main() -> string {{
                let request = baml.http.Request {{
                    method: "GET",
                    url: "{uri}/sse",
                    headers: {{}},
                    body: "",
                }};
                let sse = baml.http.fetch_sse(request);
                let events = sse.next();
                sse.close();
                if (events == null) {{ return "null"; }}
                events
            }}
        "#
        ),
        "main",
        IndexMap::new(),
        OptLevel::One,
    )
    .await;

    let result = output.result.expect("should succeed");
    let BexExternalValue::String(json) = result else {
        panic!("expected string, got {result:?}");
    };
    // The JSON array should contain both events with their data.
    assert!(
        json.contains("\"data\":\"hello\""),
        "should contain hello event: {json}"
    );
    assert!(
        json.contains("\"data\":\"world\""),
        "should contain world event: {json}"
    );
}

#[tokio::test]
async fn sse_next_returns_null_when_done() {
    let (_server, uri) = mock_sse_server(&[("message", "only-one")]).await;

    let output = engine::run_test(
        &format!(
            r#"
            function main() -> bool {{
                let request = baml.http.Request {{
                    method: "GET",
                    url: "{uri}/sse",
                    headers: {{}},
                    body: "",
                }};
                let sse = baml.http.fetch_sse(request);
                let first = sse.next();
                let second = sse.next();
                sse.close();
                second == null
            }}
        "#
        ),
        "main",
        IndexMap::new(),
        OptLevel::One,
    )
    .await;

    assert_eq!(output.result, Ok(BexExternalValue::Bool(true)));
}

#[tokio::test]
async fn sse_fetch_error_on_bad_status() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/sse"))
        .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
        .mount(&server)
        .await;
    let uri = server.uri();

    let output = engine::run_test(
        &format!(
            r#"
            function main() -> string {{
                let request = baml.http.Request {{
                    method: "GET",
                    url: "{uri}/sse",
                    headers: {{}},
                    body: "",
                }};
                let sse = baml.http.fetch_sse(request);
                "should not reach"
            }}
        "#
        ),
        "main",
        IndexMap::new(),
        OptLevel::One,
    )
    .await;

    assert!(output.result.is_err(), "Expected error for 500 response");
    let err = output.result.unwrap_err().to_string();
    assert!(
        err.contains("500"),
        "Error should mention status code: {err}"
    );
}

#[tokio::test]
async fn sse_loop_collects_all_events() {
    let (_server, uri) =
        mock_sse_server(&[("message", "a"), ("message", "b"), ("message", "c")]).await;

    let output = engine::run_test(
        &format!(
            r#"
            function main() -> int {{
                let request = baml.http.Request {{
                    method: "GET",
                    url: "{uri}/sse",
                    headers: {{}},
                    body: "",
                }};
                let sse = baml.http.fetch_sse(request);
                let count = 0;
                while (true) {{
                    let events = sse.next();
                    if (events == null) {{ break; }}
                    count += 1;
                }}
                sse.close();
                count
            }}
        "#
        ),
        "main",
        IndexMap::new(),
        OptLevel::One,
    )
    .await;

    // All events arrive in a single batch (wiremock sends full body at once),
    // so count should be 1 (one call to next() returns all events).
    assert_eq!(output.result, Ok(BexExternalValue::Int(1)));
}

// ============================================================================
// Full Streaming Orchestration Tests
// ============================================================================

#[tokio::test]
async fn stream_llm_function_openai_returns_final_value() {
    let (_server, uri) = mock_openai_streaming(&["Hello", ", ", "world", "!"]).await;

    let output = engine::run_test(
        &streaming_llm_source(&uri),
        "main",
        IndexMap::new(),
        OptLevel::One,
    )
    .await;

    assert_eq!(
        output.result,
        Ok(BexExternalValue::String("Hello, world!".to_string()))
    );
}

#[tokio::test]
async fn stream_llm_function_emits_partials() {
    let (_server, uri) = mock_openai_streaming(&["Hello", ", ", "world", "!"]).await;

    let output = engine::run_test_streaming(
        &streaming_llm_source(&uri),
        "main",
        IndexMap::new(),
        OptLevel::One,
    )
    .await;

    assert_eq!(
        output.result,
        Ok(BexExternalValue::String("Hello, world!".to_string()))
    );

    assert!(
        !output.partials.is_empty(),
        "Expected at least one partial, got none"
    );

    assert!(
        !output.ticks.is_empty(),
        "Expected at least one tick, got none"
    );
}

#[tokio::test]
async fn stream_llm_function_partials_grow_monotonically() {
    let (_server, uri) = mock_openai_streaming(&["Hello", ", ", "world", "!"]).await;

    let output = engine::run_test_streaming(
        &streaming_llm_source(&uri),
        "main",
        IndexMap::new(),
        OptLevel::One,
    )
    .await;

    assert!(output.result.is_ok());

    // Each partial should be longer than or equal to the previous one.
    for window in output.partials.windows(2) {
        assert!(
            window[1].len() >= window[0].len(),
            "Partials should grow monotonically: {:?} -> {:?}",
            window[0],
            window[1]
        );
    }

    // The last partial should be the full content.
    if let Some(last) = output.partials.last() {
        assert_eq!(last, "Hello, world!", "Last partial should be full content");
    }
}

#[tokio::test]
async fn stream_llm_function_server_error_returns_failure() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
        .mount(&server)
        .await;
    let uri = server.uri();

    let output = engine::run_test(
        &streaming_llm_source(&uri),
        "main",
        IndexMap::new(),
        OptLevel::One,
    )
    .await;

    assert!(
        output.result.is_err(),
        "Expected error for streaming with 500 response"
    );
}
