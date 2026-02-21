//! Document handlers.
//!
//! Routes:
//!   GET  /api/v1/documents/:id
//!   GET  /api/v1/documents/:id/versions
//!   GET  /api/v1/documents/:id/chunks
//!   POST /api/v1/documents/upload   (multipart PDF upload)

use std::sync::Arc;

use axum::{
    extract::{Multipart, Path, State},
    http::StatusCode,
    response::Json,
};
use serde::Serialize;
use uuid::Uuid;

use legal_core::errors::ProblemDetail;

use crate::{
    services::document::{ChunkRow, DocumentRow, DocumentService, DocumentVersionRow},
    state::AppState,
};

fn svc_err(e: legal_core::errors::LegalMcpError) -> (StatusCode, Json<ProblemDetail>) {
    let pd = e.to_problem_detail();
    let status = StatusCode::from_u16(pd.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    (status, Json(pd))
}

fn bad_request(msg: &str) -> (StatusCode, Json<ProblemDetail>) {
    (
        StatusCode::UNPROCESSABLE_ENTITY,
        Json(ProblemDetail::new(
            "about:blank",
            "Unprocessable Entity",
            422,
            msg,
        )),
    )
}

/// `GET /api/v1/documents/:id`
pub async fn get_document(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<DocumentRow>, (StatusCode, Json<ProblemDetail>)> {
    DocumentService::new(state.pool.clone())
        .get_document(id)
        .await
        .map(Json)
        .map_err(svc_err)
}

/// `GET /api/v1/documents/:id/versions`
pub async fn get_versions(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<DocumentVersionRow>>, (StatusCode, Json<ProblemDetail>)> {
    DocumentService::new(state.pool.clone())
        .get_versions(id)
        .await
        .map(Json)
        .map_err(svc_err)
}

/// `GET /api/v1/documents/:id/chunks`
pub async fn get_chunks(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<ChunkRow>>, (StatusCode, Json<ProblemDetail>)> {
    DocumentService::new(state.pool.clone())
        .get_chunks(id)
        .await
        .map(Json)
        .map_err(svc_err)
}

/// Response from a successful PDF upload.
#[derive(Debug, Serialize)]
pub struct UploadResponse {
    pub document_id: Uuid,
    pub object_key: String,
    pub file_name: String,
    pub size_bytes: usize,
}

/// `POST /api/v1/documents/upload`
///
/// Accepts a multipart form upload with a field named `file` containing a PDF.
/// Validates MIME type and size, stores the file in object storage, creates a
/// `documents` row, and returns the new document ID.
///
/// Size limit: 50 MB (enforced in middleware via `DefaultBodyLimit`).
/// MIME validation: content-type header must be `application/pdf`.
pub async fn upload_document(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> Result<(StatusCode, Json<UploadResponse>), (StatusCode, Json<ProblemDetail>)> {
    use object_store::path::Path as OsPath;
    use sha2::{Digest, Sha256};

    // Collect the `file` field from the multipart body.
    let mut file_name = String::new();
    let mut content_type = String::new();
    let mut data: bytes::Bytes = bytes::Bytes::new();

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| bad_request(&format!("multipart error: {e}")))?
    {
        let name = field.name().unwrap_or("").to_string();
        if name == "file" {
            file_name = field.file_name().unwrap_or("upload.pdf").to_string();
            content_type = field
                .content_type()
                .unwrap_or("application/pdf")
                .to_string();

            data = field
                .bytes()
                .await
                .map_err(|e| bad_request(&format!("read error: {e}")))?;
        }
    }

    if data.is_empty() {
        return Err(bad_request("missing 'file' field in multipart body"));
    }

    // Validate MIME type.
    if !content_type.starts_with("application/pdf") {
        return Err(bad_request(&format!(
            "unsupported content type '{content_type}': only application/pdf is accepted"
        )));
    }

    // Validate size (50 MB).
    const MAX_BYTES: usize = 50 * 1024 * 1024;
    if data.len() > MAX_BYTES {
        return Err((
            StatusCode::PAYLOAD_TOO_LARGE,
            Json(ProblemDetail::new(
                "about:blank",
                "Payload Too Large",
                413,
                format!("file size {} bytes exceeds the 50 MB limit", data.len()),
            )),
        ));
    }

    let size_bytes = data.len();

    // Compute content hash for deduplication / key naming.
    let mut hasher = Sha256::new();
    hasher.update(&data);
    let content_hash = hex::encode(hasher.finalize());

    // Upload to object storage.
    let doc_id = Uuid::new_v4();
    let object_key = format!("uploads/{doc_id}/{content_hash}.pdf");
    let os_path = OsPath::from(object_key.clone());

    state
        .object_store
        .put(&os_path, data)
        .await
        .map_err(svc_err)?;

    // Insert document row. We need a source_id FK; use the first source if any,
    // otherwise create an "uploads" pseudo-source.
    let source_id = ensure_uploads_source(&state).await.map_err(svc_err)?;

    sqlx::query!(
        r#"
        INSERT INTO documents (
            id, source_id, external_id, url, title,
            content_md, doc_type, jurisdiction, language,
            content_hash, object_key, metadata_json
        ) VALUES (
            $1, $2, $3, '', $4,
            '', 'upload', 'UPLOAD', 'und',
            $5, $6, '{}'::jsonb
        )
        "#,
        doc_id as Uuid,
        source_id as Uuid,
        object_key, // external_id = object key (unique per upload)
        file_name,  // title = original filename
        content_hash,
        object_key,
    )
    .execute(&state.pool)
    .await
    .map_err(|e| svc_err(legal_core::errors::LegalMcpError::Database(e)))?;

    Ok((
        StatusCode::CREATED,
        Json(UploadResponse {
            document_id: doc_id,
            object_key,
            file_name,
            size_bytes,
        }),
    ))
}

/// Return the UUID of an "uploads" pseudo-source row, creating it if absent.
async fn ensure_uploads_source(
    state: &AppState,
) -> Result<Uuid, legal_core::errors::LegalMcpError> {
    let row = sqlx::query!(
        r#"
        INSERT INTO sources (name, jurisdiction, language, base_url, crawler_type, cron_schedule)
        VALUES ('Manual Uploads', 'UPLOAD', 'und', '', 'manual', '0 0 * * *')
        ON CONFLICT (name) DO NOTHING
        "#
    )
    .execute(&state.pool)
    .await?;
    let _ = row;

    let id = sqlx::query!("SELECT id FROM sources WHERE name = 'Manual Uploads' LIMIT 1")
        .fetch_one(&state.pool)
        .await?
        .id;

    Ok(id)
}
