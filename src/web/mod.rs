mod auth;
pub mod routes;

use crate::config::DashboardAuth;
use anyhow::Result;
use axum::middleware;
use sqlx::PgPool;
use std::net::SocketAddr;
use std::sync::Arc;

/// Jalankan Axum web server. Kalau `auth` di-set, semua request
/// harus lolos HTTP Basic Auth (browser bakal prompt sekali & remember).
pub async fn serve(pool: PgPool, port: u16, auth: Option<DashboardAuth>) -> Result<()> {
    let mut app = routes::router(pool);
    if let Some(creds) = auth {
        tracing::info!("dashboard auth aktif (user: {})", creds.user);
        let mw = auth::require(Arc::new(creds));
        app = app.layer(middleware::from_fn(mw));
    } else {
        tracing::warn!("dashboard TANPA auth — set DASHBOARD_PASSWORD kalau publik");
    }
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("web dashboard di http://{addr}");
    axum::serve(listener, app).await?;
    Ok(())
}
