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
    match (llm_client.client_type) {
        baml.llm.ClientType.Primitive => {
            let resolve_fn = baml.llm.resolve_client(llm_client.name);
            let primitive = resolve_fn();
            [baml.llm.OrchestrationStep { primitive_client: primitive, delay_ms: 0 }]
        }
        baml.llm.ClientType.Fallback => {
            let steps: baml.llm.OrchestrationStep[] = [];
            for (let sub in llm_client.sub_clients) {
                for (let step in baml.llm.build_plan(sub)) {
                    steps.push(step);
                }
            }
            steps
        }
        baml.llm.ClientType.RoundRobin => {
            let idx = baml.llm.round_robin_next(llm_client.name) % llm_client.sub_clients.length();
            baml.llm.build_plan(llm_client.sub_clients.at(idx))
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
    match (llm_client.retry) {
        null => baml.llm.build_attempt(llm_client),
        r: baml.llm.RetryPolicy => {
            let result: baml.llm.OrchestrationStep[] = [];
            let current_delay = r.initial_delay_ms + 0.0;

            for (let attempt = 0; attempt <= r.max_retries; attempt += 1) {
                let delay = 0;
                if (attempt > 0) {
                    delay = baml.math.trunc(current_delay);
                    let next = current_delay * r.multiplier;
                    if (next > r.max_delay_ms + 0.0) {
                        current_delay = r.max_delay_ms + 0.0;
                    } else {
                        current_delay = next;
                    }
                }

                let inner = baml.llm.build_attempt(llm_client);
                for (let step in inner) {
                    result.push(baml.llm.OrchestrationStep {
                        primitive_client: step.primitive_client,
                        delay_ms: delay,
                    });
                }
            }

            result
        }
    }
}

/// Render a prompt for an LLM function.
///
/// Looks up the Jinja template and the client's resolve function,
/// renders the prompt, and specializes it for the provider.
function render_prompt(function_name: string, args: map<string, unknown>) -> baml.llm.PromptAst {
    let jinja_string = baml.llm.get_jinja_template(function_name);

    let llm_client = baml.llm.get_client(function_name);
    let resolve_fn = baml.llm.resolve_client(llm_client.name);
    let primitive_client = resolve_fn();

    let prompt = primitive_client.render_prompt(jinja_string, args);
    return primitive_client.specialize_prompt(prompt);
}

/// Build an HTTP request for an LLM function.
///
/// Renders the prompt and builds a provider-specific HTTP request.
function build_request(function_name: string, args: map<string, unknown>) -> baml.http.Request {
    let jinja_string = baml.llm.get_jinja_template(function_name);

    let llm_client = baml.llm.get_client(function_name);
    let resolve_fn = baml.llm.resolve_client(llm_client.name);
    let primitive_client = resolve_fn();

    let prompt = primitive_client.render_prompt(jinja_string, args);
    let specialized_prompt = primitive_client.specialize_prompt(prompt);

    return primitive_client.build_request(specialized_prompt);
}

/// Call an LLM function end-to-end with full orchestration.
///
/// Handles top-level retry directly so that round-robin counters advance
/// at execution time (only when an attempt is actually reached), matching
/// legacy behavior. Children's retry is still handled by `build_plan`.
function call_llm_function(function_name: string, args: map<string, unknown>) -> unknown {
    let jinja_string = baml.llm.get_jinja_template(function_name);
    let llm_client = baml.llm.get_client(function_name);

    let max_attempts = match (llm_client.retry) {
        null => 1,
        r: baml.llm.RetryPolicy => r.max_retries + 1
    };
    let current_delay = match (llm_client.retry) {
        null => 0.0,
        r: baml.llm.RetryPolicy => r.initial_delay_ms + 0.0
    };

    for (let attempt = 0; attempt < max_attempts; attempt += 1) {
        if (attempt > 0) {
            baml.sys.sleep(baml.math.trunc(current_delay));
            match (llm_client.retry) {
                null => {},
                r: baml.llm.RetryPolicy => {
                    let next = current_delay * r.multiplier;
                    if (next > r.max_delay_ms + 0.0) {
                        current_delay = r.max_delay_ms + 0.0;
                    } else {
                        current_delay = next;
                    }
                }
            };
        }

        let steps = baml.llm.build_attempt(llm_client);
        for (let step in steps) {
            let prompt = step.primitive_client.render_prompt(jinja_string, args);
            let specialized = step.primitive_client.specialize_prompt(prompt);
            let http_request = step.primitive_client.build_request(specialized);
            let http_response = baml.http.send(http_request);

            if (http_response.ok()) {
                let body = http_response.text();
                let return_type = baml.llm.get_return_type(function_name);
                return step.primitive_client.parse(body, return_type);
            }
        }
    }

    // TODO: This will have to be replaced by a "throw" when exceptions land.
    // TODO: Also figure out panics in Baml.
    baml.sys.panic("All orchestration steps failed");
}
