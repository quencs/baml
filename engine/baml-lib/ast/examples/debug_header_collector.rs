use std::path::Path;

use internal_baml_ast::{ast::HeaderCollector, parse};
use internal_baml_diagnostics::SourceFile;

fn main() {
    // Read the complex headers test file
    let baml_content = std::fs::read_to_string("examples/complex_headers_test.baml")
        .expect("Failed to read test file");

    // Create a SourceFile and parse the BAML file
    let source = SourceFile::from((
        Path::new("complex_headers_test.baml").to_path_buf(),
        baml_content,
    ));
    let root_path = Path::new(".");
    let (ast, _diagnostics) = parse(root_path, &source).expect("Failed to parse BAML file");

    // Collect headers using simplified API
    let index = HeaderCollector::collect(&ast);

    println!("=== All Headers Debug ===");
    for (i, h) in index.headers.iter().enumerate() {
        println!(
            "Header {}: '{}' (L: {}) @ {}:{}-{} | scope={} | parent={:?}",
            i,
            h.title,
            h.level,
            h.span.file.path(),
            h.span.start,
            h.span.end,
            h.scope.0,
            h.parent_id
        );
    }
}
