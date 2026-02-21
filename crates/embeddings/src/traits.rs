use async_trait::async_trait;
use legal_core::errors::LegalMcpError;
use pgvector::Vector;

/// Abstraction over text embedding providers.
///
/// Implementations must be `Send + Sync` to allow sharing across async tasks.
/// The primary production implementation is [`crate::vertex::VertexAiEmbedder`];
/// [`crate::mock::MockEmbedder`] is used in tests.
#[async_trait]
pub trait Embedder: Send + Sync {
    /// Embed a batch of texts, returning one 768-dimensional vector per input.
    ///
    /// Implementations MUST batch requests internally when `texts.len() > 250`.
    /// The returned `Vec` has the same length and order as `texts`.
    ///
    /// # Errors
    /// Returns [`LegalMcpError::Embedding`] on API failure or dimension mismatch.
    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vector>, LegalMcpError>;

    /// Convenience wrapper for a single text.
    async fn embed_one(&self, text: &str) -> Result<Vector, LegalMcpError> {
        let mut results = self.embed_batch(&[text.to_owned()]).await?;
        results.pop().ok_or_else(|| {
            LegalMcpError::Embedding("embed_batch returned empty results for single input".into())
        })
    }
}
