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
| literal_types | ✅ PASS | Both consistent and evaluate tests pass |
| map_types | ✅ PASS | Both consistent and evaluate tests pass |
| media_types | ✅ PASS | Both consistent and evaluate tests pass |
| mixed_complex_types | ✅ PASS | Both consistent and evaluate tests pass |
| nested_structures | ✅ PASS | Both consistent and evaluate tests pass |
| optional_nullable | ✅ PASS | Both consistent and evaluate tests pass |
| primitive_types | ✅ PASS | Both consistent and evaluate tests pass |
| recursive_types | ✅ PASS | Both consistent and evaluate tests pass |
| sample | ✅ PASS | Both consistent and evaluate tests pass |
| semantic_streaming | ✅ PASS | Both consistent and evaluate tests pass |
| unions | ✅ PASS | Both consistent and evaluate tests pass |
| union_types_extended | ❌ FAIL | Consistent test passes, evaluate test fails with compilation errors |

## Summary

- **Total Tests**: 17 test folders
- **Passing**: 16 tests (94%) ✅
- **Failing**: 1 test (6%) ❌

## Detailed Results

### Passing Tests (16/17)

All tests except `union_types_extended` are passing successfully. These tests include:

- **Basic Types**: `array_types`, `primitive_types`, `literal_types`
- **Complex Types**: `map_types`, `mixed_complex_types`, `nested_structures`
- **Advanced Features**: `enums`, `unions`, `optional_nullable`, `recursive_types`
- **Special Cases**: `edge_cases`, `asserts`, `semantic_streaming`
- **Media Types**: `media_types`
- **Sample**: `sample`

### Failing Tests (1/17)

#### `union_types_extended` ❌
- **Consistent Test**: ✅ PASS
- **Evaluate Test**: ❌ FAIL
- **Error**: Compilation errors in the generated Rust code
- **Issues**: 
  - Type conflicts with `Result` struct (line 2219)
  - String literal vs String type mismatches
  - Missing trait implementations for `&str`
  - Drop-check cycle detected for `RecursiveUnion`

## Test Command

To run individual tests:
```bash
cd /Users/han/github/baml/engine
cargo test --package generators-rust --lib -- <test_name>
```

## Last Updated

Updated: $(date)