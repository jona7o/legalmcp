//! Search handler — `GET /api/v1/search`.

use std::sync::Arc;

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Json,
};
use serde::Deserialize;
use serde_json::Value;

use legal_core::errors::ProblemDetail;

use crate::{
    services::search::{SearchFilters, SearchResponse, SearchService},
    state::AppState,
};

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub q: Option<String>,
    pub jurisdiction: Option<String>,
    pub doc_type: Option<String>,
    pub language: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 {
    10
}

pub async fn search(
    State(state): State<Arc<AppState>>,
    Query(params): Query<SearchQuery>,
) -> Result<Json<SearchResponse>, (StatusCode, Json<ProblemDetail>)> {
    let query = params.q.unwrap_or_default();

    let filters = SearchFilters {
        jurisdiction: params.jurisdiction,
        doc_type: params.doc_type,
        language: params.language,
    };

    let svc = SearchService::new(state.pool.clone(), state.embedder.clone());

    svc.search(&query, filters, params.limit, params.offset)
        .await
        .map(Json)
        .map_err(|e| {
            let pd = e.to_problem_detail();
            let status =
                StatusCode::from_u16(pd.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
            (status, Json(pd))
        })
}

pub async fn not_found() -> (StatusCode, Json<Value>) {
    (
        StatusCode::NOT_FOUND,
        Json(serde_json::json!({ "error": "route not found" })),
    )
}
