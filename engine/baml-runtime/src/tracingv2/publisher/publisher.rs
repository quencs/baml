use baml_rpc::runtime_api::trace_event_upload::{
    CreateTraceEventUploadRequest, CreateTraceEventUploadUrl, CreateTraceEventUploadUrlRequest,
};
use baml_types::tracing::events::{TraceData, TraceEvent};
use baml_types::{BamlValueWithMeta, HasFieldType};
use core::time::Duration;
use futures::StreamExt;
use once_cell::sync::OnceCell;
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

fn get_publish_channel() -> Option<&'static mpsc::UnboundedSender<PublisherMessage>> {
    let Some(join_handle) = PUBLISHING_TASK.get() else {
        baml_log::fatal_once!("Tracing publisher not started. Report this bug to the BAML team.");
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

struct RuntimeAST {
    ast: Arc<AstSignatureWrapper>,
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
    // If we've already started, do nothing.
    if let Some(channel) = get_publish_channel() {
        let _ = rt.block_on(flush());
        let _ = channel.send(PublisherMessage::UpdateRuntime(lookup));
        return;
    }

    // Create our channel
    let (tx, rx) = mpsc::unbounded_channel::<PublisherMessage>();

    // Install it into the OnceCell
    // Safe because we just checked `get().is_none()`
    PUBLISHING_CHANNEL
        .set(tx.clone())
        .expect("Failed to set PUBLISHING_CHANNEL");

    let mut publisher = TracePublisher::new(rx, lookup);

    // Spawn the background task
    #[cfg(not(target_arch = "wasm32"))]
    {
        let handle = rt.spawn(async move { publisher.run().await });
        PUBLISHING_TASK
            .set(Arc::new(handle))
            .expect("Failed to set PUBLISHING_TASK");
    }

    #[cfg(target_arch = "wasm32")]
    wasm_bindgen_futures::spawn_local(async move {
        publisher.run().await;
    });
}

/// Gracefully shutdown the TracePublisher.
/// 1. Sends a Shutdown message and waits for its ack.
/// 2. Awaits the background task’s JoinHandle so Drop runs.
pub async fn shutdown_publisher() -> anyhow::Result<()> {
    // 1. send Shutdown
    let Some(channel) = get_publish_channel() else {
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
                            // Update the lookup
                            self.lookup = lookup;
                        },
                        PublisherMessage::Trace(event) => {
                            let event_type = std::any::type_name_of_val(&event.content);
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

    /// Process a batch of events.
    ///
    /// In this example we:
    ///   1. Serialize the events into JSON.
    ///   2. Append the JSON to a file (using async file I/O on macOS).
    ///   3. Post the JSON to an HTTP API with up to 3 retries.
    async fn process_batch(&self, batch: Vec<Arc<TraceEventWithMeta>>) {
        // Assemble the upload request structure.
        let upload_request = CreateTraceEventUploadRequest {
            trace_event_batch: batch
                .iter()
                .map(|e| to_rpc_event(e, self.lookup.as_ref()))
                .collect(),
        };

        // Serialize to JSON.
        #[cfg(not(target_arch = "wasm32"))]
        {
            use tokio::fs::OpenOptions;
            if let Ok(mut file) = OpenOptions::new()
                .append(true)
                .open("/tmp/trace_events.json")
                .await
            {
                for e in upload_request.trace_event_batch.iter() {
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
        // let client = reqwest::Client::new();
        // let mut retries = 3;
        // while retries > 0 {
        //     match client
        //         .post("https://3vwc8vlts7.execute-api.us-east-1.amazonaws.com/v1/baml-traces")
        //         .json(&upload_request)
        //         .send()
        //         .await
        //     {
        //         Ok(response) => {
        //             log::info!("Upload completed with status {}", response.status());
        //             break;
        //         }
        //         Err(e) => {
        //             log::error!("Upload failed: {}", e);
        //             retries -= 1;
        //             if retries > 0 {
        //                 time::sleep(Duration::from_secs(1)).await;
        //             }
        //         }
        //     }
        // }
    }
}

pub fn publish_trace_event(event: Arc<TraceEventWithMeta>) -> anyhow::Result<()> {
    let Some(channel) = get_publish_channel() else {
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
    let Some(channel) = get_publish_channel() else {
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
