//! BOE crawler — fetches Spanish official gazette (Boletín Oficial del Estado).
//!
//! Flow:
//! 1. Fetch the daily "sumario" XML for today (and optionally a date range):
//!    `https://boe.es/diario_boe/xml.php?id=BOE-S-YYYYMMDD`
//! 2. Parse `<item>` entries from the sumario XML
//! 3. For each item with a PDF or HTML URL, fetch the full XML version:
//!    `https://boe.es/diario_boe/xml.php?id=BOE-A-YYYYMMDD-NNNNN`
//! 4. Emit `RawDocument { language: "es", jurisdiction: "ES", format: Xml }`

use async_trait::async_trait;
use chrono::{Duration, NaiveDate, Utc};
use quick_xml::events::Event;
use quick_xml::reader::Reader;
use reqwest_middleware::ClientWithMiddleware;
use serde_json::json;
use tracing::{debug, warn};
use uuid::Uuid;

use ingest::parser::DocFormat;
use legal_core::errors::LegalMcpError;

use crate::sources::base::{BaseCrawler, CrawlerState, RawDocument, SharedRateLimiter};

const BOE_BASE: &str = "https://www.boe.es";
/// Fetch sumario for the last N days.
const DAYS_BACK: i64 = 7;

pub struct BoeCrawler {
    state: CrawlerState,
    boe_base: String,
    days_back: i64,
}

impl BoeCrawler {
    pub fn new(source_id: Uuid) -> Result<Self, LegalMcpError> {
        Ok(Self {
            state: CrawlerState::new(source_id, BOE_BASE, 2)?,
            boe_base: BOE_BASE.into(),
            days_back: DAYS_BACK,
        })
    }

    #[cfg(test)]
    pub fn with_base(mut self, base: impl Into<String>) -> Self {
        let base = base.into();
        self.state = CrawlerState::new(self.state.source_id, base.clone(), 2).unwrap();
        self.boe_base = base;
        self.days_back = 1; // fetch only 1 day in tests
        self
    }
}

#[async_trait]
impl BaseCrawler for BoeCrawler {
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
        let today = Utc::now().date_naive();
        let mut docs = Vec::new();

        for day_offset in 0..self.days_back {
            let date = today - Duration::days(day_offset);
            if let Err(e) = self.crawl_day(date, &mut docs).await {
                warn!("BOE: failed to crawl {date}: {e}");
            }
        }

        Ok(docs)
    }
}

impl BoeCrawler {
    async fn crawl_day(
        &self,
        date: NaiveDate,
        docs: &mut Vec<RawDocument>,
    ) -> Result<(), LegalMcpError> {
        let date_str = date.format("%Y%m%d").to_string();
        let sumario_url = format!("{}/diario_boe/xml.php?id=BOE-S-{date_str}", self.boe_base);
        debug!("BOE: fetching sumario for {date_str}");

        let sumario_bytes = match self.fetch_bytes(&sumario_url).await {
            Ok(b) => b,
            Err(e) => {
                warn!("BOE: sumario not available for {date_str}: {e}");
                return Ok(()); // non-publishing days are fine
            }
        };

        let item_ids = parse_sumario_items(&sumario_bytes)?;
        debug!("BOE: {} items for {date_str}", item_ids.len());

        for item_id in item_ids {
            let xml_url = format!("{}/diario_boe/xml.php?id={item_id}", self.boe_base);
            match self.fetch_bytes(&xml_url).await {
                Ok(raw_bytes) => {
                    docs.push(RawDocument {
                        external_id: format!("boe_{item_id}"),
                        url: xml_url,
                        title: item_id.clone(),
                        raw_bytes,
                        format: DocFormat::Xml,
                        metadata_json: json!({
                            "boe_id": item_id,
                            "date": date.to_string(),
                            "jurisdiction": "ES",
                            "language": "es",
                            "source": "boe"
                        }),
                    });
                }
                Err(e) => {
                    warn!("BOE: failed to fetch item {item_id}: {e}");
                }
            }
        }
        Ok(())
    }
}

// ── XML parser ─────────────────────────────────────────────────────────────────

/// Extract `<item id="BOE-A-...">` identifiers from a BOE sumario XML.
fn parse_sumario_items(xml: &[u8]) -> Result<Vec<String>, LegalMcpError> {
    let mut reader = Reader::from_reader(xml);
    reader.config_mut().trim_text(true);

    let mut ids: Vec<String> = Vec::new();

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                let name = std::str::from_utf8(e.name().as_ref())
                    .unwrap_or("")
                    .to_lowercase();
                if name == "item" {
                    for attr in e.attributes().flatten() {
                        if std::str::from_utf8(attr.key.as_ref())
                            .map(|k| k == "id")
                            .unwrap_or(false)
                        {
                            if let Ok(val) = attr.unescape_value() {
                                let id = val.into_owned();
                                // Only index actual gazette entries (BOE-A-*), not
                                // section headers (BOE-S-*).
                                if id.starts_with("BOE-A-") {
                                    ids.push(id);
                                }
                            }
                        }
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(LegalMcpError::Parse(format!("BOE XML error: {e}"))),
            _ => {}
        }
    }
    Ok(ids)
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::{
        matchers::{method, path, query_param},
        Mock, MockServer, ResponseTemplate,
    };

    const FIXTURE_SUMARIO: &str = r#"<?xml version="1.0"?>
<sumario>
  <diario nbo="50">
    <seccion num="1" nombre="Disposiciones Generales">
      <departamento nombre="JEFATURA DEL ESTADO">
        <item id="BOE-A-20230301-1234">
          <titulo>Ley Orgánica 1/2023</titulo>
        </item>
        <item id="BOE-A-20230301-1235">
          <titulo>Real Decreto 456/2023</titulo>
        </item>
      </departamento>
    </seccion>
  </diario>
</sumario>"#;

    #[test]
    fn test_parse_sumario_items_extracts_boe_a() {
        let ids = parse_sumario_items(FIXTURE_SUMARIO.as_bytes()).unwrap();
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&"BOE-A-20230301-1234".to_string()));
        assert!(ids.contains(&"BOE-A-20230301-1235".to_string()));
    }

    #[test]
    fn test_parse_sumario_empty() {
        let ids = parse_sumario_items(b"<sumario></sumario>").unwrap();
        assert!(ids.is_empty());
    }

    #[tokio::test]
    async fn test_crawl_emits_raw_documents() {
        let server = MockServer::start().await;
        let today = Utc::now().date_naive().format("%Y%m%d").to_string();

        // Sumario for today.
        Mock::given(method("GET"))
            .and(path("/diario_boe/xml.php"))
            .and(query_param("id", format!("BOE-S-{today}")))
            .respond_with(ResponseTemplate::new(200).set_body_string(format!(
                r#"<?xml version="1.0"?>
<sumario>
  <diario>
    <item id="BOE-A-{today}-0001"/>
  </diario>
</sumario>"#
            )))
            .mount(&server)
            .await;

        // Item XML.
        Mock::given(method("GET"))
            .and(path("/diario_boe/xml.php"))
            .and(query_param("id", format!("BOE-A-{today}-0001")))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string("<texto><articulo>Art. 1</articulo></texto>"),
            )
            .mount(&server)
            .await;

        let crawler = BoeCrawler::new(Uuid::new_v4())
            .unwrap()
            .with_base(server.uri());
        let docs = crawler.crawl().await.unwrap();

        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0].format, DocFormat::Xml);
        assert_eq!(docs[0].metadata_json["jurisdiction"], "ES");
        assert_eq!(docs[0].metadata_json["language"], "es");
    }
}
