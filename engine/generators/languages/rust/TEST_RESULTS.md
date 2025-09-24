# Rust Generator Test Results

This document lists all test folders under `generators/languages/rust/generated_tests/` and shows which tests pass with `cargo test --package generators-rust --lib -- <test_name>`.

## Test Results

| Test Folder | Status | Notes |
|-------------|--------|-------|
| array_types | ✅ PASS | Both consistent and evaluate tests pass |
| asserts | ✅ PASS | Both consistent and evaluate tests pass |
| classes | ✅ PASS | Both consistent and evaluate tests pass |
| edge_cases | ✅ PASS | Both consistent and evaluate tests pass |
| enums | ✅ PASS | Both consistent and evaluate tests pass |
| literal_types | ❌ FAIL | Consistent test passes, evaluate test fails with compilation errors |
| map_types | ✅ PASS | Both consistent and evaluate tests pass |
| media_types | ❌ FAIL | Consistent test passes, evaluate test fails with missing type errors |
| mixed_complex_types | ❌ FAIL | Consistent test passes, evaluate test fails with syntax errors |
| nested_structures | ❌ FAIL | Consistent test passes, evaluate test fails with compilation errors |
| optional_nullable | ✅ PASS | Both consistent and evaluate tests pass |
| primitive_types | ❌ FAIL | Consistent test passes, evaluate test fails with trait bound errors |
| recursive_types | ❌ FAIL | Consistent test passes, evaluate test fails with syntax errors |
| sample | ❌ FAIL | Consistent test passes, evaluate test fails with const generic errors |
| semantic_streaming | ✅ PASS | Both consistent and evaluate tests pass |

## Summary

- **Total Tests**: 15
- **Passing**: 8 (53%)
- **Failing**: 7 (47%)

## Common Issues

The failing tests typically have one of these issues:

1. **Compilation Errors**: Missing type definitions or trait implementations
2. **Syntax Errors**: Invalid Rust syntax in generated code
3. **Trait Bound Errors**: Missing `FromBamlValue` implementations
4. **Const Generic Errors**: Invalid const generic expressions

## Test Command

To run a specific test:
```bash
cargo test --package generators-rust --lib -- <test_name>
```

For example:
```bash
cargo test --package generators-rust --lib -- array_types
```
