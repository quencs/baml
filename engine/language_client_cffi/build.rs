use std::path::Path;

use flatc_rust;

fn main() {
    let args = flatc_rust::Args {
        lang: "rust",
        inputs: &[Path::new("types/cffi.fbs")],
        out_dir: Path::new("src/cffi"),
        ..Default::default()
    };
    flatc_rust::run(args).expect("Failed to generate Rust bindings");

    let args: flatc_rust::Args<'_> = flatc_rust::Args {
        lang: "go",
        inputs: &[Path::new("types/cffi.fbs")],
        out_dir: Path::new("../language_client_go/go-sdk"),
        ..Default::default()
    };
    flatc_rust::run(args).expect("Failed to generate Rust bindings");
}
