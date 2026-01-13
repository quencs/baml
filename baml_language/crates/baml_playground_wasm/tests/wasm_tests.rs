//! WASM integration tests for baml_playground_wasm
//!
//! Run with: wasm-pack test --node

use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_node_experimental);

use baml_playground_wasm::{BamlProgram, hot_reload_test_string, version};

#[wasm_bindgen_test]
fn test_version_returns_string() {
    let ver = version();
    assert!(!ver.is_empty(), "version should not be empty");
}

#[wasm_bindgen_test]
fn test_hot_reload_test_string() {
    let s = hot_reload_test_string();
    assert!(
        s.contains("hot reload test"),
        "should contain hot reload marker"
    );
}

#[wasm_bindgen_test]
fn test_baml_program_new() {
    let program = BamlProgram::new("".to_string());
    let names = program.function_names();
    assert!(names.is_empty(), "empty source should have no functions");
}

#[wasm_bindgen_test]
fn test_baml_program_function_names() {
    let program = BamlProgram::new(
        r##"
        function MyFunc(input: string) -> string {
            client "openai/gpt-4o"
            prompt #"Hello"#
        }
    "##
        .to_string(),
    );
    let names = program.function_names();
    assert!(names.contains(&"MyFunc".to_string()), "should find MyFunc");
}

#[wasm_bindgen_test]
fn test_baml_program_set_source() {
    let mut program = BamlProgram::new("".to_string());
    assert!(program.function_names().is_empty());

    program.set_source(
        r##"
        function AnotherFunc(x: int) -> int {
            client "openai/gpt-4o"
            prompt #"Hello"#
        }
    "##
        .to_string(),
    );

    let names = program.function_names();
    assert!(
        names.contains(&"AnotherFunc".to_string()),
        "should find AnotherFunc after set_source"
    );
}
