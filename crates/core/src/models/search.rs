use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::document::DocType;

/// A search result returned from vector similarity search.
/// Not persisted — assembled at query time from chunks + documents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub chunk_id: Uuid,
    pub document_id: Uuid,
    pub title: String,
    pub url: String,
    pub jurisdiction: String,
    pub doc_type: DocType,
    pub language: String,
    /// Excerpt of matching chunk text.
    pub snippet: String,
    /// Cosine similarity score (0.0–1.0; higher is more similar).
    pub score: f32,
    pub published_at: Option<DateTime<Utc>>,
    pub source_name: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_result_derives_debug_clone_serialize() {
        let r = SearchResult {
            chunk_id: Uuid::new_v4(),
            document_id: Uuid::new_v4(),
            title: "BGB".to_string(),
            url: "https://example.com".to_string(),
            jurisdiction: "DE".to_string(),
            doc_type: DocType::Statute,
            language: "de".to_string(),
            snippet: "§ 1 ...".to_string(),
            score: 0.92,
            published_at: None,
            source_name: "Bundesrecht".to_string(),
        };
        let json = serde_json::to_string(&r).unwrap();
        assert!(json.contains("BGB"));
        let cloned = r.clone();
        assert_eq!(r.chunk_id, cloned.chunk_id);
    }
}
