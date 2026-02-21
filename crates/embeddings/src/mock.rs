use async_trait::async_trait;
use legal_core::errors::LegalMcpError;
use pgvector::Vector;

use crate::traits::Embedder;

/// Embedding dimension produced by Vertex AI `text-embedding-004`.
pub const EMBEDDING_DIM: usize = 768;

/// Test double that returns deterministic zero-vectors without network I/O.
///
/// Every call to [`embed_batch`] returns a `Vec` of length `texts.len()`,
/// each element being a 768-dimensional vector of `0.0_f32`.
///
/// This embedder is intentionally simple — its purpose is to satisfy the
/// `Embedder` contract in unit and integration tests without any external deps.
#[derive(Debug, Default)]
pub struct MockEmbedder;

#[async_trait]
impl Embedder for MockEmbedder {
    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vector>, LegalMcpError> {
        let vectors = texts
            .iter()
            .map(|_| Vector::from(vec![0.0_f32; EMBEDDING_DIM]))
            .collect();
        Ok(vectors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn returns_correct_count() {
        let embedder = MockEmbedder;
        let texts: Vec<String> = vec!["hello".into(), "world".into(), "foo".into()];
        let result = embedder.embed_batch(&texts).await.unwrap();
        assert_eq!(result.len(), 3);
    }

    #[tokio::test]
    async fn returns_correct_dimension() {
        let embedder = MockEmbedder;
        let result = embedder.embed_batch(&["test".to_string()]).await.unwrap();
        let v: Vec<f32> = result[0].clone().into();
        assert_eq!(v.len(), EMBEDDING_DIM);
    }

    #[tokio::test]
    async fn embed_one_convenience() {
        let embedder = MockEmbedder;
        let v = embedder.embed_one("test").await.unwrap();
        let floats: Vec<f32> = v.into();
        assert_eq!(floats.len(), EMBEDDING_DIM);
    }

    #[tokio::test]
    async fn empty_batch_returns_empty() {
        let embedder = MockEmbedder;
        let result = embedder.embed_batch(&[]).await.unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn large_batch_returns_correct_count() {
        let embedder = MockEmbedder;
        let texts: Vec<String> = (0..300).map(|i| format!("text {i}")).collect();
        let result = embedder.embed_batch(&texts).await.unwrap();
        assert_eq!(result.len(), 300);
    }
}
