use std::sync::Arc;

use bex_events::HostSpanContext;
use sys_types::{CallId, CancellationToken};

/// Per-call context passed to [`crate::BexEngine::call_function`].
///
/// Constructed via [`FunctionCallContextBuilder`].
pub struct FunctionCallContext {
    pub call_id: CallId,
    pub host_ctx: Option<HostSpanContext>,
    pub collectors: Vec<Arc<bex_events::Collector>>,
    pub cancel: CancellationToken,
    /// Callback for streaming partial values.
    pub stream_callback: Option<Arc<dyn Fn(String) + Send + Sync>>,
    /// Callback for raw SSE tick events.
    pub tick_callback: Option<Arc<dyn Fn(String) + Send + Sync>>,
}

/// Builder for `FunctionCallContext`.
pub struct FunctionCallContextBuilder {
    call_id: CallId,
    host_ctx: Option<bex_events::HostSpanContext>,
    collectors: Option<Vec<Arc<bex_events::Collector>>>,
    cancel: Option<CancellationToken>,
    stream_callback: Option<Arc<dyn Fn(String) + Send + Sync>>,
    tick_callback: Option<Arc<dyn Fn(String) + Send + Sync>>,
}

impl FunctionCallContextBuilder {
    pub fn new(call_id: CallId) -> Self {
        Self {
            call_id,
            host_ctx: None,
            collectors: None,
            cancel: None,
            stream_callback: None,
            tick_callback: None,
        }
    }

    #[must_use]
    pub fn build(self) -> FunctionCallContext {
        FunctionCallContext {
            call_id: self.call_id,
            host_ctx: self.host_ctx,
            collectors: self.collectors.unwrap_or_default(),
            cancel: self.cancel.unwrap_or_default(),
            stream_callback: self.stream_callback,
            tick_callback: self.tick_callback,
        }
    }

    #[must_use]
    pub fn with_host_ctx(mut self, host_ctx: bex_events::HostSpanContext) -> Self {
        self.host_ctx = Some(host_ctx);
        self
    }

    #[must_use]
    pub fn with_collectors(mut self, collectors: Vec<Arc<bex_events::Collector>>) -> Self {
        self.collectors = Some(collectors);
        self
    }

    #[must_use]
    pub fn with_cancel_token(mut self, cancel: CancellationToken) -> Self {
        self.cancel = Some(cancel);
        self
    }

    #[must_use]
    pub fn with_stream_callback(mut self, cb: Arc<dyn Fn(String) + Send + Sync>) -> Self {
        self.stream_callback = Some(cb);
        self
    }

    #[must_use]
    pub fn with_tick_callback(mut self, cb: Arc<dyn Fn(String) + Send + Sync>) -> Self {
        self.tick_callback = Some(cb);
        self
    }
}
