use std::{fs, path::Path};

use internal_baml_ast::{ast::HeaderCollector, parse};
use internal_baml_diagnostics::SourceFile;

fn main() {
    // Test with the complex headers test file
    let file_path = "examples/complex_headers_test.baml";
    let content = fs::read_to_string(file_path).expect("Failed to read test file");

    println!("=== TESTING HEADER COLLECTOR ===");
    println!("File: {}", file_path);
    println!("Content:\n{}", content);
    println!("\n=== PARSING AST ===");

    // Parse the AST
    let root_path = Path::new(".");
    let source_file = SourceFile::new_allocated(file_path.into(), content.into());
    let (ast, _diagnostics) = parse(root_path, &source_file).expect("Failed to parse AST");

    // Print the raw AST structure (focused on headers)
    println!("\n=== RAW AST STRUCTURE ===");
    dbg!(&ast);

    println!("\n=== COLLECTING HEADERS ===");

    // Collect headers using the new API
    let index = HeaderCollector::collect(&ast);

    println!("\n=== COLLECTED HEADERS ===");
    for h in &index.headers {
        println!(
            "- '{}' (L: {}) scope={} parent={:?}",
            h.title, h.level, h.scope.0, h.parent_id
        );
    }
}
