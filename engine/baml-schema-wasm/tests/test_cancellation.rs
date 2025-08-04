// Run from the baml-schema-wasm folder with:
// RUST_LOG=info wasm-pack test --node --test test_cancellation
// and make sure to set rust-analyzer target in vscode settings to:   "rust-analyzer.cargo.target": "wasm32-unknown-unknown",

// Browser test command is:
// RUST_BACKTRACE=1 RUST_LOG=info wasm-pack test --chrome --headless --test test_cancellation -- --nocapture
#[cfg(target_arch = "wasm32")]
#[cfg(test)]
mod tests {

    use std::collections::HashMap;
    use std::future::Future;
    use baml_schema_build::runtime_wasm::{WasmCancellationToken, WasmRuntime, WasmProject, CanceledError, WasmTestResponses};
    use js_sys::{Array, Object};
    use tokio_util::sync::CancellationToken;
    use wasm_bindgen::JsValue;
    use wasm_bindgen_test::*;
    use serde_wasm_bindgen;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    async fn test_concurrent_run() {
        wasm_logger::init(wasm_logger::Config::new(log::Level::Info));

        let cancel_token_src = CancellationToken::new();

        let waiting_fut = cancel_token_src.cancelled();

        eprintln!("executing");

        cancel_token_src.cancel();

        waiting_fut.await;

        // we can only get here if token is cancelled, which can only happen
        // if the two tasks run concurrently.
    }



    #[wasm_bindgen_test]
    async fn test_cancel_before_await() {
        eprintln!("Starting test_cancel_llm_call");
        

        let cancel_token_src = CancellationToken::new();

        let run_tests_future = setup_run_tests_future(cancel_token_src.clone());

        // If we cancel before awaiting the run_tests future, canceling works
        cancel_token_src.cancel();

        let results = run_tests_future.await.unwrap().into_inner();

        let e = results[0].result().as_ref().unwrap_err();

        assert!(e.downcast_ref::<CanceledError>().is_some());
        
    }

    #[wasm_bindgen_test]
    async fn test_cancel_concurrent() {
        eprintln!("Starting test_cancel_llm_call");
        

        let cancel_token_src = CancellationToken::new();

        let cancel_future = {
            let copy = cancel_token_src.clone();
            async move { copy.cancel() }
        };
        let run_tests_future = setup_run_tests_future(cancel_token_src.clone());

        let (_, test_result) = futures::join!(cancel_future, run_tests_future);

        let results = test_result.unwrap().into_inner();

        let e = results[0].result().as_ref().unwrap_err();

        assert!(e.downcast_ref::<CanceledError>().is_some());
        
    }

    fn setup_run_tests_future(cancel_token_src: CancellationToken) -> impl Future<Output = Result<WasmTestResponses, JsValue>> {
        let project = create_sample_project();
        let mut runtime = runtime_from_project(&project);
        let cancel_token_js = WasmCancellationToken::from_shared(cancel_token_src.clone());


        // Create function-test pairs
        let function_test_pairs = Array::new();
        let pair = Object::new();
        js_sys::Reflect::set(&pair, &JsValue::from_str("function"), &JsValue::from_str("ExtractResume")).unwrap();
        js_sys::Reflect::set(&pair, &JsValue::from_str("test"), &JsValue::from_str("Test1")).unwrap();
        function_test_pairs.push(&pair);

        // Create callback functions
        // TODO: remove these (make them null)
        let on_test_start = js_sys::Function::new_no_args("console.log('Test started')");
        let on_test_end = js_sys::Function::new_no_args("console.log('Test ended')");
        let options = Object::new();

        async move { runtime.run_tests(function_test_pairs, on_test_start, on_test_end, options, cancel_token_js).await }
    }


    /// Creates a `WasmRuntime` object from a project with an empty environment.
    fn runtime_from_project(project: &WasmProject) -> WasmRuntime {
        project.runtime(serde_wasm_bindgen::to_value(&HashMap::<String, String>::new()).unwrap()).unwrap()
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
        
        let Ok(project) = WasmProject::new("./sample-project/baml_src", serde_wasm_bindgen::to_value(&files).unwrap()) else {
            panic!("failed to create project");
        };

        project
    }
}
