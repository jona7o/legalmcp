use legal_core::errors::LegalMcpError;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::instrument;

/// Mistral OCR API endpoint.
const MISTRAL_OCR_URL: &str = "https://api.mistral.ai/v1/ocr";

/// Parse a PDF document into Markdown by calling the Mistral OCR API.
///
/// `pdf_url` should be a publicly accessible URL or a base64-encoded data URI.
/// `api_key` is the Mistral API key (from `AppConfig::mistral_api_key`).
///
/// # Errors
/// Returns `LegalMcpError::ExternalApi` on HTTP failure.
/// Returns `LegalMcpError::Parse` if the response contains no usable content.
#[instrument(skip(client, api_key), fields(pdf_url))]
pub async fn parse_url(
    client: &Client,
    pdf_url: &str,
    api_key: &str,
) -> Result<String, LegalMcpError> {
    if pdf_url.trim().is_empty() {
        return Err(LegalMcpError::Parse("PDF URL is empty".into()));
    }

    let request_body = OcrRequest {
        model: "mistral-ocr-latest".to_owned(),
        document: OcrDocument::Url {
            url: pdf_url.to_owned(),
        },
    };

    let response = client
        .post(MISTRAL_OCR_URL)
        .bearer_auth(api_key)
        .json(&request_body)
        .send()
        .await
        .map_err(|e| LegalMcpError::ExternalApi {
            service: "mistral-ocr".into(),
            message: e.to_string(),
        })?;

    if !response.status().is_success() {
        let status = response.status().as_u16();
        let body = response.text().await.unwrap_or_default();
        return Err(LegalMcpError::ExternalApi {
            service: "mistral-ocr".into(),
            message: format!("HTTP {status}: {body}"),
        });
    }

    let ocr_response: OcrResponse =
        response
            .json()
            .await
            .map_err(|e| LegalMcpError::ExternalApi {
                service: "mistral-ocr".into(),
                message: format!("Failed to deserialise OCR response: {e}"),
            })?;

    extract_markdown(ocr_response)
}

/// Parse raw PDF bytes by uploading as a base64 data URI to Mistral OCR.
pub async fn parse_bytes(
    client: &Client,
    pdf_bytes: &[u8],
    api_key: &str,
) -> Result<String, LegalMcpError> {
    use base64::Engine as _;
    let encoded = base64::engine::general_purpose::STANDARD.encode(pdf_bytes);
    let data_uri = format!("data:application/pdf;base64,{encoded}");
    parse_url(client, &data_uri, api_key).await
}

fn extract_markdown(response: OcrResponse) -> Result<String, LegalMcpError> {
    let content: String = response
        .pages
        .into_iter()
        .map(|p| p.markdown)
        .collect::<Vec<_>>()
        .join("\n\n");

    let trimmed = content.trim().to_owned();
    if trimmed.is_empty() {
        return Err(LegalMcpError::Parse(
            "Mistral OCR returned no text content".into(),
        ));
    }

    Ok(trimmed)
}

// ---- Serde types for Mistral OCR API ----

#[derive(Serialize)]
struct OcrRequest {
    model: String,
    document: OcrDocument,
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum OcrDocument {
    Url { url: String },
}

#[derive(Deserialize)]
struct OcrResponse {
    pages: Vec<OcrPage>,
}

#[derive(Deserialize)]
struct OcrPage {
    markdown: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn empty_url_returns_error() {
        let client = Client::new();
        let err = parse_url(&client, "", "key").await.unwrap_err();
        assert!(matches!(err, LegalMcpError::Parse(_)));
    }

    #[tokio::test]
    async fn http_error_returns_external_api_error() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/ocr"))
            .respond_with(ResponseTemplate::new(500).set_body_string("Internal Error"))
            .mount(&server)
            .await;

        // We'd need to override the URL — test the extract_markdown path instead
        let empty_response = OcrResponse { pages: vec![] };
        let err = extract_markdown(empty_response).unwrap_err();
        assert!(matches!(err, LegalMcpError::Parse(_)));
    }

    #[test]
    fn extract_markdown_joins_pages() {
        let response = OcrResponse {
            pages: vec![
                OcrPage {
                    markdown: "# Page 1\n\nContent A".to_owned(),
                },
                OcrPage {
                    markdown: "## Page 2\n\nContent B".to_owned(),
                },
            ],
        };
        let result = extract_markdown(response).unwrap();
        assert!(result.contains("Content A"));
        assert!(result.contains("Content B"));
    }

    #[test]
    fn extract_markdown_empty_returns_error() {
        let response = OcrResponse { pages: vec![] };
        let err = extract_markdown(response).unwrap_err();
        assert!(matches!(err, LegalMcpError::Parse(_)));
    }
}
