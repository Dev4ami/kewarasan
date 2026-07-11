use crate::bot::keyboards;
use crate::db::queries;
use chrono::{Timelike, Utc};
use chrono_tz::Tz;
use sqlx::PgPool;
use std::time::Duration;
use teloxide::{prelude::*, types::ChatId};
use tokio::time::{self, MissedTickBehavior};

/// Loop check-in terjadwal: tiap 60 detik cek jadwal yang match jam-menit
/// sekarang di timezone masing-masing user.
pub async fn run(bot: Bot, pool: PgPool) -> anyhow::Result<()> {
    let mut ticker = time::interval(Duration::from_secs(60));
    // Kalau tick telat (mesin sibuk/tidur), jangan burst nyusul — skip aja.
    ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);

    loop {
        ticker.tick().await;
        if let Err(e) = tick(&bot, &pool).await {
            tracing::error!("scheduler tick error: {e:?}");
        }
    }
}

async fn tick(bot: &Bot, pool: &PgPool) -> anyhow::Result<()> {
    let now = Utc::now();
    for c in queries::all_enabled_schedules(pool).await? {
        let tz: Tz = c.timezone.parse().unwrap_or(chrono_tz::Asia::Jakarta);
        let local = now.with_timezone(&tz);
        if local.hour() == c.local_time.hour() && local.minute() == c.local_time.minute() {
            send_checkin(bot, c.telegram_id).await;
        }
    }
    Ok(())
}

async fn send_checkin(bot: &Bot, telegram_id: i64) {
    // Yang didiamkan JANGAN di-follow-up — cukup kirim sekali, data bolong itu normal.
    let res = bot
        .send_message(ChatId(telegram_id), "Jeda sebentar 🌙 Lagi ngerasa apa sekarang?")
        .reply_markup(keyboards::mood_keyboard(true))
        .await;
    if let Err(e) = res {
        tracing::warn!("gagal kirim check-in ke {telegram_id}: {e}");
    }
}
