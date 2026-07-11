use sqlx::PgPool;
use teloxide::{prelude::*, utils::command::BotCommands};

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "Perintah Kewarasan:")]
pub enum Command {
    #[command(description = "daftar & atur jadwal check-in")]
    Start,
    #[command(description = "catat mood sekarang")]
    Waras,
    #[command(description = "seberapa waras kamu minggu ini")]
    Stats,
    #[command(description = "atur jam check-in harian")]
    Jadwal,
}

/// Rangkai handler tree lalu jalankan dispatcher sampai proses dimatikan.
pub async fn dispatch(bot: Bot, pool: PgPool) {
    let handler = dptree::entry().branch(
        Update::filter_message()
            .filter_command::<Command>()
            .endpoint(handle_command),
    );

    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![pool])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}

async fn handle_command(
    bot: Bot,
    msg: Message,
    cmd: Command,
    _pool: PgPool,
) -> anyhow::Result<()> {
    let text = match cmd {
        Command::Start => "Halo! Aku Kewarasan 🧠 — bakal bantu pantau mood-mu. (skeleton)",
        Command::Waras => "Catat mood: fitur segera hadir.",
        Command::Stats => "Statistik: fitur segera hadir.",
        Command::Jadwal => "Atur jadwal: fitur segera hadir.",
    };
    bot.send_message(msg.chat.id, text).await?;
    Ok(())
}
