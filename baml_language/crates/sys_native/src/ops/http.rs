//! Shared HTTP helpers used by the `SysOpHttp` trait impl.

#[cfg(feature = "bundle-http")]
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::{error::Error, fmt::Write};

use bex_heap::builtin_types;
#[cfg(feature = "bundle-http")]
use bex_resource_types::ResourceHandle;
use sys_types::OpErrorKind;
#[cfg(feature = "bundle-http")]
use tokio::sync::{Mutex as TokioMutex, Notify};

#[cfg(feature = "bundle-http")]
use crate::{
    registry::{REGISTRY, SseBuffer},
    sse_parser::SseParser,
};

/// Create an HTTP client for use within the current async context.
///
/// We intentionally do NOT use a global `LazyLock<reqwest::Client>` because
/// `reqwest::Client` spawns background connection-pool tasks on the Tokio
/// runtime that is active at creation time. In tests each `#[tokio::test]`
/// creates its own runtime, so a client created on runtime A will fail with
/// "dispatch task is gone" when used on runtime B after A shuts down.
///
/// Creating a client per request is cheap (`reqwest::Client::new()` is just
/// an `Arc` allocation) and avoids the cross-runtime lifetime issue.
fn new_http_client() -> reqwest::Client {
    reqwest::Client::new()
}

/// Format an error and its full `source()` chain so the real cause is visible
/// (e.g. reqwest's top-level "error sending request" often hides the actual reason).
pub(crate) fn format_error_chain(mut err: &dyn Error) -> String {
    let mut s = err.to_string();
    while let Some(src) = err.source() {
        let _ = write!(s, "\n  Caused by: {src}");
        err = src;
    }
    s
}

/// Send an HTTP request and return a Response resource.
///
/// Shared by both `fetch` (which creates a GET request) and `send` (which takes a Request).
pub(crate) async fn send_async(
    req: builtin_types::owned::HttpRequest,
) -> Result<builtin_types::owned::HttpResponse, OpErrorKind> {
    let method = reqwest::Method::from_bytes(req.method.as_bytes())
        .map_err(|e| OpErrorKind::Other(format!("Invalid HTTP method '{}': {e}", req.method)))?;

    let client = new_http_client();
    let mut builder = client.request(method, &req.url);

    for (key, value) in &req.headers {
        builder = builder.header(key.as_str(), value.as_str());
    }

    if !req.body.is_empty() {
        builder = builder.body(req.body);
    }

    let response = match builder.send().await {
        Ok(resp) => resp,
        Err(e) => {
            // Network error: return a synthetic response with status_code=0
            // so BAML orchestration code can check ok() and fall back.
            // The error message is available via text() for debugging.
            let error_msg = format_error_chain(&e);
            let handle = REGISTRY.register_error_http_response(req.url.clone(), error_msg.clone());
            let mut headers = indexmap::IndexMap::new();
            headers.insert("x-baml-error".to_string(), error_msg);
            return Ok(builtin_types::owned::HttpResponse {
                status_code: 0,
                headers,
                url: req.url,
                _handle: handle,
            });
        }
    };

    // Capture metadata before storing
    let status = response.status().as_u16();
    let headers = response
        .headers()
        .iter()
        .map(|(k, v)| (k.as_str().to_string(), v.to_str().unwrap_or("").to_string()))
        .collect();
    let final_url = response.url().to_string();

    let handle = REGISTRY.register_http_response(response, final_url.clone());
    Ok(builtin_types::owned::HttpResponse {
        status_code: i64::from(status),
        headers,
        url: final_url,
        _handle: handle,
    })
}

#[cfg(feature = "bundle-http")]
/// Send an HTTP request and establish an SSE stream.
///
/// Spawns a background tokio task that reads the response body, parses SSE
/// events, and buffers them. The VM retrieves batched events via `sse_stream_next`.
pub(crate) async fn send_sse_async(
    req: builtin_types::owned::HttpRequest,
) -> Result<builtin_types::owned::HttpSseStream, OpErrorKind> {
    use futures::StreamExt;

    let method = reqwest::Method::from_bytes(req.method.as_bytes())
        .map_err(|e| OpErrorKind::Other(format!("Invalid HTTP method '{}': {e}", req.method)))?;

    let client = new_http_client();
    let mut builder = client.request(method, &req.url);

    for (key, value) in &req.headers {
        builder = builder.header(key.as_str(), value.as_str());
    }

    if !req.body.is_empty() {
        builder = builder.body(req.body.clone());
    }

    let response = builder.send().await.map_err(|e| {
        OpErrorKind::Other(format!("SSE connection failed: {}", format_error_chain(&e)))
    })?;

    if !response.status().is_success() {
        let status = response.status().as_u16();
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| "<could not read body>".to_string());
        return Err(OpErrorKind::Other(format!(
            "SSE request failed with status {status}: {body}"
        )));
    }

    let url = req.url.clone();

    // Create shared buffer and notify
    let buffer = Arc::new(TokioMutex::new(SseBuffer {
        events: Vec::new(),
        done: false,
        error: None,
    }));
    let closed = Arc::new(AtomicBool::new(false));
    let notify = Arc::new(Notify::new());

    // Spawn background task to consume the byte stream and parse SSE events.
    //
    // A drop guard ensures that if the task is aborted (e.g. via cancellation),
    // `done` is set to true and the notify is fired, so `sse_stream_next` callers
    // never hang waiting on a dead task.
    let buf_clone = buffer.clone();
    let closed_clone = closed.clone();
    let notify_clone = notify.clone();
    let consumer = tokio::spawn(async move {
        /// Guard that signals SSE stream completion when the task is dropped.
        ///
        /// If the task is aborted (e.g. via cancellation) before setting
        /// `completed = true`, the guard sets `done = true` on the buffer and
        /// fires the notify, preventing consumers from hanging indefinitely.
        struct SseDropGuard {
            buffer: Arc<TokioMutex<SseBuffer>>,
            closed: Arc<AtomicBool>,
            notify: Arc<Notify>,
            completed: bool,
        }

        impl Drop for SseDropGuard {
            fn drop(&mut self) {
                if !self.completed {
                    if let Ok(mut buf) = self.buffer.try_lock() {
                        if !buf.done {
                            if !self.closed.load(Ordering::Acquire) {
                                buf.error = Some("SSE stream task was cancelled".into());
                            }
                            buf.done = true;
                        }
                    }
                    self.notify.notify_waiters();
                }
            }
        }

        let mut guard = SseDropGuard {
            buffer: buf_clone.clone(),
            closed: closed_clone.clone(),
            notify: notify_clone.clone(),
            completed: false,
        };

        let mut parser = SseParser::new();
        let mut byte_stream = response.bytes_stream();

        while let Some(chunk_result) = byte_stream.next().await {
            match chunk_result {
                Ok(bytes) => {
                    let events = parser.feed(&bytes);
                    if !events.is_empty() {
                        let mut buf = buf_clone.lock().await;
                        buf.events.extend(events);
                        notify_clone.notify_waiters();
                    }
                }
                Err(e) => {
                    let mut buf = buf_clone.lock().await;
                    buf.error = Some(format!("SSE stream error: {}", format_error_chain(&e)));
                    buf.done = true;
                    notify_clone.notify_waiters();
                    guard.completed = true;
                    return;
                }
            }
        }

        // Stream ended normally
        let mut buf = buf_clone.lock().await;
        buf.done = true;
        notify_clone.notify_waiters();
        guard.completed = true;
    });

    let handle =
        REGISTRY.register_sse_stream(buffer, closed, notify, consumer.abort_handle(), url.clone());
    Ok(builtin_types::owned::HttpSseStream {
        _handle: handle,
        url,
    })
}

#[cfg(feature = "bundle-http")]
/// Retrieve the next batch of SSE events from a stream.
///
/// Returns `Ok(Some(json_array))` with buffered events as a JSON array of
/// `{"event": "...", "data": "...", "id": "..."}` objects, or `Ok(None)`
/// when the stream is complete.
pub(crate) async fn sse_stream_next(
    handle: &ResourceHandle,
) -> Result<Option<String>, OpErrorKind> {
    let (buffer, notify, closed) = REGISTRY
        .get_sse_stream(handle.key())
        .ok_or_else(|| OpErrorKind::Other("SSE stream handle is invalid".into()))?;

    loop {
        let notified = notify.notified();
        {
            let mut buf = buffer.lock().await;
            if closed.load(Ordering::Acquire) {
                buf.done = true;
                buf.error = None;
                return Ok(None);
            }
            if !buf.events.is_empty() {
                let events: Vec<serde_json::Value> = std::mem::take(&mut buf.events)
                    .into_iter()
                    .map(|e| {
                        serde_json::json!({
                            "event": e.event,
                            "data": e.data,
                            "id": e.id,
                        })
                    })
                    .collect();
                return Ok(Some(serde_json::to_string(&events).map_err(|e| {
                    OpErrorKind::Other(format!("Failed to serialize SSE events: {e}"))
                })?));
            }
            if let Some(err) = buf.error.take() {
                return Err(OpErrorKind::Other(err));
            }
            if buf.done {
                return Ok(None);
            }
        }
        notified.await;
    }
}
