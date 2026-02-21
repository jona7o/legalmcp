//! Legifrance crawler — fetches French law via the PISTE API (OAuth2 client credentials).
//!
//! Flow:
//! 1. Obtain a Bearer token from `https://sandbox-oauth.piste.gouv.fr/api/oauth/token`
//!    using `client_credentials` grant with credentials from `source.credentials_enc`.
//! 2. Call `/consult/code/tableMatieres` to list codes (codes de droit).
//! 3. For each code, call `/consult/code` to fetch the full HTML text.
//! 4. Emit `RawDocument { language: "fr", jurisdiction: "FR", format: Html }`.
//!
//! Credentials are passed in as `client_id` / `client_secret` strings.  In
//! production they come from the `sources.credentials_enc` column (decrypted by
//! the caller before constructing the crawler).

use async_trait::async_trait;
use reqwest_middleware::ClientWithMiddleware;
use serde_json::{json, Value};
use tracing::{debug, warn};
use uuid::Uuid;

use ingest::parser::DocFormat;
use legal_core::errors::LegalMcpError;

use crate::sources::base::{BaseCrawler, CrawlerState, RawDocument, SharedRateLimiter};

const OAUTH_URL: &str = "https://sandbox-oauth.piste.gouv.fr/api/oauth/token";
const API_BASE: &str = "https://api.piste.gouv.fr/dila/legifrance/lf-engine-app";
#[allow(dead_code)]
const PAGE_SIZE: usize = 50;

pub struct LegifranceCrawler {
    state: CrawlerState,
    oauth_url: String,
    api_base: String,
    client_id: String,
    client_secret: String,
}

impl LegifranceCrawler {
    /// Create a new crawler.
    ///
    /// `client_id` and `client_secret` are the PISTE OAuth2 credentials.
    pub fn new(
        source_id: Uuid,
        client_id: impl Into<String>,
        client_secret: impl Into<String>,
    ) -> Result<Self, LegalMcpError> {
        Ok(Self {
            state: CrawlerState::new(source_id, API_BASE, 2)?,
            oauth_url: OAUTH_URL.into(),
            api_base: API_BASE.into(),
            client_id: client_id.into(),
            client_secret: client_secret.into(),
        })
    }

    /// Override endpoints (for tests).
    #[cfg(test)]
    pub fn with_endpoints(
        mut self,
        oauth_url: impl Into<String>,
        api_base: impl Into<String>,
    ) -> Self {
        self.oauth_url = oauth_url.into();
        self.api_base = api_base.into();
        self
    }
}

#[async_trait]
impl BaseCrawler for LegifranceCrawler {
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
        let token = self.fetch_token().await?;
        debug!("Legifrance: obtained OAuth2 token");

        let codes = self.list_codes(&token).await?;
        debug!("Legifrance: found {} codes", codes.len());

        let mut docs = Vec::with_capacity(codes.len());
        for code in codes {
            let cid = match code.get("id").and_then(|v| v.as_str()) {
                Some(id) => id.to_string(),
                None => continue,
            };
            let title = code
                .get("titre")
                .and_then(|v| v.as_str())
                .unwrap_or(&cid)
                .to_string();

            match self.fetch_code_html(&token, &cid).await {
                Ok(raw_bytes) => {
                    docs.push(RawDocument {
                        external_id: format!("legifrance_{cid}"),
                        url: format!("{}/consult/code?textId={cid}", self.api_base),
                        title,
                        raw_bytes,
                        format: DocFormat::Html,
                        metadata_json: json!({
                            "code_id": cid,
                            "jurisdiction": "FR",
                            "language": "fr",
                            "source": "legifrance"
                        }),
                    });
                }
                Err(e) => {
                    warn!("Failed to fetch Legifrance code {cid}: {e}");
                }
            }
        }
        Ok(docs)
    }
}

impl LegifranceCrawler {
    /// Fetch an OAuth2 Bearer token via client credentials grant.
    async fn fetch_token(&self) -> Result<String, LegalMcpError> {
        let resp = self
            .http_client()
            .post(&self.oauth_url)
            .form(&[
                ("grant_type", "client_credentials"),
                ("client_id", &self.client_id),
                ("client_secret", &self.client_secret),
                ("scope", "openid"),
            ])
            .send()
            .await
            .map_err(|e| LegalMcpError::ExternalApi {
                service: "legifrance-oauth".into(),
                message: e.to_string(),
            })?
            .error_for_status()
            .map_err(|e| LegalMcpError::ExternalApi {
                service: "legifrance-oauth".into(),
                message: e.to_string(),
            })?
            .json::<Value>()
            .await
            .map_err(|e| LegalMcpError::Parse(e.to_string()))?;

        resp.get("access_token")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| LegalMcpError::ExternalApi {
                service: "legifrance-oauth".into(),
                message: "missing access_token in response".into(),
            })
    }

    /// List all available codes from the API.
    async fn list_codes(&self, token: &str) -> Result<Vec<Value>, LegalMcpError> {
        let url = format!("{}/list/code", self.api_base);
        let resp = self
            .http_client()
            .get(&url)
            .bearer_auth(token)
            .send()
            .await
            .map_err(|e| LegalMcpError::ExternalApi {
                service: "legifrance-api".into(),
                message: e.to_string(),
            })?
            .error_for_status()
            .map_err(|e| LegalMcpError::ExternalApi {
                service: "legifrance-api".into(),
                message: e.to_string(),
            })?
            .json::<Value>()
            .await
            .map_err(|e| LegalMcpError::Parse(e.to_string()))?;

        let codes = resp
            .get("codes")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        Ok(codes)
    }

    /// Fetch the full HTML text of a code.
    async fn fetch_code_html(
        &self,
        token: &str,
        code_id: &str,
    ) -> Result<bytes::Bytes, LegalMcpError> {
        let url = format!("{}/consult/code?textId={code_id}", self.api_base);
        let resp = self
            .http_client()
            .get(&url)
            .bearer_auth(token)
            .send()
            .await
            .map_err(|e| LegalMcpError::ExternalApi {
                service: "legifrance-api".into(),
                message: e.to_string(),
            })?
            .error_for_status()
            .map_err(|e| LegalMcpError::ExternalApi {
                service: "legifrance-api".into(),
                message: e.to_string(),
            })?
            .bytes()
            .await
            .map_err(|e| LegalMcpError::ExternalApi {
                service: "legifrance-api".into(),
                message: e.to_string(),
            })?;
        Ok(resp)
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

    #[tokio::test]
    async fn test_oauth2_flow_and_crawl() {
        let server = MockServer::start().await;

        // OAuth2 token endpoint.
        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(r#"{"access_token":"test-token","token_type":"Bearer"}"#),
            )
            .mount(&server)
            .await;

        // List codes endpoint.
        Mock::given(method("GET"))
            .and(path("/list/code"))
            .respond_with(ResponseTemplate::new(200).set_body_string(
                r#"{"codes":[{"id":"LEGITEXT000006070239","titre":"Code civil"}]}"#,
            ))
            .mount(&server)
            .await;

        // Code HTML endpoint.
        Mock::given(method("GET"))
            .and(path("/consult/code"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string("<html><body>Code civil content</body></html>"),
            )
            .mount(&server)
            .await;

        let oauth_url = format!("{}/token", server.uri());
        let api_base = format!("{}", server.uri());

        let crawler = LegifranceCrawler::new(Uuid::new_v4(), "client_id", "client_secret")
            .unwrap()
            .with_endpoints(oauth_url, api_base);

        let docs = crawler.crawl().await.unwrap();
        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0].external_id, "legifrance_LEGITEXT000006070239");
        assert_eq!(docs[0].title, "Code civil");
        assert_eq!(docs[0].format, DocFormat::Html);
        assert_eq!(docs[0].metadata_json["language"], "fr");
        assert_eq!(docs[0].metadata_json["jurisdiction"], "FR");
    }

    #[tokio::test]
    async fn test_missing_token_returns_error() {
        let server = MockServer::start().await;

        // Return JSON without access_token.
        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(
                ResponseTemplate::new(200).set_body_string(r#"{"error":"invalid_client"}"#),
            )
            .mount(&server)
            .await;

        let oauth_url = format!("{}/token", server.uri());
        let crawler = LegifranceCrawler::new(Uuid::new_v4(), "bad_id", "bad_secret")
            .unwrap()
            .with_endpoints(oauth_url, "https://unused.example.com");

        let result = crawler.crawl().await;
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("access_token") || err.contains("legifrance-oauth"));
    }
}
