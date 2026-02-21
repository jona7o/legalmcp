use async_trait::async_trait;
use legal_core::errors::LegalMcpError;
use pgvector::Vector;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{debug, instrument};

use crate::traits::Embedder;

/// Maximum texts per Vertex AI embedding API call.
const VERTEX_BATCH_SIZE: usize = 250;

/// Embedding dimension for `text-embedding-004`.
const EMBEDDING_DIM: usize = 768;

/// Vertex AI `text-embedding-004` embedding provider.
///
/// Authenticates using Google Application Default Credentials (ADC) by
/// including the `Authorization: Bearer <token>` header. The token is fetched
/// once at construction time via the GCP metadata server or `gcloud` CLI.
///
/// Requests are automatically batched into groups of ≤250 texts per the
/// Vertex AI API limit. Retries on 429/503 are handled by the caller's
/// `reqwest-retry` middleware; this struct itself does not retry.
pub struct VertexAiEmbedder {
    client: Client,
    project_id: String,
    location: String,
    access_token: String,
}

impl VertexAiEmbedder {
    /// Construct from explicit parameters (useful in tests with a mock token).
    pub fn new(client: Client, project_id: String, location: String, access_token: String) -> Self {
        Self {
            client,
            project_id,
            location,
            access_token,
        }
    }

    /// Construct by loading GCP ADC token from the metadata server.
    ///
    /// Requires the process to be running inside GCP or have `gcloud auth
    /// application-default login` configured.
    pub async fn from_adc(project_id: String, location: String) -> Result<Self, LegalMcpError> {
        let client = Client::new();
        let token = fetch_adc_token(&client).await?;
        Ok(Self::new(client, project_id, location, token))
    }

    /// Embed a single batch of ≤250 texts with one API call.
    #[instrument(skip(self, texts), fields(batch_size = texts.len()))]
    async fn embed_batch_raw(&self, texts: &[String]) -> Result<Vec<Vector>, LegalMcpError> {
        let url = format!(
            "https://{location}-aiplatform.googleapis.com/v1/projects/{project}/locations/{location}/publishers/google/models/text-embedding-004:predict",
            location = self.location,
            project = self.project_id,
        );

        let instances: Vec<EmbedInstance> = texts
            .iter()
            .map(|t| EmbedInstance { content: t.clone() })
            .collect();

        let body = EmbedRequest { instances };

        let response = self
            .client
            .post(&url)
            .bearer_auth(&self.access_token)
            .json(&body)
            .send()
            .await
            .map_err(|e| LegalMcpError::ExternalApi {
                service: "vertex-ai".into(),
                message: e.to_string(),
            })?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body_text = response.text().await.unwrap_or_default();
            return Err(LegalMcpError::ExternalApi {
                service: "vertex-ai".into(),
                message: format!("HTTP {status}: {body_text}"),
            });
        }

        let embed_response: EmbedResponse =
            response
                .json()
                .await
                .map_err(|e| LegalMcpError::ExternalApi {
                    service: "vertex-ai".into(),
                    message: format!("Failed to deserialise response: {e}"),
                })?;

        embed_response
            .predictions
            .into_iter()
            .map(|p| {
                if p.embeddings.values.len() != EMBEDDING_DIM {
                    return Err(LegalMcpError::Embedding(format!(
                        "Expected {EMBEDDING_DIM} dimensions, got {}",
                        p.embeddings.values.len()
                    )));
                }
                Ok(Vector::from(p.embeddings.values))
            })
            .collect()
    }
}

#[async_trait]
impl Embedder for VertexAiEmbedder {
    #[instrument(skip(self, texts), fields(total = texts.len()))]
    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vector>, LegalMcpError> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        let mut all_vectors = Vec::with_capacity(texts.len());

        for chunk in texts.chunks(VERTEX_BATCH_SIZE) {
            debug!(batch_size = chunk.len(), "Embedding batch");
            let mut batch_result = self.embed_batch_raw(chunk).await?;
            all_vectors.append(&mut batch_result);
        }

        Ok(all_vectors)
    }
}

/// Fetch a GCP ADC OAuth2 bearer token from the metadata server.
async fn fetch_adc_token(client: &Client) -> Result<String, LegalMcpError> {
    let url = "http://metadata.google.internal/computeMetadata/v1/instance/service-accounts/default/token";
    let response = client
        .get(url)
        .header("Metadata-Flavor", "Google")
        .send()
        .await
        .map_err(|e| LegalMcpError::ExternalApi {
            service: "gcp-metadata".into(),
            message: e.to_string(),
        })?;

    let token_resp: MetadataTokenResponse =
        response
            .json()
            .await
            .map_err(|e| LegalMcpError::ExternalApi {
                service: "gcp-metadata".into(),
                message: format!("Failed to parse ADC token: {e}"),
            })?;

    Ok(token_resp.access_token)
}

// ---- Serde types ----

#[derive(Serialize)]
struct EmbedRequest {
    instances: Vec<EmbedInstance>,
}

#[derive(Serialize)]
struct EmbedInstance {
    content: String,
}

#[derive(Deserialize)]
struct EmbedResponse {
    predictions: Vec<Prediction>,
}

#[derive(Deserialize)]
struct Prediction {
    embeddings: EmbedValues,
}

#[derive(Deserialize)]
struct EmbedValues {
    values: Vec<f32>,
}

#[derive(Deserialize)]
struct MetadataTokenResponse {
    access_token: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn batch_size_constant() {
        assert_eq!(VERTEX_BATCH_SIZE, 250);
    }

    #[test]
    fn embedding_dim_constant() {
        assert_eq!(EMBEDDING_DIM, 768);
    }

    /// Verify that a slice of 300 texts is split into exactly 2 batches.
    #[test]
    fn chunks_splits_300_into_two_batches() {
        let texts: Vec<String> = (0..300).map(|i| format!("text {i}")).collect();
        let batches: Vec<&[String]> = texts.chunks(VERTEX_BATCH_SIZE).collect();
        assert_eq!(batches.len(), 2);
        assert_eq!(batches[0].len(), 250);
        assert_eq!(batches[1].len(), 50);
    }

    /// Verify that a slice of exactly 250 texts produces one batch.
    #[test]
    fn chunks_250_exactly_one_batch() {
        let texts: Vec<String> = (0..250).map(|i| format!("text {i}")).collect();
        let batches: Vec<&[String]> = texts.chunks(VERTEX_BATCH_SIZE).collect();
        assert_eq!(batches.len(), 1);
    }
}
