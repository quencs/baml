# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

BAML (Basically a Made-up Language) is a prompting language for building reliable AI workflows and agents. The project consists of a Rust-based compiler/runtime and language client generators for Python, TypeScript, Ruby, and Go.

## Common Development Commands

### Setup and Dependencies
- `./scripts/setup-dev.sh` - Complete development environment setup using mise
- `pnpm install` - Install all dependencies
- `mise install` - Install/update development tools to match mise.toml versions

### Building
- `pnpm build` - Build all components (Rust engine + TypeScript packages)
- `cargo build` - Build Rust components in engine/
- `cargo build --release` - Build optimized Rust binaries
- `pnpm build:vscode` - Build VSCode extension specifically
- `pnpm build:playground` - Build web playground
- `pnpm build:cli` - Build CLI tool

### Development
- `pnpm dev` - Start all services with hot reloading
- `pnpm dev:vscode` - Run VSCode extension development
- `pnpm dev:playground` - Run web playground in development mode
- `cargo watch -x build` - Watch and rebuild Rust components

### Code Quality
- `pnpm format` - Check code formatting with Biome
- `pnpm format:fix` - Auto-fix formatting issues
- `pnpm typecheck` - Run TypeScript type checking across all packages
- `cargo fmt` - Format Rust code
- `cargo clippy` - Run Rust linter

### Testing
- `cargo test` - Run Rust unit tests (run from engine/ directory)
- `pnpm test` - Run all TypeScript tests
- `pnpm integ-tests` - Run integration tests across all languages
- `./run-tests.sh` - Run complete test suite

### Integration Test Commands (from respective directories)
- **TypeScript**: `cd integ-tests/typescript && pnpm integ-tests`
- **Python**: `cd integ-tests/python && uv run pytest`
- **Ruby**: `cd integ-tests/ruby && rake test`
- **Go**: `cd integ-tests/go && go test`

### BAML CLI Testing
- `baml-cli test` - Run BAML function tests defined in .baml files
- `baml-cli generate` - Generate client code from BAML definitions
- `baml-cli fmt` - Format BAML files
- `baml-cli dev` - Start BAML development server

### Single Test Examples
- `cargo test specific_test_name` - Run specific Rust test
- `cd integ-tests/python && uv run pytest tests/test_specific.py::test_function_name`
- `cd integ-tests/typescript && pnpm test tests/specific.test.ts`

## Architecture Overview

### Core Components
1. **Rust Engine** (`engine/`)
   - `baml-lib/` - Core parsing, AST, and validation logic
   - `baml-runtime/` - Runtime execution engine for LLM calls
   - `cli/` - BAML CLI tool
   - `language_server/` - LSP server for editor integration
   - `language_client_*/` - Language-specific client generators

2. **TypeScript Ecosystem** (`typescript/`)
   - `apps/vscode-ext/` - VSCode extension
   - `apps/fiddle-web-app/` - Web playground (promptfiddle.com)
   - `packages/playground-common/` - Shared playground components
   - `packages/language-server/` - TypeScript wrapper for LSP

3. **Integration Tests** (`integ-tests/`)
   - `baml_src/` - Shared BAML test definitions
   - `typescript/`, `python/`, `ruby/`, `go/` - Language-specific test suites

### Code Generation Flow
1. BAML files (`.baml`) define functions, types, and clients
2. Rust parser creates AST and validates syntax
3. Code generators create language-specific client libraries
4. Generated `baml_client` provides typed interfaces for each language

### Key Rust Crates
- `baml-lib/baml-core` - IR (Intermediate Representation) and validation
- `baml-lib/baml-types` - Core type system and value representations
- `baml-runtime` - LLM execution engine with streaming, retries, fallbacks
- `jsonish` - Flexible JSON-like parsing for LLM outputs
- `llm-response-parser` - Provider-specific response parsing

### Development Environment
- Uses `mise` for tool version management (Rust 1.85.0, Go 1.23, Python 3.12, Ruby 3.2.2, Node.js LTS)
- Turbo for monorepo build orchestration
- pnpm for JavaScript package management
- Biome for code formatting and linting

## Important Development Notes

### Environment Variables for Testing
Integration tests require API keys. Set up either:
1. `.env` file in `integ-tests/` directory with `OPENAI_API_KEY=...` etc.
2. Use Infisical for internal development: `infisical run --env=test -- [command]`

### Language Client Development
- Python client: `cd engine/language_client_python && uv run maturin develop`
- TypeScript client: `cd engine/language_client_typescript && pnpm build:debug`
- Ruby client: `cd engine/language_client_ruby && rake compile`

### VSCode Extension Development
1. `cd typescript && pnpm build:vscode`
2. Open VSCode, go to Run and Debug, select "Launch VSCode Extension"
3. Use `Command + Shift + P` to reload extension after changes

### Grammar and Parser Changes
- Update `.pest` grammar files in `engine/baml-lib/`
- Modify AST parsing in corresponding Rust modules
- Update IR (Intermediate Representation) as needed
- Test with `cargo test` in `engine/baml-lib/baml/`

### Build Dependencies
- TypeScript packages depend on Rust engine being built first
- Integration tests require both engine and language clients to be built
- VSCode extension requires playground components to be built

## Troubleshooting

### Common Issues
- **mise command not found**: Add `~/.local/bin` to PATH
- **Rust compilation errors**: Ensure correct Rust version with `mise install`
- **Integration test failures**: Verify API keys are set and services are accessible
- **TypeScript build issues**: Run `pnpm typecheck` to identify type errors
- **Python client issues**: Rebuild with `uv run maturin develop --release`

### Git Hooks
Run `./tools/install-hooks` to set up pre-commit formatting hooks for Rust code.
