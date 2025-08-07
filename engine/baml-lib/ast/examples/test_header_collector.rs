use std::{fs, path::Path};

use internal_baml_ast::{
    ast::header_collector::{HeaderCollector, HeaderCollectorConfig},
    parse,
};
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

    // Collect headers using the header collector
    let header_tree = HeaderCollector::collect_headers(&ast);

    // Print collected headers
    println!("\n=== COLLECTED HEADERS ===");
    dbg!(&header_tree);

    println!("\n=== HEADER TREE STRING ===");
    println!("{}", header_tree.to_tree_string());

    println!("\n=== HEADERS BY CONTEXT ===");
    for (context, headers) in &header_tree.headers_by_context {
        println!("Context: {:?}", context);
        for header in headers {
            println!(
                "  - {} (Level: {}) at path: {}",
                header.title(),
                header.level(),
                header.ast_path_string()
            );
        }
    }

    println!("\n=== VERIFICATION ===");
    // Let's manually verify some key headers
    let all_headers = header_tree.all_headers();
    println!("Total headers collected: {}", all_headers.len());

    // Look for specific headers we expect
    for header in all_headers {
        println!(
            "Header: '{}' | Level: {} | Context: {:?} | Path: {}",
            header.title(),
            header.level(),
            header.ast_context,
            header.ast_path_string()
        );
    }
}
