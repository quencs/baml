# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Common Development Commands

### Building and Testing
```bash
# Build entire project
./tools/build

# Build core Rust engine only
cd engine && cargo build

# Run engine tests
cd engine && cargo test

# Run formatter-specific tests
cd engine/baml-lib/ast && cargo test

# Run integration tests (Python)
cd integ-tests/python && uv run pytest

# Run integration tests (TypeScript) 
cd integ-tests/typescript && npm test

# Format Rust code
cargo fmt

# Run linter
cargo clippy
```

### BAML CLI Commands
```bash
# Test BAML CLI (from engine/cli)
cargo run -- --help
cargo run -- fmt --help

# Test a single BAML file
cargo run -- test path/to/file.baml

# Format BAML files
cargo run -- fmt path/to/file.baml
```

### Testing Individual Components
```bash
# Test specific Rust crate
cd engine/baml-lib/ast && cargo test
cd engine/baml-lib/parser-database && cargo test

# Run single test by name
cargo test test_name

# Run formatter tests with output
cd engine/baml-lib/ast && cargo test formatter -- --nocapture
```

## Code Architecture

### Project Structure
- **engine/**: Core Rust implementation (parser, AST, compiler, runtime)
- **engine/cli/**: Main CLI implementation in Rust
- **baml-cli/**: Go wrapper CLI for distribution
- **integ-tests/**: Cross-language integration tests
- **typescript/**: TypeScript tooling and frontend packages
- **tools/**: Build scripts and development utilities

### Key Components

#### AST and Parser (`engine/baml-lib/ast/`)
- **AST Definition**: `src/ast.rs` - Core AST structures
- **Parser**: Uses Pest grammar in `src/parser/datamodel.pest`
- **Formatter**: `src/formatter/mod.rs` - Pretty printer using `pretty` crate
- **AST Types**: 
  - `Top` - Top-level constructs (functions, classes, enums, clients)
  - `TypeExpression` - Type definitions (classes, enums)
  - `ValueExpression` - Value definitions (functions, clients, generators)

#### Formatter Architecture
- **Input**: `SchemaAst` (parsed BAML schema)
- **Output**: `String` (formatted BAML code)
- **Implementation**: Uses `pretty` crate's `RcDoc` for document representation
- **Strategy**: Incremental formatting with fallback to original source
- **Location**: `engine/baml-lib/ast/src/formatter/`

#### CLI Integration (`engine/cli/`)
- **Main**: `src/main.rs` - Entry point and command dispatch
- **Commands**: `src/commands.rs` - Command definitions
- **Format Command**: `src/format.rs` - Format command implementation

### Testing Patterns
- **Unit Tests**: Co-located with implementation (`mod tests`)
- **Integration Tests**: In `integ-tests/` directory organized by language
- **Formatter Tests**: Snapshot testing with input/expected output pairs
- **Test Data**: Shared BAML schemas in `integ-tests/baml_src/`

### Language Server and Editor Support
- **LSP Server**: `engine/language_server/` - Rust implementation
- **VSCode Extension**: `typescript/vscode-ext/`
- **IntelliJ Plugin**: `jetbrains/`

### Multi-Language Code Generation
- **Generators**: `engine/generators/languages/` - Template-based code generation
- **Templates**: Askama templates for each target language
- **FFI Bridge**: `engine/language_client_cffi/` - C FFI for language interop

## Development Workflow

### Adding New Formatter Rules
1. Add test cases in `engine/baml-lib/ast/src/formatter/tests.rs`
2. Implement formatting logic in `engine/baml-lib/ast/src/formatter/mod.rs`
3. Run tests: `cd engine/baml-lib/ast && cargo test`
4. Test via CLI: `cd engine/cli && cargo run -- fmt test.baml`

### Working with AST
- AST is defined in `engine/baml-lib/ast/src/ast/`
- Use existing visitor patterns for traversing AST
- Parser errors and diagnostics are handled separately from AST

### Formatter Implementation Notes
- Currently uses `pretty` crate for document generation
- Supports `--dry-run` flag for testing
- Preserves original formatting for unhandled cases
- CLI command is currently hidden (`hide = true` in commands.rs)