# BAML Client Timeout Configuration Specification

## Overview

This specification defines configurable timeout options for BAML clients to handle slow connections, unresponsive providers, and long-running requests. Timeouts enable graceful failure and automatic fallback to alternative clients in composite client strategies.

## Timeout Types

Base-case clients support four orthogonal timeout types that address different stages of
the request lifecycle:

### 1. `connect_timeout_ms`
- **Purpose**: Maximum time to connect to LLM provider
- **Use case**: Detect and fail fast when provider endpoints are unreachable

### 2. `request_timeout_ms`
- **Purpose**: Maximum for the entire request-response cycle
- **Use case**: Prevent requests from running indefinitely
- **Streaming behavior**: Applies to the entire stream duration (first token to last token)

### 3. `idle_timeout_ms`
- **Purpose**: Maximum time between receiving data chunks
- **Scope**: Time between consecutive chunks of response data
- **When it triggers**: After connection, between chunks of response
- **Use case**: Detect stalled connections

### 4. `time_to_first_token_timeout_ms`
- **Purpose**: Maximum time to receive the first token/chunk of the response (Time To First Token)
- **When it triggers**: After request is sent but before any response data arrives
- **Use case**: Detect when a provider accepts the request but takes too long to start generating a response

Composite clients support the four timeout types above and a fifth: 'total_timeout_ms'.

### 5. `total_timeout_ms`
- **Purpose**: Maximum time for the entire BAML query, including all retries and fallbacks.
- **Use case**: Application-level assurance that BAML function has a time limit.

## Timeout Semantics

### Timeout Evaluation Order
When multiple timeouts are configured, they are evaluated concurrently:

1. `connect_timeout_ms` applies during connection phase
2. After connection succeeds:
   - `time_to_first_token_timeout_ms` starts when request is sent
   - `request_timeout_ms` starts when request is sent
   - `idle_timeout_ms` starts after each chunk is received

A request fails when **any** timeout is exceeded.

### Timeout Units
All timeout values are specified in **milliseconds** as integers:
- `5000` = 5 seconds
- `500` = 500 milliseconds
- `60000` = 1 minute
- `300000` = 5 minutes

**Note**: Actual timeout precision depends on the underlying HTTP client and operating system, typically providing 10-100ms accuracy. Values should be positive integers representing milliseconds.

### Relationship with Retry Policies

- Each retry attempt gets the **full timeout duration** for all timeout types
- A timeout on one attempt triggers the retry mechanism (if configured)
- Total elapsed time = (number of attempts) × (timeout duration per attempt) + (retry delays)

**Example**:
```baml
retry_policy AggressiveRetry {
  max_retries 3
  strategy {
    type exponential_backoff
  }
}

client<llm> MyClient {
  provider openai
  retry_policy AggressiveRetry
  options {
    model "gpt-4"
    api_key env.OPENAI_API_KEY
    http_client {
      request_timeout_ms 30000
    }
  }
}
```
With this configuration:
- Each attempt gets 30 seconds (30000ms)
- If attempt 1 times out at 30s, retry delay is applied, then attempt 2 starts with a fresh 30s timeout
- Maximum possible time: ~30s × 4 attempts + exponential backoff delays

## Client Configuration Syntax

### Non-Composite Clients (OpenAI, Anthropic, etc.)

All four timeout types are available for leaf clients:

```baml
client<llm> GPT4Fast {
  provider openai
  options {
    model "gpt-4"
    api_key env.OPENAI_API_KEY

    // Timeout configuration (in milliseconds)
    http_client {
      connect_timeout_ms 5000             // 5 seconds to establish connection
      time_to_first_token_timeout_ms 10000  // 10 seconds to first token
      idle_timeout_ms 15000               // 15 seconds max between chunks
      request_timeout_ms 60000            // 60 seconds total request time
    }
  }
}

client<llm> Claude {
  provider anthropic
  options {
    model "claude-3-5-sonnet-20241022"
    api_key env.ANTHROPIC_API_KEY

    // Partial timeout configuration
    http_client {
      connect_timeout_ms 3000
      request_timeout_ms 45000
      // idle_timeout_ms and time_to_first_token_timeout_ms not specified = no timeout
    }
  }
}
```

### Composite Clients (Fallback, Round-Robin)

Composite clients support:
1. All four per-request timeout types (inherited by subclients)
2. **Special property**: `total_timeout` - maximum time for the entire strategy

```baml
client<llm> ResilientClient {
  provider fallback
  options {
    strategy [
      PrimaryClient,
      BackupClient,
      LastResortClient
    ]

    // Per-request timeouts (apply to each subclient attempt)
    http_client {
      connect_timeout_ms 5000
      time_to_first_token_timeout_ms 10000
      idle_timeout_ms 15000
      request_timeout_ms 30000
    }

    // Total timeout for the entire fallback chain
    total_timeout_ms 120000  // Max 2 minutes for all attempts combined
  }
}

client<llm> LoadBalanced {
  provider round-robin
  options {
    strategy [
      EndpointA,
      EndpointB
    ]

    // Only total_timeout_ms, no per-request limits
    total_timeout_ms 60000
  }
}
```

## Timeout Composition Rules

When a composite client references subclients that have their own timeout configurations, the following composition rules apply:

### Rule 1: Minimum Timeout Wins
For all per-request timeout properties (`connect_timeout`, `ttft_timeout`, `idle_timeout`, `request_timeout`):
- If both parent and subclient define the same timeout property, use the **smaller (more restrictive)** value
- If only one defines it, use that value
- If neither defines it, no timeout (infinite)

**Rationale**: Safety and predictability. The system should use the most restrictive timeout to ensure timely failure.

### Rule 2: Total Timeout is Absolute
For `total_timeout` on composite clients:
- Applies to the **entire strategy execution** (all subclient attempts combined)
- Overrides any subclient timeouts when the total time budget is exhausted
- Not applicable to non-composite clients

### Composition Examples

#### Example 1: Partial Override
```baml
client<llm> SlowClient {
  provider openai
  options {
    model "gpt-4"
    api_key env.OPENAI_API_KEY
    http_client {
      connect_timeout_ms 10000
      request_timeout_ms 120000
    }
  }
}

client<llm> FastClient {
  provider openai
  options {
    model "gpt-3.5-turbo"
    api_key env.OPENAI_API_KEY
    http_client {
      connect_timeout_ms 3000
      request_timeout_ms 30000
    }
  }
}

client<llm> FallbackClient {
  provider fallback
  options {
    strategy [SlowClient, FastClient]
    http_client {
      connect_timeout_ms 5000   // Parent defines stricter connect timeout
      idle_timeout_ms 20000      // Parent adds idle timeout (subclients don't define it)
    }
    total_timeout_ms 180000
  }
}
```

**Effective timeouts when FallbackClient calls SlowClient**:
- `connect_timeout_ms`: `min(5000, 10000)` = **5000ms** (parent is stricter)
- `request_timeout_ms`: `min(∞, 120000)` = **120000ms** (only subclient defines it)
- `idle_timeout_ms`: `min(20000, ∞)` = **20000ms** (only parent defines it)
- `time_to_first_token_timeout_ms`: `min(∞, ∞)` = **∞** (neither defines it)
- `total_timeout_ms`: **180000ms** (applies to entire strategy)

**Effective timeouts when FallbackClient calls FastClient**:
- `connect_timeout_ms`: `min(5000, 3000)` = **3000ms** (subclient is stricter)
- `request_timeout_ms`: `min(∞, 30000)` = **30000ms** (only subclient defines it)
- `idle_timeout_ms`: `min(20000, ∞)` = **20000ms** (only parent defines it)
- `time_to_first_token_timeout_ms`: `min(∞, ∞)` = **∞** (neither defines it)

#### Example 2: Nested Composites
```baml
client<llm> Primary {
  provider openai
  options {
    model "gpt-4"
    http_client {
      request_timeout_ms 60000
    }
  }
}

client<llm> Secondary {
  provider anthropic
  options {
    model "claude-3-5-sonnet-20241022"
    http_client {
      request_timeout_ms 45000
    }
  }
}

client<llm> TierOne {
  provider fallback
  options {
    strategy [Primary, Secondary]
    http_client {
      connect_timeout_ms 10000
    }
    total_timeout_ms 150000
  }
}

client<llm> Tertiary {
  provider openai
  options {
    model "gpt-3.5-turbo"
    http_client {
      request_timeout_ms 30000
    }
  }
}

client<llm> UltraResilient {
  provider fallback
  options {
    strategy [TierOne, Tertiary]
    http_client {
      connect_timeout_ms 5000
      idle_timeout_ms 20000
    }
    total_timeout_ms 300000
  }
}
```

When `UltraResilient` executes:
1. Tries `TierOne` (which is itself a composite):
   - `TierOne.total_timeout_ms = 150000` is not overridden by `UltraResilient.total_timeout_ms`
   - `TierOne` gets up to 150 seconds to try `Primary` and `Secondary`
   - If `TierOne` exhausts 150 seconds, it fails and `UltraResilient` tries next client
2. If `TierOne` fails, tries `Tertiary`:
   - Effective `connect_timeout_ms`: `min(5000, ∞)` = 5000
   - Effective `request_timeout_ms`: `min(∞, 30000)` = 30000
   - Effective `idle_timeout_ms`: `min(20000, ∞)` = 20000
3. `UltraResilient.total_timeout_ms = 300000` applies to entire execution (TierOne time + Tertiary time)

## Runtime Override via Client Registry

All timeout configurations can be overridden at runtime using the client registry API. This enables dynamic timeout adjustment based on runtime conditions.

### TypeScript API
```typescript
import { b } from './baml_client';

// Override timeouts for a specific invocation
const result = await b.MyFunction(
  { input: "test" },
  {
    clientRegistry: b.ClientRegistry.override({
      "GPT4Fast": {
        options: {
          http_client: {
            connect_timeout_ms: 3000,
            request_timeout_ms: 30000,
          }
        }
      },
      "FallbackClient": {
        options: {
          http_client: {
            idle_timeout_ms: 10000,
          },
          total_timeout_ms: 60000,
        }
      }
    })
  }
);
```

### Python API
```python
from baml_client import b

# Override timeouts for a specific invocation
result = await b.MyFunction(
    "test",
    baml_options={
        "client_registry": b.ClientRegistry.override({
            "GPT4Fast": {
                "options": {
                    "http_client": {
                        "connect_timeout_ms": 3000,
                        "request_timeout_ms": 30000,
                    }
                }
            },
            "FallbackClient": {
                "options": {
                    "http_client": {
                        "idle_timeout_ms": 10000,
                    },
                    "total_timeout_ms": 60000,
                }
            }
        })
    }
)
```

### Ruby API
```ruby
require 'baml_client'

# Override timeouts for a specific invocation
result = b.MyFunction(
  "test",
  baml_options: {
    client_registry: b.ClientRegistry.override({
      "GPT4Fast" => {
        options: {
          http_client: {
            connect_timeout_ms: 3000,
            request_timeout_ms: 30000,
          }
        }
      },
      "FallbackClient" => {
        options: {
          http_client: {
            idle_timeout_ms: 10000,
          },
          total_timeout_ms: 60000,
        }
      }
    })
  }
)
```

### Go API
```go
import "baml_client/b"

// Override timeouts for a specific invocation
result, err := b.MyFunction(
    ctx,
    "test",
    &b.BamlOptions{
        ClientRegistry: b.NewClientRegistry().Override(map[string]b.ClientOptions{
            "GPT4Fast": {
                Options: map[string]interface{}{
                    "http_client": map[string]interface{}{
                        "connect_timeout_ms": 3000,
                        "request_timeout_ms": 30000,
                    },
                },
            },
            "FallbackClient": {
                Options: map[string]interface{}{
                    "http_client": map[string]interface{}{
                        "idle_timeout_ms": 10000,
                    },
                    "total_timeout_ms": 60000,
                },
            },
        }),
    },
)
```

### Runtime Override Behavior
- Runtime overrides follow the same composition rules (minimum timeout wins)
- Runtime values compose with both config file values and inherited values
- Evaluation order: config file → inherited from parent → runtime override (take minimum at each step)

## Implementation Guide

### Config Level Implementation

#### 1. Parser Changes
**Location**: `engine/baml-lib/parser-database/src/walkers/client.rs`

Add timeout property accessors to `ClientWalker`:
```rust
impl ClientWalker<'_> {
    pub fn connect_timeout_ms(&self) -> Option<i64> { /* ... */ }
    pub fn time_to_first_token_timeout_ms(&self) -> Option<i64> { /* ... */ }
    pub fn idle_timeout_ms(&self) -> Option<i64> { /* ... */ }
    pub fn request_timeout_ms(&self) -> Option<i64> { /* ... */ }
    pub fn total_timeout_ms(&self) -> Option<i64> { /* ... */ }
}
```

#### 2. Client Property Structures
**Location**: `engine/baml-lib/llm-client/src/clients/`

Add timeout fields to each client type's property structures:

```rust
// For leaf clients (openai, anthropic, etc.)
pub struct UnresolvedOpenAI<Meta> {
    // ... existing fields
    pub connect_timeout_ms: Option<(i64, Meta)>,
    pub time_to_first_token_timeout_ms: Option<(i64, Meta)>,
    pub idle_timeout_ms: Option<(i64, Meta)>,
    pub request_timeout_ms: Option<(i64, Meta)>,
}

pub struct ResolvedOpenAI {
    // ... existing fields
    pub connect_timeout_ms: Option<i64>,
    pub time_to_first_token_timeout_ms: Option<i64>,
    pub idle_timeout_ms: Option<i64>,
    pub request_timeout_ms: Option<i64>,
}

// For composite clients (fallback, round-robin)
pub struct UnresolvedFallback<Meta> {
    // ... existing fields
    pub connect_timeout_ms: Option<(i64, Meta)>,
    pub time_to_first_token_timeout_ms: Option<(i64, Meta)>,
    pub idle_timeout_ms: Option<(i64, Meta)>,
    pub request_timeout_ms: Option<(i64, Meta)>,
    pub total_timeout_ms: Option<(i64, Meta)>,
}

pub struct ResolvedFallback {
    // ... existing fields
    pub connect_timeout_ms: Option<i64>,
    pub time_to_first_token_timeout_ms: Option<i64>,
    pub idle_timeout_ms: Option<i64>,
    pub request_timeout_ms: Option<i64>,
    pub total_timeout_ms: Option<i64>,
}
```

#### 3. Property Parsing
**Location**: `engine/baml-lib/llm-client/src/clients/helpers.rs`

Extend `PropertyHandler` to recognize timeout properties:
```rust
impl<Meta: Clone> PropertyHandler<Meta> {
    pub fn timeout_property(&mut self, name: &str) -> Result<Option<(i64, Meta)>, Error<Meta>> {
        match name {
            "connect_timeout_ms" | "time_to_first_token_timeout_ms" | "idle_timeout_ms"
            | "request_timeout_ms" | "total_timeout_ms" => {
                self.get_int_with_meta(name)
            }
            _ => Ok(None)
        }
    }
}
```

Update each client's `create_from` method:
```rust
impl UnresolvedOpenAI<Meta> {
    pub fn create_standard(mut properties: PropertyHandler<Meta>)
        -> Result<Self, Vec<Error<Meta>>> {
        // ... existing property parsing

        let connect_timeout_ms = properties.timeout_property("connect_timeout_ms")?;
        let time_to_first_token_timeout_ms = properties.timeout_property("time_to_first_token_timeout_ms")?;
        let idle_timeout_ms = properties.timeout_property("idle_timeout_ms")?;
        let request_timeout_ms = properties.timeout_property("request_timeout_ms")?;

        // Validate: total_timeout_ms not allowed on leaf clients
        if properties.has_property("total_timeout_ms") {
            errors.push(Error::invalid_property(
                "total_timeout_ms",
                "total_timeout_ms is only valid on composite clients (fallback, round-robin)"
            ));
        }

        Ok(Self {
            // ... existing fields
            connect_timeout_ms,
            time_to_first_token_timeout_ms,
            idle_timeout_ms,
            request_timeout_ms,
        })
    }
}
```

#### 4. Validation
**Location**: `engine/baml-lib/baml-core/src/validate/validation_pipeline/validations/clients.rs`

Add timeout validation:
```rust
pub(super) fn validate(ctx: &mut Context<'_>) {
    for client in ctx.db.walk_clients() {
        // Validate timeout values
        validate_timeouts(&client, ctx);

        // Existing validation...
    }
}

fn validate_timeouts(client: &ClientWalker, ctx: &mut Context<'_>) {
    let is_composite = matches!(
        client.properties().provider,
        ClientProvider::Strategy(_)
    );

    // Validate timeout values are positive
    for (timeout_name, timeout_value, span) in [
        ("connect_timeout_ms", client.connect_timeout_ms()),
        ("time_to_first_token_timeout_ms", client.time_to_first_token_timeout_ms()),
        ("idle_timeout_ms", client.idle_timeout_ms()),
        ("request_timeout_ms", client.request_timeout_ms()),
        ("total_timeout_ms", client.total_timeout_ms()),
    ] {
        if let Some(value) = timeout_value {
            if value <= 0 {
                ctx.push_error(DatamodelError::new_validation_error(
                    &format!("{} must be positive, got {}", timeout_name, value),
                    span.clone(),
                ));
            }
        }
    }

    // Validate total_timeout_ms only on composites
    if let Some((_, span)) = client.properties().options.total_timeout_ms() {
        if !is_composite {
            ctx.push_error(DatamodelError::new_validation_error(
                "total_timeout_ms is only valid on composite clients (fallback, round-robin)",
                span.clone(),
            ));
        }
    }

    // Validate request_timeout_ms >= time_to_first_token_timeout_ms if both specified
    if let (Some(request_timeout_ms), Some(ttft_timeout_ms)) =
        (client.request_timeout_ms(), client.time_to_first_token_timeout_ms()) {
        if request_timeout_ms < ttft_timeout_ms {
            ctx.push_error(DatamodelError::new_validation_error(
                &format!(
                    "request_timeout_ms ({}) must be >= time_to_first_token_timeout_ms ({})",
                    request_timeout_ms, ttft_timeout_ms
                ),
                client.span().clone(),
            ));
        }
    }
}
```

### Runtime Level Implementation

The runtime implementation integrates timeouts into BAML's orchestration system, which manages client execution, retries, and fallback strategies.

#### Architecture Overview

BAML's orchestration system consists of:
- **OrchestratorNode**: Represents a single client execution attempt with its scope (Direct, Retry, Fallback, RoundRobin)
- **Orchestration functions**: `orchestrate_call` and `orchestrate_stream` that iterate through nodes until success
- **Tripwire**: Existing cancellation mechanism for user-initiated aborts
- **LLMPrimitiveProvider**: The actual HTTP client implementations (OpenAI, Anthropic, etc.)

#### 1. Timeout Configuration Structure
**Location**: `engine/baml-runtime/src/internal/llm_client/`

Create a timeout configuration structure that composes with parent timeouts:

```rust
#[derive(Clone, Debug, Default)]
pub struct TimeoutConfig {
    pub connect_timeout: Option<Duration>,
    pub ttft_timeout: Option<Duration>,
    pub idle_timeout: Option<Duration>,
    pub request_timeout: Option<Duration>,
    pub total_timeout: Option<Duration>,
}

impl TimeoutConfig {
    /// Compose two timeout configs using minimum rule
    pub fn compose_with(&self, other: &TimeoutConfig) -> TimeoutConfig {
        TimeoutConfig {
            connect_timeout: min_duration(self.connect_timeout, other.connect_timeout),
            ttft_timeout: min_duration(self.ttft_timeout, other.ttft_timeout),
            idle_timeout: min_duration(self.idle_timeout, other.idle_timeout),
            request_timeout: min_duration(self.request_timeout, other.request_timeout),
            total_timeout: other.total_timeout.or(self.total_timeout),
        }
    }

    pub fn from_resolved_client(client: &ResolvedClientProperty) -> Self {
        match client {
            ResolvedClientProperty::OpenAI(c) => Self {
                connect_timeout: c.connect_timeout_ms.map(Duration::from_millis),
                ttft_timeout: c.time_to_first_token_timeout_ms.map(Duration::from_millis),
                idle_timeout: c.idle_timeout_ms.map(Duration::from_millis),
                request_timeout: c.request_timeout_ms.map(Duration::from_millis),
                total_timeout: None,
            },
            ResolvedClientProperty::Fallback(c) => Self {
                connect_timeout: c.connect_timeout_ms.map(Duration::from_millis),
                ttft_timeout: c.time_to_first_token_timeout_ms.map(Duration::from_millis),
                idle_timeout: c.idle_timeout_ms.map(Duration::from_millis),
                request_timeout: c.request_timeout_ms.map(Duration::from_millis),
                total_timeout: c.total_timeout_ms.map(Duration::from_millis),
            },
            // ... other client types
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
```

#### 2. Integrate Timeouts into Orchestrator
**Location**: `engine/baml-runtime/src/internal/llm_client/orchestrator/call.rs` and `stream.rs`

The orchestration functions already handle cancellation via `Tripwire`. Extend them to handle timeouts:

```rust
// In orchestrator/call.rs
pub async fn orchestrate(
    iter: OrchestratorNodeIterator,
    ir: &IntermediateRepr,
    ctx: &RuntimeContext,
    prompt: &PromptRenderer,
    params: &BamlValue,
    parse_fn: impl Fn(&str) -> Result<ResponseBamlValue>,
    cancel_tripwire: Option<Tripwire>,
) -> (Vec<(OrchestrationScope, LLMResponse, Option<Result<ResponseBamlValue>>)>, Duration) {
    let mut results = Vec::new();
    let mut total_sleep_duration = Duration::from_secs(0);
    let orchestration_start = Instant::now();

    // Extract total_timeout from the first node (if it's a composite client)
    let total_timeout = iter.first()
        .and_then(|node| node.provider.timeout_config().total_timeout);

    let cancel_future = match cancel_tripwire {
        Some(tripwire) => Box::pin(async move { tripwire.await; }),
        None => Box::pin(futures::future::pending()),
    };
    tokio::pin!(cancel_future);

    for node in iter {
        // Check total_timeout before attempting next client
        if let Some(total_timeout) = total_timeout {
            if orchestration_start.elapsed() >= total_timeout {
                results.push((
                    node.scope.clone(),
                    LLMResponse::LLMFailure(LLMErrorResponse {
                        client: node.provider.name().into(),
                        message: format!("Total timeout of {:?} exceeded", total_timeout),
                        code: ErrorCode::Timeout,
                        // ... other fields
                    }),
                    Some(Err(anyhow!("Total timeout exceeded"))),
                ));
                break;
            }
        }

        tokio::select! {
            biased;
            _ = &mut cancel_future => {
                // User cancellation via Tripwire
                results.push((/* ... cancelled response ... */));
                break;
            }
            result = async {
                let prompt = match node.render_prompt(ir, prompt, ctx, params).await {
                    Ok(p) => p,
                    Err(e) => return Some((/* ... internal failure ... */)),
                };

                let ctx = CtxWithHttpRequestId::from(ctx);

                // Compose timeout config from parent and this node
                let timeout_config = compose_timeout_for_node(&node, total_timeout, &orchestration_start);

                // single_call now respects timeout_config
                let response = node.single_call_with_timeout(&ctx, &prompt, &timeout_config).await;

                // ... existing result handling ...
            } => {
                // ... existing result processing ...
            }
        }
    }

    (results, total_sleep_duration)
}

fn compose_timeout_for_node(
    node: &OrchestratorNode,
    parent_total_timeout: Option<Duration>,
    orchestration_start: &Instant,
) -> TimeoutConfig {
    let mut config = node.provider.timeout_config();

    // Adjust request_timeout to respect remaining total_timeout budget
    if let Some(total_timeout) = parent_total_timeout {
        let elapsed = orchestration_start.elapsed();
        let remaining = total_timeout.saturating_sub(elapsed);

        config.request_timeout = match config.request_timeout {
            Some(rt) => Some(rt.min(remaining)),
            None => Some(remaining),
        };
    }

    config
}
```

#### 3. Primitive Provider Timeout Integration
**Location**: `engine/baml-runtime/src/internal/llm_client/primitive/`

Each primitive provider (OpenAI, Anthropic, etc.) needs to apply timeouts in their HTTP requests:

```rust
// In primitive/openai.rs (and similar for other providers)
impl OpenAIClient {
    pub fn timeout_config(&self) -> TimeoutConfig {
        TimeoutConfig {
            connect_timeout: self.connect_timeout_ms.map(|ms| Duration::from_millis(ms as u64)),
            ttft_timeout: self.time_to_first_token_timeout_ms.map(|ms| Duration::from_millis(ms as u64)),
            idle_timeout: self.idle_timeout_ms.map(|ms| Duration::from_millis(ms as u64)),
            request_timeout: self.request_timeout_ms.map(|ms| Duration::from_millis(ms as u64)),
            total_timeout: None, // Only relevant for composite clients
        }
    }
}

impl WithSingleCallable for OpenAIClient {
    async fn single_call(&self, ctx: &impl HttpContext, prompt: &RenderedPrompt) -> LLMResponse {
        self.single_call_with_timeout(ctx, prompt, &self.timeout_config()).await
    }

    async fn single_call_with_timeout(
        &self,
        ctx: &impl HttpContext,
        prompt: &RenderedPrompt,
        timeout_config: &TimeoutConfig,
    ) -> LLMResponse {
        // Build reqwest client with connect_timeout
        let mut client_builder = reqwest::Client::builder();
        if let Some(connect_timeout) = timeout_config.connect_timeout {
            client_builder = client_builder.connect_timeout(connect_timeout);
        }
        let client = client_builder.build().expect("Failed to build HTTP client");

        // Prepare request
        let request = self.prepare_request(ctx, prompt)?;

        // Apply request_timeout wrapper
        let response_future = async {
            let start = Instant::now();
            let response = client.execute(request).await?;

            // Check TTFT
            let ttft = start.elapsed();
            if let Some(ttft_timeout) = timeout_config.ttft_timeout {
                if ttft > ttft_timeout {
                    return Err(anyhow!("TTFT timeout: {:?} > {:?}", ttft, ttft_timeout));
                }
            }

            // Read response body (with idle timeout handled by reqwest internals)
            let body = response.text().await?;
            Ok(body)
        };

        // Wrap with request_timeout
        let result = if let Some(request_timeout) = timeout_config.request_timeout {
            match tokio::time::timeout(request_timeout, response_future).await {
                Ok(r) => r,
                Err(_) => Err(anyhow!("Request timeout: {:?}", request_timeout)),
            }
        } else {
            response_future.await
        };

        // Convert result to LLMResponse
        match result {
            Ok(body) => self.parse_response(body),
            Err(e) => LLMResponse::LLMFailure(LLMErrorResponse {
                client: self.name.clone(),
                message: e.to_string(),
                code: ErrorCode::Timeout,
                // ... other fields
            }),
        }
    }
}
```

#### 4. Streaming with Idle Timeout
**Location**: `engine/baml-runtime/src/internal/llm_client/primitive/stream_request.rs`

For streaming, idle timeout is implemented at the SSE chunk level:

```rust
// Extend the existing streaming implementation
impl WithStreamable for OpenAIClient {
    async fn stream(&self, ctx: &impl HttpContext, prompt: &RenderedPrompt) -> StreamResponse {
        let timeout_config = self.timeout_config();

        // ... existing setup code ...

        // Wrap the SSE stream with idle timeout monitoring
        let response_stream = if let Some(idle_timeout) = timeout_config.idle_timeout {
            Box::pin(IdleTimeoutStream::new(sse_stream, idle_timeout))
        } else {
            sse_stream
        };

        Ok(response_stream)
    }
}

pub struct IdleTimeoutStream<S> {
    inner: S,
    idle_timeout: Duration,
    last_activity: Instant,
}

impl<S> IdleTimeoutStream<S> {
    pub fn new(inner: S, idle_timeout: Duration) -> Self {
        Self {
            inner,
            idle_timeout,
            last_activity: Instant::now(),
        }
    }
}

impl<S> Stream for IdleTimeoutStream<S>
where
    S: Stream + Unpin,
{
    type Item = Result<S::Item, anyhow::Error>;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        // Check if we've exceeded idle timeout
        if self.last_activity.elapsed() > self.idle_timeout {
            return Poll::Ready(Some(Err(
                anyhow!("Idle timeout: no data received for {:?}", self.idle_timeout)
            )));
        }

        match self.inner.poll_next_unpin(cx) {
            Poll::Ready(Some(item)) => {
                self.last_activity = Instant::now();
                Poll::Ready(Some(Ok(item)))
            }
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}
```

#### 5. Fallback Strategy Integration
**Location**: `engine/baml-runtime/src/internal/llm_client/strategy/fallback.rs`

Fallback strategies already use `IterOrchestrator` to generate `OrchestratorNode`s. Timeout composition happens at the orchestrator level (see step 2), but the fallback strategy needs to expose its `total_timeout`:

```rust
impl IterOrchestrator for FallbackStrategy {
    fn iter_orchestrator<'a>(
        &self,
        state: &mut OrchestrationState,
        previous: OrchestrationScope,
        ctx: &RuntimeContext,
        client_lookup: &'a dyn InternalClientLookup<'a>,
    ) -> Result<OrchestratorNodeIterator> {
        let items = self
            .client_specs
            .iter()
            .enumerate()
            .map(|(idx, client)| {
                match client_lookup.get_llm_provider(client, ctx) {
                    Ok(mut client) => {
                        // Compose timeouts: parent (self) with subclient
                        let parent_config = TimeoutConfig {
                            connect_timeout: self.connect_timeout_ms.map(|ms| Duration::from_millis(ms as u64)),
                            ttft_timeout: self.time_to_first_token_timeout_ms.map(|ms| Duration::from_millis(ms as u64)),
                            idle_timeout: self.idle_timeout_ms.map(|ms| Duration::from_millis(ms as u64)),
                            request_timeout: self.request_timeout_ms.map(|ms| Duration::from_millis(ms as u64)),
                            total_timeout: self.total_timeout_ms.map(|ms| Duration::from_millis(ms as u64)),
                        };

                        let subclient_config = client.timeout_config();
                        let effective_config = parent_config.compose_with(&subclient_config);

                        // Update client with composed timeout config
                        client.set_timeout_config(effective_config);

                        Ok(client.iter_orchestrator(
                            state,
                            ExecutionScope::Fallback(self.name.clone(), idx).into(),
                            ctx,
                            client_lookup,
                        ))
                    }
                    Err(e) => Err(e),
                }
            })
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .flatten()
            .flatten()
            .collect();

        Ok(items)
    }
}
```

#### 6. Client Registry Support
**Location**: `engine/baml-runtime/src/client_registry.rs`

Add methods to override timeout configurations at runtime:
```rust
impl ClientRegistry {
    pub fn override_timeouts(
        &mut self,
        client_name: &str,
        timeout_overrides: TimeoutConfig,
    ) {
        if let Some(client_property) = self.clients.get_mut(client_name) {
            // Apply overrides by composing with existing config
            let existing_config = TimeoutConfig::from_resolved_client(client_property);
            let new_config = existing_config.compose_with(&timeout_overrides);

            // Update the client property with new timeouts
            match client_property {
                ResolvedClientProperty::OpenAI(ref mut c) => {
                    c.connect_timeout_ms = new_config.connect_timeout.map(|d| d.as_millis() as i64);
                    c.time_to_first_token_timeout_ms = new_config.ttft_timeout.map(|d| d.as_millis() as i64);
                    c.idle_timeout_ms = new_config.idle_timeout.map(|d| d.as_millis() as i64);
                    c.request_timeout_ms = new_config.request_timeout.map(|d| d.as_millis() as i64);
                }
                ResolvedClientProperty::Fallback(ref mut c) => {
                    c.connect_timeout_ms = new_config.connect_timeout.map(|d| d.as_millis() as i64);
                    c.time_to_first_token_timeout_ms = new_config.ttft_timeout.map(|d| d.as_millis() as i64);
                    c.idle_timeout_ms = new_config.idle_timeout.map(|d| d.as_millis() as i64);
                    c.request_timeout_ms = new_config.request_timeout.map(|d| d.as_millis() as i64);
                    c.total_timeout_ms = new_config.total_timeout.map(|d| d.as_millis() as i64);
                }
                // ... other client types
            }
        }
    }
}
```

## Error Messages

Timeout errors are represented by a `BamlTimeoutError` class that derives from `BamlClientError`. This allows users to catch timeout-specific errors separately from other client errors.

### Error Hierarchy

```
BamlError
└── BamlClientError
    └── BamlTimeoutError
        ├── ConnectTimeoutError
        ├── TimeToFirstTokenTimeoutError
        ├── IdleTimeoutError
        ├── RequestTimeoutError
        └── TotalTimeoutError
```

All timeout errors include structured fields:
- `client`: The client name that timed out
- `timeout_type`: The specific timeout that was exceeded
- `configured_value_ms`: The configured timeout value in milliseconds
- `elapsed_ms`: The actual elapsed time in milliseconds
- `message`: A human-readable error message

### Error Message Examples

Timeout errors should provide clear, actionable messages:

### Connection Timeout
```
Connection timeout after 5000ms while connecting to https://api.openai.com/v1/chat/completions
Client: GPT4Fast
Timeout type: connect_timeout_ms
Configured value: 5000ms
Suggestion: Increase connect_timeout_ms or check network connectivity
```

### Time to First Token Timeout
```
Time to first token timeout after 10000ms
Client: GPT4Fast
Timeout type: time_to_first_token_timeout_ms
Configured value: 10000ms
Request was sent successfully but provider did not start responding within the timeout
Suggestion: Increase time_to_first_token_timeout_ms or check provider status
```

### Idle Timeout
```
Idle timeout after 15000ms: no data received
Client: GPT4Fast
Timeout type: idle_timeout_ms
Configured value: 15000ms
Last chunk received: 15200ms ago
Suggestion: Increase idle_timeout_ms or check for stalled connection
```

### Request Timeout
```
Request timeout after 60000ms
Client: GPT4Fast
Timeout type: request_timeout_ms
Configured value: 60000ms
Total elapsed time: 60100ms
Suggestion: Increase request_timeout_ms or optimize prompt/response size
```

### Total Timeout (Composite)
```
Fallback total timeout after 120000ms
Client: ResilientClient
Timeout type: total_timeout_ms
Configured value: 120000ms
Attempts made: 2/3 (PrimaryClient failed, BackupClient timed out)
Total elapsed time: 120200ms
Suggestion: Increase total_timeout_ms or reduce per-request timeouts
```

## Testing Strategy

### Unit Tests
1. **Timeout Parsing**: Verify timeout values are correctly parsed from BAML config
2. **Validation**: Test invalid timeout values (negative, zero, incompatible combinations)
3. **Composition**: Test minimum-rule composition logic with various combinations
4. **Runtime Override**: Test that runtime overrides correctly compose with config values

### Integration Tests
1. **Connection Timeout**: Mock unreachable endpoint, verify connect_timeout triggers
2. **TTFT Timeout**: Mock slow-to-respond endpoint, verify ttft_timeout triggers
3. **Idle Timeout**: Mock endpoint that stalls mid-stream, verify idle_timeout triggers
4. **Request Timeout**: Mock long-running request, verify request_timeout triggers
5. **Fallback with Timeouts**: Verify fallback strategy tries next client when timeout occurs
6. **Total Timeout**: Verify composite client respects total_timeout budget
7. **Retry + Timeout**: Verify each retry gets full timeout duration

### Test Files
Create test BAML files:
- `integ-tests/baml_src/test-files/timeouts/basic-timeouts.baml`
- `integ-tests/baml_src/test-files/timeouts/composite-timeouts.baml`
- `integ-tests/baml_src/test-files/timeouts/timeout-composition.baml`
- `integ-tests/baml_src/test-files/timeouts/runtime-override.baml`

## Recommended Values for New Users

For production applications, we recommend starting with:
```baml
client<llm> MyClient {
  provider openai
  options {
    model "gpt-4"
    api_key env.OPENAI_API_KEY

    // Recommended production defaults
    connect_timeout_ms 10000                // 10s to establish connection
    time_to_first_token_timeout_ms 30000    // 30s to first token
    idle_timeout_ms 60000                   // 60s between chunks
    request_timeout_ms 300000               // 5 minutes total
  }
}

client<llm> MyFallback {
  provider fallback
  options {
    strategy [Primary, Secondary, Tertiary]

    connect_timeout_ms 5000                 // Stricter for fallback
    time_to_first_token_timeout_ms 15000
    idle_timeout_ms 30000
    request_timeout_ms 120000               // 2 min per attempt
    total_timeout_ms 600000                 // 10 min for entire strategy
  }
}
```

## Summary

This specification provides a comprehensive, composable timeout system for BAML clients:

- **5 timeout types** for different lifecycle stages (connect, ttft, idle, request)
- **Composability** via minimum-rule for composite clients
- **Special `total_timeout`** for fallback/round-robin strategies
- **Runtime override** via client registry API
- **Clear error messages** with actionable suggestions
- **Non-breaking** migration path
