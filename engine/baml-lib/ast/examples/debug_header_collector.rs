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

    // Debug: Print all headers with their details
    println!("=== All Headers Debug ===");
    let all_headers = header_tree.all_headers();
    for (i, header) in all_headers.iter().enumerate() {
        println!(
            "Header {}: '{}' (Level: {})",
            i,
            header.title(),
            header.level()
        );
        println!("  AST Path: {}", header.ast_path_string());
        println!("  AST Parent: {:?}", header.ast_parent);
        println!("  Header Parent: {}", header.has_header_parent());
        println!("  Span: {:?}", header.span);
        println!();
    }

    // Debug: Check if hierarchy is enabled
    println!("=== Configuration Debug ===");
    let config = internal_baml_ast::ast::HeaderCollectorConfig::default();
    println!("Preserve hierarchy: {}", config.preserve_hierarchy);
    println!("Include context: {}", config.include_context);
    println!("Track AST paths: {}", config.track_ast_paths);

    // Debug: Check roots
    println!("=== Roots Debug ===");
    println!("Number of root headers: {}", header_tree.roots.len());
    for (i, root) in header_tree.roots.iter().enumerate() {
        println!("Root {}: '{}' (Level: {})", i, root.title(), root.level());
    }
}
