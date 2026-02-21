use thiserror::Error;

/// Central error type for all LegalMCP operations.
///
/// All subsystems (crawler, ingest, embeddings, API) convert their local errors
/// into this type before propagating up the call stack.
#[derive(Debug, Error)]
pub enum LegalMcpError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Embedding error: {0}")]
    Embedding(String),

    #[error("Crawl error for source {source_name}: {message}")]
    Crawl {
        source_name: String,
        message: String,
    },

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Object store error: {0}")]
    ObjectStore(#[from] object_store::Error),

    #[error("External API error [{service}]: {message}")]
    ExternalApi { service: String, message: String },

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Validation error: {0}")]
    Validation(String),
}

/// RFC 7807 Problem Details response body.
///
/// Serialised to JSON by the API layer when returning HTTP error responses.
#[derive(Debug, serde::Serialize)]
pub struct ProblemDetail {
    #[serde(rename = "type")]
    pub problem_type: String,
    pub title: String,
    pub status: u16,
    pub detail: String,
}

impl ProblemDetail {
    pub fn new(
        problem_type: impl Into<String>,
        title: impl Into<String>,
        status: u16,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            problem_type: problem_type.into(),
            title: title.into(),
            status,
            detail: detail.into(),
        }
    }
}

impl LegalMcpError {
    /// Map this error to an HTTP status code.
    pub fn http_status(&self) -> u16 {
        match self {
            Self::NotFound(_) => 404,
            Self::Unauthorized => 401,
            Self::Validation(_) => 422,
            Self::Database(_) => 500,
            Self::Embedding(_) => 502,
            Self::Crawl { .. } => 502,
            Self::Parse(_) => 422,
            Self::ObjectStore(_) => 502,
            Self::ExternalApi { .. } => 502,
        }
    }

    /// Convert to an RFC 7807 ProblemDetail for HTTP responses.
    pub fn to_problem_detail(&self) -> ProblemDetail {
        let base = "https://legalmcp.example.com/errors";
        match self {
            Self::NotFound(msg) => {
                ProblemDetail::new(format!("{base}/not-found"), "Not Found", 404, msg)
            }
            Self::Unauthorized => ProblemDetail::new(
                format!("{base}/unauthorized"),
                "Unauthorized",
                401,
                "Authentication required",
            ),
            Self::Validation(msg) => {
                ProblemDetail::new(format!("{base}/validation"), "Validation Error", 422, msg)
            }
            Self::Database(e) => ProblemDetail::new(
                format!("{base}/internal"),
                "Internal Server Error",
                500,
                e.to_string(),
            ),
            Self::Embedding(msg) | Self::Parse(msg) => {
                ProblemDetail::new(format!("{base}/processing"), "Processing Error", 502, msg)
            }
            Self::ObjectStore(e) => ProblemDetail::new(
                format!("{base}/storage"),
                "Storage Error",
                502,
                e.to_string(),
            ),
            Self::Crawl { message, .. } => {
                ProblemDetail::new(format!("{base}/crawl"), "Crawl Error", 502, message)
            }
            Self::ExternalApi { message, .. } => ProblemDetail::new(
                format!("{base}/external-api"),
                "External API Error",
                502,
                message,
            ),
        }
    }
}

/// Convenience alias used throughout the codebase.
pub type Result<T> = std::result::Result<T, LegalMcpError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn not_found_maps_to_404() {
        let err = LegalMcpError::NotFound("doc-123".into());
        assert_eq!(err.http_status(), 404);
        let pd = err.to_problem_detail();
        assert_eq!(pd.status, 404);
        assert!(pd.problem_type.contains("not-found"));
    }

    #[test]
    fn unauthorized_maps_to_401() {
        let err = LegalMcpError::Unauthorized;
        assert_eq!(err.http_status(), 401);
    }

    #[test]
    fn validation_maps_to_422() {
        let err = LegalMcpError::Validation("field required".into());
        assert_eq!(err.http_status(), 422);
        let pd = err.to_problem_detail();
        assert_eq!(pd.detail, "field required");
    }

    #[test]
    fn crawl_error_includes_source() {
        let err = LegalMcpError::Crawl {
            source_name: "bundestag".into(),
            message: "timeout".into(),
        };
        let display = err.to_string();
        assert!(display.contains("bundestag"));
        assert!(display.contains("timeout"));
    }

    #[test]
    fn external_api_maps_to_502() {
        let err = LegalMcpError::ExternalApi {
            service: "vertex-ai".into(),
            message: "quota exceeded".into(),
        };
        assert_eq!(err.http_status(), 502);
        let pd = err.to_problem_detail();
        assert!(pd.problem_type.contains("external-api"));
    }

    #[test]
    fn problem_detail_serializes_to_json() {
        let pd = ProblemDetail::new(
            "https://legalmcp.example.com/errors/not-found",
            "Not Found",
            404,
            "No document found",
        );
        let json = serde_json::to_string(&pd).unwrap();
        assert!(json.contains("\"type\""));
        assert!(json.contains("404"));
    }
}
