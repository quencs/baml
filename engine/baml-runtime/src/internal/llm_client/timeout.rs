use web_time::Duration;

/// Runtime timeout configuration for LLM requests
#[derive(Clone, Debug, Default)]
pub struct TimeoutConfig {
    pub connect_timeout: Option<Duration>,
    pub ttft_timeout: Option<Duration>,
    pub idle_timeout: Option<Duration>,
    pub request_timeout: Option<Duration>,
    pub total_timeout: Option<Duration>,
}

impl TimeoutConfig {
    /// Compose two timeout configs using minimum rule for per-request timeouts
    pub fn compose_with(&self, other: &TimeoutConfig) -> TimeoutConfig {
        TimeoutConfig {
            connect_timeout: min_duration(self.connect_timeout, other.connect_timeout),
            ttft_timeout: min_duration(self.ttft_timeout, other.ttft_timeout),
            idle_timeout: min_duration(self.idle_timeout, other.idle_timeout),
            request_timeout: min_duration(self.request_timeout, other.request_timeout),
            // total_timeout is not composed - only the parent's total_timeout applies
            total_timeout: other.total_timeout.or(self.total_timeout),
        }
    }

    /// Create from resolved client timeouts (for primitive clients)
    pub fn from_primitive_timeouts(timeouts: &internal_llm_client::ResolvedPrimitiveClientTimeouts) -> Self {
        TimeoutConfig {
            connect_timeout: timeouts.connect_timeout_ms.map(|ms| Duration::from_millis(ms as u64)),
            ttft_timeout: timeouts.time_to_first_token_timeout_ms.map(|ms| Duration::from_millis(ms as u64)),
            idle_timeout: timeouts.idle_timeout_ms.map(|ms| Duration::from_millis(ms as u64)),
            request_timeout: timeouts.request_timeout_ms.map(|ms| Duration::from_millis(ms as u64)),
            total_timeout: None,
        }
    }

    /// Create from resolved client timeouts (for composite clients)
    pub fn from_composite_timeouts(timeouts: &internal_llm_client::ResolvedCompositeClientTimeouts) -> Self {
        TimeoutConfig {
            connect_timeout: timeouts.primitive.connect_timeout_ms.map(|ms| Duration::from_millis(ms as u64)),
            ttft_timeout: timeouts.primitive.time_to_first_token_timeout_ms.map(|ms| Duration::from_millis(ms as u64)),
            idle_timeout: timeouts.primitive.idle_timeout_ms.map(|ms| Duration::from_millis(ms as u64)),
            request_timeout: timeouts.primitive.request_timeout_ms.map(|ms| Duration::from_millis(ms as u64)),
            total_timeout: timeouts.total_timeout_ms.map(|ms| Duration::from_millis(ms as u64)),
        }
    }
}

fn min_duration(a: Option<Duration>, b: Option<Duration>) -> Option<Duration> {
    match (a, b) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    }
}
