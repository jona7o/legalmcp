use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A legal document (statute, regulation, case, directive).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: Uuid,
    pub source_id: Uuid,
    pub external_id: String,
    pub url: String,
    pub title: String,
    pub content_md: String,
    pub summary: Option<String>,
    pub doc_type: DocType,
    pub jurisdiction: String,
    pub language: String,
    pub published_at: Option<DateTime<Utc>>,
    pub effective_at: Option<DateTime<Utc>>,
    pub content_hash: String,
    pub object_key: Option<String>,
    pub metadata_json: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Classification of a legal document.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DocType {
    Statute,
    Regulation,
    Case,
    Directive,
}

impl DocType {
    pub fn as_str(&self) -> &str {
        match self {
            DocType::Statute => "statute",
            DocType::Regulation => "regulation",
            DocType::Case => "case",
            DocType::Directive => "directive",
        }
    }
}

impl std::fmt::Display for DocType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for DocType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "statute" => Ok(DocType::Statute),
            "regulation" => Ok(DocType::Regulation),
            "case" => Ok(DocType::Case),
            "directive" => Ok(DocType::Directive),
            other => Err(format!("unknown doc_type: {other}")),
        }
    }
}

/// An immutable snapshot of a document at a previous version (ADR-12).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentVersion {
    pub id: Uuid,
    pub document_id: Uuid,
    pub version_number: i32,
    pub content_hash: String,
    pub content_md: String,
    pub changed_at: DateTime<Utc>,
    pub change_summary: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn doc_type_roundtrip_serde() {
        for dt in [
            DocType::Statute,
            DocType::Regulation,
            DocType::Case,
            DocType::Directive,
        ] {
            let json = serde_json::to_string(&dt).unwrap();
            let back: DocType = serde_json::from_str(&json).unwrap();
            assert_eq!(dt, back);
        }
    }

    #[test]
    fn doc_type_from_str_valid() {
        use std::str::FromStr;
        assert_eq!(DocType::from_str("statute").unwrap(), DocType::Statute);
        assert_eq!(DocType::from_str("directive").unwrap(), DocType::Directive);
    }

    #[test]
    fn doc_type_from_str_invalid() {
        use std::str::FromStr;
        assert!(DocType::from_str("unknown").is_err());
    }
}
