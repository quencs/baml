# BAML Rust Integration Tests

This directory contains comprehensive integration tests for the BAML Rust client, testing the CFFI-based implementation against real BAML functions.

## 🚀 Quick Start

### Prerequisites

- Rust 1.75+ with Cargo
- BAML CLI (`cargo install baml-cli`)
- Environment variables for testing (see below)

### Setup

1. **Generate BAML client code:**
   ```bash
   cd ../baml_src
   baml-cli generate
   ```

2. **Set up environment variables:**
   ```bash
   export OPENAI_API_KEY="your-api-key-here"
   # Optional: Other provider keys
   export ANTHROPIC_API_KEY="your-anthropic-key"
   export VERTEX_AI_KEY="your-vertex-key"
   ```

3. **Run smoke test:**
   ```bash
   cargo run --bin simple-test
   ```

4. **Run all integration tests:**
   ```bash
   cargo test
   ```

## 🧪 Test Categories

### Core Function Tests
- **Basic Functions** (`test_functions_basic.rs`) - Single inputs, named args, basic types
- **Data Types** (`test_functions_data_types.rs`) - Complex types, collections, optionals  
- **Streaming** (`test_functions_streaming.rs`) - Async streams, partial results
- **Media** (`test_functions_media.rs`) - Images, audio, PDFs, videos
- **Constraints** (`test_functions_constraints.rs`) - Validation, type checking
- **Recursive Types** (`test_functions_recursive.rs`) - Self-referencing structures

### Client & Infrastructure Tests  
- **Client Registry** (`test_client_registry.rs`) - Dynamic client configuration
- **Error Handling** (`test_error_handling.rs`) - Network, validation, parsing errors
- **Type Builder** (`test_type_builder.rs`) - Dynamic type construction
- **Parser** (`test_parser.rs`) - JSON parsing, type coercion
- **Environment Variables** (`test_env_var.rs`) - Configuration loading

### CFFI & Performance Tests
- **CFFI Integration** (`test_cffi.rs`) - Library loading, callbacks, memory safety
- **Performance** (`test_memory_performance.rs`) - Memory usage, concurrent calls
- **Providers** (`test_providers.rs`) - OpenAI, Anthropic, Azure, local models
- **Retries & Fallbacks** (`test_retries_fallbacks.rs`) - Error recovery strategies

### Advanced Tests
- **Collector** (`test_collector_comprehensive.rs`) - Usage tracking, tracing
- **Modular API** (`test_modular_api.rs`) - Client builder, configuration chaining

## 🏗️ Architecture

The Rust integration tests validate:

1. **CFFI Architecture**: Tests the shared `baml_cffi.dylib` integration
2. **Type Safety**: Ensures generated Rust types work correctly  
3. **Async Operations**: Validates tokio-based async function calls
4. **Memory Safety**: Confirms no leaks at FFI boundaries
5. **Real-world Usage**: Tests actual API calls and responses

## 📁 Generated Files

After running `baml-cli generate`, you'll see:

```
rust/
├── baml_client/           # Generated BAML client
│   ├── client.rs          # Main client implementation  
│   ├── types.rs          # Generated types
│   ├── lib.rs           # Library exports
│   └── Cargo.toml       # Generated dependencies
├── src/                  # Test framework
├── tests/               # Integration test suites
└── Cargo.toml          # Test dependencies
```

## 🔧 Test Configuration

### Environment Variables
- `OPENAI_API_KEY` - Required for OpenAI provider tests
- `ANTHROPIC_API_KEY` - Optional for Anthropic tests
- `VERTEX_AI_KEY` - Optional for Google Vertex AI tests  
- `BAML_LOG` - Set to `DEBUG` for detailed logging

### Running Specific Tests
```bash
# Run only basic function tests
cargo test test_functions_basic

# Run with logging
RUST_LOG=debug cargo test

# Run single test
cargo test test_sync_function_call -- --nocapture
```

### Performance Testing
```bash
# Run performance tests
cargo test test_memory_performance -- --ignored

# Run with release optimizations
cargo test --release
```

## 🐛 Troubleshooting

### Common Issues

1. **Library Loading Errors**: 
   - Ensure BAML dylib is built: `cd ../../engine && cargo build`
   - Check library search paths in logs

2. **API Key Issues**:
   - Verify environment variables are set
   - Check API key validity with provider

3. **Generation Issues**:
   - Run `baml-cli generate` from `../baml_src` directory
   - Ensure BAML source files are valid

### Debug Mode
```bash
BAML_LOG=DEBUG RUST_LOG=debug cargo test -- --nocapture
```

## 🚦 CI/CD Integration

Tests are integrated into the main BAML CI pipeline:

```bash
# Run from repo root
./integ-tests/run-tests.sh
```

The test runner includes:
- Dependency installation
- Code generation
- Full test suite execution  
- Performance benchmarks
- Memory leak detection

## 📊 Current Status

✅ **Completed:**
- Integration test framework structure  
- Basic function call tests
- Error handling test suite
- Data types and collections tests
- Streaming functionality tests
- CFFI validation tests
- Test utilities and helpers
- Comprehensive documentation

⚠️ **Known Issues:**
- Generated BAML client has compilation issues that need to be addressed in the Rust generator:
  - Type generation with `crate::` prefix issues
  - Stream state module generation
  - Checked type constraint syntax
  - Debug trait implementation issues  
  - Union type generation problems

🔧 **Next Steps:**
1. Fix Rust generator compilation issues
2. Re-enable generated client integration
3. Complete test implementation
4. Add performance benchmarks

## 📊 Test Coverage

Once generator issues are resolved, the test suite will cover:
- ✅ All BAML function types and signatures
- ✅ Error conditions and edge cases  
- ✅ Provider integrations
- ✅ Performance characteristics
- ✅ Memory safety guarantees
- ✅ Concurrent usage patterns

For detailed coverage reports:
```bash
cargo tarpaulin --out Html
```

## 🏗️ Implementation Details

### Test Design Philosophy

The integration tests are designed with the following principles:

- **Resilient to Environment Issues:** Tests gracefully handle API failures and timeouts
- **Comprehensive Coverage:** Test all major code paths and error conditions
- **Rust Idiomatic:** Use Rust best practices and patterns throughout
- **Future-Proof:** Easy to extend as BAML features evolve
- **CI/CD Ready:** Suitable for automated testing pipelines

### Comparison with Other Languages

The Rust integration tests maintain feature parity with:
- Go integration tests for core functionality
- Python tests for error handling patterns
- TypeScript tests for async/streaming behavior

While adapting for Rust-specific concerns:
- Memory safety and ownership
- Error handling with `Result<T, E>`
- Async/await patterns with Tokio
- FFI safety and thread safety