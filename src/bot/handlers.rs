use crate::bot::{
    callbacks::{self, Action},
    keyboards,
};
use crate::db::{models::EntryType, queries};
use sqlx::PgPool;
use teloxide::{
    prelude::*,
    types::{ChatId, MaybeInaccessibleMessage, MessageId, ParseMode},
    utils::command::BotCommands,
};

/// Telegram id pemilik bot — dapet sapaan spesial.
const OWNER_ID: i64 = 1069319412;

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

/// Rangkai handler tree (command + callback) lalu jalankan dispatcher.
pub async fn dispatch(bot: Bot, pool: PgPool) {
    let handler = dptree::entry()
        .branch(
            Update::filter_message()
                .filter_command::<Command>()
                .endpoint(handle_command),
        )
        .branch(Update::filter_callback_query().endpoint(handle_callback));

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
    pool: PgPool,
) -> anyhow::Result<()> {
    match cmd {
        Command::Start => {
            // Chat privat: chat.id == telegram user id.
            let telegram_id = msg.chat.id.0;
            queries::ensure_user(&pool, telegram_id).await?;

            let nama = msg
                .from
                .as_ref()
                .map(|u| u.first_name.clone())
                .unwrap_or_else(|| "kamu".to_string());
            let sapaan = if telegram_id == OWNER_ID {
                "si ganteng".to_string()
            } else {
                nama
            };

            let text = format!(
                "Halo, <b>{sapaan}</b> 👋🧠\n\n\
                 ID kamu: <code>{telegram_id}</code>\n\n\
                 Tugasku simpel: sesekali nanya kabar batinmu, kamu cukup tap emoji. \
                 Nggak usah panjang-panjang — emosi cuma riak, biar aku yang petain arusnya.\n\n\
                 /waras — kapan pun kamu butuh jujur sama diri sendiri.",
                sapaan = html_escape(&sapaan),
            );
            bot.send_message(msg.chat.id, text)
                .parse_mode(ParseMode::Html)
                .await?;
        }
        Command::Waras => {
            bot.send_message(msg.chat.id, "Lagi ngerasa apa sekarang? Jujur aja, nggak ada yang nilai. 👇")
                .reply_markup(keyboards::mood_keyboard())
                .await?;
        }
        Command::Stats => {
            bot.send_message(
                msg.chat.id,
                "Polamu belum kebaca — ceritanya masih ngumpul. Kasih aku waktu ya 📊",
            )
            .await?;
        }
        Command::Jadwal => {
            bot.send_message(msg.chat.id, "Ngatur jam aku nyapa kamu — segera hadir ⏰")
                .await?;
        }
    }
    Ok(())
}

async fn handle_callback(bot: Bot, q: CallbackQuery, pool: PgPool) -> anyhow::Result<()> {
    let cq_id = q.id.clone();

    let Some(action) = q.data.as_deref().and_then(callbacks::parse) else {
        bot.answer_callback_query(cq_id).await?;
        return Ok(());
    };
    let Some((chat_id, message_id)) = message_ref(&q) else {
        bot.answer_callback_query(cq_id).await?;
        return Ok(());
    };
    let telegram_id = q.from.id.0 as i64;
    let user_id = queries::ensure_user(&pool, telegram_id).await?.id;

    match action {
        Action::Score(score) => {
            let tags = queries::list_tags(&pool, user_id).await?;
            bot.edit_message_text(chat_id, message_id, "Ada apa di baliknya? (boleh lebih dari satu)")
                .reply_markup(keyboards::tags_keyboard(score, &[], &tags))
                .await?;
        }
        Action::Toggle { score, tags: sel } => {
            let tags = queries::list_tags(&pool, user_id).await?;
            bot.edit_message_reply_markup(chat_id, message_id)
                .reply_markup(keyboards::tags_keyboard(score, &sel, &tags))
                .await?;
        }
        Action::Finalize { score, tags: sel } => {
            let all = queries::list_tags(&pool, user_id).await?;
            let entry_id =
                queries::insert_entry(&pool, user_id, score, EntryType::Spontaneous, None).await?;
            queries::attach_tags(&pool, entry_id, &sel).await?;

            let names: Vec<&str> = all
                .iter()
                .filter(|t| sel.contains(&t.id))
                .map(|t| t.name.as_str())
                .collect();
            bot.edit_message_text(chat_id, message_id, summary_text(score, &names))
                .await?;
        }
    }

    bot.answer_callback_query(cq_id).await?;
    Ok(())
}

/// Ambil (chat_id, message_id) dari pesan callback, apa pun variannya.
fn message_ref(q: &CallbackQuery) -> Option<(ChatId, MessageId)> {
    match &q.message {
        Some(MaybeInaccessibleMessage::Regular(m)) => Some((m.chat.id, m.id)),
        Some(MaybeInaccessibleMessage::Inaccessible(m)) => Some((m.chat.id, m.message_id)),
        None => None,
    }
}

/// Escape karakter yang bermakna di HTML parse-mode Telegram.
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn summary_text(score: i16, tags: &[&str]) -> String {
    let emoji = keyboards::mood_emoji(score);
    let vibe = match score {
        5 => "Waras maksimal hari ini ✨ Nikmatin — kamu pantas kok.",
        4 => "Adem. Semoga betah lama-lama di sini 🌤",
        3 => "Datar aja. Nggak semua hari harus istimewa, dan itu gapapa.",
        2 => "Lagi berat sebelah ya… pelan-pelan, nggak usah buru-buru pulih 🫂",
        _ => "Berat banget hari ini. Tapi kamu masih di sini — dan itu udah cukup 🖤",
    };
    let jejak = if tags.is_empty() {
        format!("📝 tercatat {emoji}")
    } else {
        format!("📝 tercatat {emoji} · {}", tags.join(", "))
    };
    format!("{vibe}\n\n{jejak}")
}
