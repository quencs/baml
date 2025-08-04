// Run from the baml-schema-wasm folder with:
// RUST_LOG=info wasm-pack test --node --test test_cancellation
// and make sure to set rust-analyzer target in vscode settings to:   "rust-analyzer.cargo.target": "wasm32-unknown-unknown",

// Browser test command is:
// RUST_BACKTRACE=1 RUST_LOG=info wasm-pack test --chrome --headless --test test_cancellation -- --nocapture
#[cfg(target_arch = "wasm32")]
#[cfg(test)]
mod tests {

    use baml_schema_build::runtime_wasm::{
        CanceledError, WasmCancellationToken, WasmProject, WasmRuntime, WasmTestResponses,
    };
    use js_sys::{Array, Object};
    use serde_wasm_bindgen;
    use std::collections::HashMap;
    use std::future::Future;
    use tokio_util::sync::CancellationToken;
    use wasm_bindgen::JsValue;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    async fn cancel_request_inside() {
        let will_be_canceled = CancellationToken::new();

        // select between run tests & token, but make select! poll run_tests first.
        let run_tests = setup_run_tests_future(will_be_canceled.clone());

        let (result, _) = futures::join!(run_tests, async move { will_be_canceled.cancel() });

        let list = result.unwrap().into_inner();

        let first_err = list[0].result().as_ref().unwrap_err();

        assert!(first_err.downcast_ref::<CanceledError>().is_some());
    }

    #[wasm_bindgen_test]
    async fn cancel_request_outside() {
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        enum WhichReturned {
            Request,
            Token,
        }

        let will_be_canceled = CancellationToken::new();

        // select between run tests & token, but make select! poll run_tests first.
        let response_token_below = {
            let tok = will_be_canceled.clone();
            // use a blank, never accessed token so that the inner select doesn't cancel - this will
            // allow us to act as-if we didn't have any cancellation tokens inside.
            let run_tests = setup_run_tests_future(CancellationToken::new());
            async move {
                tokio::select! {
                    _ = run_tests => WhichReturned::Request,
                    _ = tok.cancelled() => WhichReturned::Token,
                }
            }
        };

        let (which, _) = futures::join!(
            response_token_below,
            async move { will_be_canceled.cancel() }
        );

        assert_eq!(which, WhichReturned::Token);
    }

    #[wasm_bindgen_test]
    async fn run_tests_works() {
        let resp = setup_run_tests_future(CancellationToken::new())
            .await
            .unwrap()
            .into_inner();

        // should have "invalid authentication", which means that the request was made.
        let resp = resp[0].result().as_ref().unwrap_err().to_string();

        assert!(resp.starts_with("InvalidAuthentication"));
    }

    fn setup_run_tests_future(
        cancel_token_src: CancellationToken,
    ) -> impl Future<Output = Result<WasmTestResponses, JsValue>> {
        let project = create_sample_project();
        let mut runtime = runtime_from_project(&project);
        let cancel_token_js = WasmCancellationToken::from_shared(cancel_token_src.clone());

        // Create function-test pairs
        let function_test_pairs = Array::new();
        let pair = Object::new();
        js_sys::Reflect::set(
            &pair,
            &JsValue::from_str("functionName"),
            &JsValue::from_str("ExtractResume"),
        )
        .unwrap();
        js_sys::Reflect::set(
            &pair,
            &JsValue::from_str("testName"),
            &JsValue::from_str("Test1"),
        )
        .unwrap();
        function_test_pairs.push(&pair);

        // Create callback functions
        // TODO: remove these (make them null)
        let on_test_start = js_sys::Function::new_no_args("console.log('Test started')");
        let on_test_end = js_sys::Function::new_no_args("console.log('Test ended')");
        let options = Object::new();

        async move {
            runtime
                .run_tests(
                    function_test_pairs,
                    on_test_start,
                    on_test_end,
                    options,
                    cancel_token_js,
                )
                .await
        }
    }

    /// Creates a `WasmRuntime` object from a project with an empty environment.
    fn runtime_from_project(project: &WasmProject) -> WasmRuntime {
        project
            .runtime(serde_wasm_bindgen::to_value(&HashMap::<String, String>::new()).unwrap())
            .unwrap()
    }

    /// Creates a project from the sample files in `tests/sample-project`.
    fn create_sample_project() -> WasmProject {
        // Read the BAML files
        let clients_content = include_str!("./sample-project/baml_src/clients.baml");
        let generators_content = include_str!("./sample-project/baml_src/generators.baml");
        let resume_content = include_str!("./sample-project/baml_src/resume.baml");

        // Create a HashMap with file contents
        let mut files = std::collections::HashMap::new();
        files.insert("clients.baml".to_string(), clients_content);
        files.insert("generators.baml".to_string(), generators_content);
        files.insert("resume.baml".to_string(), resume_content);

        let Ok(project) = WasmProject::new(
            "./sample-project/baml_src",
            serde_wasm_bindgen::to_value(&files).unwrap(),
        ) else {
            panic!("failed to create project");
        };

        project
    }
}
