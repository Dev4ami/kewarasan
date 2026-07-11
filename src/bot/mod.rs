pub mod callbacks;
pub mod handlers;
pub mod keyboards;
pub mod scheduler;

use anyhow::Result;
use sqlx::PgPool;
use teloxide::{prelude::*, utils::command::BotCommands};

use handlers::Command;

/// Bangun bot, daftarkan menu command, lalu jalankan dispatcher (long polling).
pub async fn run(token: String, pool: PgPool) -> Result<()> {
    let bot = Bot::new(token);

    // Daftar menu command = nice-to-have. Kalau jaringan ke Telegram lagi ngadat,
    // jangan bikin bot mati — cukup warn, dispatcher tetap jalan (punya retry sendiri).
    if let Err(e) = bot.set_my_commands(Command::bot_commands()).await {
        tracing::warn!("gagal daftarin menu command (lanjut aja): {e}");
    }

    handlers::dispatch(bot, pool).await;
    Ok(())
}
