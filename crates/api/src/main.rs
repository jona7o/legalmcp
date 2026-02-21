//! api-service binary entry point.
//!
//! Loads configuration from environment, establishes a database pool,
//! initialises the embedder, and starts the Axum HTTP server.

use std::sync::Arc;

use legal_core::{config::AppConfig, db::create_pool};
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load .env file if present (no-op when the file is absent)
    let _ = dotenvy::dotenv();

    // Initialise structured logging from RUST_LOG env var
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    // Load and validate configuration
    let config = AppConfig::from_env()?;
    info!(
        port = config.api_port,
        vertex_project = %config.vertex_ai_project,
        "starting api-service"
    );

    // Establish database connection pool
    let pool = create_pool(&config.database_url).await?;

    // Initialise embedder: prefer Vertex AI when credentials are configured
    let embedder: Arc<dyn embeddings::Embedder> = if config.vertex_ai_project.is_empty()
        || config.google_application_credentials.is_empty()
    {
        info!("VERTEX_AI_PROJECT or GOOGLE_APPLICATION_CREDENTIALS not set — using MockEmbedder");
        Arc::new(embeddings::MockEmbedder)
    } else {
        Arc::new(
            embeddings::VertexAiEmbedder::from_adc(
                config.vertex_ai_project.clone(),
                config.vertex_ai_location.clone(),
            )
            .await?,
        )
    };

    // Build Axum router with shared state
    let object_store = ingest::object_store::ObjectStoreClient::from_config(&config)?;
    let state = api::state::AppState::new(pool, embedder, object_store, config.clone());
    let router = api::router::app_router(state);

    let addr = format!("0.0.0.0:{}", config.api_port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    info!(addr = %addr, "api-service listening");

    axum::serve(listener, router).await?;

    Ok(())
}
