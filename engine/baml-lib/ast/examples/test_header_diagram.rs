use std::{fs, path::Path};

use internal_baml_ast::{ast::MermaidDiagramGenerator, parse};
use internal_baml_diagnostics::SourceFile;

fn main() {
    // Test with the complex headers test file
    let file_path = "examples/complex_headers_test.baml";
    let content = fs::read_to_string(file_path).expect("Failed to read test file");

    println!("=== TESTING HEADER DIAGRAM ===");
    println!("File: {}", file_path);
    println!("Content:\n{}", content);
    println!("\n=== PARSING AST ===");

    // Parse the AST
    let root_path = Path::new(".");
    let source_file = SourceFile::new_allocated(file_path.into(), content.into());
    let (ast, _diagnostics) = parse(root_path, &source_file).expect("Failed to parse AST");

    println!("\n=== GENERATING HEADER DIAGRAM ===");

    // Generate header-specific mermaid diagram
    let header_diagram = MermaidDiagramGenerator::generate_headers_diagram(&ast);

    println!("Generated Header Mermaid Diagram:");
    println!("{}", header_diagram);

    println!("\n=== HEADER DIAGRAM (NO STYLING) ===");

    // Also generate without styling for cleaner output
    let header_diagram_no_style =
        MermaidDiagramGenerator::generate_headers_diagram_with_styling(&ast, false);

    println!("Generated Header Mermaid Diagram (No Styling):");
    println!("{}", header_diagram_no_style);

    println!("\nTo visualize this diagram:");
    println!("1. Copy the diagram output above");
    println!("2. Go to https://mermaid.live/");
    println!("3. Paste the diagram code");
    println!("4. View the rendered header diagram!");
}
