//! Admin handlers — stats, API key management.
//!
//! Routes (all require Bearer auth):
//!   GET  /api/v1/admin/stats
//!   GET  /api/v1/admin/api-keys
//!   POST /api/v1/admin/api-keys
//!   DELETE /api/v1/admin/api-keys/:id

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use uuid::Uuid;

use legal_core::errors::ProblemDetail;

use crate::{
    services::admin::{AdminService, AdminStats, ApiKeyRow, CreateApiKeyRequest, CreatedApiKey},
    state::AppState,
};

/// Helper that converts a `LegalMcpError` into an Axum-compatible error tuple.
fn svc_err(e: legal_core::errors::LegalMcpError) -> (StatusCode, Json<ProblemDetail>) {
    let pd = e.to_problem_detail();
    let status = StatusCode::from_u16(pd.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    (status, Json(pd))
}

/// `GET /api/v1/admin/stats`
pub async fn get_stats(
    State(state): State<Arc<AppState>>,
) -> Result<Json<AdminStats>, (StatusCode, Json<ProblemDetail>)> {
    AdminService::new(state.pool.clone())
        .stats()
        .await
        .map(Json)
        .map_err(svc_err)
}

/// `GET /api/v1/admin/api-keys`
pub async fn list_api_keys(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<ApiKeyRow>>, (StatusCode, Json<ProblemDetail>)> {
    AdminService::new(state.pool.clone())
        .list_api_keys()
        .await
        .map(Json)
        .map_err(svc_err)
}

/// `POST /api/v1/admin/api-keys`
pub async fn create_api_key(
    State(state): State<Arc<AppState>>,
    Json(body): Json<CreateApiKeyRequest>,
) -> Result<(StatusCode, Json<CreatedApiKey>), (StatusCode, Json<ProblemDetail>)> {
    AdminService::new(state.pool.clone())
        .create_api_key(body)
        .await
        .map(|key| (StatusCode::CREATED, Json(key)))
        .map_err(svc_err)
}

/// `DELETE /api/v1/admin/api-keys/:id`
pub async fn revoke_api_key(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ProblemDetail>)> {
    AdminService::new(state.pool.clone())
        .revoke_api_key(id)
        .await
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(svc_err)
}
