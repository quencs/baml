//! Render options for controlling output formatting.

/// Controls rendering behavior for curl and request building.
#[derive(Debug, Clone, Default)]
pub struct RenderOptions {
    /// Whether to show actual API keys or mask them.
    pub expose_secrets: bool,
    /// Whether to expand media URLs to base64 inline data.
    pub expand_media: bool,
}

impl RenderOptions {
    /// Create options that expose secrets (for actual API calls).
    pub fn for_execution() -> Self {
        Self {
            expose_secrets: true,
            expand_media: true,
        }
    }

    /// Create options for display (masks secrets, shows URLs).
    pub fn for_display() -> Self {
        Self::default()
    }
}
