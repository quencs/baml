// BAML Builtin: LLM Operations
// Namespace: baml.llm
//
// This file contains the orchestrator functions for LLM operations.
// These are compiled as part of the builtin library and available
// to all BAML programs.

/// A single step in the orchestration plan.
/// Each step represents one LLM call attempt with a specific client and delay.
class OrchestrationStep {
    primitive_client baml.llm.PrimitiveClient
    delay_ms int
}

/// Result of executing an orchestration branch.
class ExecutionResult {
    ok bool
    value unknown
}

/// Immutable metadata shared across orchestration attempts.
class ExecutionContext {
    jinja_string string
    args map<string, unknown>
    function_name string
}

/// Mutable planner-local state for side-effect-free `build_plan` expansion.
///
/// For each RR client name, we append one marker per selection inside a single
/// plan expansion. The number of prior markers for a client is its local offset.
class PlannerState {
    rr_client_names string[]

    function rr_local_offset(self, client_name: string) -> int {
        let offset = 0;
        for (let seen_name in self.rr_client_names) {
            if (seen_name == client_name) {
                offset += 1;
            }
        }
        offset
    }

    function rr_pick_index(self, client_name: string, sub_client_count: int) -> int {
        let local_offset = self.rr_local_offset(client_name);
        self.rr_client_names.push(client_name);
        let base = baml.llm.round_robin_peek(client_name);
        (base + local_offset) % sub_client_count
    }
}

/// Expand a client tree into steps for a single attempt (no retry wrapping).
///
/// - Primitive: resolves to one step.
/// - Fallback: concatenates children's full plans (including their own retries).
/// - RoundRobin: picks one child via atomic counter, returns that child's full plan.
///
/// Children are expanded with `build_plan` so they get their own retry wrapping.
/// This function is called once per retry attempt by `build_plan`, which means
/// RoundRobin picks a fresh child on each retry.
function build_attempt(llm_client: baml.llm.Client) -> baml.llm.OrchestrationStep[] {
    let planner_state = baml.llm.PlannerState { rr_client_names: [] };
    baml.llm.build_attempt_with_state(llm_client, planner_state)
}

/// Expand a client tree into steps for a single attempt using planner-local state.
function build_attempt_with_state(
    llm_client: baml.llm.Client,
    planner_state: baml.llm.PlannerState,
) -> baml.llm.OrchestrationStep[] {
    match (llm_client.client_type) {
        baml.llm.ClientType.Primitive => {
            let resolve_fn = baml.llm.resolve_client(llm_client.name);
            let primitive = resolve_fn();
            [baml.llm.OrchestrationStep { primitive_client: primitive, delay_ms: 0 }]
        }

        baml.llm.ClientType.Fallback => {
            let steps: baml.llm.OrchestrationStep[] = [];
            for (let sub in llm_client.sub_clients) {
                for (let step in baml.llm.build_plan_with_state(sub, planner_state)) {
                    steps.push(step);
                }
            }
            steps
        }

        baml.llm.ClientType.RoundRobin => {
            if (llm_client.sub_clients.length() == 0) {
                []
            } else {
                let idx = planner_state.rr_pick_index(
                    llm_client.name,
                    llm_client.sub_clients.length(),
                );
                baml.llm.build_plan_with_state(
                    llm_client.sub_clients.at(idx),
                    planner_state,
                )
            }
        }
    }
}

/// Build an orchestration plan from a Client tree.
///
/// Calls `build_attempt` per retry attempt so that stateful strategies
/// (round-robin) can rotate across retries — matching legacy behavior.
/// Each attempt re-expands the client tree, picking a fresh RR child.
///
/// For clients without a retry policy, this is a single call to `build_attempt`.
function build_plan(llm_client: baml.llm.Client) -> baml.llm.OrchestrationStep[] {
    let planner_state = baml.llm.PlannerState { rr_client_names: [] };
    baml.llm.build_plan_with_state(llm_client, planner_state)
}

/// Build a plan with planner-local state threaded through nested retries/composition.
function build_plan_with_state(
    llm_client: baml.llm.Client,
    planner_state: baml.llm.PlannerState,
) -> baml.llm.OrchestrationStep[] {
    let retry = llm_client.retry;
    if (retry == null) {
        return baml.llm.build_attempt_with_state(llm_client, planner_state);
    }

    let result: baml.llm.OrchestrationStep[] = [];
    let current_delay = retry.initial_delay_ms + 0.0;

    for (let attempt = 0; attempt <= retry.max_retries; attempt += 1) {
        let delay = 0;
        if (attempt > 0) {
            delay = baml.math.trunc(current_delay);
            let next = current_delay * retry.multiplier;
            if (next > retry.max_delay_ms + 0.0) {
                current_delay = retry.max_delay_ms + 0.0;
            } else {
                current_delay = next;
            }
        }

        let inner = baml.llm.build_attempt_with_state(llm_client, planner_state);
        for (let step in inner) {
            result.push(baml.llm.OrchestrationStep {
                primitive_client: step.primitive_client,
                delay_ms: delay,
            });
        }
    }

    result
}

/// Execute a client with retry semantics, evaluating strategies lazily.
///
/// `inherited_delay_ms` is the nearest retry delay from outer wrappers.
/// If this client has its own retry policy, it overrides that delay per attempt.
function execute_client(
    llm_client: baml.llm.Client,
    context: baml.llm.ExecutionContext,
    inherited_delay_ms: int,
) -> baml.llm.ExecutionResult {
    let retry = llm_client.retry;
    if (retry == null) {
        return baml.llm.execute_client_once(
            llm_client,
            context,
            inherited_delay_ms,
        );
    }

    let current_delay = retry.initial_delay_ms + 0.0;

    for (let attempt = 0; attempt <= retry.max_retries; attempt += 1) {
        let attempt_delay = inherited_delay_ms;
        if (attempt > 0) {
            attempt_delay = baml.math.trunc(current_delay);
            let next = current_delay * retry.multiplier;
            if (next > retry.max_delay_ms + 0.0) {
                current_delay = retry.max_delay_ms + 0.0;
            } else {
                current_delay = next;
            }
        }
        // Legacy behavior: the last local retry attempt carries no local
        // delay (fallback to inherited delay from outer wrappers, if any).
        if (attempt == retry.max_retries) {
            attempt_delay = inherited_delay_ms;
        }

        let result = baml.llm.execute_client_once(
            llm_client,
            context,
            attempt_delay,
        );

        if (result.ok) {
            return result;
        }
    }

    baml.llm.ExecutionResult { ok: false, value: null }
}

/// Execute a single attempt for a client (no retry expansion here).
function execute_client_once(
    llm_client: baml.llm.Client,
    context: baml.llm.ExecutionContext,
    active_delay_ms: int,
) -> baml.llm.ExecutionResult {
    match (llm_client.client_type) {
        baml.llm.ClientType.Primitive => {
            let resolve_fn = baml.llm.resolve_client(llm_client.name);
            let primitive = resolve_fn();

            let prompt = primitive.render_prompt(context.jinja_string, context.args);
            let specialized = primitive.specialize_prompt(prompt);
            let http_request = primitive.build_request(specialized);
            let http_response = baml.http.send(http_request);

            if (http_response.ok()) {
                let body = http_response.text();
                let return_type = baml.llm.get_return_type(context.function_name);
                return baml.llm.ExecutionResult {
                    ok: true,
                    value: primitive.parse(body, return_type),
                };
            }

            if (active_delay_ms > 0) {
                baml.sys.sleep(active_delay_ms);
            }

            baml.llm.ExecutionResult { ok: false, value: null }
        }

        baml.llm.ClientType.Fallback => {
            for (let sub in llm_client.sub_clients) {
                let result = baml.llm.execute_client(
                    sub,
                    context,
                    active_delay_ms,
                );
                if (result.ok) {
                    return result;
                }
            }
            baml.llm.ExecutionResult { ok: false, value: null }
        }

        baml.llm.ClientType.RoundRobin => {
            let idx = baml.llm.round_robin_next(llm_client.name) % llm_client.sub_clients.length();
            baml.llm.execute_client(
                llm_client.sub_clients.at(idx),
                context,
                active_delay_ms,
            )
        }
    }
}

/// Resolve the primitive client for a BAML function.
function resolve_primitive(function_name: string) -> baml.llm.PrimitiveClient {
    let llm_client = baml.llm.get_client(function_name);
    let resolve_fn = baml.llm.resolve_client(llm_client.name);
    resolve_fn()
}

/// Render a prompt for an LLM function.
function render_prompt(function_name: string, args: map<string, unknown>) -> baml.llm.PromptAst {
    let jinja_string = baml.llm.get_jinja_template(function_name);
    let primitive_client = baml.llm.resolve_primitive(function_name);
    let prompt = primitive_client.render_prompt(jinja_string, args);

    return primitive_client.specialize_prompt(prompt);
}

/// Build an HTTP request for an LLM function.
function build_request(function_name: string, args: map<string, unknown>) -> baml.http.Request {
    let primitive_client = baml.llm.resolve_primitive(function_name);
    let specialized_prompt = baml.llm.render_prompt(function_name, args);

    return primitive_client.build_request(specialized_prompt);
}

/// Call an LLM function end-to-end with full orchestration.
///
/// Handles top-level retry directly so that round-robin counters advance
/// at execution time (only when an attempt is actually reached), matching
/// legacy behavior. Nested strategies and retries are handled recursively
/// by `execute_client`.
function call_llm_function(function_name: string, args: map<string, unknown>) -> unknown throws string {
    let jinja_string = baml.llm.get_jinja_template(function_name);
    let llm_client = baml.llm.get_client(function_name);

    let context = baml.llm.ExecutionContext {
        jinja_string: jinja_string,
        args: args,
        function_name: function_name,
    };

    let result = baml.llm.execute_client(llm_client, context, 0);

    if (result.ok) {
        return result.value;
    }

    throw "All orchestration steps failed";
}

// ============================================================================
// Streaming Orchestration
// ============================================================================

/// Execute a streaming call against a single primitive client.
///
/// Opens an SSE connection, accumulates provider-specific events, and emits
/// partial values + raw ticks via the engine's streaming callbacks.
/// Returns the final parsed result as an `ExecutionResult`.
function stream_primitive(
    primitive: baml.llm.PrimitiveClient,
    context: baml.llm.ExecutionContext,
) -> baml.llm.ExecutionResult {
    let prompt = primitive.render_prompt(context.jinja_string, context.args);
    let specialized = primitive.specialize_prompt(prompt);
    let http_request = primitive.build_request_stream(specialized);
    let sse = baml.http.fetch_sse(http_request);
    let accumulator = primitive.new_stream_accumulator();
    let return_type = baml.llm.get_return_type(context.function_name);

    while (true) {
        let events = sse.next();
        if (events == null) { break; }
        accumulator.add_events(events);
        baml.stream.emit_tick(events);

        let content = accumulator.content();
        let parsed = primitive.partial_parse(content, return_type);
        baml.stream.emit_partial(parsed);

        if (accumulator.is_done()) {
            break;
        }
    }

    sse.close();

    if (accumulator.is_done() == false) {
        return baml.llm.ExecutionResult { ok: false, value: null };
    }

    let final_content = accumulator.content();
    let final_value = primitive.parse(final_content, return_type);
    baml.llm.ExecutionResult { ok: true, value: final_value }
}

/// Execute a streaming client with retry semantics, evaluating strategies lazily.
///
/// Mirrors `execute_client` but uses `stream_primitive` at the leaf level.
function execute_client_stream(
    llm_client: baml.llm.Client,
    context: baml.llm.ExecutionContext,
    inherited_delay_ms: int,
) -> baml.llm.ExecutionResult {
    let retry = llm_client.retry;
    if (retry == null) {
        return baml.llm.execute_client_once_stream(
            llm_client,
            context,
            inherited_delay_ms,
        );
    }

    let current_delay = retry.initial_delay_ms + 0.0;

    for (let attempt = 0; attempt <= retry.max_retries; attempt += 1) {
        let attempt_delay = inherited_delay_ms;
        if (attempt > 0) {
            attempt_delay = baml.math.trunc(current_delay);
            let next = current_delay * retry.multiplier;
            if (next > retry.max_delay_ms + 0.0) {
                current_delay = retry.max_delay_ms + 0.0;
            } else {
                current_delay = next;
            }
        }
        if (attempt == retry.max_retries) {
            attempt_delay = inherited_delay_ms;
        }

        let result = baml.llm.execute_client_once_stream(
            llm_client,
            context,
            attempt_delay,
        );

        if (result.ok) {
            return result;
        }
    }

    baml.llm.ExecutionResult { ok: false, value: null }
}

/// Execute a single streaming attempt for a client (no retry expansion here).
///
/// Mirrors `execute_client_once` but calls `stream_primitive` for primitive clients.
function execute_client_once_stream(
    llm_client: baml.llm.Client,
    context: baml.llm.ExecutionContext,
    active_delay_ms: int,
) -> baml.llm.ExecutionResult {
    match (llm_client.client_type) {
        baml.llm.ClientType.Primitive => {
            let resolve_fn = baml.llm.resolve_client(llm_client.name);
            let primitive = resolve_fn();

            let result = baml.llm.stream_primitive(primitive, context);

            if (result.ok) {
                return result;
            }

            if (active_delay_ms > 0) {
                baml.sys.sleep(active_delay_ms);
            }

            baml.llm.ExecutionResult { ok: false, value: null }
        }

        baml.llm.ClientType.Fallback => {
            for (let sub in llm_client.sub_clients) {
                let result = baml.llm.execute_client_stream(
                    sub,
                    context,
                    active_delay_ms,
                );
                if (result.ok) {
                    return result;
                }
            }
            baml.llm.ExecutionResult { ok: false, value: null }
        }

        baml.llm.ClientType.RoundRobin => {
            let idx = baml.llm.round_robin_next(llm_client.name) % llm_client.sub_clients.length();
            baml.llm.execute_client_stream(
                llm_client.sub_clients.at(idx),
                context,
                active_delay_ms,
            )
        }
    }
}

/// Stream an LLM function end-to-end with full orchestration.
///
/// The streaming counterpart of `call_llm_function`. Uses the same client tree
/// resolution and retry/fallback/round-robin logic, but opens SSE connections
/// and emits partial values via streaming callbacks.
function stream_llm_function(function_name: string, args: map<string, unknown>) -> unknown throws string {
    let jinja_string = baml.llm.get_jinja_template(function_name);
    let llm_client = baml.llm.get_client(function_name);

    let context = baml.llm.ExecutionContext {
        jinja_string: jinja_string,
        args: args,
        function_name: function_name,
    };

    let result = baml.llm.execute_client_stream(llm_client, context, 0);

    if (result.ok) {
        return result.value;
    }

    throw "All streaming orchestration steps failed";
}
