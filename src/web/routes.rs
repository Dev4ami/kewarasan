use axum::{routing::get, Router};
use sqlx::PgPool;

/// Router web. Route API (/api/trend, /api/tags, /api/heatmap) diisi di Step 6.
pub fn router(pool: PgPool) -> Router {
    Router::new().route("/", get(index)).with_state(pool)
}

async fn index() -> &'static str {
    "Kewarasan dashboard — segera hadir."
}
