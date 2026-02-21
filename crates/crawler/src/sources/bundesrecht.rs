//! Bundesrecht crawler — fetches German federal law from Gesetze-im-Internet.
//!
//! Flow:
//! 1. Fetch the TOC XML at `https://www.gesetze-im-internet.de/gii-toc.xml`
//! 2. Parse each `<item>` to get `<title>` and `<link>` (relative XML zip URL)
//! 3. For each item, resolve the absolute XML URL and emit a `RawDocument`

use async_trait::async_trait;
use quick_xml::events::Event;
use quick_xml::reader::Reader;
use reqwest_middleware::ClientWithMiddleware;
use serde_json::json;
use tracing::{debug, warn};
use uuid::Uuid;

use ingest::parser::DocFormat;
use legal_core::errors::LegalMcpError;

use crate::sources::base::{BaseCrawler, CrawlerState, RawDocument, SharedRateLimiter};

const BASE_URL: &str = "https://www.gesetze-im-internet.de";
const TOC_URL: &str = "https://www.gesetze-im-internet.de/gii-toc.xml";

pub struct BundesrechtCrawler {
    state: CrawlerState,
    toc_url: String,
}

impl BundesrechtCrawler {
    /// Create a new crawler using the given `source_id` from the `sources` table.
    pub fn new(source_id: Uuid) -> Result<Self, LegalMcpError> {
        Ok(Self {
            state: CrawlerState::new(source_id, BASE_URL, 2)?,
            toc_url: TOC_URL.into(),
        })
    }

    /// Override the TOC URL (useful for tests).
    #[cfg(test)]
    pub fn with_toc_url(mut self, url: impl Into<String>) -> Self {
        self.toc_url = url.into();
        self
    }
}

#[async_trait]
impl BaseCrawler for BundesrechtCrawler {
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
        debug!("Fetching Bundesrecht TOC from {}", self.toc_url);
        let toc_bytes = self.fetch_bytes(&self.toc_url).await?;
        let items = parse_toc(&toc_bytes)?;

        debug!("TOC contains {} items", items.len());
        let mut docs = Vec::with_capacity(items.len());

        for item in items {
            let xml_url = resolve_url(&item.link);
            debug!("Fetching {xml_url}");
            match self.fetch_bytes(&xml_url).await {
                Ok(raw_bytes) => {
                    let external_id = extract_external_id(&item.link);
                    docs.push(RawDocument {
                        external_id,
                        url: xml_url,
                        title: item.title,
                        raw_bytes,
                        format: DocFormat::Xml,
                        metadata_json: json!({
                            "abbreviation": item.abbreviation,
                            "source": "bundesrecht"
                        }),
                    });
                }
                Err(e) => {
                    warn!("Failed to fetch {xml_url}: {e}");
                }
            }
        }

        Ok(docs)
    }
}

// ── TOC XML parser ────────────────────────────────────────────────────────────

#[derive(Debug, Default)]
struct TocItem {
    title: String,
    link: String,
    abbreviation: String,
}

/// Parse the Bundesrecht TOC XML and return a list of items.
///
/// The feed looks like:
/// ```xml
/// <items>
///   <item>
///     <title>Bürgerliches Gesetzbuch</title>
///     <link>https://www.gesetze-im-internet.de/bgb/xml.zip</link>
///     <description>BGB</description>
///   </item>
///   ...
/// </items>
/// ```
fn parse_toc(xml: &[u8]) -> Result<Vec<TocItem>, LegalMcpError> {
    let mut reader = Reader::from_reader(xml);
    reader.config_mut().trim_text(true);

    let mut items: Vec<TocItem> = Vec::new();
    let mut current: Option<TocItem> = None;
    let mut current_tag = String::new();

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                let name = std::str::from_utf8(e.name().as_ref())
                    .unwrap_or("")
                    .to_lowercase();
                match name.as_str() {
                    "item" => current = Some(TocItem::default()),
                    _ => current_tag = name,
                }
            }
            Ok(Event::Text(e)) => {
                if let Some(ref mut item) = current {
                    let text = e.unescape().map(|s| s.into_owned()).unwrap_or_default();
                    match current_tag.as_str() {
                        "title" => item.title = text,
                        "link" => item.link = text,
                        "description" | "kurzue" | "jurabk" => {
                            if item.abbreviation.is_empty() {
                                item.abbreviation = text;
                            }
                        }
                        _ => {}
                    }
                }
            }
            Ok(Event::End(e)) => {
                let name = std::str::from_utf8(e.name().as_ref())
                    .unwrap_or("")
                    .to_lowercase();
                if name == "item" {
                    if let Some(item) = current.take() {
                        if !item.link.is_empty() {
                            items.push(item);
                        }
                    }
                }
                current_tag.clear();
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(LegalMcpError::Parse(format!(
                    "Bundesrecht TOC XML error: {e}"
                )));
            }
            _ => {}
        }
    }

    Ok(items)
}

/// Make an absolute URL from an item link (which may already be absolute or
/// may be relative like `/bgb/xml.zip`).
fn resolve_url(link: &str) -> String {
    if link.starts_with("http://") || link.starts_with("https://") {
        link.to_string()
    } else {
        format!("{BASE_URL}/{}", link.trim_start_matches('/'))
    }
}

/// Extract a short identifier from the XML URL, e.g. `"bgb"` from
/// `https://www.gesetze-im-internet.de/bgb/xml.zip`.
fn extract_external_id(link: &str) -> String {
    // Strip trailing /xml.zip, then take the last path component.
    let path = link
        .trim_end_matches('/')
        .trim_end_matches("xml.zip")
        .trim_end_matches('/');
    path.rsplit('/').next().unwrap_or(link).to_string()
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

    const FIXTURE_TOC: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<items>
  <item>
    <title>Bürgerliches Gesetzbuch</title>
    <link>/bgb/xml.zip</link>
    <description>BGB</description>
  </item>
  <item>
    <title>Strafgesetzbuch</title>
    <link>/stgb/xml.zip</link>
    <description>StGB</description>
  </item>
  <item>
    <title>No link item</title>
    <link></link>
  </item>
</items>"#;

    #[test]
    fn test_parse_toc_known_laws() {
        let items = parse_toc(FIXTURE_TOC.as_bytes()).unwrap();
        // The empty-link item should be filtered out.
        assert_eq!(items.len(), 2);
        let ids: Vec<&str> = items.iter().map(|i| i.link.as_str()).collect();
        assert!(ids.contains(&"/bgb/xml.zip"));
        assert!(ids.contains(&"/stgb/xml.zip"));
    }

    #[test]
    fn test_parse_toc_titles() {
        let items = parse_toc(FIXTURE_TOC.as_bytes()).unwrap();
        assert_eq!(items[0].title, "Bürgerliches Gesetzbuch");
        assert_eq!(items[0].abbreviation, "BGB");
    }

    #[test]
    fn test_extract_external_id() {
        assert_eq!(extract_external_id("/bgb/xml.zip"), "bgb");
        assert_eq!(
            extract_external_id("https://www.gesetze-im-internet.de/stgb/xml.zip"),
            "stgb"
        );
        assert_eq!(extract_external_id("/bgb/"), "bgb");
    }

    #[test]
    fn test_resolve_url_relative() {
        assert_eq!(
            resolve_url("/bgb/xml.zip"),
            "https://www.gesetze-im-internet.de/bgb/xml.zip"
        );
    }

    #[test]
    fn test_resolve_url_absolute() {
        let url = "https://www.gesetze-im-internet.de/bgb/xml.zip";
        assert_eq!(resolve_url(url), url);
    }

    #[tokio::test]
    async fn test_crawl_emits_raw_documents() {
        let server = MockServer::start().await;

        // Serve the TOC
        Mock::given(method("GET"))
            .and(path("/gii-toc.xml"))
            .respond_with(ResponseTemplate::new(200).set_body_string(format!(
                r#"<?xml version="1.0"?>
<items>
  <item>
    <title>Bürgerliches Gesetzbuch</title>
    <link>{}/bgb/xml.zip</link>
    <description>BGB</description>
  </item>
</items>"#,
                server.uri()
            )))
            .mount(&server)
            .await;

        // Serve the law XML
        Mock::given(method("GET"))
            .and(path("/bgb/xml.zip"))
            .respond_with(
                ResponseTemplate::new(200).set_body_bytes(b"<law>BGB content</law>".to_vec()),
            )
            .mount(&server)
            .await;

        let toc_url = format!("{}/gii-toc.xml", server.uri());
        let crawler = BundesrechtCrawler::new(Uuid::new_v4())
            .unwrap()
            .with_toc_url(toc_url);

        let docs = crawler.crawl().await.unwrap();
        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0].external_id, "bgb");
        assert_eq!(docs[0].title, "Bürgerliches Gesetzbuch");
        assert_eq!(docs[0].format, DocFormat::Xml);
        assert!(!docs[0].raw_bytes.is_empty());
    }
}
