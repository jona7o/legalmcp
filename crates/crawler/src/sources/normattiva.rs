//! Normattiva crawler — fetches Italian law via the Normattiva REST API.
//!
//! Normattiva: https://www.normattiva.it/rest
//!
//! Flow:
//! 1. Query `/search/atto` with type filters (legge, decreto-legge, decreto-legislativo)
//! 2. Paginate results, collecting atto IDs
//! 3. For each atto, fetch the full HTML via `/caricaArticolo`
//! 4. Emit `RawDocument { language: "it", jurisdiction: "IT" }`

use async_trait::async_trait;
use reqwest_middleware::ClientWithMiddleware;
use serde_json::{json, Value};
use tracing::{debug, warn};
use uuid::Uuid;

use ingest::parser::DocFormat;
use legal_core::errors::LegalMcpError;

use crate::sources::base::{BaseCrawler, CrawlerState, RawDocument, SharedRateLimiter};

const NORMATTIVA_BASE: &str = "https://www.normattiva.it/rest";
const PAGE_SIZE: usize = 20;

/// Act types to crawl from Normattiva.
const ACT_TYPES: &[&str] = &["LEGGE", "DECRETO_LEGGE", "DECRETO_LEGISLATIVO"];

pub struct NormattivaCrawler {
    state: CrawlerState,
    api_base: String,
}

impl NormattivaCrawler {
    pub fn new(source_id: Uuid) -> Result<Self, LegalMcpError> {
        Ok(Self {
            state: CrawlerState::new(source_id, NORMATTIVA_BASE, 2)?,
            api_base: NORMATTIVA_BASE.into(),
        })
    }

    #[cfg(test)]
    pub fn with_api_base(mut self, base: impl Into<String>) -> Self {
        self.api_base = base.into();
        self
    }
}

#[async_trait]
impl BaseCrawler for NormattivaCrawler {
    fn source_id(&self) -> Uuid {
        self.state.source_id
    }
    fn crawl_url(&self) -> &str {
        &self.state.crawl_url
    }
    fn rate_limiter(&self) -> &SharedRateLimiter {
        &self.state.rate_limiter
    }
    fn http_client(&self) -> &ClientWithMiddleware {
        &self.state.client
    }

    async fn crawl(&self) -> Result<Vec<RawDocument>, LegalMcpError> {
        let mut docs = Vec::new();

        for act_type in ACT_TYPES {
            let entries = self.search_acts(act_type).await?;
            debug!("Normattiva: {} {} entries", entries.len(), act_type);

            for entry in entries {
                let atto_id = match entry.get("id").and_then(|v| v.as_str()) {
                    Some(id) => id.to_string(),
                    None => continue,
                };
                let title = entry
                    .get("titoloAtto")
                    .and_then(|v| v.as_str())
                    .unwrap_or(&atto_id)
                    .to_string();

                let html_url = format!(
                    "{}/caricaArticolo?dataGU=&dataTesto=&normaInVigoreDal=&redaz=&id={}&version=",
                    self.api_base, atto_id
                );

                match self.fetch_bytes(&html_url).await {
                    Ok(raw_bytes) => {
                        docs.push(RawDocument {
                            external_id: format!("normattiva_{atto_id}"),
                            url: html_url,
                            title,
                            raw_bytes,
                            format: DocFormat::Html,
                            metadata_json: json!({
                                "act_type": act_type,
                                "jurisdiction": "IT",
                                "language": "it",
                                "source": "normattiva"
                            }),
                        });
                    }
                    Err(e) => {
                        warn!("Failed to fetch Normattiva {atto_id}: {e}");
                    }
                }
            }
        }

        Ok(docs)
    }
}

impl NormattivaCrawler {
    async fn search_acts(&self, act_type: &str) -> Result<Vec<Value>, LegalMcpError> {
        let url = format!(
            "{}/search/atto?tipoAtto={act_type}&articolazione=0&start=0&rows={PAGE_SIZE}",
            self.api_base
        );
        let resp = self
            .http_client()
            .get(&url)
            .send()
            .await
            .map_err(|e| LegalMcpError::ExternalApi {
                service: "normattiva".into(),
                message: e.to_string(),
            })?
            .error_for_status()
            .map_err(|e| LegalMcpError::ExternalApi {
                service: "normattiva".into(),
                message: e.to_string(),
            })?
            .json::<Value>()
            .await
            .map_err(|e| LegalMcpError::Parse(e.to_string()))?;

        // Response shape: { "atti": [...] } or { "docs": [...] } depending on endpoint version.
        let acts = resp
            .get("atti")
            .or_else(|| resp.get("docs"))
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        Ok(acts)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::{
        matchers::{method, path_regex},
        Mock, MockServer, ResponseTemplate,
    };

    #[tokio::test]
    async fn test_crawl_emits_raw_documents() {
        let server = MockServer::start().await;

        // Search endpoint — same response for all act types.
        Mock::given(method("GET"))
            .and(path_regex("/search/atto.*"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(r#"{"atti":[{"id":"12345","titoloAtto":"Legge 123/2023"}]}"#),
            )
            .mount(&server)
            .await;

        // Article fetch endpoint.
        Mock::given(method("GET"))
            .and(path_regex("/caricaArticolo.*"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string("<html><body>Legge content</body></html>"),
            )
            .mount(&server)
            .await;

        let crawler = NormattivaCrawler::new(Uuid::new_v4())
            .unwrap()
            .with_api_base(server.uri());

        let docs = crawler.crawl().await.unwrap();
        // 3 act types × 1 entry each.
        assert_eq!(docs.len(), 3);
        assert_eq!(docs[0].metadata_json["jurisdiction"], "IT");
        assert_eq!(docs[0].metadata_json["language"], "it");
        assert_eq!(docs[0].format, DocFormat::Html);
    }

    #[tokio::test]
    async fn test_missing_id_entry_skipped() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path_regex("/search/atto.*"))
            .respond_with(ResponseTemplate::new(200).set_body_string(
                // Entry without "id" field should be skipped gracefully.
                r#"{"atti":[{"titoloAtto":"Legge senza ID"}]}"#,
            ))
            .mount(&server)
            .await;

        let crawler = NormattivaCrawler::new(Uuid::new_v4())
            .unwrap()
            .with_api_base(server.uri());

        let docs = crawler.crawl().await.unwrap();
        assert_eq!(docs.len(), 0);
    }
}
