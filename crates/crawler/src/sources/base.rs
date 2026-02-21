//! `BaseCrawler` trait and `RawDocument` type.
//!
//! Every source crawler implements `BaseCrawler`.  The trait provides default
//! helpers for robots.txt checking and rate-limited HTTP, but requires each
//! implementor to supply `source_id`, `crawl_url` (base URL used for robots),
//! and `crawl` (the actual crawl logic).

use std::{num::NonZeroU32, sync::Arc};

use async_trait::async_trait;
use bytes::Bytes;
use governor::{
    clock::DefaultClock,
    middleware::NoOpMiddleware,
    state::{InMemoryState, NotKeyed},
    Quota, RateLimiter,
};
use reqwest_middleware::ClientWithMiddleware;
use serde_json::Value;
use tracing::{debug, warn};
use uuid::Uuid;

use ingest::parser::DocFormat;
use legal_core::errors::LegalMcpError;

use crate::http_client::build_http_client;

/// A single raw document fetched by a crawler.
#[derive(Debug, Clone)]
pub struct RawDocument {
    /// Source-specific identifier (e.g. `"bgb"` for the BGB).
    pub external_id: String,
    /// URL the document was fetched from.
    pub url: String,
    /// Document title as found on the source.
    pub title: String,
    /// Raw bytes of the fetched content.
    pub raw_bytes: Bytes,
    /// Detected or known format of the content.
    pub format: DocFormat,
    /// Extra metadata (JSON object) that the source crawler wants to attach.
    pub metadata_json: Value,
}

/// A shared rate-limiter type alias.
pub type SharedRateLimiter =
    Arc<RateLimiter<NotKeyed, InMemoryState, DefaultClock, NoOpMiddleware>>;

/// Creates a rate-limiter that allows `requests_per_second` requests per second.
///
/// Minimum of 1 request/s; uses a direct (non-keyed) token-bucket.
pub fn make_rate_limiter(requests_per_second: u32) -> SharedRateLimiter {
    let rps = NonZeroU32::new(requests_per_second.max(1)).expect("rps >= 1");
    Arc::new(RateLimiter::direct(Quota::per_second(rps)))
}

/// Trait that every source crawler must implement.
#[async_trait]
pub trait BaseCrawler: Send + Sync {
    /// The `sources.id` UUID that identifies this source in the database.
    fn source_id(&self) -> Uuid;

    /// The base URL of the source, used to construct the robots.txt URL.
    fn crawl_url(&self) -> &str;

    /// The per-source rate limiter.
    fn rate_limiter(&self) -> &SharedRateLimiter;

    /// Shared HTTP client with retry middleware already attached.
    fn http_client(&self) -> &ClientWithMiddleware;

    /// Execute the full crawl and return a list of `RawDocument` items.
    async fn crawl(&self) -> Result<Vec<RawDocument>, LegalMcpError>;

    // ── Default helpers ────────────────────────────────────────────────────────

    /// Fetch bytes from `url` respecting the source rate limiter.
    async fn fetch_bytes(&self, url: &str) -> Result<Bytes, LegalMcpError> {
        self.rate_limiter().until_ready().await;
        debug!("GET {url}");
        let response =
            self.http_client()
                .get(url)
                .send()
                .await
                .map_err(|e| LegalMcpError::ExternalApi {
                    service: "http".into(),
                    message: e.to_string(),
                })?;
        let response = response
            .error_for_status()
            .map_err(|e| LegalMcpError::ExternalApi {
                service: "http".into(),
                message: e.to_string(),
            })?;
        let bytes = response
            .bytes()
            .await
            .map_err(|e| LegalMcpError::ExternalApi {
                service: "http".into(),
                message: e.to_string(),
            })?;
        Ok(bytes)
    }

    /// Fetch `url` as UTF-8 text, respecting rate limits.
    async fn fetch_text(&self, url: &str) -> Result<String, LegalMcpError> {
        let bytes: Bytes = self.fetch_bytes(url).await?;
        String::from_utf8(bytes.to_vec()).map_err(|e| LegalMcpError::Parse(e.to_string()))
    }

    /// Returns `true` if the crawler is allowed to access `path` according to
    /// the source's `robots.txt`.
    ///
    /// A missing or inaccessible robots.txt is treated as "allow all".
    async fn is_allowed_by_robots(&self, path: &str) -> bool {
        let robots_url = format!("{}/robots.txt", self.crawl_url().trim_end_matches('/'));
        let text = match self.fetch_text(&robots_url).await {
            Ok(t) => t,
            Err(_) => {
                debug!("robots.txt not found at {robots_url} — assuming allowed");
                return true;
            }
        };
        let disallowed = parse_disallowed_paths(&text);
        let blocked = disallowed
            .iter()
            .any(|d| !d.is_empty() && (path == d || path.starts_with(d.as_str())));
        if blocked {
            warn!("robots.txt blocks path {path} on {}", self.crawl_url());
        }
        !blocked
    }
}

// ── robots.txt mini-parser ─────────────────────────────────────────────────────

/// Extracts `Disallow` paths from the `User-agent: *` section.
pub(crate) fn parse_disallowed_paths(robots_txt: &str) -> Vec<String> {
    let mut in_star_section = false;
    let mut disallowed = Vec::new();

    for line in robots_txt.lines() {
        let line = line.trim();
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        if let Some(agent) = line.strip_prefix("User-agent:") {
            in_star_section = agent.trim() == "*";
            continue;
        }
        if in_star_section {
            if let Some(path) = line.strip_prefix("Disallow:") {
                let path = path.trim().to_string();
                if !path.is_empty() {
                    disallowed.push(path);
                }
            }
        }
    }
    disallowed
}

// ── CrawlerState helper ────────────────────────────────────────────────────────

/// Convenience struct holding state shared across all crawlers.
pub struct CrawlerState {
    pub source_id: Uuid,
    pub crawl_url: String,
    pub rate_limiter: SharedRateLimiter,
    pub client: ClientWithMiddleware,
}

impl CrawlerState {
    /// Create a new `CrawlerState`.
    ///
    /// * `requests_per_second` — max throughput; minimum 1.
    pub fn new(
        source_id: Uuid,
        crawl_url: impl Into<String>,
        requests_per_second: u32,
    ) -> Result<Self, LegalMcpError> {
        let client = build_http_client().map_err(|e| LegalMcpError::ExternalApi {
            service: "http".into(),
            message: e.to_string(),
        })?;
        Ok(Self {
            source_id,
            crawl_url: crawl_url.into(),
            rate_limiter: make_rate_limiter(requests_per_second),
            client,
        })
    }
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

    // ── helper: minimal in-proc crawler for testing ───────────────────────────

    struct TestCrawler {
        state: CrawlerState,
    }

    impl TestCrawler {
        fn new(base_url: &str) -> Self {
            Self {
                state: CrawlerState::new(Uuid::new_v4(), base_url, 10).unwrap(),
            }
        }
    }

    #[async_trait]
    impl BaseCrawler for TestCrawler {
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
            Ok(vec![])
        }
    }

    // ── robots.txt ────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_robots_allows_unlisted_path() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/robots.txt"))
            .respond_with(
                ResponseTemplate::new(200).set_body_string("User-agent: *\nDisallow: /blocked\n"),
            )
            .mount(&server)
            .await;

        let crawler = TestCrawler::new(&server.uri());
        assert!(crawler.is_allowed_by_robots("/allowed").await);
    }

    #[tokio::test]
    async fn test_robots_blocks_disallowed_path() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/robots.txt"))
            .respond_with(
                ResponseTemplate::new(200).set_body_string("User-agent: *\nDisallow: /blocked\n"),
            )
            .mount(&server)
            .await;

        let crawler = TestCrawler::new(&server.uri());
        assert!(!crawler.is_allowed_by_robots("/blocked").await);
        assert!(!crawler.is_allowed_by_robots("/blocked/subpath").await);
    }

    #[tokio::test]
    async fn test_robots_missing_allows_all() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/robots.txt"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;

        let crawler = TestCrawler::new(&server.uri());
        assert!(crawler.is_allowed_by_robots("/anything").await);
    }

    // ── rate-limiter smoke test ───────────────────────────────────────────────

    #[test]
    fn test_make_rate_limiter_nonzero() {
        let rl = make_rate_limiter(5);
        assert!(rl.check().is_ok());
    }

    // ── parse_disallowed_paths ────────────────────────────────────────────────

    #[test]
    fn test_parse_disallowed_only_star_section() {
        let robots =
            "User-agent: Googlebot\nDisallow: /private\n\nUser-agent: *\nDisallow: /blocked\n";
        let paths = parse_disallowed_paths(robots);
        assert_eq!(paths, vec!["/blocked"]);
    }

    #[test]
    fn test_parse_disallowed_empty_disallow_ignored() {
        let robots = "User-agent: *\nDisallow:\nDisallow: /secret\n";
        let paths = parse_disallowed_paths(robots);
        assert_eq!(paths, vec!["/secret"]);
    }
}
