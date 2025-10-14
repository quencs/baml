# AST Examples

This directory contains small binaries that exercise pieces of the AST library. The most interesting example today is `generate_mermaid_headers`, a CLI that renders Markdown-style headers in a `.baml` file as a Mermaid diagram. Below is a deep dive that explains how it works and how the supporting infrastructure is structured.

## CLI Entry Point: `generate_mermaid_headers`
- **Usage**: `cargo run --example generate_mermaid_headers -- <path/to/file.baml>`
- **Flow**: the binary reads the target file, wraps it in an `internal_baml_diagnostics::SourceFile`, and calls `internal_baml_ast::parse`. If diagnostics report errors, the program prints them and aborts.
- **Output**: on success it calls `BamlVisDiagramGenerator::generate_with_styling(ast, true)` and prints Mermaid `flowchart TD` text with optional class definitions for nicer styling (`engine/baml-lib/ast/examples/generate_mermaid_headers.rs`).

## Core Components
### `HeaderCollector`
Located in `engine/baml-lib/ast/src/ast/header_collector.rs`, this walker traverses the full AST to build a `HeaderIndex`.
- Tracks lexical scopes via a `ScopeId` stack and records every Markdown header annotation attached to statements, final expressions, and loops.
- Allocates dense `Hid` identifiers per header, keeps source spans, normalizes header levels within each scope, and resolves parent/child relationships implied by Markdown nesting.
- Captures cross-scope “nested edges” whenever a header introduces a new block (for example, an `if` or `for` body), and records top-level function call names labelled by each header for optional visualization.

### `BamlVisDiagramGenerator`
Implemented in `engine/baml-lib/ast/src/ast/baml_vis.rs`.
- Calls `HeaderCollector::collect` and feeds the resulting `HeaderIndex` into a `GraphBuilder`.
- `GraphBuilder` precomputes markdown children, nested scope relationships, and the first header for each scope. It then builds a renderer-agnostic graph composed of:
  - `Node` values tagged as plain headers, decision nodes (`HeaderLabelKind::If`), or call nodes (currently behind the `SHOW_CALL_NODES` flag).
  - `Cluster` containers that become Mermaid subgraphs so nested scopes render as grouped regions.
  - `Edge` links that connect sibling headers linearly, stitch branch exits together, and hook nested scopes back into their parent flow.
- Heuristics flatten simple containers (≤1 child) to keep diagrams compact, unless the child scope itself expands into multiple items.
- Span metadata is serialized into a side-channel map attached to node IDs, powering downstream “click to source” integrations.

### `MermaidRenderer`
Also in `baml_vis.rs`.
- Emits the final Mermaid text, adding optional `classDef` rules when `use_fancy` is true.
- Recursively renders clusters with indentation so nested scopes are easy to read and keeps decision node styling stable for snapshot tests.
- Deduplicates edges and escapes quotes in labels before writing the graph.

## Tests & Fixtures
Snapshot tests in `engine/baml-lib/baml/tests/mermaid_graph_tests.rs` cover the generator end-to-end.
- Iterate all `.baml` fixtures under `tests/validation_files/headers`, parse them, and run `BamlVisDiagramGenerator::generate_headers_flowchart`.
- Compare output against checked-in `.mmd` files (or `.err` diagnostics for negative cases). Set `UPDATE=1` to refresh snapshots when behavior changes.
- Normalization utilities strip ANSI codes and standardize newline handling so snapshots stay stable across platforms.

## Working With the Example
1. Pick a fixture (e.g., `tests/validation_files/headers/basic_sections.baml`) or create your own `.baml` file.
2. Run the example as shown above or execute the snapshot test with `cargo test -p baml-lib mermaid_graph_tests`.
3. Paste the Mermaid output into the [Mermaid Live Editor](https://mermaid.live/) or any compatible renderer to visualize header structure.

This pipeline demonstrates how the AST layer turns annotated BAML code into navigable diagrams, balancing markdown hierarchy with control-flow nesting.
