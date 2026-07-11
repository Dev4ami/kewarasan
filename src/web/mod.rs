mod auth;
pub mod routes;
mod session;

use crate::config::DashboardAuth;
use anyhow::Result;
use sqlx::PgPool;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

pub use auth::AuthState;

/// Session cookie TTL — refresh tiap request, cukup lama buat "remember me".
const SESSION_TTL: Duration = Duration::from_secs(60 * 60 * 24 * 7); // 7 hari

/// State router web. `auth = None` = dashboard terbuka (dev lokal).
/// FromRef<AppState> untuk AppState sudah dicover blanket impl axum.
#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub auth: Option<AuthState>,
}

/// Jalankan Axum web server. Kalau `auth` di-set, halaman & API di-guard
/// cookie session (login form + tombol logout).
pub async fn serve(pool: PgPool, port: u16, auth: Option<DashboardAuth>) -> Result<()> {
    let auth_state = auth.map(|creds| {
        tracing::info!("dashboard auth aktif (user: {})", creds.user);
        AuthState {
            creds: Arc::new(creds),
            sessions: Arc::new(session::Sessions::new(SESSION_TTL)),
        }
    });
    if auth_state.is_none() {
        tracing::warn!("dashboard TANPA auth — set DASHBOARD_PASSWORD kalau publik");
    }

    let state = AppState {
        pool,
        auth: auth_state,
    };
    let app = routes::router(state);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("web dashboard di http://{addr}");
    axum::serve(listener, app).await?;
    Ok(())
}
