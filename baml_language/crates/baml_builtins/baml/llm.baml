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

/// Build an orchestration plan from a Client tree.
///
/// Recursively expands composite clients (fallback, round-robin) into a flat
/// list of baml.llm.OrchestrationSteps, wrapping with retry delays as configured.
function build_plan(c: baml.llm.Client) -> baml.llm.OrchestrationStep[] {
    let retry = c.retry;

    let inner = match (c.client_type) {
        baml.llm.ClientType.Primitive => {
            let resolve_fn = baml.llm.resolve_client(c.name);
            let primitive = resolve_fn();
            [baml.llm.OrchestrationStep { primitive_client: primitive, delay_ms: 0 }]
        }
        baml.llm.ClientType.Fallback => {
            let steps: baml.llm.OrchestrationStep[] = [];
            for (let sub in c.sub_clients) {
                for (let step in baml.llm.build_plan(sub)) {
                    steps.push(step);
                }
            }
            steps
        }
        baml.llm.ClientType.RoundRobin => {
            let idx = baml.llm.round_robin_next(c.name) % c.sub_clients.length();
            baml.llm.build_plan(c.sub_clients.at(idx))
        }
    };

    return baml.llm.wrap_with_retry(inner, retry);
}

/// Wrap a list of orchestration steps with retry logic.
///
/// Duplicates the inner steps for each retry attempt, adding exponential
/// backoff delays between attempts.
///
/// Note: the retry delay is applied uniformly to all steps within a retry
/// attempt. This is intentional — the delay represents the backoff before
/// re-attempting the entire inner sequence, not between individual sub-steps.
function wrap_with_retry(inner: baml.llm.OrchestrationStep[], retry: baml.llm.RetryPolicy?) -> baml.llm.OrchestrationStep[] {
    match (retry) {
        null => inner,
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

    let c = baml.llm.get_client(function_name);
    let resolve_fn = baml.llm.resolve_client(c.name);
    let primitive_client = resolve_fn();

    let prompt = primitive_client.render_prompt(jinja_string, args);
    return primitive_client.specialize_prompt(prompt);
}

/// Build an HTTP request for an LLM function.
///
/// Renders the prompt and builds a provider-specific HTTP request.
function build_request(function_name: string, args: map<string, unknown>) -> baml.http.Request {
    let jinja_string = baml.llm.get_jinja_template(function_name);

    let c = baml.llm.get_client(function_name);
    let resolve_fn = baml.llm.resolve_client(c.name);
    let primitive_client = resolve_fn();

    let prompt = primitive_client.render_prompt(jinja_string, args);
    let specialized_prompt = primitive_client.specialize_prompt(prompt);

    return primitive_client.build_request(specialized_prompt);
}

/// Call an LLM function end-to-end with full orchestration.
///
/// Builds an orchestration plan from the function's client tree, then
/// executes each step in order. For simple clients this is a single call;
/// for fallback/round-robin clients with retry policies, this implements
/// the full retry + fallback loop.
function call_llm_function(function_name: string, args: map<string, unknown>) -> unknown {
    let jinja_string = baml.llm.get_jinja_template(function_name);
    let c = baml.llm.get_client(function_name);
    let plan = baml.llm.build_plan(c);

    for (let step in plan) {
        let prompt = step.primitive_client.render_prompt(jinja_string, args);
        let specialized = step.primitive_client.specialize_prompt(prompt);
        let http_request = step.primitive_client.build_request(specialized);
        let http_response = baml.http.send(http_request);

        if (http_response.ok()) {
            let body = http_response.text();
            return step.primitive_client.parse(body, function_name);
        }

        if (step.delay_ms > 0) {
            baml.sys.sleep(step.delay_ms);
        }
    }

    baml.sys.panic("All orchestration steps failed");
}
