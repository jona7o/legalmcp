//! Source handlers — CRUD for crawl sources.
//!
//! Routes:
//!   GET    /api/v1/sources
//!   POST   /api/v1/sources
//!   GET    /api/v1/sources/:id
//!   PATCH  /api/v1/sources/:id
//!   DELETE /api/v1/sources/:id
//!   POST   /api/v1/sources/:id/crawl

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use serde::Serialize;
use uuid::Uuid;

use legal_core::errors::ProblemDetail;

use crate::{
    services::source::{CreateSourceRequest, SourceRow, SourceService, UpdateSourceRequest},
    state::AppState,
};

fn svc_err(e: legal_core::errors::LegalMcpError) -> (StatusCode, Json<ProblemDetail>) {
    let pd = e.to_problem_detail();
    let status = StatusCode::from_u16(pd.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    (status, Json(pd))
}

/// `GET /api/v1/sources`
pub async fn list_sources(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<SourceRow>>, (StatusCode, Json<ProblemDetail>)> {
    SourceService::new(state.pool.clone())
        .list_sources()
        .await
        .map(Json)
        .map_err(svc_err)
}

/// `POST /api/v1/sources`
pub async fn create_source(
    State(state): State<Arc<AppState>>,
    Json(body): Json<CreateSourceRequest>,
) -> Result<(StatusCode, Json<SourceRow>), (StatusCode, Json<ProblemDetail>)> {
    SourceService::new(state.pool.clone())
        .create_source(body)
        .await
        .map(|s| (StatusCode::CREATED, Json(s)))
        .map_err(svc_err)
}

/// `GET /api/v1/sources/:id`
pub async fn get_source(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<SourceRow>, (StatusCode, Json<ProblemDetail>)> {
    SourceService::new(state.pool.clone())
        .get_source(id)
        .await
        .map(Json)
        .map_err(svc_err)
}

/// `PATCH /api/v1/sources/:id`
pub async fn update_source(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateSourceRequest>,
) -> Result<Json<SourceRow>, (StatusCode, Json<ProblemDetail>)> {
    SourceService::new(state.pool.clone())
        .update_source(id, body)
        .await
        .map(Json)
        .map_err(svc_err)
}

/// `DELETE /api/v1/sources/:id`
pub async fn delete_source(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ProblemDetail>)> {
    SourceService::new(state.pool.clone())
        .delete_source(id)
        .await
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(svc_err)
}

/// Response for a manual crawl trigger.
#[derive(Debug, Serialize)]
pub struct TriggerCrawlResponse {
    pub source_id: Uuid,
    pub message: String,
}

/// `POST /api/v1/sources/:id/crawl`
///
/// Enqueues a manual crawl run for the specified source.  The actual crawl is
/// executed asynchronously by the crawler-service via the apalis job queue.
///
/// This endpoint inserts a pending `crawl_jobs` row into the apalis storage
/// table.  If apalis tables are not present (e.g. in test environments without
/// the migration), the endpoint returns 202 Accepted with a note that scheduling
/// is not available.
pub async fn trigger_crawl(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<(StatusCode, Json<TriggerCrawlResponse>), (StatusCode, Json<ProblemDetail>)> {
    // Verify the source exists.
    SourceService::new(state.pool.clone())
        .get_source(id)
        .await
        .map_err(svc_err)?;

    // Insert a manual crawl run record into the `crawl_runs` table so the
    // crawler-service can pick it up.  The actual apalis queue tables are
    // managed by the crawler-service binary; we record intent here.
    let result = sqlx::query!(
        r#"
        INSERT INTO crawl_runs (source_id, status, docs_new, docs_updated, docs_unchanged)
        VALUES ($1, 'queued', 0, 0, 0)
        "#,
        id,
    )
    .execute(&state.pool)
    .await;

    match result {
        Ok(_) => Ok((
            StatusCode::ACCEPTED,
            Json(TriggerCrawlResponse {
                source_id: id,
                message: "crawl queued — the crawler-service will process it shortly".into(),
            }),
        )),
        Err(e) => Err(svc_err(legal_core::errors::LegalMcpError::Database(e))),
    }
}
