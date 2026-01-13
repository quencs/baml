//! Media rewriting for LLM requests.

use crate::errors::MediaResolveError;

/// Trait for provider-specific requests that support media rewriting.
pub trait MediaRewritable {
    fn has_unresolved_media(&self) -> bool;

    #[cfg(feature = "native")]
    fn resolve_media(
        &mut self,
    ) -> impl std::future::Future<Output = Result<(), MediaResolveError>> + Send;

    #[cfg(all(target_arch = "wasm32", not(feature = "native")))]
    fn resolve_media(&mut self)
    -> impl std::future::Future<Output = Result<(), MediaResolveError>>;
}
