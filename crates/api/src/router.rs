// router — Axum router wiring all handlers to their routes.

use std::{net::IpAddr, sync::Arc};

use axum::{
    extract::{ConnectInfo, Request, State},
    http::StatusCode,
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{delete, get, patch, post},
    Router,
};
use governor::{clock::DefaultClock, state::keyed::DefaultKeyedStateStore, Quota, RateLimiter};
use std::num::NonZeroU32;

use crate::{
    handlers::{admin, changes, documents, health, search, sources},
    mcp::build_mcp_service,
    middleware::require_api_key,
    state::AppState,
};

/// Shared rate limiter: keyed by peer IP address.
type IpRateLimiter = RateLimiter<IpAddr, DefaultKeyedStateStore<IpAddr>, DefaultClock>;

/// Rate-limit middleware: 100 requests per 60 seconds per IP (QR-S3).
async fn rate_limit(
    State(limiter): State<Arc<IpRateLimiter>>,
    req: Request,
    next: Next,
) -> Response {
    // Extract peer IP from ConnectInfo if available, fall back to 127.0.0.1.
    let ip = req
        .extensions()
        .get::<ConnectInfo<std::net::SocketAddr>>()
        .map(|ci| ci.0.ip())
        .unwrap_or(IpAddr::V4(std::net::Ipv4Addr::LOCALHOST));

    match limiter.check_key(&ip) {
        Ok(_) => next.run(req).await,
        Err(_) => (StatusCode::TOO_MANY_REQUESTS, "Rate limit exceeded").into_response(),
    }
}

/// Build and return the application router.
///
/// Auth policy per SDD (CON-8):
/// - Public (no auth required): GET /health, GET /ready, GET /api/v1/search,
///   GET /api/v1/documents/*, GET /api/v1/sources (list only), GET /api/v1/changes
/// - Requires Bearer API key: POST/PATCH/DELETE /api/v1/sources, all /api/v1/admin/*,
///   POST /api/v1/documents/upload, POST /api/v1/sources/:id/crawl
pub fn app_router(state: Arc<AppState>) -> Router {
    // Rate limiter for public endpoints: 100 req/min per IP (QR-S3).
    let limiter = Arc::new(RateLimiter::keyed(Quota::per_minute(
        NonZeroU32::new(100).expect("non-zero"),
    )));

    // Admin sub-router — all routes require a valid Bearer API key.
    let admin_routes = Router::new()
        .route("/stats", get(admin::get_stats))
        .route("/api-keys", get(admin::list_api_keys))
        .route("/api-keys", post(admin::create_api_key))
        .route("/api-keys/{id}", delete(admin::revoke_api_key))
        .route_layer(middleware::from_fn_with_state(
            Arc::clone(&state),
            require_api_key,
        ));

    // Source-mutation sub-router — POST/PATCH/DELETE require a valid Bearer API key.
    let source_mutation_routes = Router::new()
        .route("/", post(sources::create_source))
        .route("/{id}", patch(sources::update_source))
        .route("/{id}", delete(sources::delete_source))
        .route_layer(middleware::from_fn_with_state(
            Arc::clone(&state),
            require_api_key,
        ));

    // Auth-protected state-changing endpoints for upload and crawl trigger.
    let protected_actions = Router::new()
        .route("/api/v1/documents/upload", post(documents::upload_document))
        .route("/api/v1/sources/{id}/crawl", post(sources::trigger_crawl))
        .route_layer(middleware::from_fn_with_state(
            Arc::clone(&state),
            require_api_key,
        ));

    // MCP Streamable HTTP service (stateful, sessions).
    let mcp_service = build_mcp_service(Arc::clone(&state));

    // Rate-limited public routes.
    let public_routes = Router::new()
        // ── Health ──────────────────────────────────────────────────────────
        .route("/health", get(health::health))
        .route("/ready", get(health::ready))
        // ── Search ──────────────────────────────────────────────────────────
        .route("/api/v1/search", get(search::search))
        // ── Documents ───────────────────────────────────────────────────────
        .route("/api/v1/documents/{id}", get(documents::get_document))
        .route(
            "/api/v1/documents/{id}/versions",
            get(documents::get_versions),
        )
        .route("/api/v1/documents/{id}/chunks", get(documents::get_chunks))
        // ── Sources ─────────────────────────────────────────────────────────
        .route("/api/v1/sources", get(sources::list_sources))
        .route("/api/v1/sources/{id}", get(sources::get_source))
        // ── Changes ─────────────────────────────────────────────────────────
        .route("/api/v1/changes", get(changes::list_changes))
        .route_layer(middleware::from_fn_with_state(limiter, rate_limit));

    Router::new()
        .merge(public_routes)
        // ── Auth-protected state-changing endpoints ──────────────────────────
        .merge(protected_actions)
        .nest("/api/v1/sources", source_mutation_routes)
        // ── Admin (auth-protected) ───────────────────────────────────────────
        .nest("/api/v1/admin", admin_routes)
        // ── MCP (Streamable HTTP, stateful sessions) ─────────────────────────
        .nest_service("/mcp", mcp_service)
        // ── Fallback ─────────────────────────────────────────────────────────
        .fallback(search::not_found)
        .with_state(state)
}
