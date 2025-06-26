# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Development Commands

### Building
- `cargo build` - Build all Rust components in engine/
- `cargo build --release` - Build optimized release version

### Testing
- `cargo test` - Run all Rust unit tests
- `cd engine/baml-lib/baml/ && cargo test` - Run grammar linting tests specifically
- `./tools/bctl integ-tests` - Run all integration tests (Python, TypeScript, Ruby)
- `./tools/bctl integ-tests --suite python` - Run Python integration tests only
- `./tools/bctl integ-tests --suite typescript` - Run TypeScript integration tests only
- `./tools/bctl integ-tests --suite ruby` - Run Ruby integration tests only
- **Go tests**: `source ~/secrets/openai-key-baml.txt && BAML_LOG=info RUN_GENERATOR_TESTS=1 BAML_LOG=debug cargo test --lib --no-fail-fast -p generators-go -- go_tests::test_sample --exact --show-output`

### Formatting and Linting
- `cargo fmt` - Format Rust code (use in engine/ directory)
- `cargo fmt -- --config imports_granularity="Crate" --config group_imports="StdExternalCrate"` - Format with import grouping
- `cd typescript/ && pnpm biome check` - Lint TypeScript/JavaScript code
- `cd typescript/ && pnpm biome check --write` - Auto-fix TypeScript/JavaScript issues

### Language-Specific Client Generation
- **TypeScript**: `cd integ-tests/typescript && pnpm generate`
- **Python**: `cd integ-tests/python && uv run baml-cli generate --from ../baml_src`
- **Ruby**: `cd integ-tests/ruby && mise exec -- baml-cli generate --from ../baml_src`

### Pre-commit Hooks
- `./tools/install-hooks` - Install git pre-commit hooks that auto-format Rust code

## High-Level Architecture

BAML is a domain-specific language for building AI workflows and agents. The architecture consists of:

### Core Components

**Rust Engine** (`engine/`):
- **baml-lib/**: Core parsing, validation, and code generation
  - **ast/**: Abstract Syntax Tree definitions
  - **baml-core/**: IR (Intermediate Representation) and validation logic
  - **baml-types/**: Core type system and value representations
  - **parser-database/**: Parsing database and semantic analysis
  - **prompt-parser/**: BAML prompt parsing and templating
  - **jinja/**, **jinja-runtime/**: Jinja2 templating support for prompts
  - **jsonish/**: Flexible JSON parsing for LLM outputs
  - **llm-client/**: LLM provider integrations and client specs
- **baml-runtime/**: Core runtime for executing BAML functions
- **language_server/**: LSP server for IDE integration
- **cli/**: Command-line interface

**Language Clients** (Generate native bindings):
- **language_client_python/**: Python native bindings via PyO3
- **language_client_typescript/**: TypeScript/Node.js native bindings via Napi
- **language_client_ruby/**: Ruby native bindings via FFI
- **language_client_go/**: Go client via CGO/FFI
- **language_client_cffi/**: C FFI layer for other languages

**Development Tools**:
- **baml-schema-wasm/**: WebAssembly version for browser/web environments
- **generators/**: Code generation utilities for client libraries
- **sandbox/**: Development sandbox and testing utilities

### Key Workflows

1. **BAML Source → AST → IR → Code Generation**: BAML files are parsed into AST, validated into IR, then generate client code
2. **Runtime**: Generated clients call into Rust runtime which handles LLM requests, response parsing, and type coercion
3. **IDE Integration**: Language server provides syntax highlighting, validation, and playground features

### Multi-Language Support

BAML compiles to native bindings for:
- **Python**: Uses PyO3 for Python-Rust bindings
- **TypeScript/Node.js**: Uses Napi-rs for Node.js native modules  
- **Ruby**: Uses FFI for Ruby-Rust interop
- **Go**: Uses CGO with C FFI layer
- **OpenAPI/REST**: Generates OpenAPI specs for any language

## Testing Strategy

### Unit Tests
- Grammar and parsing tests in `engine/baml-lib/`
- Runtime functionality tests in `engine/baml-runtime/`

### Integration Tests  
- Cross-language integration tests in `integ-tests/`
- Each language has comprehensive test suites that verify:
  - BAML function execution
  - Type coercion and validation
  - Streaming responses
  - Error handling
  - Multi-modal inputs (images, audio)

### Environment Setup for Tests
Integration tests require API keys. Set up `.env` file in `integ-tests/` with:
```bash
OPENAI_API_KEY=your_key_here
ANTHROPIC_API_KEY=your_key_here
# Additional provider keys as needed
```

## Development Tips

- Use `cargo build` to verify Rust compilation before running integration tests
- The `./tools/bctl` utility provides convenient commands for common development tasks
- Integration tests require building native client libraries first
- VSCode extension development requires TypeScript build: `cd typescript/ && npx turbo build --force`
- For faster iteration on UI components, use `cd typescript/fiddle-frontend && pnpm dev`

## Code Style Notes

- Rust formatting uses default rustfmt with import organization
- TypeScript/JavaScript uses Biome for formatting and linting
- Pre-commit hooks automatically format Rust code on commit
- No specific style requirements beyond standard formatters