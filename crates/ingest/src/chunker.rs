use legal_core::errors::LegalMcpError;
use text_splitter::{ChunkConfig, TextSplitter};
use tiktoken_rs::cl100k_base;

/// Maximum tokens per chunk (matches SDD spec).
pub const MAX_TOKENS: usize = 512;

/// A single text chunk ready for embedding.
#[derive(Debug, Clone)]
pub struct ChunkText {
    /// Zero-based position within the parent document.
    pub chunk_index: usize,
    /// Chunk text content.
    pub content: String,
    /// Approximate token count (from the splitter's tokenizer).
    pub token_count: usize,
}

/// Split `content` into chunks of at most [`MAX_TOKENS`] tokens.
///
/// Uses `text_splitter` with the `cl100k_base` BPE tokenizer (same tokenizer
/// used by Vertex AI `text-embedding-004`). Runs the CPU-bound splitting in a
/// `tokio::task::spawn_blocking` context.
///
/// Returns `LegalMcpError::Parse` if the BPE tokenizer cannot be initialised.
pub async fn chunk_text(content: &str) -> Result<Vec<ChunkText>, LegalMcpError> {
    let content = content.to_owned();
    tokio::task::spawn_blocking(move || chunk_text_sync(&content))
        .await
        .map_err(|e| LegalMcpError::Parse(format!("Chunker task panicked: {e}")))?
}

/// Synchronous chunk implementation — called inside `spawn_blocking`.
fn chunk_text_sync(content: &str) -> Result<Vec<ChunkText>, LegalMcpError> {
    if content.trim().is_empty() {
        return Ok(Vec::new());
    }

    let tokenizer = cl100k_base()
        .map_err(|e| LegalMcpError::Parse(format!("Failed to initialise BPE tokenizer: {e}")))?;

    let config = ChunkConfig::new(MAX_TOKENS).with_sizer(tokenizer);
    let splitter = TextSplitter::new(config);

    let chunks: Vec<ChunkText> = splitter
        .chunks(content)
        .enumerate()
        .map(|(idx, chunk_str)| {
            let token_count = estimate_tokens(chunk_str);
            ChunkText {
                chunk_index: idx,
                content: chunk_str.to_owned(),
                token_count,
            }
        })
        .collect();

    Ok(chunks)
}

/// Rough token estimate: use char count / 4 as a fast lower-bound.
/// The actual count from the splitter is more accurate but this avoids
/// double-tokenising in the metadata field.
fn estimate_tokens(text: &str) -> usize {
    text.chars().count().div_ceil(4)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Generate a string with approximately `n` tokens worth of content.
    fn make_content(words: usize) -> String {
        let word = "legal ";
        word.repeat(words)
    }

    #[tokio::test]
    async fn short_text_produces_single_chunk() {
        let content = "§ 1 Die Rechtsfähigkeit des Menschen beginnt mit der Geburt.";
        let chunks = chunk_text(content).await.unwrap();
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].chunk_index, 0);
    }

    #[tokio::test]
    async fn empty_text_produces_no_chunks() {
        let chunks = chunk_text("").await.unwrap();
        assert!(chunks.is_empty());
    }

    #[tokio::test]
    async fn whitespace_only_produces_no_chunks() {
        let chunks = chunk_text("   \n\t  ").await.unwrap();
        assert!(chunks.is_empty());
    }

    #[tokio::test]
    async fn long_text_splits_into_multiple_chunks() {
        // ~3000 words should produce several 512-token chunks
        let content = make_content(3000);
        let chunks = chunk_text(&content).await.unwrap();
        assert!(
            chunks.len() > 1,
            "Expected multiple chunks, got {}",
            chunks.len()
        );
    }

    #[tokio::test]
    async fn chunk_indices_are_sequential() {
        let content = make_content(3000);
        let chunks = chunk_text(&content).await.unwrap();
        for (i, chunk) in chunks.iter().enumerate() {
            assert_eq!(chunk.chunk_index, i);
        }
    }

    #[tokio::test]
    async fn no_chunk_is_empty() {
        let content = make_content(3000);
        let chunks = chunk_text(&content).await.unwrap();
        for chunk in &chunks {
            assert!(
                !chunk.content.trim().is_empty(),
                "Chunk {}: empty content",
                chunk.chunk_index
            );
        }
    }
}
