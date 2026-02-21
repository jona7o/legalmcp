//! Shared HTTP client with retry middleware and a consistent User-Agent.

use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use reqwest_retry::{policies::ExponentialBackoff, RetryTransientMiddleware};

/// User-Agent sent on every request.
pub const USER_AGENT: &str = "LegalMCP-Crawler/1.0";

/// Build a pre-configured `reqwest-middleware` client.
///
/// * 3 retries with exponential backoff (retries on 429 / 5xx by default)
/// * `User-Agent: LegalMCP-Crawler/1.0`
/// * 30-second timeout per request
pub fn build_http_client() -> Result<ClientWithMiddleware, reqwest::Error> {
    let retry_policy = ExponentialBackoff::builder().build_with_max_retries(3);

    let inner = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(std::time::Duration::from_secs(30))
        .build()?;

    Ok(ClientBuilder::new(inner)
        .with(RetryTransientMiddleware::new_with_policy(retry_policy))
        .build())
}
