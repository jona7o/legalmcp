//! MCP tool server — rmcp 0.16 Streamable HTTP transport.
//!
//! Exposes 4 MCP tools:
//!   - `search_laws`      — semantic vector search over `chunks`
//!   - `get_law_by_id`    — fetch full document by UUID
//!   - `get_sources`      — list configured crawl sources
//!   - `get_legal_changes`— list recently changed documents
//!
//! Mounted at `POST /mcp` via [`build_mcp_service`].

use std::sync::Arc;

use chrono::{DateTime, Utc};
use rmcp::{
    handler::server::router::tool::ToolRouter,
    model::{CallToolResult, Content, ServerCapabilities, ServerInfo},
    schemars, tool, tool_handler, tool_router,
    transport::streamable_http_server::{
        session::local::LocalSessionManager, StreamableHttpServerConfig, StreamableHttpService,
    },
    ErrorData as McpError, ServerHandler,
};
use schemars::JsonSchema;
use serde::Deserialize;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::{
    services::{
        changes::ChangesService,
        document::DocumentService,
        search::{SearchFilters, SearchService},
        source::SourceService,
    },
    state::AppState,
};

// ─── Input parameter structs ────────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SearchLawsParams {
    /// The search query (natural language or legal term).
    pub query: String,
    /// Optional jurisdiction filter (e.g. "DE", "EU", "FR", "IT", "ES").
    pub jurisdiction: Option<String>,
    /// Optional document type filter (e.g. "statute", "regulation", "case").
    pub doc_type: Option<String>,
    /// Maximum number of results to return (1–20, default 10).
    pub limit: Option<u8>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetLawByIdParams {
    /// UUID of the document to retrieve.
    pub document_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetLegalChangesParams {
    /// Return changes after this ISO-8601 timestamp (default: 30 days ago).
    pub since: Option<String>,
    /// Maximum results to return (1–50, default 20).
    pub limit: Option<u8>,
}

// ─── Server handler ──────────────────────────────────────────────────────────

/// MCP tool handler holding shared app state.
#[derive(Clone)]
pub struct LegalMcpServer {
    state: Arc<AppState>,
    #[allow(dead_code)] // used by #[tool_router] macro
    tool_router: ToolRouter<Self>,
}

impl LegalMcpServer {
    pub fn new(state: Arc<AppState>) -> Self {
        Self {
            state,
            tool_router: Self::tool_router(),
        }
    }
}

#[tool_router]
impl LegalMcpServer {
    /// Search EU and national laws using semantic vector similarity.
    #[tool(
        name = "search_laws",
        description = "Perform semantic search over EU and national legal texts. Returns ranked results with title, URL, jurisdiction, and a relevant excerpt."
    )]
    async fn search_laws(
        &self,
        rmcp::handler::server::wrapper::Parameters(params): rmcp::handler::server::wrapper::Parameters<SearchLawsParams>,
    ) -> Result<CallToolResult, McpError> {
        let svc = SearchService::new(self.state.pool.clone(), Arc::clone(&self.state.embedder));

        let limit = params.limit.unwrap_or(10).min(20) as i64;
        let filters = SearchFilters {
            jurisdiction: params.jurisdiction,
            doc_type: params.doc_type,
            language: None,
        };

        let resp = svc
            .search(&params.query, filters, limit, 0)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        if resp.results.is_empty() {
            return Ok(CallToolResult::success(vec![Content::text(
                "No results found for your query.",
            )]));
        }

        let mut lines = Vec::with_capacity(resp.results.len() * 5 + 2);
        lines.push(format!(
            "Found {} result(s) for \"{}\"\n",
            resp.total, params.query
        ));

        for (i, r) in resp.results.iter().enumerate() {
            lines.push(format!(
                "{}. **{}** ({})\n   Source: {} | Jurisdiction: {} | Type: {}\n   Score: {:.3}\n   URL: {}\n   Excerpt: {}\n",
                i + 1,
                r.title,
                r.language.to_uppercase(),
                r.source_name,
                r.jurisdiction,
                r.doc_type,
                r.score,
                r.url,
                r.snippet.chars().take(300).collect::<String>(),
            ));
        }

        Ok(CallToolResult::success(vec![Content::text(
            lines.join("\n"),
        )]))
    }

    /// Retrieve the full text and metadata of a specific law or document.
    #[tool(
        name = "get_law_by_id",
        description = "Retrieve the full text, summary, and metadata for a specific legal document by its UUID."
    )]
    async fn get_law_by_id(
        &self,
        rmcp::handler::server::wrapper::Parameters(params): rmcp::handler::server::wrapper::Parameters<GetLawByIdParams>,
    ) -> Result<CallToolResult, McpError> {
        let id = Uuid::parse_str(&params.document_id).map_err(|_| {
            McpError::invalid_params(format!("Invalid UUID: {}", params.document_id), None)
        })?;

        let svc = DocumentService::new(self.state.pool.clone());
        let doc = svc
            .get_document(id)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let published = doc
            .published_at
            .map(|d| d.format("%Y-%m-%d").to_string())
            .unwrap_or_else(|| "unknown".into());

        let summary_section = doc
            .summary
            .as_deref()
            .map(|s| format!("\n**Summary:**\n{s}\n"))
            .unwrap_or_default();

        let text = format!(
            "# {title}\n\n\
             - **ID:** {id}\n\
             - **Jurisdiction:** {jurisdiction}\n\
             - **Type:** {doc_type}\n\
             - **Language:** {language}\n\
             - **Published:** {published}\n\
             - **URL:** {url}\n\
             {summary_section}\n\
             **Full Text:**\n{content}",
            title = doc.title,
            id = doc.id,
            jurisdiction = doc.jurisdiction,
            doc_type = doc.doc_type,
            language = doc.language,
            url = doc.url,
            content = doc.content_md,
        );

        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    /// List all configured legal source crawlers.
    #[tool(
        name = "get_sources",
        description = "List all configured legal source crawlers including their jurisdiction, language, and crawl schedule."
    )]
    async fn get_sources(&self) -> Result<CallToolResult, McpError> {
        let svc = SourceService::new(self.state.pool.clone());
        let sources = svc
            .list_sources()
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        if sources.is_empty() {
            return Ok(CallToolResult::success(vec![Content::text(
                "No sources configured.",
            )]));
        }

        let mut lines = vec![format!("{} source(s) configured:\n", sources.len())];
        for s in &sources {
            lines.push(format!(
                "- **{}** | Jurisdiction: {} | Language: {} | Schedule: {} | Enabled: {}",
                s.name, s.jurisdiction, s.language, s.cron_schedule, s.enabled,
            ));
        }

        Ok(CallToolResult::success(vec![Content::text(
            lines.join("\n"),
        )]))
    }

    /// Retrieve recently changed legal documents.
    #[tool(
        name = "get_legal_changes",
        description = "List legal documents that have changed recently, including a summary of what changed and when."
    )]
    async fn get_legal_changes(
        &self,
        rmcp::handler::server::wrapper::Parameters(params): rmcp::handler::server::wrapper::Parameters<GetLegalChangesParams>,
    ) -> Result<CallToolResult, McpError> {
        let since: DateTime<Utc> = if let Some(ref s) = params.since {
            s.parse::<DateTime<Utc>>().map_err(|_| {
                McpError::invalid_params(
                    format!("Invalid timestamp: {s}. Use ISO-8601 format."),
                    None,
                )
            })?
        } else {
            Utc::now() - chrono::Duration::days(30)
        };

        let limit = params.limit.unwrap_or(20).min(50) as i64;
        let svc = ChangesService::new(self.state.pool.clone());
        let changes = svc
            .list_changes(since, limit, 0)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        if changes.is_empty() {
            return Ok(CallToolResult::success(vec![Content::text(format!(
                "No changes found since {}.",
                since.format("%Y-%m-%d")
            ))]));
        }

        let mut lines = vec![format!(
            "{} document(s) changed since {}:\n",
            changes.len(),
            since.format("%Y-%m-%d")
        )];

        for c in &changes {
            let summary = c
                .change_summary
                .as_deref()
                .unwrap_or("(no summary available)");
            lines.push(format!(
                "- **{}** (v{}) | {} | {} | Changed: {}\n  Summary: {}",
                c.title,
                c.version_number,
                c.jurisdiction,
                c.url,
                c.changed_at.format("%Y-%m-%d %H:%M UTC"),
                summary,
            ));
        }

        Ok(CallToolResult::success(vec![Content::text(
            lines.join("\n"),
        )]))
    }
}

#[tool_handler]
impl ServerHandler for LegalMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "Legal MCP — semantic search and retrieval over EU and national legal texts. \
                 Use search_laws to find relevant laws, get_law_by_id to read full text, \
                 get_sources to see available jurisdictions, and get_legal_changes to track updates."
                    .into(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

// ─── Service factory ─────────────────────────────────────────────────────────

/// Build the Streamable HTTP MCP service, ready to be mounted with
/// `router.nest_service("/mcp", build_mcp_service(state))`.
pub fn build_mcp_service(
    state: Arc<AppState>,
) -> StreamableHttpService<LegalMcpServer, LocalSessionManager> {
    let ct = CancellationToken::new();
    StreamableHttpService::new(
        move || Ok(LegalMcpServer::new(Arc::clone(&state))),
        Default::default(),
        StreamableHttpServerConfig {
            stateful_mode: true,
            cancellation_token: ct,
            ..Default::default()
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_laws_params_deserializes() {
        let json = r#"{"query":"Mietrecht","jurisdiction":"DE","limit":5}"#;
        let p: SearchLawsParams = serde_json::from_str(json).unwrap();
        assert_eq!(p.query, "Mietrecht");
        assert_eq!(p.jurisdiction.as_deref(), Some("DE"));
        assert_eq!(p.limit, Some(5));
    }

    #[test]
    fn test_get_law_by_id_params_deserializes() {
        let uuid = "550e8400-e29b-41d4-a716-446655440000";
        let json = format!(r#"{{"document_id":"{uuid}"}}"#);
        let p: GetLawByIdParams = serde_json::from_str(&json).unwrap();
        assert_eq!(p.document_id, uuid);
        // Must parse as valid UUID
        assert!(Uuid::parse_str(&p.document_id).is_ok());
    }

    #[test]
    fn test_invalid_uuid_detected() {
        assert!(Uuid::parse_str("not-a-uuid").is_err());
    }

    #[test]
    fn test_since_default_is_30_days_ago() {
        let now = Utc::now();
        let thirty_days_ago = now - chrono::Duration::days(30);
        assert!(thirty_days_ago < now);
    }

    #[test]
    fn test_get_legal_changes_params_deserializes_with_since() {
        let json = r#"{"since":"2024-01-01T00:00:00Z","limit":10}"#;
        let p: GetLegalChangesParams = serde_json::from_str(json).unwrap();
        assert!(p.since.is_some());
        let ts: DateTime<Utc> = p.since.unwrap().parse().unwrap();
        assert_eq!(ts.format("%Y-%m-%d").to_string(), "2024-01-01");
    }
}
