//! EUR-Lex crawler — fetches EU legislation via the SPARQL endpoint.
//!
//! Flow:
//! 1. POST SPARQL query to `https://publications.europa.eu/webapi/rdf/sparql`
//!    to list recent EU Regulations (REG) and Directives (DIR)
//! 2. Paginate with OFFSET/LIMIT (100 results per page)
//! 3. For each CELEX number, resolve the HTML document URL and fetch it
//! 4. Emit `RawDocument { format: DocFormat::Html }`

use async_trait::async_trait;
use reqwest_middleware::ClientWithMiddleware;
use serde_json::{json, Value};
use tracing::{debug, warn};
use uuid::Uuid;

use ingest::parser::DocFormat;
use legal_core::errors::LegalMcpError;

use crate::sources::base::{BaseCrawler, CrawlerState, RawDocument, SharedRateLimiter};

const SPARQL_ENDPOINT: &str = "https://publications.europa.eu/webapi/rdf/sparql";
const CELLAR_BASE: &str = "https://eur-lex.europa.eu/legal-content";
const PAGE_SIZE: usize = 100;

pub struct EurLexCrawler {
    state: CrawlerState,
    sparql_endpoint: String,
    cellar_base: String,
    /// Languages to query (ISO 639-1 lowercase).
    #[allow(dead_code)]
    languages: Vec<String>,
}

impl EurLexCrawler {
    pub fn new(source_id: Uuid) -> Result<Self, LegalMcpError> {
        Ok(Self {
            state: CrawlerState::new(source_id, CELLAR_BASE, 2)?,
            sparql_endpoint: SPARQL_ENDPOINT.into(),
            cellar_base: CELLAR_BASE.into(),
            languages: vec!["de".into(), "en".into(), "fr".into()],
        })
    }

    /// Override endpoints for testing.
    #[cfg(test)]
    pub fn with_endpoints(
        mut self,
        sparql_endpoint: impl Into<String>,
        cellar_base: impl Into<String>,
    ) -> Self {
        self.sparql_endpoint = sparql_endpoint.into();
        self.cellar_base = cellar_base.into();
        self
    }
}

#[async_trait]
impl BaseCrawler for EurLexCrawler {
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
        let entries = self.query_all_legislation().await?;
        debug!("EUR-Lex SPARQL returned {} entries", entries.len());

        let mut docs = Vec::with_capacity(entries.len());
        for entry in entries {
            let celex = match entry.get("celex").and_then(|v| v.as_str()) {
                Some(c) => c.to_string(),
                None => continue,
            };
            let title = entry
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or(&celex)
                .to_string();
            let doc_url = self.build_document_url(&celex);

            match self.fetch_bytes(&doc_url).await {
                Ok(raw_bytes) => {
                    let doc_type = celex_doc_type(&celex);
                    docs.push(RawDocument {
                        external_id: format!("eurlex_{celex}"),
                        url: doc_url,
                        title,
                        raw_bytes,
                        format: DocFormat::Html,
                        metadata_json: json!({
                            "celex": celex,
                            "doc_type": doc_type,
                            "jurisdiction": "EU",
                            "source": "eurlex"
                        }),
                    });
                }
                Err(e) => {
                    warn!("Failed to fetch EUR-Lex {celex}: {e}");
                }
            }
        }
        Ok(docs)
    }
}

impl EurLexCrawler {
    /// Issue paginated SPARQL queries and collect all result bindings.
    async fn query_all_legislation(&self) -> Result<Vec<Value>, LegalMcpError> {
        let mut all: Vec<Value> = Vec::new();
        let mut offset = 0usize;

        loop {
            let query = sparql_query(offset, PAGE_SIZE);
            let resp = self
                .http_client()
                .post(&self.sparql_endpoint)
                .header("Accept", "application/sparql-results+json")
                .header("Content-Type", "application/sparql-query")
                .body(query)
                .send()
                .await
                .map_err(|e| LegalMcpError::ExternalApi {
                    service: "eurlex-sparql".into(),
                    message: e.to_string(),
                })?
                .error_for_status()
                .map_err(|e| LegalMcpError::ExternalApi {
                    service: "eurlex-sparql".into(),
                    message: e.to_string(),
                })?
                .json::<Value>()
                .await
                .map_err(|e| LegalMcpError::Parse(e.to_string()))?;

            let bindings = parse_sparql_bindings(&resp);
            let count = bindings.len();
            all.extend(bindings);

            if count < PAGE_SIZE {
                break; // last page
            }
            offset += PAGE_SIZE;
        }

        Ok(all)
    }

    fn build_document_url(&self, celex: &str) -> String {
        format!("{}/DE/TXT/HTML/?uri=CELEX:{celex}", self.cellar_base)
    }
}

// ── SPARQL helpers ─────────────────────────────────────────────────────────────

fn sparql_query(offset: usize, limit: usize) -> String {
    format!(
        r#"PREFIX cdm: <http://publications.europa.eu/ontology/cdm#>
PREFIX xsd: <http://www.w3.org/2001/XMLSchema#>

SELECT DISTINCT ?celex ?title ?type ?date
WHERE {{
  ?work cdm:work_id_document ?celex .
  ?work cdm:resource_legal_type ?type .
  ?work cdm:work_date_document ?date .
  ?work cdm:expression_title ?title .
  FILTER (lang(?title) = "de")
  FILTER (?type IN (
    <http://publications.europa.eu/resource/authority/resource-type/REG>,
    <http://publications.europa.eu/resource/authority/resource-type/DIR>
  ))
  FILTER (?date >= "2015-01-01"^^xsd:date)
}}
ORDER BY DESC(?date)
LIMIT {limit}
OFFSET {offset}"#
    )
}

/// Extract `[{ "celex": "...", "title": "...", "type": "...", "date": "..." }, ...]`
/// from a SPARQL JSON results object.
fn parse_sparql_bindings(json: &Value) -> Vec<Value> {
    let bindings = json
        .get("results")
        .and_then(|r| r.get("bindings"))
        .and_then(|b| b.as_array());

    match bindings {
        None => vec![],
        Some(rows) => rows
            .iter()
            .filter_map(|row| {
                let celex = row
                    .get("celex")
                    .and_then(|v| v.get("value"))
                    .and_then(|v| v.as_str())?
                    .to_string();
                let title = row
                    .get("title")
                    .and_then(|v| v.get("value"))
                    .and_then(|v| v.as_str())
                    .unwrap_or(&celex)
                    .to_string();
                let doc_type = row
                    .get("type")
                    .and_then(|v| v.get("value"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                Some(json!({ "celex": celex, "title": title, "type": doc_type }))
            })
            .collect(),
    }
}

/// Derive doc_type string from CELEX number (e.g. `32016R0679` → `"regulation"`).
fn celex_doc_type(celex: &str) -> &'static str {
    if celex.len() >= 6 {
        match celex.chars().nth(5) {
            Some('R') => return "regulation",
            Some('L') => return "directive",
            Some('D') => return "decision",
            _ => {}
        }
    }
    "unknown"
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::{
        matchers::{method, path},
        Mock, MockServer, ResponseTemplate,
    };

    const SPARQL_FIXTURE: &str = r#"{
      "results": {
        "bindings": [
          {
            "celex": { "type": "literal", "value": "32016R0679" },
            "title": { "type": "literal", "xml:lang": "de", "value": "Datenschutz-Grundverordnung" },
            "type":  { "type": "uri", "value": "http://publications.europa.eu/resource/authority/resource-type/REG" }
          },
          {
            "celex": { "type": "literal", "value": "32022L2555" },
            "title": { "type": "literal", "xml:lang": "de", "value": "NIS2-Richtlinie" },
            "type":  { "type": "uri", "value": "http://publications.europa.eu/resource/authority/resource-type/DIR" }
          }
        ]
      }
    }"#;

    #[test]
    fn test_parse_sparql_bindings() {
        let json: Value = serde_json::from_str(SPARQL_FIXTURE).unwrap();
        let bindings = parse_sparql_bindings(&json);
        assert_eq!(bindings.len(), 2);
        assert_eq!(bindings[0]["celex"], "32016R0679");
        assert_eq!(bindings[0]["title"], "Datenschutz-Grundverordnung");
    }

    #[test]
    fn test_celex_doc_type() {
        assert_eq!(celex_doc_type("32016R0679"), "regulation");
        assert_eq!(celex_doc_type("32022L2555"), "directive");
        assert_eq!(celex_doc_type("32021D0001"), "decision");
        assert_eq!(celex_doc_type("short"), "unknown");
    }

    #[test]
    fn test_sparql_query_contains_offset() {
        let q = sparql_query(200, 100);
        assert!(q.contains("OFFSET 200"));
        assert!(q.contains("LIMIT 100"));
    }

    #[tokio::test]
    async fn test_crawl_emits_raw_documents() {
        let server = MockServer::start().await;

        // SPARQL response (single page, < PAGE_SIZE → no second request).
        Mock::given(method("POST"))
            .and(path("/sparql"))
            .respond_with(ResponseTemplate::new(200).set_body_string(SPARQL_FIXTURE))
            .mount(&server)
            .await;

        // Document HTML pages.
        Mock::given(method("GET"))
            .and(path("/DE/TXT/HTML/"))
            .respond_with(
                ResponseTemplate::new(200).set_body_string("<html><body>GDPR</body></html>"),
            )
            .mount(&server)
            .await;

        let sparql_url = format!("{}/sparql", server.uri());
        let cellar_url = format!("{}", server.uri());
        let crawler = EurLexCrawler::new(Uuid::new_v4())
            .unwrap()
            .with_endpoints(sparql_url, cellar_url);

        let docs = crawler.crawl().await.unwrap();
        assert_eq!(docs.len(), 2);
        assert!(docs.iter().any(|d| d.external_id == "eurlex_32016R0679"));
        assert!(docs
            .iter()
            .any(|d| d.metadata_json["doc_type"] == "directive"));
        for doc in &docs {
            assert_eq!(doc.format, DocFormat::Html);
        }
    }
}
