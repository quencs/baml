### Buffered, non-blocking LLM streaming with latest-first parsing

#### Problem statement

- Current streaming in `engine/baml-runtime/src/internal/llm_client/primitive/stream_request.rs` builds a `reqwest` SSE stream and parses each event inline using a `.scan` adapter. Provider-specific scanners (e.g., `scan_openai_chat_completion_stream`, `scan_openai_responses_stream`) mutate an accumulated `LLMCompleteResponse` per event.
- Because parsing happens inline with the SSE stream, any slow parsing (schema parsing downstream, heavy validations, or transient delays) can stall event consumption. If the parser falls behind, we risk buffering at the transport layer or missing the opportunity to present the freshest state to callers.
- We want to buffer inbound SSE events and decouple ingestion from parsing, so that:
  - Ingestion never blocks on parsing.
  - The parser always processes the latest available event(s) when it wakes up. If multiple events arrived while parsing, it quickly catches up by applying them in a batch and emitting the up-to-date state.

#### Goals

- Preserve existing public stream API (`StreamResponse` as `Stream<Item = LLMResponse>`).
- Keep provider scanning logic (e.g., `scan_openai_*`) unchanged where possible.
- Avoid data loss. For delta-style providers (OpenAI Chat Completions), do not drop deltas; instead, drain and apply all pending deltas in one batch update.
- Emit fewer, more meaningful stream updates when the parser is behind (batch drain), while still producing the final complete response.
- Preserve existing tracing of SSE events.

#### Non-goals

- This proposal does not change how downstream typed parsing works in other layers. It provides hooks so that downstream parsers can easily adopt latest-first semantics (e.g., via a watch channel), but those changes are optional and out of scope here.

---

### Current architecture (simplified)

- `make_stream_request` uses:
  - `resp.bytes_stream().eventsource()`
  - `.take_while(...)` to stop on `"[DONE]"`
  - `.map(...)` to parse `event.data` JSON
  - `.scan(...)` to mutate an `LLMCompleteResponse` by calling `scan_*` per provider
  - emits `LLMResponse::Success(accumulated.clone())` each event

This ties ingestion and parsing together in a single async stream pipeline.

---

### Proposed architecture

Two-stage pipeline with explicit buffering between ingestion and parsing:

1) Ingestion task
   - Reads SSE events as fast as possible.
   - Logs each SSE chunk to `BAML_TRACER` (unchanged behavior).
   - Parses `event.data` into `serde_json::Value` and sends into an unbounded channel (never blocks ingestion).
   - Closes the channel when a terminal marker is seen (`"[DONE]"` or stream error).

2) Parser/assembler task
   - Maintains the existing `accumulated: Result<LLMCompleteResponse>` state.
   - Waits for at least one event from the channel, then immediately drains all ready events (if any) into a batch.
   - Applies provider scanners (`scan_*`) to each event in the drained batch, updating `accumulated` in-order, then emits a single `LLMResponse::Success(accumulated.clone())` reflecting the latest state.
   - On scanner error, emits `LLMResponse::LLMFailure` and terminates the stream.

This ensures we never drop delta content (we still apply all events), but we minimize update chatter by batching when the parser was temporarily slow. It also ensures ingestion of SSE is never blocked by parsing work.

---

### Key changes in `make_stream_request`

Below is illustrative Rust showing the structure. It preserves provider scanning and tracing; only the orchestration changes. File: `engine/baml-runtime/src/internal/llm_client/primitive/stream_request.rs`.

```rust
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;
use futures::StreamExt;

pub async fn make_stream_request( /* ... */ ) -> StreamResponse {
    let (start_time_system, start_time_instant, built_req) =
        build_and_log_outbound_request(client, prompt, true, true, runtime_context).await?;

    let resp = match execute_request(
        client,
        built_req,
        prompt,
        start_time_system,
        start_time_instant,
        runtime_context,
        false,
    ).await? {
        (EitherResponse::Raw(resp), _, _) => Ok(resp),
        _ => unreachable!("streaming mode never consumes body"),
    }?;

    // Channels: raw JSON events from SSE, and assembled LLMResponse to callers
    let (raw_tx, mut raw_rx) = mpsc::unbounded_channel::<serde_json::Value>();
    let (out_tx, out_rx) = mpsc::unbounded_channel::<LLMResponse>();

    // For tracing
    let call_id_stack = runtime_context.runtime_context().call_id_stack.clone();
    let http_request_id = std::sync::Arc::new(runtime_context.http_request_id().clone());

    // Copy provider context for parser
    let client_name = client.context().name.clone();
    let params = client.request_options().clone();
    let prompt = to_prompt(prompt);
    let model_name = model_name.clone();
    let response_type = response_type.clone();

    // Ingestion task: never blocks on parsing
    tokio::spawn({
        let raw_tx = raw_tx.clone();
        async move {
            let mut sse = resp.bytes_stream().eventsource();
            while let Some(evt) = sse.next().await {
                match evt {
                    Ok(event) => {
                        // Trace every SSE event
                        let trace_event = TraceEvent::new_raw_llm_response_stream(
                            call_id_stack.clone(),
                            std::sync::Arc::new(HTTPResponseStream::new(
                                http_request_id.deref().clone(),
                                SSEEvent::new(event.event.clone(), event.data.clone(), event.id.clone()),
                            )),
                        );
                        BAML_TRACER.lock().unwrap().put(std::sync::Arc::new(trace_event));

                        if event.data == "[DONE]" { break; }
                        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&event.data) {
                            let _ = raw_tx.send(val); // unbounded, never back-pressures
                        } else {
                            // Optionally: forward a parse error to out_tx
                        }
                    }
                    Err(err) => {
                        let _ = out_tx.send(LLMResponse::LLMFailure(LLMErrorResponse {
                            client: client_name.clone(),
                            model: model_name.clone(),
                            prompt: prompt.clone(),
                            start_time: start_time_system,
                            request_options: params.clone(),
                            latency: start_time_instant.elapsed(),
                            message: format!("SSE error: {err:#?}"),
                            code: ErrorCode::Other(2),
                        }));
                        break;
                    }
                }
            }
            // Close raw channel
            drop(raw_tx);
        }
    });

    // Parser/assembler task: drain backlog and emit only the latest state per batch
    tokio::spawn(async move {
        let mut accumulated: Result<LLMCompleteResponse, LLMErrorResponse> = Ok(LLMCompleteResponse {
            client: client_name.clone(),
            prompt: prompt.clone(),
            content: String::new(),
            start_time: start_time_system,
            latency: start_time_instant.elapsed(),
            model: model_name.clone().unwrap_or_else(|| "<unknown>".into()),
            request_options: params.clone(),
            metadata: LLMCompleteResponseMetadata {
                baml_is_complete: false,
                finish_reason: None,
                prompt_tokens: None,
                output_tokens: None,
                total_tokens: None,
                cached_input_tokens: None,
            },
        });

        while let Some(first) = raw_rx.recv().await {
            // Drain any backlog so we operate on the freshest view
            let mut batch = vec![first];
            while let Ok(next) = raw_rx.try_recv() { batch.push(next); }

            // Apply provider scanners on the drained batch; emit one update
            for event_body in batch {
                let res = match response_type {
                    ResponseType::OpenAI => scan_openai_chat_completion_stream(
                        &client_name, &params, &prompt, &start_time_system, &start_time_instant,
                        &model_name, &mut accumulated, event_body,
                    ),
                    ResponseType::OpenAIResponses => scan_openai_responses_stream(
                        &client_name, &params, &prompt, &start_time_system, &start_time_instant,
                        &model_name, &mut accumulated, event_body,
                    ),
                    ResponseType::Anthropic => scan_anthropic_response_stream(
                        &client_name, &params, &prompt, &start_time_system, &start_time_instant,
                        &model_name, &mut accumulated, event_body,
                    ),
                    ResponseType::Google => scan_google_response_stream(
                        &client_name, &params, &prompt, &start_time_system, &start_time_instant,
                        &model_name, &mut accumulated, event_body,
                    ),
                    ResponseType::Vertex => scan_vertex_response_stream(
                        &client_name, &params, &prompt, &start_time_system, &start_time_instant,
                        &model_name, &mut accumulated, event_body,
                    ),
                };

                if let Err(e) = res {
                    let _ = out_tx.send(LLMResponse::LLMFailure(e));
                    return;
                }
            }

            // Emit only the latest accumulated state after applying the batch
            match &accumulated {
                Ok(v) => { let _ = out_tx.send(LLMResponse::Success(v.clone())); }
                Err(_) => { /* already emitted failure above */ return; }
            }
        }

        // raw_rx closed -> end of stream
        // Optionally, emit a final Success if not already completed
    });

    Ok(Box::pin(UnboundedReceiverStream::new(out_rx)))
}
```

Notes:
- Provider scanners remain unchanged and keep correctness for delta-based providers by applying every drained event in-order. We only reduce emission frequency, not the number of events applied.
- Ingestion uses unbounded channel to ensure no backpressure from the parser. We can later make this bounded with a ring buffer, but that would require provider-aware coalescing on the ingestion side to guarantee correctness for delta protocols.

---

### Optional: provider-aware coalescing helper (if we later bound the buffer)

If we adopt a bounded buffer, we can add a coalescer that merges multiple OpenAI deltas into a single synthetic delta before scanning. This preserves content integrity even if we must collapse many events into one parse step.

```rust
fn coalesce_openai_chat_deltas(batch: &[serde_json::Value]) -> serde_json::Value {
    use serde_json::json;

    let mut content = String::new();
    let mut model: Option<String> = None;
    let mut finish_reason: Option<String> = None;
    let mut usage: Option<serde_json::Value> = None;

    for evt in batch {
        if let Some(m) = evt.get("model").and_then(|m| m.as_str()) {
            model = Some(m.to_string());
        }
        if let Some(choice) = evt.get("choices").and_then(|c| c.get(0)) {
            if let Some(delta) = choice.get("delta") {
                if let Some(part) = delta.get("content").and_then(|s| s.as_str()) {
                    content.push_str(part);
                }
            }
            if let Some(fr) = choice.get("finish_reason").and_then(|s| s.as_str()) {
                finish_reason = Some(fr.to_string());
            }
        }
        if let Some(u) = evt.get("usage").cloned() { usage = Some(u); }
    }

    json!({
        "object": "chat.completion.chunk",
        "model": model,
        "choices": [{ "delta": { "content": content }, "finish_reason": finish_reason }],
        "usage": usage,
    })
}
```

With a coalescer, the parser can call the existing `scan_openai_chat_completion_stream` once per batch instead of iterating each event. This is an optimization, not required for correctness in the unbounded-buffer baseline.

---

### Tracing behavior

- We keep tracing at ingestion time by creating `TraceEvent::new_raw_llm_response_stream` for every SSE chunk, exactly as today.
- Stream response logging for final HTTP body is unchanged (this only affects streaming/sse path).

---

### Compatibility and behavior changes

- The external `StreamResponse` type remains the same. Callers will still receive a stream of `LLMResponse` values.
- When the parser is fast, emissions will look the same as today (roughly one update per SSE event).
- When the parser is slow, the new pipeline will send fewer updates, each representing the latest fully-applied state after draining the backlog.
- Final response semantics and metadata are unchanged (e.g., finish reason, usage fields).

---

### Testing plan

- Unit test: ensure a series of N OpenAI delta events yields content exactly equal to the concatenation across events when processed via the new two-stage pipeline.
- Unit test: inject artificial delay in parsing stage (e.g., sleep) to create backlog; assert that
  - the ingestion loop continues (no deadlock),
  - the parser drains multiple events and emits one up-to-date `LLMResponse` per drain,
  - the final accumulated content matches the sequential application of all deltas.
- Unit test: for Responses API streaming, verify that `ResponseCompleted` overwrites prior deltas as in current code, and metadata is preserved.
- Run `cargo test --lib` in `engine/` after implementation per repository rules.

---

### Migration plan (incremental)

1) Refactor `make_stream_request` to the two-stage pipeline shown above.
2) Keep provider scanners (`scan_openai_*`, `scan_anthropic_*`, etc.) unchanged.
3) Validate tests and adjust only where tests asserted exact per-chunk emission counts (behaviorally benign to now receive fewer updates when under load).
4) Optional follow-ups:
   - Introduce a configurable emission policy (e.g., time-based throttle like 20–50ms) to further smooth updates.
   - Add a `watch` channel for downstream typed parsers who want latest-first parsing without queue churn.
   - If needed, implement provider-aware coalescers and move to a bounded buffer.

---

### Why this meets the requirements

- Ingestion is fully decoupled from parsing via an unbounded channel, so we do not block on parsing.
- The parser always operates on the latest available state by draining the queue and applying all pending events in one step before emitting, ensuring consumers see the freshest snapshot even when the parser is under load.
- We maintain correctness for delta-style protocols by applying all events in-order; we do not lose content.

