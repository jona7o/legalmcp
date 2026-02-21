use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A text chunk with its pgvector embedding, stored in the `chunks` table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    pub id: Uuid,
    pub document_id: Uuid,
    pub chunk_index: i32,
    pub content: String,
    /// 768-dimension Vertex AI embedding (None before ingestion).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding: Option<pgvector::Vector>,
    pub token_count: i32,
    pub metadata_json: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunk_derives_debug_clone() {
        let chunk = Chunk {
            id: Uuid::new_v4(),
            document_id: Uuid::new_v4(),
            chunk_index: 0,
            content: "test content".to_string(),
            embedding: None,
            token_count: 5,
            metadata_json: serde_json::json!({}),
            created_at: DateTime::<Utc>::default(),
        };
        let cloned = chunk.clone();
        assert_eq!(chunk.chunk_index, cloned.chunk_index);
    }

    #[test]
    fn chunk_serializes_without_embedding() {
        let chunk = Chunk {
            id: Uuid::new_v4(),
            document_id: Uuid::new_v4(),
            chunk_index: 0,
            content: "test".to_string(),
            embedding: None,
            token_count: 1,
            metadata_json: serde_json::json!({}),
            created_at: DateTime::<Utc>::default(),
        };
        let json = serde_json::to_string(&chunk).unwrap();
        assert!(!json.contains("embedding"));
    }
}
