use crate::bot::{
    callbacks::{self, Action},
    keyboards,
};
use crate::config::OWNER_ID;
use crate::db::{models::EntryType, queries};
use chrono::{Duration, NaiveTime, TimeZone, Utc};
use chrono_tz::Tz;
use sqlx::PgPool;
use teloxide::{
    prelude::*,
    types::{ChatId, MaybeInaccessibleMessage, MessageId, ParseMode},
    utils::command::BotCommands,
};

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "Perintah Kewarasan:")]
pub enum Command {
    #[command(description = "daftar & lihat cara pakai")]
    Start,
    #[command(description = "catat mood sekarang")]
    Waras,
    #[command(description = "seberapa waras kamu minggu ini")]
    Stats,
    #[command(description = "atur jam check-in (mis. /jadwal 09:00)")]
    Jadwal(String),
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

async fn handle_command(bot: Bot, msg: Message, cmd: Command, pool: PgPool) -> anyhow::Result<()> {
    match cmd {
        Command::Start => {
            // Chat privat: chat.id == telegram user id.
            let telegram_id = msg.chat.id.0;
            let (_, just_onboarded) =
                queries::onboard_user(&pool, telegram_id, &default_schedule_times()).await?;

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

            let mut text = format!(
                "Halo, <b>{sapaan}</b> 👋🧠\n\n\
                 ID kamu: <code>{telegram_id}</code>\n\n\
                 Tugasku simpel: sesekali nanya kabar batinmu, kamu cukup tap emoji. \
                 Nggak usah panjang-panjang — emosi cuma riak, biar aku yang petain arusnya.\n\n\
                 /waras — catat mood kapan aja\n\
                 /jadwal — atur biar aku yang nyapa duluan",
                sapaan = html_escape(&sapaan),
            );
            if just_onboarded {
                text.push_str(
                    "\n\n⏰ Aku pasangin jadwal check-in default: <b>09:00 · 15:00 · 21:00</b>. \
                     Ganti atau hapus lewat /jadwal.",
                );
            }
            bot.send_message(msg.chat.id, text)
                .parse_mode(ParseMode::Html)
                .await?;
        }
        Command::Waras => {
            bot.send_message(
                msg.chat.id,
                "Lagi ngerasa apa sekarang? Jujur aja, nggak ada yang nilai. 👇",
            )
            .reply_markup(keyboards::mood_keyboard(false))
            .await?;
        }
        Command::Stats => {
            handle_stats(&bot, &msg, &pool).await?;
        }
        Command::Jadwal(arg) => {
            handle_jadwal(&bot, &msg, &pool, &arg).await?;
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
        Action::Score { scheduled, score } => {
            let tags = queries::list_tags(&pool, user_id).await?;
            bot.edit_message_text(chat_id, message_id, "Ada apa di baliknya? (boleh lebih dari satu)")
                .reply_markup(keyboards::tags_keyboard(scheduled, score, &[], &tags))
                .await?;
        }
        Action::Toggle {
            scheduled,
            score,
            tags: sel,
        } => {
            let tags = queries::list_tags(&pool, user_id).await?;
            bot.edit_message_reply_markup(chat_id, message_id)
                .reply_markup(keyboards::tags_keyboard(scheduled, score, &sel, &tags))
                .await?;
        }
        Action::Finalize {
            scheduled,
            score,
            tags: sel,
        } => {
            let all = queries::list_tags(&pool, user_id).await?;
            let etype = if scheduled {
                EntryType::Scheduled
            } else {
                EntryType::Spontaneous
            };
            let entry_id = queries::insert_entry(&pool, user_id, score, etype, None).await?;
            queries::attach_tags(&pool, entry_id, &sel).await?;

            let names: Vec<&str> = all
                .iter()
                .filter(|t| sel.contains(&t.id))
                .map(|t| t.name.as_str())
                .collect();
            bot.edit_message_text(chat_id, message_id, summary_text(score, &names))
                .await?;
        }
        Action::Cancel => {
            bot.edit_message_text(chat_id, message_id, "Oke, nggak jadi. Kapan-kapan aja ya 🌙")
                .await?;
        }
    }

    bot.answer_callback_query(cq_id).await?;
    Ok(())
}

/// /stats — ringkasan mood 7 hari terakhir: rata-rata, jumlah check-in,
/// sparkline harian, dan tag yang paling sering nempel.
async fn handle_stats(bot: &Bot, msg: &Message, pool: &PgPool) -> anyhow::Result<()> {
    let user = queries::ensure_user(pool, msg.chat.id.0).await?;
    let tz: Tz = user.timezone.parse().unwrap_or(chrono_tz::Asia::Jakarta);

    // Jendela 7 hari: dari awal hari (today-6) lokal user → batas bawah UTC.
    let today = Utc::now().with_timezone(&tz).date_naive();
    let start_day = today - Duration::days(6);
    let start_local = start_day.and_hms_opt(0, 0, 0).unwrap();
    let since = tz
        .from_local_datetime(&start_local)
        .earliest()
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|| Utc::now() - Duration::days(7));

    let daily = queries::weekly_daily(pool, user.id, &user.timezone, since).await?;

    // Isi 7 slot: hari tanpa entry jadi '·' (gap), bukan bar rendah (biar nggak
    // kebaca sebagai mood 1).
    let mut bars = String::with_capacity(7 * 3);
    let mut total_n: i64 = 0;
    let mut weighted: f64 = 0.0;
    for i in 0..7 {
        let day = start_day + Duration::days(i);
        match daily.iter().find(|d| d.day == day) {
            Some(d) => {
                bars.push(bar(d.avg));
                total_n += d.n;
                weighted += d.avg * d.n as f64;
            }
            None => bars.push('·'),
        }
    }

    if total_n == 0 {
        bot.send_message(
            msg.chat.id,
            "Minggu ini masih kosong 🫙\nCatat dulu yuk — /waras — biar ada arus yang bisa kubaca 📊",
        )
        .await?;
        return Ok(());
    }

    let avg = weighted / total_n as f64;
    let emoji = keyboards::mood_emoji(avg.round() as i16);

    let mut text = format!(
        "📊 <b>Kewarasan kamu</b> · seminggu terakhir\n\n\
         Rata-rata <b>{avg:.1}</b>/5 {emoji} · <b>{total_n}</b> check-in\n\n\
         <code>{bars}</code>\n\
         <i>← 7 hari lalu · hari ini →</i>",
    );

    let tags = queries::weekly_tags(pool, user.id, since).await?;
    if !tags.is_empty() {
        text.push_str("\n\n<b>Paling sering nongol:</b>");
        for t in &tags {
            let te = keyboards::mood_emoji(t.avg.round() as i16);
            text.push_str(&format!(
                "\n• {} · {}× (rata {:.1} {te})",
                html_escape(&t.name),
                t.n,
                t.avg,
            ));
        }
    }

    bot.send_message(msg.chat.id, text)
        .parse_mode(ParseMode::Html)
        .await?;
    Ok(())
}

/// Petakan rata-rata mood (1–5) ke satu bar sparkline.
fn bar(avg: f64) -> char {
    const BARS: [char; 5] = ['▁', '▂', '▄', '▅', '▇'];
    let idx = (avg.round() as i64).clamp(1, 5) - 1;
    BARS[idx as usize]
}

/// /jadwal — tanpa arg = lihat daftar; `09:00` = tambah; `hapus 09:00` = hapus.
async fn handle_jadwal(bot: &Bot, msg: &Message, pool: &PgPool, arg: &str) -> anyhow::Result<()> {
    let user = queries::ensure_user(pool, msg.chat.id.0).await?;
    let arg = arg.trim();

    // Lihat daftar.
    if arg.is_empty() || arg.eq_ignore_ascii_case("list") {
        let times = queries::list_schedules(pool, user.id).await?;
        let text = if times.is_empty() {
            format!(
                "Belum ada jadwal check-in 🌙\n\n\
                 Tambah jam: /jadwal 09:00 (boleh beberapa)\n\
                 Hapus: /jadwal hapus 09:00\n\n\
                 Zona waktu kamu: {}",
                user.timezone
            )
        } else {
            let list = times
                .iter()
                .map(|t| format!("• {}", t.format("%H:%M")))
                .collect::<Vec<_>>()
                .join("\n");
            format!(
                "Jadwal check-in kamu ({}):\n{list}\n\n\
                 Tambah: /jadwal 21:00 · Hapus: /jadwal hapus 21:00",
                user.timezone
            )
        };
        bot.send_message(msg.chat.id, text).await?;
        return Ok(());
    }

    // Hapus.
    if let Some(rest) = arg.strip_prefix("hapus").or_else(|| arg.strip_prefix("off")) {
        match parse_time(rest.trim()) {
            Some(t) => {
                let removed = queries::remove_schedule(pool, user.id, t).await?;
                let text = if removed {
                    format!("Oke, jadwal {} dihapus. Aku diam di jam itu ya.", t.format("%H:%M"))
                } else {
                    "Nggak nemu jadwal di jam itu.".to_string()
                };
                bot.send_message(msg.chat.id, text).await?;
            }
            None => {
                bot.send_message(msg.chat.id, "Format: /jadwal hapus 09:00").await?;
            }
        }
        return Ok(());
    }

    // Tambah.
    match parse_time(arg) {
        Some(t) => {
            queries::add_schedule(pool, user.id, t).await?;
            bot.send_message(
                msg.chat.id,
                format!(
                    "Sip 🌙 aku bakal nyapa kamu tiap {} ({}).\n\
                     Mood dari check-in ini kesimpen sebagai 'terjadwal'.",
                    t.format("%H:%M"),
                    user.timezone
                ),
            )
            .await?;
        }
        None => {
            bot.send_message(msg.chat.id, "Format jamnya HH:MM ya, contoh: /jadwal 09:00")
                .await?;
        }
    }
    Ok(())
}

fn parse_time(s: &str) -> Option<NaiveTime> {
    NaiveTime::parse_from_str(s, "%H:%M").ok()
}

/// Jadwal check-in default buat user baru: pagi, sore, malam.
fn default_schedule_times() -> Vec<NaiveTime> {
    [9, 15, 21]
        .iter()
        .filter_map(|h| NaiveTime::from_hms_opt(*h, 0, 0))
        .collect()
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
