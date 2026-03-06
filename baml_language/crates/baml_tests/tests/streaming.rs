//! Tests for BEP-009 streaming primitives and full streaming orchestration.

use std::time::Duration;

use baml_tests::engine::{self, IndexMap, OptLevel};
use bex_engine::BexExternalValue;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    sync::oneshot,
};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path},
};

// ============================================================================
// Helpers
// ============================================================================

/// Build an OpenAI-style SSE body with a configurable finish reason.
fn openai_sse_chunks_with_finish_reason(chunks: &[&str], finish_reason: &str) -> Vec<String> {
    let mut body_chunks: Vec<String> = chunks
        .iter()
        .map(|chunk| {
            format!(
                "event: message\ndata: {{\"choices\":[{{\"delta\":{{\"content\":\"{chunk}\"}}}}]}}\n\n"
            )
        })
        .collect();
    body_chunks.push(format!(
        "event: message\ndata: {{\"choices\":[{{\"delta\":{{}},\"finish_reason\":\"{finish_reason}\"}}]}}\n\n"
    ));
    body_chunks.push("event: message\ndata: [DONE]\n\n".to_string());
    body_chunks
}

/// Build an OpenAI-style SSE body with a configurable finish reason.
fn openai_sse_body_with_finish_reason(chunks: &[&str], finish_reason: &str) -> String {
    openai_sse_chunks_with_finish_reason(chunks, finish_reason).concat()
}

/// Start a mock server that serves an OpenAI-style SSE response with a custom finish reason.
async fn mock_openai_streaming_with_finish_reason(
    chunks: &[&str],
    finish_reason: &str,
) -> (MockServer, String) {
    let server = MockServer::start().await;
    let body = openai_sse_body_with_finish_reason(chunks, finish_reason);
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

/// Read a full HTTP request, including the request body if Content-Length is set.
async fn read_http_request(socket: &mut tokio::net::TcpStream) -> String {
    let mut request = Vec::new();
    let mut buf = [0_u8; 1024];
    let mut headers_end = None;
    let mut content_length = 0_usize;

    loop {
        let read = socket.read(&mut buf).await.unwrap();
        if read == 0 {
            return String::from_utf8_lossy(&request).into_owned();
        }
        request.extend_from_slice(&buf[..read]);

        if headers_end.is_none() {
            headers_end = request
                .windows(4)
                .position(|window| window == b"\r\n\r\n")
                .map(|idx| idx + 4);

            if let Some(end) = headers_end {
                let headers = String::from_utf8_lossy(&request[..end]);
                content_length = headers
                    .lines()
                    .find_map(|line| {
                        let (name, value) = line.split_once(':')?;
                        if name.eq_ignore_ascii_case("content-length") {
                            value.trim().parse().ok()
                        } else {
                            None
                        }
                    })
                    .unwrap_or(0);
            }
        }

        if let Some(end) = headers_end
            && request.len() >= end + content_length
        {
            return String::from_utf8_lossy(&request).into_owned();
        }
    }
}

/// Start a TCP SSE server that records the incoming request and sends delayed writes.
async fn observed_sse_server(
    writes: Vec<(u64, String)>,
) -> (
    tokio::task::JoinHandle<()>,
    String,
    oneshot::Receiver<String>,
) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap().to_string();
    let (request_tx, request_rx) = oneshot::channel();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let request = read_http_request(&mut socket).await;
        let _ = request_tx.send(request);
        socket
            .write_all(
                b"HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\ncache-control: no-cache\r\n\r\n",
            )
            .await
            .unwrap();
        socket.flush().await.unwrap();

        for (delay_ms, chunk) in writes {
            tokio::time::sleep(Duration::from_millis(delay_ms)).await;
            socket.write_all(chunk.as_bytes()).await.unwrap();
            socket.flush().await.unwrap();
        }

        socket.shutdown().await.unwrap();
    });

    (server, format!("http://{addr}"), request_rx)
}

/// Start a TCP SSE server that sends chunks with delays between writes.
async fn delayed_sse_server(writes: Vec<(u64, String)>) -> (tokio::task::JoinHandle<()>, String) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap().to_string();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let _ = read_http_request(&mut socket).await;
        socket
            .write_all(
                b"HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\ncache-control: no-cache\r\n\r\n",
            )
            .await
            .unwrap();
        socket.flush().await.unwrap();

        for (delay_ms, chunk) in writes {
            tokio::time::sleep(Duration::from_millis(delay_ms)).await;
            socket.write_all(chunk.as_bytes()).await.unwrap();
            socket.flush().await.unwrap();
        }

        socket.shutdown().await.unwrap();
    });

    (server, format!("http://{addr}"))
}

/// Start an OpenAI-style SSE server that records the request and streams chunks incrementally.
async fn delayed_openai_streaming_server(
    chunks: &[&str],
    finish_reason: &str,
    inter_chunk_delay_ms: u64,
) -> (
    tokio::task::JoinHandle<()>,
    String,
    oneshot::Receiver<String>,
) {
    let writes = openai_sse_chunks_with_finish_reason(chunks, finish_reason)
        .into_iter()
        .enumerate()
        .map(|(idx, chunk)| {
            let delay_ms = if idx == 0 { 0 } else { inter_chunk_delay_ms };
            (delay_ms, chunk)
        })
        .collect();
    observed_sse_server(writes).await
}

/// Start a TCP SSE server and report when the client disconnects after close().
async fn close_observing_sse_server() -> (tokio::task::JoinHandle<()>, String, oneshot::Receiver<()>)
{
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap().to_string();
    let (closed_tx, closed_rx) = oneshot::channel();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let _ = read_http_request(&mut socket).await;
        socket
            .write_all(
                b"HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\ncache-control: no-cache\r\n\r\n",
            )
            .await
            .unwrap();
        socket
            .write_all(b"event: message\ndata: hello\n\n")
            .await
            .unwrap();
        socket.flush().await.unwrap();

        let mut buf = [0_u8; 32];
        loop {
            let read = socket.read(&mut buf).await.unwrap();
            if read == 0 {
                let _ = closed_tx.send(());
                break;
            }
        }
    });

    (server, format!("http://{addr}"), closed_rx)
}

/// Build BAML source for a streaming LLM test with a mock server URL.
fn streaming_llm_source(base_url: &str) -> String {
    streaming_llm_source_with_options(base_url, "")
}

/// Build BAML source for a streaming LLM test with extra client options.
fn streaming_llm_source_with_options(base_url: &str, extra_options: &str) -> String {
    format!(
        r##"
        client<llm> TestClient {{
            provider openai
            options {{
                model "gpt-4o"
                api_key "test-key"
                base_url "{base_url}"
                {extra_options}
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
            function main() -> string {{
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
                if (first == null) {{ return "first-null"; }}
                if (second != null) {{ return "second-non-null"; }}
                first
            }}
        "#
        ),
        "main",
        IndexMap::new(),
        OptLevel::One,
    )
    .await;

    let result = output.result.expect("should succeed");
    let BexExternalValue::String(events) = result else {
        panic!("expected string, got {result:?}");
    };
    assert!(
        events.contains("\"data\":\"only-one\""),
        "first batch should contain the only event: {events}"
    );
}

#[tokio::test]
async fn sse_close_terminates_the_connection() {
    let (server, uri, closed_rx) = close_observing_sse_server().await;

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
                let events = sse.next();
                sse.close();
                events != null
            }}
        "#
        ),
        "main",
        IndexMap::new(),
        OptLevel::One,
    )
    .await;

    assert_eq!(output.result, Ok(BexExternalValue::Bool(true)));
    tokio::time::timeout(Duration::from_millis(500), closed_rx)
        .await
        .expect("client should close the SSE connection promptly")
        .expect("close observer should receive disconnect notification");
    server.await.unwrap();
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
    let (server, uri) = delayed_sse_server(vec![
        (0, "event: message\ndata: a\n\n".to_string()),
        (50, "event: message\ndata: b\n\n".to_string()),
        (50, "event: message\ndata: c\n\n".to_string()),
    ])
    .await;

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
                let collected = "";
                while (true) {{
                    let events = sse.next();
                    if (events == null) {{ break; }}
                    collected = collected + events;
                }}
                sse.close();
                collected
            }}
        "#
        ),
        "main",
        IndexMap::new(),
        OptLevel::One,
    )
    .await;

    let result = output.result.expect("should succeed");
    let BexExternalValue::String(collected) = result else {
        panic!("expected string, got {result:?}");
    };
    assert!(
        collected.contains("\"data\":\"a\""),
        "missing first event: {collected}"
    );
    assert!(
        collected.contains("\"data\":\"b\""),
        "missing second event: {collected}"
    );
    assert!(
        collected.contains("\"data\":\"c\""),
        "missing third event: {collected}"
    );

    server.await.unwrap();
}

// ============================================================================
// Full Streaming Orchestration Tests
// ============================================================================

#[tokio::test]
async fn stream_llm_function_openai_returns_final_value() {
    let (server, uri, request_rx) =
        delayed_openai_streaming_server(&["Hello", ", ", "world", "!"], "stop", 25).await;

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

    let request = request_rx
        .await
        .expect("server should capture the outbound request");
    assert!(
        request.contains("\"stream\":true") || request.contains("\"stream\": true"),
        "streaming request body should include stream=true: {request}"
    );

    server.await.unwrap();
}

#[tokio::test]
async fn stream_llm_function_emits_partials() {
    let (server, uri, _request_rx) =
        delayed_openai_streaming_server(&["Hello", ", ", "world", "!"], "stop", 50).await;

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
        output.partials.len() >= 2,
        "Expected multiple incremental partials, got {:?}",
        output.partials
    );
    assert!(
        output.partials.iter().any(|partial| partial == "Hello"),
        "Expected an early partial before EOF, got {:?}",
        output.partials
    );
    assert_eq!(
        output.partials.last().map(String::as_str),
        Some("Hello, world!"),
        "Last partial should match the final value"
    );

    assert!(
        output.ticks.len() >= 2,
        "Expected multiple incremental tick batches, got {:?}",
        output.ticks
    );

    server.await.unwrap();
}

#[tokio::test]
async fn stream_llm_function_partials_grow_monotonically() {
    let (server, uri, _request_rx) =
        delayed_openai_streaming_server(&["Hello", ", ", "world", "!"], "stop", 50).await;

    let output = engine::run_test_streaming(
        &streaming_llm_source(&uri),
        "main",
        IndexMap::new(),
        OptLevel::One,
    )
    .await;

    assert!(output.result.is_ok());
    assert!(
        output.partials.len() >= 2,
        "Expected more than one partial, got {:?}",
        output.partials
    );
    assert_ne!(
        output.partials.first(),
        output.partials.last(),
        "Expected at least one intermediate partial before the final value"
    );

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

    server.await.unwrap();
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

#[tokio::test]
async fn stream_llm_function_deduplicates_finish_only_batches() {
    let (server, uri) = delayed_sse_server(vec![
        (
            0,
            "event: message\ndata: {\"choices\":[{\"delta\":{\"content\":\"Hello\"}}]}\n\n"
                .to_string(),
        ),
        (
            50,
            "event: message\ndata: {\"choices\":[{\"delta\":{},\"finish_reason\":\"stop\"}]}\n\n"
                .to_string(),
        ),
        (50, "event: message\ndata: [DONE]\n\n".to_string()),
    ])
    .await;

    let output = engine::run_test_streaming(
        &streaming_llm_source(&uri),
        "main",
        IndexMap::new(),
        OptLevel::One,
    )
    .await;

    assert_eq!(
        output.result,
        Ok(BexExternalValue::String("Hello".to_string()))
    );
    assert_eq!(
        output.partials,
        vec!["Hello".to_string()],
        "finish-only batches should not re-emit the same partial"
    );

    server.await.unwrap();
}

#[tokio::test]
async fn stream_llm_function_respects_finish_reason_filters() {
    let (_server, uri) = mock_openai_streaming_with_finish_reason(&["truncated"], "length").await;

    let output = engine::run_test(
        &streaming_llm_source_with_options(&uri, r#"finish_reason_deny_list ["length"]"#),
        "main",
        IndexMap::new(),
        OptLevel::One,
    )
    .await;

    let err = output
        .result
        .expect_err("streaming should reject a denied finish reason")
        .to_string();
    assert!(
        err.contains("Finish reason not allowed"),
        "unexpected error: {err}"
    );
}
