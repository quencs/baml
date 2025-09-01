# Expanded `create_code_gen_test_suites!` Macro Example

## **Generated Macro Code**

Based on the build script logic, here's what the `create_code_gen_test_suites!` macro expands to when used with different language generators:

## **For Go Generator** (`create_code_gen_test_suites!(crate::GoLanguageFeatures)`)

```rust
#[macro_export]
macro_rules! create_code_gen_test_suites {
    (crate::GoLanguageFeatures) => {
        #[test]
        fn test_asserts_evaluate() -> anyhow::Result<()> {
            let test_harness = test_harness::TestHarness::load_test("asserts", <crate::GoLanguageFeatures>::default(), true)?;
            test_harness.run()
        }

        #[test]
        fn test_asserts_consistent() -> anyhow::Result<()> {
            let test_harness = test_harness::TestHarness::load_test("asserts", <crate::GoLanguageFeatures>::default(), false)?;
            test_harness.ensure_consistent_codegen()
        }

        #[test]
        fn test_array_types_evaluate() -> anyhow::Result<()> {
            let test_harness = test_harness::TestHarness::load_test("array_types", <crate::GoLanguageFeatures>::default(), true)?;
            test_harness.run()
        }

        #[test]
        fn test_array_types_consistent() -> anyhow::Result<()> {
            let test_harness = test_harness::TestHarness::load_test("array_types", <crate::GoLanguageFeatures>::default(), false)?;
            test_harness.ensure_consistent_codegen()
        }

        #[test]
        fn test_classes_evaluate() -> anyhow::Result<()> {
            let test_harness = test_harness::TestHarness::load_test("classes", <crate::GoLanguageFeatures>::default(), true)?;
            test_harness.run()
        }

        #[test]
        fn test_classes_consistent() -> anyhow::Result<()> {
            let test_harness = test_harness::TestHarness::load_test("classes", <crate::GoLanguageFeatures>::default(), false)?;
            test_harness.ensure_consistent_codegen()
        }

        #[test]
        fn test_edge_cases_evaluate() -> anyhow::Result<()> {
            let test_harness = test_harness::TestHarness::load_test("edge_cases", <crate::GoLanguageFeatures>::default(), true)?;
            test_harness.run()
        }

        #[test]
        fn test_edge_cases_consistent() -> anyhow::Result<()> {
            let test_harness = test_harness::TestHarness::load_test("edge_cases", <crate::GoLanguageFeatures>::default(), false)?;
            test_harness.ensure_consistent_codegen()
        }

        #[test]
        fn test_enums_evaluate() -> anyhow::Result<()> {
            let test_harness = test_harness::TestHarness::load_test("enums", <crate::GoLanguageFeatures>::default(), true)?;
            test_harness.run()
        }

        #[test]
        fn test_enums_consistent() -> anyhow::Result<()> {
            let test_harness = test_harness::TestHarness::load_test("enums", <crate::GoLanguageFeatures>::default(), false)?;
            test_harness.ensure_consistent_codegen()
        }

        #[test]
        fn test_literal_types_evaluate() -> anyhow::Result<()> {
            let test_harness = test_harness::TestHarness::load_test("literal_types", <crate::GoLanguageFeatures>::default(), true)?;
            test_harness.run()
        }

        #[test]
        fn test_literal_types_consistent() -> anyhow::Result<()> {
            let test_harness = test_harness::TestHarness::load_test("literal_types", <crate::GoLanguageFeatures>::default(), false)?;
            test_harness.ensure_consistent_codegen()
        }

        #[test]
        fn test_map_types_evaluate() -> anyhow::Result<()> {
            let test_harness = test_harness::TestHarness::load_test("map_types", <crate::GoLanguageFeatures>::default(), true)?;
            test_harness.run()
        }

        #[test]
        fn test_map_types_consistent() -> anyhow::Result<()> {
            let test_harness = test_harness::TestHarness::load_test("map_types", <crate::GoLanguageFeatures>::default(), false)?;
            test_harness.ensure_consistent_codegen()
        }

        #[test]
        fn test_media_types_evaluate() -> anyhow::Result<()> {
            let test_harness = test_harness::TestHarness::load_test("media_types", <crate::GoLanguageFeatures>::default(), true)?;
            test_harness.run()
        }

        #[test]
        fn test_media_types_consistent() -> anyhow::Result<()> {
            let test_harness = test_harness::TestHarness::load_test("media_types", <crate::GoLanguageFeatures>::default(), false)?;
            test_harness.ensure_consistent_codegen()
        }

        #[test]
        fn test_mixed_complex_types_evaluate() -> anyhow::Result<()> {
            let test_harness = test_harness::TestHarness::load_test("mixed_complex_types", <crate::GoLanguageFeatures>::default(), true)?;
            test_harness.run()
        }

        #[test]
        fn test_mixed_complex_types_consistent() -> anyhow::Result<()> {
            let test_harness = test_harness::TestHarness::load_test("mixed_complex_types", <crate::GoLanguageFeatures>::default(), false)?;
            test_harness.ensure_consistent_codegen()
        }

        #[test]
        fn test_nested_structures_evaluate() -> anyhow::Result<()> {
            let test_harness = test_harness::TestHarness::load_test("nested_structures", <crate::GoLanguageFeatures>::default(), true)?;
            test_harness.run()
        }

        #[test]
        fn test_nested_structures_consistent() -> anyhow::Result<()> {
            let test_harness = test_harness::TestHarness::load_test("nested_structures", <crate::GoLanguageFeatures>::default(), false)?;
            test_harness.ensure_consistent_codegen()
        }

        #[test]
        fn test_optional_nullable_evaluate() -> anyhow::Result<()> {
            let test_harness = test_harness::TestHarness::load_test("optional_nullable", <crate::GoLanguageFeatures>::default(), true)?;
            test_harness.run()
        }

        #[test]
        fn test_optional_nullable_consistent() -> anyhow::Result<()> {
            let test_harness = test_harness::TestHarness::load_test("optional_nullable", <crate::GoLanguageFeatures>::default(), false)?;
            test_harness.ensure_consistent_codegen()
        }

        #[test]
        fn test_primitive_types_evaluate() -> anyhow::Result<()> {
            let test_harness = test_harness::TestHarness::load_test("primitive_types", <crate::GoLanguageFeatures>::default(), true)?;
            test_harness.run()
        }

        #[test]
        fn test_primitive_types_consistent() -> anyhow::Result<()> {
            let test_harness = test_harness::TestHarness::load_test("primitive_types", <crate::GoLanguageFeatures>::default(), false)?;
            test_harness.ensure_consistent_codegen()
        }

        #[test]
        fn test_recursive_types_evaluate() -> anyhow::Result<()> {
            let test_harness = test_harness::TestHarness::load_test("recursive_types", <crate::GoLanguageFeatures>::default(), true)?;
            test_harness.run()
        }

        #[test]
        fn test_recursive_types_consistent() -> anyhow::Result<()> {
            let test_harness = test_harness::TestHarness::load_test("recursive_types", <crate::GoLanguageFeatures>::default(), false)?;
            test_harness.ensure_consistent_codegen()
        }

        #[test]
        fn test_sample_evaluate() -> anyhow::Result<()> {
            let test_harness = test_harness::TestHarness::load_test("sample", <crate::GoLanguageFeatures>::default(), true)?;
            test_harness.run()
        }

        #[test]
        fn test_sample_consistent() -> anyhow::Result<()> {
            let test_harness = test_harness::TestHarness::load_test("sample", <crate::GoLanguageFeatures>::default(), false)?;
            test_harness.ensure_consistent_codegen()
        }

        #[test]
        fn test_semantic_streaming_evaluate() -> anyhow::Result<()> {
            let test_harness = test_harness::TestHarness::load_test("semantic_streaming", <crate::GoLanguageFeatures>::default(), true)?;
            test_harness.run()
        }

        #[test]
        fn test_semantic_streaming_consistent() -> anyhow::Result<()> {
            let test_harness = test_harness::TestHarness::load_test("semantic_streaming", <crate::GoLanguageFeatures>::default(), false)?;
            test_harness.ensure_consistent_codegen()
        }

        #[test]
        fn test_unions_evaluate() -> anyhow::Result<()> {
            let test_harness = test_harness::TestHarness::load_test("unions", <crate::GoLanguageFeatures>::default(), true)?;
            test_harness.run()
        }

        #[test]
        fn test_unions_consistent() -> anyhow::Result<()> {
            let test_harness = test_harness::TestHarness::load_test("unions", <crate::GoLanguageFeatures>::default(), false)?;
            test_harness.ensure_consistent_codegen()
        }

        #[test]
        fn test_union_types_extended_evaluate() -> anyhow::Result<()> {
            let test_harness = test_harness::TestHarness::load_test("union_types_extended", <crate::GoLanguageFeatures>::default(), true)?;
            test_harness.run()
        }

        #[test]
        fn test_union_types_extended_consistent() -> anyhow::Result<()> {
            let test_harness = test_harness::TestHarness::load_test("union_types_extended", <crate::GoLanguageFeatures>::default(), false)?;
            test_harness.ensure_consistent_codegen()
        }
    };
}
```

## **For Python Generator** (`create_code_gen_test_suites!(crate::PyLanguageFeatures)`)

```rust
#[macro_export]
macro_rules! create_code_gen_test_suites {
    (crate::PyLanguageFeatures) => {
        #[test]
        fn test_asserts_evaluate() -> anyhow::Result<()> {
            let test_harness = test_harness::TestHarness::load_test("asserts", <crate::PyLanguageFeatures>::default(), true)?;
            test_harness.run()
        }

        #[test]
        fn test_asserts_consistent() -> anyhow::Result<()> {
            let test_harness = test_harness::TestHarness::load_test("asserts", <crate::PyLanguageFeatures>::default(), false)?;
            test_harness.ensure_consistent_codegen()
        }

        // ... same pattern for all other test directories
    };
}
```

## **For Rust Generator** (`create_code_gen_test_suites!(crate::RustLanguageFeatures)`)

```rust
#[macro_export]
macro_rules! create_code_gen_test_suites {
    (crate::RustLanguageFeatures) => {
        #[test]
        fn test_asserts_evaluate() -> anyhow::Result<()> {
            let test_harness = test_harness::TestHarness::load_test("asserts", <crate::RustLanguageFeatures>::default(), true)?;
            test_harness.run()
        }

        #[test]
        fn test_asserts_consistent() -> anyhow::Result<()> {
            let test_harness = test_harness::TestHarness::load_test("asserts", <crate::RustLanguageFeatures>::default(), false)?;
            test_harness.ensure_consistent_codegen()
        }

        // ... same pattern for all other test directories
    };
}
```

## **Key Points**

1. **Same Structure**: All language generators get the same test functions
2. **Language-Specific**: Only the generator type changes (`GoLanguageFeatures`, `PyLanguageFeatures`, etc.)
3. **Two Tests Per Directory**: Each test directory gets both an `_evaluate` and `_consistent` test
4. **Automatic Discovery**: The macro is generated based on actual directories in `engine/generators/data/`
5. **Consistent Naming**: Test function names follow the pattern `test_{directory_name}_{test_type}`

## **Test Function Types**

- **`_evaluate` tests**: Run the full test with `persist=true` and potentially execute generated code
- **`_consistent` tests**: Ensure code generation is deterministic with `persist=false`

This design ensures that every language generator gets comprehensive test coverage for all BAML test cases automatically.
