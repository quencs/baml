#[cfg(test)]
mod tests {
    use anyhow::Result;
    use baml_client::{BamlClient, ComplexLiterals, MixedLiterals, StreamState, StringLiterals};
    use futures::StreamExt;

    #[tokio::test]
    async fn string_literals_match_expected_values() -> Result<()> {
        let client = BamlClient::new()?;
        let result = client.test_string_literals("test string literals").await?;

        assert_eq!(result.status, "active");
        assert_eq!(result.environment, "prod");
        assert_eq!(result.method, "POST");
        Ok(())
    }

    #[tokio::test]
    async fn integer_literals_match_expected_values() -> Result<()> {
        let client = BamlClient::new()?;
        let result = client
            .test_integer_literals("test integer literals")
            .await?;

        assert_eq!(result.priority, 3);
        assert_eq!(result.http_status, 201);
        assert_eq!(result.max_retries, 3);
        Ok(())
    }

    #[tokio::test]
    async fn boolean_literals_match_expected_values() -> Result<()> {
        let client = BamlClient::new()?;
        let result = client
            .test_boolean_literals("test boolean literals")
            .await?;

        assert!(result.always_true);
        assert!(!result.always_false);
        assert_eq!(result.either_bool, true);
        Ok(())
    }

    #[tokio::test]
    async fn mixed_literals_match_expected_values() -> Result<()> {
        let client = BamlClient::new()?;
        let result = client.test_mixed_literals("test mixed literals").await?;

        assert_eq!(result.id, 12_345);
        assert_eq!(result.r#type, "admin");
        assert_eq!(result.level, 2);
        assert_eq!(result.is_active, true);
        assert_eq!(result.api_version, "v2");
        Ok(())
    }

    #[tokio::test]
    async fn complex_literals_match_expected_values() -> Result<()> {
        let client = BamlClient::new()?;
        let result = client
            .test_complex_literals("test complex literals")
            .await?;

        assert_eq!(result.state, "published");
        assert_eq!(result.retry_count, 5);
        assert_eq!(result.response, "success");
        assert_eq!(result.flags.len(), 3);
        assert_eq!(result.codes.len(), 3);
        Ok(())
    }

    async fn collect_stream<T>(
        mut stream: impl futures::Stream<Item = baml_client::BamlResult<StreamState<T>>>,
    ) -> Result<(Vec<T>, Option<T>)> {
        let mut partials = Vec::new();
        let mut final_value = None;
        while let Some(item) = stream.next().await {
            match item? {
                StreamState::Partial(value) => partials.push(value),
                StreamState::Final(value) => final_value = Some(value),
            }
        }
        Ok((partials, final_value))
    }

    #[tokio::test]
    async fn string_literals_stream_yields_final_value() -> Result<()> {
        let client = BamlClient::new()?;
        let stream = client
            .test_string_literals_stream("test string literals stream")
            .await?;

        let (_partials, final_value) = collect_stream(stream).await?;
        assert!(final_value.is_some(), "expected a final result");

        if let Some(final_result) = final_value {
            assert_eq!(final_result.status, "active");
            assert_eq!(final_result.environment, "prod");
            assert_eq!(final_result.method, "POST");
        }
        Ok(())
    }

    #[tokio::test]
    async fn mixed_literals_stream_validates_final_payload() -> Result<()> {
        let client = BamlClient::new()?;
        let stream = client
            .test_mixed_literals_stream("test mixed literals stream")
            .await?;

        let (_partials, final_value) = collect_stream(stream).await?;
        let final_value: MixedLiterals = final_value.expect("expected final mixed literals result");

        assert_eq!(final_value.id, 12_345);
        assert_eq!(final_value.r#type, "admin");
        assert_eq!(final_value.level, 2);
        assert!(final_value.is_active);
        assert_eq!(final_value.api_version, "v2");
        Ok(())
    }

    #[tokio::test]
    async fn complex_literals_stream_validates_final_payload() -> Result<()> {
        let client = BamlClient::new()?;
        let stream = client
            .test_complex_literals_stream("test complex literals stream")
            .await?;

        let (_partials, final_value) = collect_stream(stream).await?;
        let final_value: ComplexLiterals =
            final_value.expect("expected final complex literals result");

        assert_eq!(final_value.state, "published");
        assert_eq!(final_value.retry_count, 5);
        assert_eq!(final_value.response, "success");
        assert_eq!(final_value.flags.len(), 3);
        assert_eq!(final_value.codes.len(), 3);
        Ok(())
    }
}
