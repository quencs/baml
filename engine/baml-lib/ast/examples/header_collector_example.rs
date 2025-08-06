use internal_baml_ast::ast::{
    ASTContext, Ast, HeaderCollectorConfig, HeaderTree, MermaidDiagramGenerator,
};

fn main() {
    println!("=== Header Collector Example ===\n");
    println!("This example demonstrates the header collector data structures.\n");

    // Demonstrate the different header contexts
    demonstrate_header_contexts();

    // Demonstrate header collector configuration
    demonstrate_collector_config();

    // Create an empty AST to test basic functionality
    test_empty_ast();

    println!("Example completed successfully!");
}

fn demonstrate_header_contexts() {
    println!("1. AST Context Types:");

    let function_context = ASTContext::TopLevel("function".to_string());
    println!("   Function context: {:?}", function_context);

    let expr_function_context = ASTContext::TopLevel("expr_function".to_string());
    println!("   ExprFunction context: {:?}", expr_function_context);

    let statement_context = ASTContext::Statement;
    println!("   Statement context: {:?}", statement_context);

    let final_expr_context = ASTContext::ExpressionBlockFinal;
    println!("   ExpressionBlockFinal context: {:?}", final_expr_context);
}

fn demonstrate_collector_config() {
    println!("\n2. Header Collector Configuration:");

    let default_config = HeaderCollectorConfig::default();
    println!("   Default config: {:?}", default_config);

    let custom_config = HeaderCollectorConfig {
        preserve_hierarchy: false,
        include_context: false,
        track_ast_paths: false,
    };
    println!("   Custom config: {:?}", custom_config);

    let hierarchy_only_config = HeaderCollectorConfig {
        preserve_hierarchy: true,
        include_context: false,
        track_ast_paths: false,
    };
    println!("   Hierarchy-only config: {:?}", hierarchy_only_config);
}

fn test_empty_ast() {
    println!("\n3. Testing with empty AST:");

    // Create an empty AST
    let ast = Ast { tops: Vec::new() };

    // Test header collection on empty AST
    let header_tree = internal_baml_ast::ast::HeaderCollector::collect_headers(&ast);

    println!(
        "   Total headers found: {}",
        header_tree.all_headers().len()
    );
    println!("   Root headers: {}", header_tree.root_headers().len());
    println!(
        "   Contexts found: {}",
        header_tree.headers_by_context.len()
    );

    // Test the tree string representation
    let tree_string = header_tree.to_tree_string();
    println!("   Tree representation:\n{}", tree_string);

    // Test querying methods
    let function_headers = header_tree.headers_in_functions();
    println!("   Headers in functions: {}", function_headers.len());

    let final_expr_headers = header_tree.headers_for_final_expressions();
    println!(
        "   Headers for final expressions: {}",
        final_expr_headers.len()
    );

    let header_by_title = header_tree.find_header_by_title("NonExistentHeader");
    println!("   Found header by title: {}", header_by_title.is_some());

    // Test new mermaid visualization methods
    println!("\n4. Testing new Mermaid visualization methods:");

    // TODO: Implement these new methods for header visualization
    println!("   Note: Header visualization methods not yet implemented");
    // Test the new header collector-based diagrams
    // println!("   Generating header tree diagram using HeaderCollector...");
    // let tree_diagram = MermaidDiagramGenerator::generate_headers_diagram_from_collector(&ast);
    // println!("   Header tree diagram:\n{}", tree_diagram);

    // println!("   Generating header hierarchy diagram...");
    // let hierarchy_diagram = MermaidDiagramGenerator::generate_header_hierarchy_diagram(&ast);
    // println!("   Header hierarchy diagram:\n{}", hierarchy_diagram);

    // println!("   Generating header context diagram...");
    // let context_diagram = MermaidDiagramGenerator::generate_header_context_diagram(&ast);
    // println!("   Header context diagram:\n{}", context_diagram);
}

#[cfg(test)]
mod tests {
    use internal_baml_ast::ast::{HeaderCollector, HeaderCollectorConfig};

    use super::*;

    #[test]
    fn test_empty_ast_collection() {
        let ast = Ast { tops: Vec::new() };
        let header_tree = HeaderCollector::collect_headers(&ast);

        // Empty AST should have no headers
        assert_eq!(header_tree.all_headers().len(), 0);
        assert_eq!(header_tree.root_headers().len(), 0);
        assert_eq!(header_tree.headers_by_context.len(), 0);
    }

    #[test]
    fn test_header_collector_config() {
        let default_config = HeaderCollectorConfig::default();
        assert!(default_config.preserve_hierarchy);
        assert!(default_config.include_context);
        assert!(default_config.track_ast_paths);

        let custom_config = HeaderCollectorConfig {
            preserve_hierarchy: false,
            include_context: false,
            track_ast_paths: false,
        };
        assert!(!custom_config.preserve_hierarchy);
        assert!(!custom_config.include_context);
        assert!(!custom_config.track_ast_paths);
    }

    #[test]
    fn test_ast_context_types() {
        let function_context = ASTContext::TopLevel("function".to_string());
        let expr_function_context = ASTContext::TopLevel("expr_function".to_string());
        let statement_context = ASTContext::Statement;
        let final_expr_context = ASTContext::ExpressionBlockFinal;

        // Test that contexts are different
        assert_ne!(function_context, expr_function_context);
        assert_ne!(function_context, statement_context);
        assert_ne!(statement_context, final_expr_context);

        // Test that same contexts are equal
        let another_function_context = ASTContext::TopLevel("function".to_string());
        assert_eq!(function_context, another_function_context);
    }

    #[test]
    fn test_header_tree_query_methods() {
        let ast = Ast { tops: Vec::new() };
        let header_tree = HeaderCollector::collect_headers(&ast);

        // Test query methods on empty tree
        assert!(header_tree.headers_in_functions().is_empty());
        assert!(header_tree.headers_for_final_expressions().is_empty());
        assert!(header_tree.find_header_by_title("test").is_none());

        let context = ASTContext::Statement;
        assert!(header_tree.headers_in_context(&context).is_empty());
    }
}
