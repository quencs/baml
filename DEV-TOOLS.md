# DEV-TOOLS.md

Comprehensive guide to all development tools and configurations used in the BAML project.

## Overview

BAML uses a complex multi-language toolchain spanning Rust, TypeScript, Python, Ruby, and Go. This document outlines every tool, configuration, and setup required for development.

## 📋 Prerequisites

### Version Management
- **mise** - Primary tool version manager (replaces multiple version managers)
  - Defined in `mise.toml`
  - Manages Rust, Go, Python, Ruby, Node.js versions
  - Install: `curl https://mise.sh | sh`

### Core Languages & Runtimes
```toml
# From mise.toml
rust = "1.88.0"
go = "1.23.11" 
python = "3.13"
ruby = "3.2.2"
node = "lts"
```

## 🦀 Rust Ecosystem

### Core Tools
- **rustc** - Rust compiler (stable channel)
- **cargo** - Package manager and build tool
- **rustfmt** - Code formatter
- **clippy** - Linter

### Rust-specific Tools (via mise)
```toml
"cargo:wasm-pack" = "0.13.1"     # WebAssembly packaging
"cargo:cross" = "latest"          # Cross-compilation
"cargo:cargo-watch" = "latest"    # File watching for auto-rebuild
```

### Configuration Files
- `rust-toolchain.toml` - Toolchain specification
- `engine/.cargo/config.toml` - Build configuration with tracing features
- `engine/Cross.toml` - Cross-compilation settings
- `rustfmt.toml` - Code formatting rules

### Build Features
- Tracing unstable features enabled via rustflags
- Cross-compilation for Linux musl targets
- WebAssembly compilation support

## 🟦 TypeScript/JavaScript Ecosystem

### Package Management
- **pnpm** v9.12.0 - Fast, disk-efficient package manager
- **Node.js** LTS - Runtime environment

### Build & Development Tools
- **Turbo** v2.5.4 - Monorepo build orchestration
- **TypeScript** v5.8.3 - Type checking
- **Biome** v1.9.4 - Formatting and linting (replaces ESLint/Prettier)
- **Vite** - Frontend build tool (for playground)
- **Next.js** - React framework (for web apps)
- **esbuild** v0.25.2 - Fast bundler

### Testing
- **Jest** v29.7.0 - Testing framework (integration tests)
- **Vitest** v3.2.3 - Fast unit testing
- **@swc/jest** v0.2.36 - Fast Jest transforms

### TypeScript Configuration
- Multiple `tsconfig.json` files for different packages
- Workspace-based TypeScript project references
- Strict type checking enabled

### Biome Configuration (`biome.json`)
```json
{
  "formatter": {
    "indentStyle": "space",
    "lineWidth": 100
  },
  "javascript": {
    "formatter": {
      "quoteStyle": "single",
      "trailingCommas": "all",
      "semicolons": "always"
    }
  }
}
```

## 🐍 Python Ecosystem

### Package Management
- **uv** - Fast Python package installer and resolver (via pipx)
- **pip** v25.1.1 - Fallback package installer
- **maturin** - Rust-Python binding builder (via pipx)

### Code Quality
- **ruff** v0.9.10+ - Fast linter and formatter (via pipx)
- **pytest** - Testing framework
- **pytest-asyncio** - Async testing support

### Configuration
- `pyproject.toml` - Project configuration and dependencies
- `uv.lock` - Locked dependency versions
- Python version: ~3.9+ required, 3.13 preferred

### Key Dependencies
```toml
dependencies = [
    "openai>=1.93.0",
    "anthropic (>=0.49.0,<0.50.0)", 
    "google-genai (>=1.5.0,<2.0.0)",
    "boto3>=1.37.37",
    "pydantic>=2.10.6"
]
```

## 💎 Ruby Ecosystem

### Package Management
- **Bundler** - Gem dependency management
- **Rake** v13.0 - Build automation

### Development Tools
- **Sorbet** - Static type checker
- **Tapioca** - RBI file generator

### Testing
- **Minitest** - Testing framework
- **minitest-reporters** - Enhanced test output

### Configuration
- `Gemfile` - Dependency specification
- `Gemfile.lock` - Locked versions
- Ruby version: 3.2.2

## 🐹 Go Ecosystem

### Version & Modules
- **Go** 1.24.0
- **go mod** - Module management
- **goimports** - Import organization (via mise)

### Protocol Buffers
- **protoc-gen-go** v1.36.6 - Protocol buffer compiler (via aqua)

### Testing
- **testify** v1.10.0 - Testing toolkit

### Configuration
- `go.mod` - Module definition
- `go.sum` - Dependency checksums

## 🏗️ Build System

### Turbo (Monorepo Orchestration)
**Configuration**: `turbo.json`

#### Key Features
- 20 concurrent task execution
- Intelligent caching
- Task dependency management
- Environment variable passing

#### Critical Tasks
```json
{
  "build": "Builds all packages with dependency resolution",
  "dev": "Development servers with hot reload", 
  "typecheck": "TypeScript type checking across workspace",
  "generate": "BAML client code generation",
  "test": "Test execution with coverage",
  "integ-tests": "Integration tests across all languages"
}
```

### Build Dependencies
1. **Rust engine** must be built first
2. **WASM packages** required for web components  
3. **TypeScript packages** depend on Rust engine
4. **Integration tests** require all clients built

## 🧪 Testing Infrastructure

### Test Types
- **Unit Tests**: Language-specific (cargo test, jest, pytest, etc.)
- **Integration Tests**: Cross-language BAML function testing
- **Memory Tests**: Memory leak detection
- **Browser Tests**: WASM compatibility testing

### Test Commands by Language
```bash
# Rust
cargo test                              # Unit tests
UPDATE_EXPECT=1 cargo test             # Update snapshots

# TypeScript
pnpm test                              # Jest tests
pnpm integ-tests                       # Integration tests

# Python  
uv run pytest                         # All Python tests
uv run pytest tests/test_specific.py  # Specific test

# Ruby
rake test                              # Ruby test suite

# Go
go test                                # Go tests
```

### Environment Variables for Testing
```bash
# Required for integration tests
OPENAI_API_KEY=...
ANTHROPIC_API_KEY=...
GOOGLE_API_KEY=...
AWS_* # For Bedrock testing
```

## 🔧 Development Environment

### Setup Script
```bash
./scripts/setup-dev.sh  # Complete environment setup
```

### IDE Integration
- **VSCode Extension** - BAML language support
- **Language Server** - LSP implementation in Rust
- **Zed Editor** - Alternative editor support

### Hot Reloading
```bash
pnpm dev                    # All services with hot reload
cargo watch -x build       # Rust file watching
turbo run dev --filter=*   # Filtered development
```

## 📦 Code Generation

### BAML CLI
- **baml-cli generate** - Generate language clients from .baml files
- **baml-cli test** - Run BAML function tests
- **baml-cli fmt** - Format BAML files
- **baml-cli dev** - Development server

### Client Generation Flow
1. Parse `.baml` files (Rust parser)
2. Create Abstract Syntax Tree (AST)  
3. Validate syntax and types
4. Generate language-specific clients
5. Output typed interfaces for each language

## 🌐 Web Development

### Frontend Frameworks
- **Next.js** - React applications (fiddle-web-app, sage-backend)
- **Vite** - Development server and bundling (playground)
- **React** - UI components

### Styling & UI
- **Tailwind CSS** - Utility-first CSS framework
- **PostCSS** - CSS processing
- **Component libraries** - Custom UI components

### Deployment
- **Vercel** - Hosting platform
- **Docker** - Containerization support

## 📊 Monitoring & Debugging  

### Logging
- **tracing** - Rust structured logging
- **BAML_LOG** environment variable - Log level control
- **BAML_LOG_INTERNAL** - Internal debugging

### Development Tools
- **Chrome DevTools** - Browser debugging
- **Rust Analyzer** - IDE support for Rust
- **TypeScript Language Server** - IDE TypeScript support

## 🔐 Security & Authentication

### API Key Management
- **Infisical CLI** v0.41.89 - Secret management
- **.env** files - Local environment variables
- **Environment-based** configuration

### Credential Providers
- OpenAI API
- Anthropic API  
- Google AI API
- AWS Bedrock
- Azure OpenAI

## 📁 Configuration Files Summary

### Root Level
- `package.json` - Root package configuration
- `turbo.json` - Monorepo build orchestration  
- `biome.json` - Code formatting and linting
- `mise.toml` - Tool version management
- `rust-toolchain.toml` - Rust toolchain specification

### Rust Specific  
- `Cargo.toml` files - Rust package manifests
- `engine/.cargo/config.toml` - Build configuration
- `Cross.toml` - Cross-compilation settings
- `rustfmt.toml` - Rust formatting rules

### Language Clients
- `pyproject.toml` - Python package configuration
- `Gemfile` - Ruby dependencies
- `go.mod` - Go module definition
- `tsconfig.json` files - TypeScript configurations

### Testing
- `jest.config.js` - Jest test configuration
- `vitest.config.ts` - Vitest configuration
- `pytest.ini` - Python test settings (implicit)

## 🚀 Common Development Workflows

### Full Build
```bash
pnpm install     # Install all dependencies
pnpm build       # Build everything (Rust + TS)
```

### Development
```bash
pnpm dev         # Start all dev servers
pnpm dev:vscode  # VSCode extension development
pnpm dev:playground # Web playground development
```

### Testing
```bash
pnpm test        # All tests
pnpm integ-tests # Integration tests only
cargo test       # Rust tests (from engine/)
```

### Code Quality
```bash
pnpm format      # Check formatting
pnpm format:fix  # Fix formatting issues  
pnpm typecheck   # TypeScript type checking
cargo fmt        # Rust formatting
cargo clippy     # Rust linting
```

## 🤖 AI/LLM Integration Tools

### LLM Provider SDKs
BAML integrates with multiple AI providers through official SDKs:

```bash
# Integration test dependencies
openai>=1.93.0              # OpenAI GPT models
anthropic>=0.49.0           # Claude models  
google-genai>=1.5.0         # Google Gemini models
boto3>=1.37.37              # AWS Bedrock models
@anthropic-ai/sdk@0.39.0    # TypeScript Anthropic SDK
@google/generative-ai@0.24.0 # TypeScript Google AI SDK
```

### BAML Language Features
- **Function Definitions**: Define AI functions with typed inputs/outputs
- **Client Configuration**: Multi-provider client setup with fallbacks
- **Prompt Templates**: Jinja2-based templating with constraints
- **Type Safety**: Generated clients with full type checking
- **Streaming Support**: Real-time response streaming
- **Testing Framework**: `baml-cli test` for function validation

## 🐳 Containerization & Deployment

### Docker Support
```dockerfile
# typescript/packages/fiddle-proxy/Dockerfile
FROM node:18
WORKDIR /usr/src/app
COPY package.json package-lock.json ./
RUN npm install --prod
COPY . .
CMD ["node", "server.js"]
```

### Deployment Platforms
- **Vercel** - Next.js applications (fiddle-web-app, sage-backend)
- **GitHub Actions** - CI/CD pipelines
- **npm Registry** - TypeScript package publishing  
- **PyPI** - Python package distribution
- **RubyGems** - Ruby gem publishing
- **VSCode Marketplace** - Extension distribution

## 🔧 Environment Management

### Nix Development Environment
**Nix Flake Support**: `flake.nix` provides reproducible development environment
- Rust toolchain with WASM targets
- All language runtimes (Python 3.9, Ruby, Node.js, Go)
- Build tools (cmake, protoc, wasm-pack)
- Platform-specific dependencies (macOS frameworks, Linux libraries)

### direnv Integration
**`.envrc`** - Automatic environment setup:
```bash
# Cross-compilation support
export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc
export RB_SYS_CARGO_PROFILE="dev"  # Faster Ruby builds

# Nix or mise activation
if command -v nix-shell >/dev/null 2>&1; then
    use flake
else
    eval "$(mise activate bash)"
fi
```

### GitHub Codespaces/Dev Containers
- Pre-configured development containers
- Automated tool installation via mise
- Environment variable management

## 🏗️ Advanced Build Tools & Automation

### Act (Local GitHub Actions)
**`.actrc`** - Local GitHub Actions runner configuration:
```bash
--artifact-server-path ./tmp/artifacts
```

### Git Hooks System
**`tools/install-hooks`** - Automated code formatting:
```bash
#!/bin/bash
# Installs pre-commit hook that runs 'cargo fmt'
cp "$REPO_ROOT/tools/hooks/pre-commit" "$REPO_ROOT/.git/hooks/pre-commit"
chmod +x "$REPO_ROOT/.git/hooks/pre-commit"
```

### Version Management
**`tools/versions/`** - Centralized version configuration:
- `engine.cfg` - Rust crate versions
- `typescript.cfg` - npm package versions  
- `python.cfg` - PyPI package versions
- `ruby.cfg` - Gem versions
- `go.cfg` - Go module versions

### Cross-Compilation Tools
- **cross** - Rust cross-compilation (via mise)
- **Cross.toml** - Target-specific build configuration
- **setup-cross-compile-env.sh** - Environment setup for cross-compilation

### Development Utilities
**`tools/` directory**:
- `bctl` - Build control utility (Python)
- `bump-version` - Automated version bumping
- `build` - Build automation script
- `curl-example.sh` - API testing utilities

## 🚀 GitHub Actions CI/CD

### Comprehensive Workflow Matrix
**`.github/workflows/primary.yml`**:

#### Quality Checks (Parallel)
- TypeScript linting with Biome
- Rust formatting with rustfmt
- Rust linting with Clippy
- WASM-specific Clippy checks
- Python linting with ruff

#### Build Matrix
- **Multi-platform CLI builds**: Ubuntu, macOS, Windows
- **Cross-compilation**: x86_64, ARM64 targets
- **WASM compilation**: Browser compatibility
- **Language clients**: Python wheels, Ruby gems, npm packages

#### Integration Testing
- **Rust unit tests** with Ruby library linking
- **Python integration tests** with all AI providers
- **Code generation validation** (3-pass stability check)
- **Memory leak testing**

#### Specialized Workflows
- **Release automation** - Multi-package coordinated releases
- **Coverage reporting** - Rust code coverage with tarpaulin
- **Documentation publishing** - Automated doc site updates
- **Extension syncing** - Zed editor extension sync

### GitHub Actions Integrations
- **Dependabot** - Automated dependency updates
- **Issue templates** - Bug reports, feature requests, documentation
- **PR templates** - Structured pull request format

## 🔐 Secret & Environment Management

### Infisical Integration
```bash
"@infisical/cli": "0.41.89"  # Secret management
infisical run --env=test -- [command]  # Inject secrets for testing
```

### Environment Variables (CI/CD)
```yaml
# From turbo.json
env:
  - "OPENAI_API_KEY"
  - "ANTHROPIC_API_KEY" 
  - "GOOGLE_API_KEY"
  - "AWS_*"
  - "BOUNDARY_API_*"
  - "DATABASE_URL*"
  - "SENTRY_AUTH_TOKEN"
  - "TURBO_TOKEN"
```

## 📊 Monitoring & Observability

### Tracing & Logging
- **tracing** (Rust) - Structured logging with JSON output
- **BAML_LOG** - Environment-based log level control
- **BAML_LOG_INTERNAL** - Internal debugging flags
- **Sentry** - Error tracking and performance monitoring

### Testing & Quality Metrics
- **Jest/Vitest** - JavaScript test coverage
- **pytest** - Python test coverage
- **cargo test** - Rust test coverage with tarpaulin
- **Memory profiling** - Memory leak detection in integration tests

## 🛠️ IDE & Editor Support

### VSCode Integration
- **BAML Language Extension** - Syntax highlighting, LSP support
- **Language Server** - Rust-based LSP implementation
- **Debug Configuration** - Extension development debugging
- **Snippet Library** - BAML code snippets

### Alternative Editors
- **Zed** - Native BAML support with automatic syncing
- **JetBrains** - Plugin development in progress
- **Generic LSP** - Works with any LSP-compatible editor

### Development Tools
- **Rust Analyzer** - IDE support for Rust
- **TypeScript Language Server** - TS/JS intellisense
- **Python Language Server** - Python development support

This comprehensive tooling ecosystem supports BAML's multi-language architecture while maintaining developer productivity through automation, reproducible environments, extensive testing, and intelligent build orchestration across all supported platforms and deployment targets.