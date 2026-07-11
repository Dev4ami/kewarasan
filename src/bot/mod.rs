pub mod callbacks;
pub mod handlers;
pub mod keyboards;
pub mod scheduler;

use anyhow::Result;
use sqlx::PgPool;
use teloxide::Bot;

/// Bangun bot + jalankan dispatcher (long polling).
pub async fn run(token: String, pool: PgPool) -> Result<()> {
    let bot = Bot::new(token);
    handlers::dispatch(bot, pool).await;
    Ok(())
}
