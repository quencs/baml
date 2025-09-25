#[cfg(test)]
mod tests {
    use anyhow::Result;
    use baml_client::{
        BamlAudio, BamlClient, BamlImage, BamlPdf, BamlVideo, MediaAnalysisResult,
        Union4AudioOrImageOrPdfOrVideo,
    };

    fn assert_analysis_populated(result: &MediaAnalysisResult) {
        assert!(
            !result.analysis_text.trim().is_empty(),
            "expected analysis text to be non-empty"
        );
    }

    #[tokio::test]
    async fn image_input_produces_analysis() -> Result<()> {
        let client = BamlClient::new()?;
        let media = BamlImage::from_url(
            "https://upload.wikimedia.org/wikipedia/en/4/4d/Shrek_%28character%29.png",
            None,
        );
        let union = Union4AudioOrImageOrPdfOrVideo::image(media);
        let result = client.test_media_input(union, "Analyze this image").await?;

        assert_analysis_populated(&result);
        Ok(())
    }

    #[tokio::test]
    async fn audio_input_produces_analysis() -> Result<()> {
        let client = BamlClient::new()?;
        let media = BamlAudio::from_url("https://download.samplelib.com/mp3/sample-3s.mp3", None);
        let union = Union4AudioOrImageOrPdfOrVideo::audio(media);
        let result = client
            .test_media_input(union, "This is music used for an intro")
            .await?;

        assert_analysis_populated(&result);
        Ok(())
    }

    #[tokio::test]
    async fn pdf_input_produces_analysis() -> Result<()> {
        let client = BamlClient::new()?;
        let media = BamlPdf::from_url(
            "https://www.w3.org/WAI/ER/tests/xhtml/testfiles/resources/pdf/dummy.pdf",
            None,
        );
        let union = Union4AudioOrImageOrPdfOrVideo::pdf(media);
        let result = client.test_media_input(union, "Analyze this PDF").await?;

        assert_analysis_populated(&result);
        Ok(())
    }

    #[tokio::test]
    async fn video_input_produces_analysis() -> Result<()> {
        let client = BamlClient::new()?;
        let media = BamlVideo::from_url("https://www.youtube.com/watch?v=1O0yazhqaxs", None);
        let union = Union4AudioOrImageOrPdfOrVideo::video(media);
        let result = client.test_media_input(union, "Analyze this video").await?;

        assert_analysis_populated(&result);
        Ok(())
    }

    #[tokio::test]
    async fn image_array_input_counts_media() -> Result<()> {
        let client = BamlClient::new()?;
        let images = vec![
            BamlImage::from_url(
                "https://upload.wikimedia.org/wikipedia/en/4/4d/Shrek_%28character%29.png",
                None,
            ),
            BamlImage::from_url(
                "https://upload.wikimedia.org/wikipedia/commons/thumb/a/a7/React-icon.svg/1200px-React-icon.svg.png",
                None,
            ),
        ];

        let result = client
            .test_media_array_inputs(images, "Analyze these images")
            .await?;

        assert!(result.media_count > 0, "expected positive media count");
        assert!(
            !result.analysis_text.trim().is_empty(),
            "expected analysis text to be non-empty"
        );
        Ok(())
    }
}
