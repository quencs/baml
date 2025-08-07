use internal_baml_ast::ast::{Ast, HeaderCollector, HeaderIndex, MermaidDiagramGenerator};

fn main() {
    println!("=== Header Collector Example ===\n");
    println!("This example demonstrates the header collector data structures.\n");

    // Demonstrate simple HeaderCollector API
    demonstrate_simple_api();

    // Create an empty AST to test basic functionality
    test_empty_ast();

    println!("Example completed successfully!");
}

fn demonstrate_simple_api() {
    println!("1. Simple API overview:");
    println!("   - HeaderCollector::collect(&Ast) -> HeaderIndex");
    println!("   - HeaderIndex contains normalized levels and markdown parents per scope");
}

fn test_empty_ast() {
    println!("\n3. Testing with empty AST:");

    // Create an empty AST
    let ast = Ast { tops: Vec::new() };

    // Test header collection on empty AST
    let index: HeaderIndex = HeaderCollector::collect(&ast);
    println!("   Total headers found: {}", index.headers.len());
    let scopes: std::collections::HashSet<_> = index.headers.iter().map(|h| h.scope.0).collect();
    println!("   Scopes found: {}", scopes.len());

    // Test new mermaid visualization methods
    println!("\n4. Testing new Mermaid visualization methods:");

    println!("   Generating headers-only diagram (empty AST)...");
    let diagram = MermaidDiagramGenerator::generate_headers_diagram(&ast);
    println!("{}", diagram);
}

#[cfg(test)]
mod tests {
    use internal_baml_ast::ast::HeaderCollector;

    use super::*;

    #[test]
    fn test_empty_ast_collection() {
        let ast = Ast { tops: Vec::new() };
        let index = HeaderCollector::collect(&ast);
        assert_eq!(index.headers.len(), 0);
    }
}
