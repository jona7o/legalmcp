use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A scheduled crawl run recorded in `crawl_runs`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawlRun {
    pub id: Uuid,
    pub source_id: Uuid,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub status: CrawlRunStatus,
    pub docs_discovered: i32,
    pub docs_new: i32,
    pub docs_updated: i32,
    pub docs_unchanged: i32,
    pub error_message: Option<String>,
    pub metadata_json: serde_json::Value,
}

/// Status values for `crawl_runs.status`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CrawlRunStatus {
    Running,
    Completed,
    Failed,
}

impl CrawlRunStatus {
    pub fn as_str(&self) -> &str {
        match self {
            CrawlRunStatus::Running => "running",
            CrawlRunStatus::Completed => "completed",
            CrawlRunStatus::Failed => "failed",
        }
    }
}

impl std::fmt::Display for CrawlRunStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// An ingestion job payload pushed to the apalis queue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestJob {
    pub document_id: Uuid,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crawl_run_status_roundtrip() {
        for s in [
            CrawlRunStatus::Running,
            CrawlRunStatus::Completed,
            CrawlRunStatus::Failed,
        ] {
            let json = serde_json::to_string(&s).unwrap();
            let back: CrawlRunStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(s, back);
        }
    }

    #[test]
    fn ingest_job_roundtrip() {
        let job = IngestJob {
            document_id: Uuid::new_v4(),
        };
        let json = serde_json::to_string(&job).unwrap();
        let back: IngestJob = serde_json::from_str(&json).unwrap();
        assert_eq!(job.document_id, back.document_id);
    }
}
