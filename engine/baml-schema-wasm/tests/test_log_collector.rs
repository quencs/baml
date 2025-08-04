// Run from the baml-schema-wasm folder with:
// RUST_LOG=info wasm-pack test --node --test test_log_collector
// and make sure to set rust-analyzer target in vscode settings to:   "rust-analyzer.cargo.target": "wasm32-unknown-unknown",

// Browser test command is:
// RUST_BACKTRACE=1 RUST_LOG=info wasm-pack test --chrome --headless --test test_log_collector -- --nocapture
#[cfg(target_arch = "wasm32")]
#[cfg(test)]
mod tests {

    use std::collections::HashMap;

    use baml_runtime::tracingv2::{publisher::publisher::flush, storage::storage::BAML_TRACER};
    use baml_schema_build::runtime_wasm::WasmProject;
    use serde_wasm_bindgen::to_value;
    use wasm_bindgen::JsValue;
    use wasm_bindgen_test::*;

    // instantiate logger

    wasm_bindgen_test_configure!(run_in_browser);

    // TODO: add a flag to run_test that's debug, to attach a collector on every test. The collector can be created inside run_test. Make the collector call track_function(..)
    // Then we can check the baml_tracer events since they'll be kept.
    #[wasm_bindgen_test]
    async fn test_run_tests() {
        wasm_logger::init(wasm_logger::Config::new(log::Level::Info));
        let sample_baml_content = r##"
    function Func(name: string ) -> string {
            client "openai/gpt-4o"
            prompt #"
            Return the name of {{name}}
            "#
    }

    test One {
        functions [Func]
        args {
            name "john"
        }
    }


            "##;
        let mut files = HashMap::new();
        files.insert("error.baml".to_string(), sample_baml_content.to_string());
        let files_js = to_value(&files).unwrap();
        let project = WasmProject::new("baml_src", files_js)
            .map_err(JsValue::from)
            .unwrap();

        let env_vars = [("OPENAI_API_KEY", "12345")]
            .iter()
            .cloned()
            .collect::<HashMap<_, _>>();
        let env_vars_js = to_value(&env_vars).unwrap();

        let mut current_runtime = project.runtime(env_vars_js.clone()).unwrap();

        let diagnostics = project.diagnostics(&current_runtime);
        assert!(diagnostics.errors().is_empty());

        let functions = current_runtime.list_functions();

        for f in functions.iter() {
            f.run_test(
                &mut current_runtime,
                "One".to_string(),
                js_sys::Function::new_no_args(""),
                js_sys::Function::new_no_args(""),
                env_vars_js.clone().into(),
            )
            .await;
        }

        let events = BAML_TRACER.lock().unwrap().events();
        log::info!("Events {events:#?}");
        // TODO: this makes the test hang on Node, but not in browser.
        flush().await;

        log::info!("done!!")
    }
}
