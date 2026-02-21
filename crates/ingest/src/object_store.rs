//! Object storage client wrapping [`object_store`] for GCS / Azure Blob.
//!
//! Implements ADR-4: original document files (HTML, XML, PDF) are stored with
//! key pattern `{source_id}/{external_id}/{content_hash}.{ext}`.  The backend
//! (GCS or Azure Blob) is selected at runtime from `AppConfig`.

use std::sync::Arc;

use bytes::Bytes;
use legal_core::{config::AppConfig, errors::LegalMcpError};
use object_store::{path::Path, Error as OsError, ObjectStore, PutPayload};

/// Thin wrapper around `Arc<dyn ObjectStore>` that encodes the key-naming
/// convention and maps `object_store::Error` into `LegalMcpError`.
#[derive(Clone)]
pub struct ObjectStoreClient {
    inner: Arc<dyn ObjectStore>,
}

impl ObjectStoreClient {
    /// Construct a client from an already-built store (useful in tests).
    pub fn new(store: Arc<dyn ObjectStore>) -> Self {
        Self { inner: store }
    }

    /// Build a client from [`AppConfig`].
    ///
    /// Selects the backend based on `config.object_store_type`:
    /// - `"gcs"` → Google Cloud Storage (uses `config.gcs_bucket` +
    ///   `GOOGLE_APPLICATION_CREDENTIALS` env var)
    /// - `"azure"` → Azure Blob Storage (uses `config.azure_storage_account` +
    ///   `config.azure_container`)
    /// - anything else (or empty) → in-process `InMemory` store (local dev / CI)
    pub fn from_config(config: &AppConfig) -> Result<Self, LegalMcpError> {
        let store: Arc<dyn ObjectStore> = match config.object_store_type.as_str() {
            "gcs" => {
                use object_store::gcp::GoogleCloudStorageBuilder;
                let store = GoogleCloudStorageBuilder::new()
                    .with_bucket_name(&config.gcs_bucket)
                    .build()
                    .map_err(LegalMcpError::ObjectStore)?;
                Arc::new(store)
            }
            "azure" => {
                use object_store::azure::MicrosoftAzureBuilder;
                let store = MicrosoftAzureBuilder::new()
                    .with_account(&config.azure_storage_account)
                    .with_container_name(&config.azure_container)
                    .build()
                    .map_err(LegalMcpError::ObjectStore)?;
                Arc::new(store)
            }
            _ => {
                // Local dev / CI: in-process memory store (no I/O).
                Arc::new(object_store::memory::InMemory::new())
            }
        };
        Ok(Self { inner: store })
    }

    /// Build the canonical object key.
    ///
    /// Pattern: `{source_id}/{external_id}/{content_hash}.{ext}`
    pub fn build_key(source_id: &str, external_id: &str, content_hash: &str, ext: &str) -> Path {
        Path::from(format!("{source_id}/{external_id}/{content_hash}.{ext}"))
    }

    /// Store raw bytes at `key`.
    pub async fn put(&self, key: &Path, data: Bytes) -> Result<(), LegalMcpError> {
        self.inner
            .put(key, PutPayload::from_bytes(data))
            .await
            .map(|_| ())
            .map_err(LegalMcpError::ObjectStore)
    }

    /// Retrieve raw bytes from `key`.
    pub async fn get(&self, key: &Path) -> Result<Bytes, LegalMcpError> {
        let result = self
            .inner
            .get(key)
            .await
            .map_err(LegalMcpError::ObjectStore)?;
        result.bytes().await.map_err(LegalMcpError::ObjectStore)
    }

    /// Delete the object at `key` (idempotent — no error if missing).
    pub async fn delete(&self, key: &Path) -> Result<(), LegalMcpError> {
        match self.inner.delete(key).await {
            Ok(_) => Ok(()),
            Err(OsError::NotFound { .. }) => Ok(()),
            Err(e) => Err(LegalMcpError::ObjectStore(e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn in_memory_client() -> ObjectStoreClient {
        ObjectStoreClient::new(Arc::new(object_store::memory::InMemory::new()))
    }

    #[test]
    fn build_key_format() {
        let key = ObjectStoreClient::build_key("src-123", "doc-456", "abcdef1234567890", "html");
        assert_eq!(key.as_ref(), "src-123/doc-456/abcdef1234567890.html");
    }

    #[tokio::test]
    async fn put_and_get_roundtrip() {
        let client = in_memory_client();
        let key = Path::from("test/a/b.html");
        let data = Bytes::from_static(b"<html>test</html>");

        client.put(&key, data.clone()).await.unwrap();
        let retrieved = client.get(&key).await.unwrap();
        assert_eq!(retrieved, data);
    }

    #[tokio::test]
    async fn get_missing_key_returns_error() {
        let client = in_memory_client();
        let key = Path::from("missing/key.html");
        let result = client.get(&key).await;
        assert!(result.is_err(), "Expected error for missing key");
    }

    #[tokio::test]
    async fn delete_existing_key() {
        let client = in_memory_client();
        let key = Path::from("to/delete.xml");
        client
            .put(&key, Bytes::from_static(b"content"))
            .await
            .unwrap();
        client.delete(&key).await.unwrap();
        assert!(client.get(&key).await.is_err());
    }

    #[tokio::test]
    async fn delete_missing_key_is_ok() {
        let client = in_memory_client();
        let key = Path::from("never/existed.pdf");
        // Should not error (idempotent)
        client.delete(&key).await.unwrap();
    }

    #[tokio::test]
    async fn from_config_empty_type_uses_in_memory() {
        // object_store_type defaults to "" — should select InMemory
        let config = AppConfig {
            database_url: "postgresql://test".into(),
            vertex_ai_project: "proj".into(),
            vertex_ai_location: "europe-west3".into(),
            google_application_credentials: "/tmp/creds.json".into(),
            mistral_api_key: String::new(),
            object_store_type: String::new(), // empty → InMemory
            gcs_bucket: String::new(),
            azure_storage_account: String::new(),
            azure_storage_key: String::new(),
            azure_container: String::new(),
            api_port: 8000,
            max_db_connections: 20,
            rust_log: "info".into(),
        };
        let client = ObjectStoreClient::from_config(&config).unwrap();
        let key = Path::from("smoke/test.txt");
        client.put(&key, Bytes::from_static(b"ok")).await.unwrap();
        let got = client.get(&key).await.unwrap();
        assert_eq!(got.as_ref(), b"ok");
    }
}
