mod baml_runtime;
mod hot_reload_testdata;

pub use baml_runtime::BamlRuntime;
pub use hot_reload_testdata::hot_reload_test_string;
use wasm_bindgen::prelude::*;

#[cfg(feature = "console_error_panic")]
extern crate console_error_panic_hook;

#[cfg(feature = "small_allocator")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen(start)]
pub fn start() {
    #[cfg(feature = "console_error_panic")]
    console_error_panic_hook::set_once();
}

/// Returns the version of the BAML compiler.
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}
