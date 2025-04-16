use baml_rpc::runtime_api::trace_event_upload::{
    CreateTraceEventUploadRequest, CreateTraceEventUploadUrl, CreateTraceEventUploadUrlRequest,
};
use baml_types::tracing::events::{TraceData, TraceEvent};
use baml_types::{BamlValueWithMeta, HasFieldType};
use core::time::Duration;
use futures::StreamExt;
use std::borrow::Cow;
use std::sync::Arc;
use tokio::sync::mpsc;
#[cfg(not(target_family = "wasm"))]
use tokio::time::*;
#[cfg(target_family = "wasm")]
use wasmtimer::tokio::*;

use crate::tracingv2::storage::interface::TraceEventWithMeta;

use super::rpc_converters::to_rpc_event;

pub enum PublisherMessage {
    Trace(Arc<TraceEventWithMeta>),
    Flush(tokio::sync::oneshot::Sender<()>),
}

/// Global publisher channel.
/// When the module is first used, we create an unbounded channel and then spawn the publisher task.
static PUBLISHING_CHANNEL: once_cell::sync::Lazy<mpsc::UnboundedSender<PublisherMessage>> =
    once_cell::sync::Lazy::new(|| {
        let (tx, rx) = mpsc::unbounded_channel::<PublisherMessage>();
        // Spawn the publisher task.
        #[cfg(not(target_arch = "wasm32"))]
        {
            tokio::spawn(async move {
                let mut publisher = TracePublisher::new(rx);
                publisher.run().await;
            });
        }
        #[cfg(target_arch = "wasm32")]
        {
            wasm_bindgen_futures::spawn_local(async move {
                let mut publisher = TracePublisher::new(rx);
                publisher.run().await;
            });
        }
        tx
    });

struct TracePublisher {
    batch_size: usize,
    rx: mpsc::UnboundedReceiver<PublisherMessage>,
}

impl TracePublisher {
    pub fn new(rx: mpsc::UnboundedReceiver<PublisherMessage>) -> Self {
        Self { rx, batch_size: 10 }
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
            trace_event_batch: batch.iter().map(|e| to_rpc_event(e)).collect(),
        };

        // Serialize to JSON.
        #[cfg(not(target_arch = "wasm32"))]
        if let Ok(json) = serde_json::to_string(&upload_request) {
            // Write the batch to a file asynchronously.
            use tokio::fs::OpenOptions;
            if let Ok(mut file) = OpenOptions::new()
                .append(true)
                .open("/tmp/trace_events.json")
                .await
            {
                use tokio::io::AsyncWriteExt;
                if let Err(e) = file.write_all(format!("{}\n", json).as_bytes()).await {
                    log::error!("Failed to write to trace file: {}", e);
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
    PUBLISHING_CHANNEL
        .send(PublisherMessage::Trace(event))
        .map_err(|e| e.into())
}

// Note, the library we are using doesnt seem to work well for flushing in Node
// but that's ok since noone uses our wasm build in node for logging.
// https://github.com/whizsid/wasmtimer-rs/issues/26
pub async fn flush() -> anyhow::Result<()> {
    let (ack_tx, ack_rx) = tokio::sync::oneshot::channel();
    if let Err(e) = PUBLISHING_CHANNEL.send(PublisherMessage::Flush(ack_tx)) {
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
