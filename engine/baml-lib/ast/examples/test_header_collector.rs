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

    // Collect headers using our refactored collector
    let header_tree = HeaderCollector::collect_headers(&ast);

    // Print the header tree
    println!("=== Header Tree Structure ===");
    println!("{}", header_tree.to_tree_string());

    // Print headers by context
    println!("\n=== Headers in Functions ===");
    let function_headers = header_tree.headers_in_functions();
    for header in function_headers {
        println!(
            "Function Header: {} (Level: {}, AST Path: {})",
            header.title(),
            header.level(),
            header.ast_path_string()
        );
    }

    // Print headers for final expressions
    println!("\n=== Headers for Final Expressions ===");
    let final_headers = header_tree.headers_for_final_expressions();
    for header in final_headers {
        println!(
            "Final Expression Header: {} (Level: {}, AST Path: {})",
            header.title(),
            header.level(),
            header.ast_path_string()
        );
    }

    // Print all headers with hierarchy information
    println!("\n=== All Headers with Hierarchy ===");
    let all_headers = header_tree.all_headers();
    for header in all_headers {
        println!(
            "Header: {} (Level: {}) - AST Parent: {:?}, Header Parent: {}",
            header.title(),
            header.level(),
            header.ast_parent,
            header.has_header_parent()
        );
    }
}
