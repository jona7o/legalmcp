//! Bayern.Recht crawler — fetches Bavarian state law via HTML scraping.
//!
//! Flow:
//! 1. Fetch the index page (catalog listing all laws)
//! 2. Extract law URLs using CSS selectors
//! 3. Paginate until no more "next page" link is found
//! 4. For each law URL, fetch and emit a `RawDocument { format: DocFormat::Html }`

use async_trait::async_trait;
use reqwest_middleware::ClientWithMiddleware;
use scraper::{Html, Selector};
use serde_json::json;
use tracing::{debug, warn};
use uuid::Uuid;

use ingest::parser::DocFormat;
use legal_core::errors::LegalMcpError;

use crate::sources::base::{BaseCrawler, CrawlerState, RawDocument, SharedRateLimiter};

const BASE_URL: &str = "https://www.gesetze-bayern.de";
const INDEX_PATH: &str = "/Content/Catalog";

pub struct BayernCrawler {
    state: CrawlerState,
    index_url: String,
}

impl BayernCrawler {
    pub fn new(source_id: Uuid) -> Result<Self, LegalMcpError> {
        let base = BASE_URL.to_string();
        let index_url = format!("{base}{INDEX_PATH}");
        Ok(Self {
            state: CrawlerState::new(source_id, base, 1)?,
            index_url,
        })
    }

    /// Override the index URL (for tests).
    #[cfg(test)]
    pub fn with_index_url(mut self, url: impl Into<String>) -> Self {
        self.index_url = url.into();
        self
    }
}

#[async_trait]
impl BaseCrawler for BayernCrawler {
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
        // Collect all law URLs from paginated index.
        let law_urls = self.collect_law_urls().await?;
        debug!("Bayern.Recht: found {} law URLs", law_urls.len());

        let mut docs = Vec::with_capacity(law_urls.len());
        for law_url in law_urls {
            match self.fetch_bytes(&law_url).await {
                Ok(raw_bytes) => {
                    let external_id = url_to_id(&law_url);
                    // Extract title from HTML without full parsing to keep crawl lightweight.
                    let title = extract_title_from_html(&raw_bytes);
                    docs.push(RawDocument {
                        external_id,
                        url: law_url,
                        title,
                        raw_bytes,
                        format: DocFormat::Html,
                        metadata_json: json!({ "jurisdiction": "BY", "source": "bayernrecht" }),
                    });
                }
                Err(e) => {
                    warn!("Failed to fetch {law_url}: {e}");
                }
            }
        }
        Ok(docs)
    }
}

impl BayernCrawler {
    /// Walk paginated index pages and collect all individual law URLs.
    async fn collect_law_urls(&self) -> Result<Vec<String>, LegalMcpError> {
        let mut urls: Vec<String> = Vec::new();
        let mut next_url: Option<String> = Some(self.index_url.clone());

        while let Some(page_url) = next_url.take() {
            let html_bytes = self.fetch_bytes(&page_url).await?;
            let html_str = String::from_utf8_lossy(&html_bytes);
            let document = Html::parse_document(&html_str);

            // Derive the base origin from the current page URL (host + scheme).
            let base = origin_of(&page_url);

            // Collect law links: <a href="/Document/..."> or <a href="/Norm/...">
            let link_sel = Selector::parse("a[href]").expect("valid selector");
            for el in document.select(&link_sel) {
                if let Some(href) = el.value().attr("href") {
                    if href.contains("/Document/") || href.contains("/Norm/") {
                        let full = resolve_href(&base, href);
                        if !urls.contains(&full) {
                            urls.push(full);
                        }
                    }
                }
            }

            // Check for a "next page" link.
            let next_sel = Selector::parse("a.next, a[rel='next']").expect("valid selector");
            next_url = document.select(&next_sel).next().and_then(|el| {
                el.value()
                    .attr("href")
                    .map(|href| resolve_href(&base, href))
            });
        }

        Ok(urls)
    }
}

// ── helpers ───────────────────────────────────────────────────────────────────

fn resolve_href(base: &str, href: &str) -> String {
    if href.starts_with("http://") || href.starts_with("https://") {
        return href.to_string();
    }
    format!(
        "{}/{}",
        base.trim_end_matches('/'),
        href.trim_start_matches('/')
    )
}

/// Return the origin (scheme + host + port) of a URL string.
/// E.g. "http://127.0.0.1:8080/some/path" → "http://127.0.0.1:8080"
fn origin_of(url: &str) -> String {
    // Find end of scheme ("://")
    if let Some(after_scheme) = url.find("://").map(|i| i + 3) {
        // Find first '/' after the authority
        if let Some(slash) = url[after_scheme..].find('/') {
            return url[..after_scheme + slash].to_string();
        }
    }
    url.to_string()
}

fn url_to_id(url: &str) -> String {
    // Take the meaningful path segment, e.g. "Document_BayBG_12345" from the URL.
    let path = url.trim_end_matches('/').rsplit('/').next().unwrap_or(url);
    format!("bayern_{}", path.replace('.', "_"))
}

/// Quickly extract `<title>` text from raw HTML bytes without full parsing.
fn extract_title_from_html(bytes: &[u8]) -> String {
    let s = String::from_utf8_lossy(bytes);
    if let Some(start) = s.to_lowercase().find("<title") {
        if let Some(close) = s[start..].find('>') {
            let after = &s[start + close + 1..];
            if let Some(end) = after.to_lowercase().find("</title>") {
                return after[..end].trim().to_string();
            }
        }
    }
    String::new()
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

    #[test]
    fn test_url_to_id() {
        assert_eq!(
            url_to_id("https://example.com/Document/BayBG"),
            "bayern_BayBG"
        );
        assert_eq!(
            url_to_id("https://example.com/Norm/BayPO.1"),
            "bayern_BayPO_1"
        );
    }

    #[test]
    fn test_extract_title_from_html() {
        let html = b"<html><head><title>Bayerisches Schulgesetz</title></head><body></body></html>";
        assert_eq!(extract_title_from_html(html), "Bayerisches Schulgesetz");
    }

    #[test]
    fn test_extract_title_missing() {
        assert_eq!(extract_title_from_html(b"<html></html>"), "");
    }

    #[test]
    fn test_resolve_href_relative() {
        let result = resolve_href("https://example.com", "/Document/BayBG");
        assert_eq!(result, "https://example.com/Document/BayBG");
    }

    #[test]
    fn test_resolve_href_absolute() {
        let url = "https://other.com/Document/X";
        assert_eq!(resolve_href("https://example.com", url), url);
    }

    #[tokio::test]
    async fn test_crawl_emits_raw_documents() {
        let server = MockServer::start().await;

        // Index page with two law links.
        Mock::given(method("GET"))
            .and(path("/Content/Catalog"))
            .respond_with(ResponseTemplate::new(200).set_body_string(format!(
                r#"<html><body>
                    <a href="/Document/BayBG">Bayerisches Gemeindeordnung</a>
                    <a href="/Norm/BaySchG">Bayerisches Schulgesetz</a>
                  </body></html>"#,
            )))
            .mount(&server)
            .await;

        // Serve each law page.
        Mock::given(method("GET"))
            .and(path("/Document/BayBG"))
            .respond_with(
                ResponseTemplate::new(200).set_body_string(
                    "<html><head><title>BayBG</title></head><body>Law</body></html>",
                ),
            )
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/Norm/BaySchG"))
            .respond_with(ResponseTemplate::new(200).set_body_string(
                "<html><head><title>BaySchG</title></head><body>Law</body></html>",
            ))
            .mount(&server)
            .await;

        let index_url = format!("{}/Content/Catalog", server.uri());
        let crawler = BayernCrawler::new(Uuid::new_v4())
            .unwrap()
            .with_index_url(index_url);

        // Override crawl_url so resolve_href works in tests.
        let docs = crawler.crawl().await.unwrap();
        assert_eq!(docs.len(), 2);
        for doc in &docs {
            assert_eq!(doc.format, DocFormat::Html);
            assert!(!doc.raw_bytes.is_empty());
        }
    }
}
