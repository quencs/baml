# Header Collector

The Header Collector is a utility module for extracting and organizing headers (markdown-style headers like `#`, `##`, etc.) from BAML AST structures into a structured, queryable data structure.

## Overview

The header collector walks through the entire AST and extracts all headers while preserving:
- **Hierarchical relationships** between headers (based on header levels)
- **Positional context** (where the header appears in the AST)
- **AST path information** (the path through the AST to reach each header)
- **Nested structure relationships** (headers within for loops, let statements, etc.)

## Key Components

### `HeaderCollector`
The main collector that walks the AST and extracts headers.

```rust
use internal_baml_ast::ast::{HeaderCollector, Ast};

let ast: Ast = // ... your parsed AST
let header_tree = HeaderCollector::collect_headers(&ast);
```

### `HeaderTree`
The result structure containing all collected headers with various access methods.

```rust
// Get all headers in order
let all_headers = header_tree.all_headers();

// Find headers by title
let header = header_tree.find_header_by_title("My Header");

// Get headers in a specific function
let function_headers = header_tree.headers_in_function("MyFunction");

// Get headers by context type
let let_headers = header_tree.headers_in_context(&HeaderContext::LetStatement {
    variable_name: "my_var".to_string()
});
```

### `HeaderContext`
Represents where a header appears in the AST:

- `Function { name }` - Header at the function level
- `ExprFunction { name }` - Header at the expression function level
- `LetStatement { variable_name }` - Header within a let statement
- `ForLoopStatement { variable_name }` - Header within a for loop
- `ExpressionBlock` - Header within an expression block
- `FinalExpression` - Header that applies to a final expression
- `Nested { parent, depth }` - Header within nested structures

### `ContextualHeader`
A header with its full contextual information:

```rust
let header: &ContextualHeader = // ...
println!("Title: {}", header.title());
println!("Level: {}", header.level());
println!("Context: {:?}", header.context);
println!("AST Path: {}", header.ast_path_string());
```

## Configuration

You can customize the collection behavior with `HeaderCollectorConfig`:

```rust
use internal_baml_ast::ast::{HeaderCollector, HeaderCollectorConfig};

let config = HeaderCollectorConfig {
    preserve_hierarchy: true,    // Build parent-child relationships
    include_context: true,       // Track where headers appear
    track_ast_paths: true,       // Track path through AST
};

let header_tree = HeaderCollector::collect_headers_with_config(&ast, config);
```

## Use Cases

### 1. Documentation Generation
Extract headers to generate table of contents or documentation structure:

```rust
let header_tree = HeaderCollector::collect_headers(&ast);
for header in header_tree.all_headers() {
    let indent = "  ".repeat(header.level() as usize);
    println!("{}- {}", indent, header.title());
}
```

### 2. Function Analysis
Find all headers within a specific function:

```rust
let function_headers = header_tree.headers_in_function("ProcessData");
for header in function_headers {
    println!("Found header in ProcessData: {}", header.title());
}
```

### 3. Debugging and Visualization
Get a tree representation of all headers:

```rust
println!("{}", header_tree.to_tree_string());
```

### 4. Code Navigation
Find headers by title for navigation or refactoring tools:

```rust
if let Some(header) = header_tree.find_header_by_title("Data Processing") {
    println!("Found at: {}", header.ast_path_string());
}
```

## Example

See `examples/header_collector_example.rs` for a complete working example that demonstrates:
- Different header context types
- Configuration options
- Query methods
- Basic functionality with an empty AST

Run the example with:
```bash
cargo run --example header_collector_example
```

Run the tests with:
```bash
cargo test --example header_collector_example
```

## Integration with Mermaid Debug

The header collector complements the existing `MermaidDiagramGenerator` which focuses on visual representation. Use the header collector when you need structured, queryable access to header information rather than visual diagrams.

## Performance Considerations

- The collector performs a single pass through the AST
- Memory usage scales with the number of headers found
- Hierarchy building is optional and can be disabled for better performance
- Context tracking can be disabled if not needed

## Limitations

- Currently only extracts headers from expressions and statements
- Hierarchy relationships are based on header levels, not AST structure
- Parent-child header relationships are immutable once built (requires RefCell for dynamic updates) 