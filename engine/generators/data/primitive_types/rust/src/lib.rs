#[cfg(test)]
mod tests {
    use anyhow::Result;
    use baml_client::baml;

    #[tokio::test]
    async fn test_primitive_types() -> Result<()> {
        let result = baml::TestPrimitiveTypes("test primitive types").await?;

        // Verify primitive field values
        assert_eq!(
            result.string_field, "Hello, BAML!",
            "Expected string_field to be 'Hello, BAML!'"
        );
        assert_eq!(result.int_field, 42, "Expected int_field to be 42");
        assert!(
            (result.float_field - 3.14159).abs() < 0.001,
            "Expected float_field to be approximately 3.14159"
        );
        assert_eq!(result.bool_field, true, "Expected bool_field to be true");
        assert!(
            result.null_field.is_none(),
            "Expected null_field to be None"
        );

        println!("✓ PrimitiveTypes test passed");
        Ok(())
    }

    #[tokio::test]
    async fn test_primitive_arrays() -> Result<()> {
        let result = baml::TestPrimitiveArrays("test primitive arrays").await?;

        // Verify array values
        assert_eq!(
            result.string_array,
            vec!["hello", "world", "baml"],
            "Expected string_array values"
        );
        assert_eq!(
            result.int_array,
            vec![1, 2, 3, 4, 5],
            "Expected int_array values"
        );
        assert_eq!(result.float_array.len(), 4, "Expected float_array length 4");
        assert!(
            (result.float_array[0] - 1.1).abs() < 0.001,
            "Expected first float to be approximately 1.1"
        );
        assert_eq!(
            result.bool_array,
            vec![true, false, true, false],
            "Expected bool_array values"
        );

        println!("✓ PrimitiveArrays test passed");
        Ok(())
    }

    #[tokio::test]
    async fn test_primitive_maps() -> Result<()> {
        let result = baml::TestPrimitiveMaps("test primitive maps").await?;

        // Verify map contents
        assert_eq!(
            result.string_map.len(),
            2,
            "Expected string_map to have 2 entries"
        );
        assert!(
            result.string_map.contains_key("key1"),
            "Expected string_map to contain 'key1'"
        );
        assert!(
            result.string_map.contains_key("key2"),
            "Expected string_map to contain 'key2'"
        );

        assert_eq!(
            result.int_map.len(),
            3,
            "Expected int_map to have 3 entries"
        );
        assert!(
            result.int_map.contains_key("one"),
            "Expected int_map to contain 'one'"
        );
        assert!(
            result.int_map.contains_key("two"),
            "Expected int_map to contain 'two'"
        );
        assert!(
            result.int_map.contains_key("three"),
            "Expected int_map to contain 'three'"
        );

        assert_eq!(
            result.float_map.len(),
            2,
            "Expected float_map to have 2 entries"
        );
        assert!(
            result.float_map.contains_key("pi"),
            "Expected float_map to contain 'pi'"
        );
        assert!(
            result.float_map.contains_key("e"),
            "Expected float_map to contain 'e'"
        );

        assert_eq!(
            result.bool_map.len(),
            2,
            "Expected bool_map to have 2 entries"
        );
        assert!(
            result.bool_map.contains_key("isTrue"),
            "Expected bool_map to contain 'isTrue'"
        );
        assert!(
            result.bool_map.contains_key("isFalse"),
            "Expected bool_map to contain 'isFalse'"
        );

        println!("✓ PrimitiveMaps test passed");
        Ok(())
    }

    #[tokio::test]
    async fn test_mixed_primitives() -> Result<()> {
        let result = baml::TestMixedPrimitives("test mixed primitives").await?;

        // Verify structure - just check that fields exist and have correct types
        assert!(!result.name.is_empty(), "Expected name to be non-empty");
        assert!(result.age > 0, "Expected age to be positive");
        assert!(result.height > 0.0, "Expected height to be positive");
        assert!(result.tags.len() > 0, "Expected tags to have content");
        assert!(result.scores.len() > 0, "Expected scores to have content");
        assert!(
            result.measurements.len() > 0,
            "Expected measurements to have content"
        );
        assert!(result.flags.len() > 0, "Expected flags to have content");

        println!("✓ MixedPrimitives test passed");
        Ok(())
    }

    #[tokio::test]
    async fn test_empty_collections() -> Result<()> {
        let result = baml::TestEmptyCollections("test empty collections").await?;

        // Verify all arrays are empty
        assert_eq!(result.string_array.len(), 0, "Expected empty string_array");
        assert_eq!(result.int_array.len(), 0, "Expected empty int_array");
        assert_eq!(result.float_array.len(), 0, "Expected empty float_array");
        assert_eq!(result.bool_array.len(), 0, "Expected empty bool_array");

        println!("✓ EmptyCollections test passed");
        Ok(())
    }

    // Test top-level primitive return types
    #[tokio::test]
    async fn test_top_level_string() -> Result<()> {
        let result = baml::TestTopLevelString("test string").await?;
        assert_eq!(result, "Hello from BAML!", "Expected 'Hello from BAML!'");
        println!("✓ TopLevelString test passed");
        Ok(())
    }

    #[tokio::test]
    async fn test_top_level_int() -> Result<()> {
        let result = baml::TestTopLevelInt("test int").await?;
        assert_eq!(result, 42, "Expected 42");
        println!("✓ TopLevelInt test passed");
        Ok(())
    }

    #[tokio::test]
    async fn test_top_level_float() -> Result<()> {
        let result = baml::TestTopLevelFloat("test float").await?;
        assert!(
            (result - 3.14159).abs() < 0.001,
            "Expected approximately 3.14159"
        );
        println!("✓ TopLevelFloat test passed");
        Ok(())
    }

    #[tokio::test]
    async fn test_top_level_bool() -> Result<()> {
        let result = baml::TestTopLevelBool("test bool").await?;
        assert_eq!(result, true, "Expected true");
        println!("✓ TopLevelBool test passed");
        Ok(())
    }

    #[tokio::test]
    async fn test_top_level_null() -> Result<()> {
        let result = baml::TestTopLevelNull("test null").await?;
        assert!(result.is_none(), "Expected None");
        println!("✓ TopLevelNull test passed");
        Ok(())
    }
}
