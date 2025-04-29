use anyhow::{Context, Result};
use baml_rpc::{
    ApiEndpoint, CreateTraceEventUploadUrl, CreateTraceEventUploadUrlRequest,
    CreateTraceEventUploadUrlResponse, S3UploadMetadata, TraceEventBatch,
};
use baml_types::tracing::events::{TraceData, TraceEvent};
use baml_types::{BamlValueWithMeta, HasFieldType};
use core::time::Duration;
use futures::StreamExt;
use http::{HeaderMap, HeaderName, HeaderValue};
use once_cell::sync::OnceCell;
use serde::Serialize;
use std::any::type_name;
use std::borrow::Cow;
use std::sync::Arc;
use tokio::sync::mpsc;
#[cfg(not(target_family = "wasm"))]
use tokio::time::*;

#[cfg(target_family = "wasm")]
use wasmtimer::tokio::*;

use crate::runtime::{AstSignatureWrapper, InternalBamlRuntime};
use crate::tracingv2::storage::interface::TraceEventWithMeta;

use super::rpc_converters::{to_rpc_event, TypeLookup};

enum PublisherMessage {
    Trace(Arc<TraceEventWithMeta>),
    Flush(tokio::sync::oneshot::Sender<()>),
    UpdateRuntime(Arc<RuntimeAST>),
    Shutdown(tokio::sync::oneshot::Sender<()>),
}

/// Global publisher channel.
/// When the module is first used, we create an unbounded channel and then spawn the publisher task.
static PUBLISHING_CHANNEL: OnceCell<mpsc::UnboundedSender<PublisherMessage>> = OnceCell::new();
static PUBLISHING_TASK: OnceCell<Arc<tokio::task::JoinHandle<()>>> = OnceCell::new();

fn get_publish_channel(
    allow_missing: bool,
) -> Option<&'static mpsc::UnboundedSender<PublisherMessage>> {
    let Some(join_handle) = PUBLISHING_TASK.get() else {
        if !allow_missing {
            baml_log::fatal_once!(
                "Tracing publisher not started. Report this bug to the BAML team."
            );
        }
        return None;
    };
    if join_handle.is_finished() {
        baml_log::fatal_once!(
            "Tracing publisher ended unexpectedly. Report this bug to the BAML team."
        );
        return None;
    }
    let channel = PUBLISHING_CHANNEL.get();
    channel
}

#[derive(Serialize)]
struct RuntimeAST {
    ast: Arc<AstSignatureWrapper>,
}

impl RuntimeAST {
    pub fn base_url(&self) -> String {
        self.ast
            .env_var("BOUNDARY_API_URL")
            .cloned()
            .unwrap_or_else(|| "https://api.boundaryml.com".to_string())
    }

    pub fn api_key(&self) -> String {
        self.ast
            .env_var("BOUNDARY_API_KEY")
            .cloned()
            .unwrap_or_else(|| "".to_string())
    }
}

impl TypeLookup for RuntimeAST {
    fn type_lookup(
        &self,
        name: &str,
    ) -> Option<Arc<baml_rpc::ast::types::type_definition::TypeId>> {
        self.ast.type_lookup(name)
    }

    fn function_lookup(&self, name: &str) -> Option<Arc<baml_rpc::ast::tops::BamlFunctionId>> {
        self.ast.function_lookup(name)
    }
}

pub fn start_publisher(lookup: Arc<AstSignatureWrapper>, rt: Arc<tokio::runtime::Runtime>) {
    let lookup = Arc::new(RuntimeAST { ast: lookup });

    // Use get_or_init to ensure thread-safe initialization
    let channel = PUBLISHING_CHANNEL.get_or_init(|| {
        let (tx, rx) = mpsc::unbounded_channel::<PublisherMessage>();
        let mut publisher = TracePublisher::new(rx, lookup.clone());

        #[cfg(not(target_arch = "wasm32"))]
        {
            let handle = rt.spawn(async move { publisher.run().await });
            PUBLISHING_TASK.get_or_init(|| Arc::new(handle));
        }

        #[cfg(target_arch = "wasm32")]
        wasm_bindgen_futures::spawn_local(async move {
            publisher.run().await;
        });

        tx
    });

    // Update runtime if channel already existed
    let _ = rt.block_on(flush());
    let _ = channel.send(PublisherMessage::UpdateRuntime(lookup));
}

/// Gracefully shutdown the TracePublisher.
/// 1. Sends a Shutdown message and waits for its ack.
/// 2. Awaits the background task's JoinHandle so Drop runs.
pub async fn shutdown_publisher() -> anyhow::Result<()> {
    // 1. send Shutdown
    let Some(channel) = get_publish_channel(true) else {
        return Ok(());
    };
    let (ack_tx, ack_rx) = tokio::sync::oneshot::channel();
    channel
        .send(PublisherMessage::Shutdown(ack_tx))
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;

    // 2. wait for the ack (so we flush remaining events)
    ack_rx
        .await
        .map_err(|e| anyhow::anyhow!("shutdown ack failed: {}", e))?;

    Ok(())
}

struct TracePublisher {
    batch_size: usize,
    rx: mpsc::UnboundedReceiver<PublisherMessage>,
    lookup: Arc<RuntimeAST>,
}

impl TracePublisher {
    pub fn new(rx: mpsc::UnboundedReceiver<PublisherMessage>, lookup: Arc<RuntimeAST>) -> Self {
        Self {
            rx,
            batch_size: 10,
            lookup,
        }
    }

    /// Runs the publisher loop.
    ///
    /// The loop collects incoming events until a batch condition is reached, a timer expires,
    /// or a flush command is received.
    pub async fn run(&mut self) {
        let mut buffer: Vec<Arc<TraceEventWithMeta>> = Vec::new();
        let mut tick_interval = interval(Duration::from_secs(2));

        loop {
            tokio::select! {
                // Process any incoming command or event.
                Some(message) = self.rx.recv() => {
                    match message {
                        PublisherMessage::UpdateRuntime(lookup) => {
                            // Empty the buffer
                            self.process_batch(std::mem::take(&mut buffer)).await;
                            // lookup.
                            let base_url = lookup.base_url();
                            let api_key = lookup.api_key();
                            let url = format!("{}/v1/baml-traces", base_url);
                            let body = serde_json::to_string(&lookup).unwrap();
                            log::info!("Updating runtime with lookup: {}", body);
                            let client = reqwest::Client::new();

                            if let Ok(response) = client.post(url.clone()).bearer_auth(api_key).body(body).send().await {
                                if response.status().is_success() {
                                    log::info!("Uploaded trace events to {}", url);
                                } else {
                                    log::error!("Failed to upload trace events to {}", url);
                                }
                            } else {
                                log::error!("Failed to send request to {}", url);
                            }

                            // Update the lookup
                            self.lookup = lookup;
                        },
                        PublisherMessage::Trace(event) => {
                            buffer.push(event);
                            if buffer.len() >= self.batch_size {
                                self.process_batch(std::mem::take(&mut buffer)).await;
                            }
                        },
                        PublisherMessage::Flush(flush_ack) => {
                            // Flush the current buffer if it has any pending events.
                            if !buffer.is_empty() {
                                self.process_batch(std::mem::take(&mut buffer)).await;
                            }
                            // Signal flush completion.
                            let _ = flush_ack.send(());
                        },
                        PublisherMessage::Shutdown(shutdown_ack) => {
                            if !buffer.is_empty() {
                                self.process_batch(std::mem::take(&mut buffer)).await;
                            }
                            let _ = shutdown_ack.send(());
                            break;
                        }
                    }
                }
                // Periodic flush of pending events.
                _ = tick_interval.tick() => {
                    if !buffer.is_empty() {
                        self.process_batch(std::mem::take(&mut buffer)).await;
                    }
                }
            }
        }
    }

    async fn process_batch(&self, batch: Vec<Arc<TraceEventWithMeta>>) {
        let batch_result = self.process_batch_impl(batch).await;
        if let Err(e) = batch_result {
            tracing::error!("Failed to upload trace events: {}", e);
        }
    }

    /// Process a batch of events.
    ///
    /// In this example we:
    ///   1. Serialize the events into JSON.
    ///   2. Append the JSON to a file (using async file I/O on macOS).
    ///   3. Post the JSON to an HTTP API with up to 3 retries.
    async fn process_batch_impl(&self, batch: Vec<Arc<TraceEventWithMeta>>) -> Result<()> {
        // Assemble the upload request structure.
        let trace_event_batch = TraceEventBatch {
            events: batch
                .iter()
                .map(|e| to_rpc_event(e, self.lookup.as_ref()))
                .collect(),
        };

        tracing::info!(
            message = "Trying to upload trace events",
            batch_size = batch.len()
        );

        // Serialize to JSON.
        #[cfg(not(target_arch = "wasm32"))]
        {
            use tokio::fs::OpenOptions;
            if let Ok(mut file) = OpenOptions::new()
                .create(true)
                .append(true)
                .open("/tmp/trace_events.json")
                .await
            {
                for e in trace_event_batch.events.iter() {
                    if let Ok(json) = serde_json::to_string(e) {
                        use tokio::io::AsyncWriteExt;
                        if let Err(e) = file.write_all(format!("{}\n", json).as_bytes()).await {
                            log::error!("Failed to write to trace file: {}", e);
                        }
                    }
                }
            }
        }

        // Upload via HTTP with retry logic.
        // TODO watch out with time crate
        let client = reqwest::Client::new();
        let response = client
            .post(format!(
                // "https://abe8c5ez29.execute-api.us-east-1.amazonaws.com/{}",
                "https://o2em3sulde.execute-api.us-east-1.amazonaws.com/{}",
                CreateTraceEventUploadUrl::path()
            ))
            .json(&CreateTraceEventUploadUrlRequest {})
            .send()
            .await
            .context(format!(
                "Failed to send {}",
                type_name::<CreateTraceEventUploadUrlRequest>(),
            ))?;
        let upload_url_details: CreateTraceEventUploadUrlResponse =
            response.json().await.context(format!(
                "Failed to parse {}",
                type_name::<CreateTraceEventUploadUrlResponse>(),
            ))?;

        client
            .put(upload_url_details.upload_url)
            .json(&trace_event_batch)
            .headers(
                upload_url_details
                    .upload_metadata
                    .as_reqwest_headers()
                    .context(format!(
                        "Failed to convert {} to HeaderMap",
                        type_name::<S3UploadMetadata>(),
                    ))?,
            )
            .send()
            .await
            .context("Failed to upload trace events to S3")?;

        Ok(())
    }
}

trait AsReqwestHeaders {
    fn as_reqwest_headers(&self) -> Result<HeaderMap>;
}

impl AsReqwestHeaders for S3UploadMetadata {
    fn as_reqwest_headers(&self) -> Result<HeaderMap> {
        let as_map = serde_json::to_value(self).expect("Failed to serialize S3UploadMetadata");
        as_map
            .as_object()
            .expect("Failed to convert S3UploadMetadata to object")
            .iter()
            .map(|(k, v)| {
                Ok((
                    HeaderName::from_bytes(format!("x-amz-meta-{}", k).as_bytes())?,
                    HeaderValue::from_str(v.as_str().unwrap())?,
                ))
            })
            .collect::<Result<HeaderMap>>()
    }
}
pub fn publish_trace_event(event: Arc<TraceEventWithMeta>) -> anyhow::Result<()> {
    let Some(channel) = get_publish_channel(false) else {
        return Ok(());
    };
    channel
        .send(PublisherMessage::Trace(event))
        .map_err(|e| e.into())
}

// Note, the library we are using doesnt seem to work well for flushing in Node
// but that's ok since noone uses our wasm build in node for logging.
// https://github.com/whizsid/wasmtimer-rs/issues/26
pub async fn flush() -> anyhow::Result<()> {
    let Some(channel) = get_publish_channel(false) else {
        return Ok(());
    };
    let (ack_tx, ack_rx) = tokio::sync::oneshot::channel();
    if let Err(e) = channel.send(PublisherMessage::Flush(ack_tx)) {
        return Err(e.into());
    }

    // Set a timeout to avoid waiting indefinitely.
    let timeout_duration = Duration::from_secs(3);

    match timeout(timeout_duration, ack_rx).await {
        Ok(Ok(())) => Ok(()),
        Ok(Err(e)) => Err(e.into()),
        Err(_) => Err(anyhow::anyhow!(
            "Flush timed out after {:?}",
            timeout_duration
        )),
    }
}
