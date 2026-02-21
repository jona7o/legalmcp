//! Shared application state threaded through Axum via `Extension`.

use std::sync::Arc;

use embeddings::traits::Embedder;
use ingest::object_store::ObjectStoreClient;
use legal_core::config::AppConfig;
use sqlx::PgPool;

/// Shared state for every request handler.
pub struct AppState {
    pub pool: PgPool,
    pub embedder: Arc<dyn Embedder>,
    pub object_store: ObjectStoreClient,
    pub config: AppConfig,
}

impl AppState {
    pub fn new(
        pool: PgPool,
        embedder: Arc<dyn Embedder>,
        object_store: ObjectStoreClient,
        config: AppConfig,
    ) -> Arc<Self> {
        Arc::new(Self {
            pool,
            embedder,
            object_store,
            config,
        })
    }
}
