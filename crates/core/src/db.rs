use sqlx::postgres::{PgPool, PgPoolOptions};
use std::time::Duration;

use crate::errors::{LegalMcpError, Result};

/// Default maximum connections in the pool.
const DEFAULT_MAX_CONNECTIONS: u32 = 20;

/// Default connection acquisition timeout in seconds.
const DEFAULT_CONNECT_TIMEOUT_SECS: u64 = 10;

/// Create a `PgPool` from a database URL string.
///
/// Applies sensible defaults for connection limits and timeouts. Callers that
/// need different settings should configure `PgPoolOptions` directly.
///
/// # Errors
/// Returns `LegalMcpError::Database` if the pool cannot be established.
pub async fn create_pool(database_url: &str) -> Result<PgPool> {
    create_pool_with_options(
        database_url,
        DEFAULT_MAX_CONNECTIONS,
        DEFAULT_CONNECT_TIMEOUT_SECS,
    )
    .await
}

/// Create a `PgPool` with explicit limits and timeout.
pub async fn create_pool_with_options(
    database_url: &str,
    max_connections: u32,
    connect_timeout_secs: u64,
) -> Result<PgPool> {
    let pool = PgPoolOptions::new()
        .max_connections(max_connections)
        .acquire_timeout(Duration::from_secs(connect_timeout_secs))
        .connect(database_url)
        .await
        .map_err(LegalMcpError::Database)?;

    Ok(pool)
}

/// Run all pending sqlx migrations from `crates/core/migrations/`.
///
/// Migrations are embedded at compile time via the `migrate!` macro.
///
/// # Errors
/// Returns `LegalMcpError::Database` if any migration fails.
pub async fn run_migrations(pool: &PgPool) -> Result<()> {
    sqlx::migrate!("./migrations")
        .run(pool)
        .await
        .map_err(|e| LegalMcpError::Database(e.into()))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_constants_are_reasonable() {
        assert!(DEFAULT_MAX_CONNECTIONS > 0);
        assert!(DEFAULT_CONNECT_TIMEOUT_SECS > 0);
    }
}
