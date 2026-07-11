pub mod routes;

use anyhow::Result;
use sqlx::PgPool;
use std::net::SocketAddr;

/// Jalankan Axum web server.
pub async fn serve(pool: PgPool, port: u16) -> Result<()> {
    let app = routes::router(pool);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("web dashboard di http://{addr}");
    axum::serve(listener, app).await?;
    Ok(())
}
