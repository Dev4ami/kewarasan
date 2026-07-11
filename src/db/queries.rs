// Query database — compile-time checked (query!/query_as!) lawan DB dev,
// dengan offline cache (.sqlx) supaya Docker build tidak butuh DB hidup.

use crate::db::models::{EntryType, Tag, User};
use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use sqlx::PgPool;

/// Cari user by telegram_id, bikin kalau belum ada. Selalu balikin row-nya.
pub async fn ensure_user(pool: &PgPool, telegram_id: i64) -> anyhow::Result<User> {
    let user = sqlx::query_as!(
        User,
        r#"INSERT INTO users (telegram_id) VALUES ($1)
           ON CONFLICT (telegram_id) DO UPDATE SET telegram_id = EXCLUDED.telegram_id
           RETURNING id, telegram_id, timezone, created_at"#,
        telegram_id
    )
    .fetch_one(pool)
    .await?;
    Ok(user)
}

/// Seperti [`ensure_user`], tapi kalau user baru pertama kali onboarding,
/// pasang `default_times` sebagai jadwal check-in — SEKALI saja (ditandai
/// kolom `onboarded_at`). Balikin (user, true) kalau baru saja di-onboard.
pub async fn onboard_user(
    pool: &PgPool,
    telegram_id: i64,
    default_times: &[NaiveTime],
) -> anyhow::Result<(User, bool)> {
    let user = ensure_user(pool, telegram_id).await?;

    let row = sqlx::query!("SELECT onboarded_at FROM users WHERE id = $1", user.id)
        .fetch_one(pool)
        .await?;
    let just_onboarded = row.onboarded_at.is_none();

    if just_onboarded {
        for t in default_times {
            add_schedule(pool, user.id, *t).await?;
        }
        sqlx::query!("UPDATE users SET onboarded_at = now() WHERE id = $1", user.id)
            .execute(pool)
            .await?;
    }

    Ok((user, just_onboarded))
}

/// Tag bawaan sistem (user_id NULL) + tag milik user, urut by id.
pub async fn list_tags(pool: &PgPool, user_id: i64) -> anyhow::Result<Vec<Tag>> {
    let tags = sqlx::query_as!(
        Tag,
        r#"SELECT id, user_id, name FROM tags
           WHERE user_id IS NULL OR user_id = $1
           ORDER BY id"#,
        user_id
    )
    .fetch_all(pool)
    .await?;
    Ok(tags)
}

/// Simpan satu mood entry, balikin id-nya.
pub async fn insert_entry(
    pool: &PgPool,
    user_id: i64,
    score: i16,
    entry_type: EntryType,
    note: Option<&str>,
) -> anyhow::Result<i64> {
    let rec = sqlx::query!(
        r#"INSERT INTO mood_entries (user_id, score, entry_type, note)
           VALUES ($1, $2, $3::text::entry_type, $4)
           RETURNING id"#,
        user_id,
        score,
        entry_type.as_str(),
        note
    )
    .fetch_one(pool)
    .await?;
    Ok(rec.id)
}

/// Sambungkan entry ke daftar tag (junction). No-op kalau kosong.
pub async fn attach_tags(pool: &PgPool, entry_id: i64, tag_ids: &[i64]) -> anyhow::Result<()> {
    if tag_ids.is_empty() {
        return Ok(());
    }
    sqlx::query!(
        r#"INSERT INTO entry_tags (entry_id, tag_id)
           SELECT $1, tid FROM UNNEST($2::bigint[]) AS t(tid)"#,
        entry_id,
        tag_ids
    )
    .execute(pool)
    .await?;
    Ok(())
}

// ============================================================
// Jadwal check-in
// ============================================================

/// Satu jadwal aktif + data buat ngirim pesannya.
pub struct DueCheckin {
    pub telegram_id: i64,
    pub timezone: String,
    pub local_time: NaiveTime,
}

/// Semua jadwal aktif (dari semua user). Matching jam dilakukan di scheduler
/// pakai timezone masing-masing user.
pub async fn all_enabled_schedules(pool: &PgPool) -> anyhow::Result<Vec<DueCheckin>> {
    let rows = sqlx::query_as!(
        DueCheckin,
        r#"SELECT u.telegram_id, u.timezone, s.local_time
           FROM checkin_schedules s
           JOIN users u ON u.id = s.user_id
           WHERE s.enabled = true"#
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Tambah jadwal (idempotent: jam yang sama = di-enable ulang).
pub async fn add_schedule(pool: &PgPool, user_id: i64, local_time: NaiveTime) -> anyhow::Result<()> {
    sqlx::query!(
        r#"INSERT INTO checkin_schedules (user_id, local_time, enabled)
           VALUES ($1, $2, true)
           ON CONFLICT (user_id, local_time) DO UPDATE SET enabled = true"#,
        user_id,
        local_time
    )
    .execute(pool)
    .await?;
    Ok(())
}

/// Hapus jadwal. Balikin true kalau ada yang kehapus.
pub async fn remove_schedule(
    pool: &PgPool,
    user_id: i64,
    local_time: NaiveTime,
) -> anyhow::Result<bool> {
    let r = sqlx::query!(
        "DELETE FROM checkin_schedules WHERE user_id = $1 AND local_time = $2",
        user_id,
        local_time
    )
    .execute(pool)
    .await?;
    Ok(r.rows_affected() > 0)
}

/// Daftar jam jadwal aktif milik user, urut.
pub async fn list_schedules(pool: &PgPool, user_id: i64) -> anyhow::Result<Vec<NaiveTime>> {
    let rows = sqlx::query!(
        r#"SELECT local_time FROM checkin_schedules
           WHERE user_id = $1 AND enabled = true
           ORDER BY local_time"#,
        user_id
    )
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(|r| r.local_time).collect())
}

// ============================================================
// Statistik (/stats)
// ============================================================

/// Rata-rata mood + jumlah check-in untuk satu hari (lokal user).
#[derive(serde::Serialize)]
pub struct DayAgg {
    pub day: NaiveDate,
    pub avg: f64,
    pub n: i64,
}

/// Agregasi mood per hari sejak `since` (UTC), di-bucket pakai timezone user.
/// Hanya hari yang ada entry yang muncul — pengisian slot kosong dilakukan di
/// pemanggil (biar sparkline selalu 7 kolom).
pub async fn weekly_daily(
    pool: &PgPool,
    user_id: i64,
    tz: &str,
    since: DateTime<Utc>,
) -> anyhow::Result<Vec<DayAgg>> {
    let rows = sqlx::query_as!(
        DayAgg,
        r#"SELECT (created_at AT TIME ZONE $2::text)::date AS "day!",
                  AVG(score)::float8               AS "avg!",
                  COUNT(*)                         AS "n!"
           FROM mood_entries
           WHERE user_id = $1 AND created_at >= $3
           GROUP BY 1
           ORDER BY 1"#,
        user_id,
        tz,
        since
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Rata-rata mood per tag dalam satu rentang (tag yang paling sering muncul).
#[derive(serde::Serialize)]
pub struct TagAgg {
    pub name: String,
    pub n: i64,
    pub avg: f64,
}

/// Tag paling sering nempel di entry sejak `since` (UTC), top 5.
/// Sengaja tanpa `HAVING COUNT >= 5` (itu buat korelasi dashboard) — di rentang
/// mingguan sampel kecil itu wajar, di sini kita cuma nunjukin "sering muncul".
pub async fn weekly_tags(
    pool: &PgPool,
    user_id: i64,
    since: DateTime<Utc>,
) -> anyhow::Result<Vec<TagAgg>> {
    let rows = sqlx::query_as!(
        TagAgg,
        r#"SELECT t.name           AS "name!",
                  COUNT(*)         AS "n!",
                  AVG(m.score)::float8 AS "avg!"
           FROM entry_tags et
           JOIN mood_entries m ON m.id = et.entry_id
           JOIN tags t         ON t.id = et.tag_id
           WHERE m.user_id = $1 AND m.created_at >= $2
           GROUP BY t.name
           ORDER BY COUNT(*) DESC, t.name
           LIMIT 5"#,
        user_id,
        since
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

// ============================================================
// Dashboard web — agregasi rentang tanggal (owner-scoped, timezone-aware)
// Batas waktu dikirim sebagai UTC: `created_at >= from AND < to`.
// ============================================================

/// Lookup user by telegram_id TANPA bikin baru (beda dari [`ensure_user`]).
/// Dipakai dashboard read-only: user yang belum pernah /start = None.
pub async fn find_user_by_telegram(
    pool: &PgPool,
    telegram_id: i64,
) -> anyhow::Result<Option<User>> {
    let user = sqlx::query_as!(
        User,
        r#"SELECT id, telegram_id, timezone, created_at
           FROM users WHERE telegram_id = $1"#,
        telegram_id
    )
    .fetch_optional(pool)
    .await?;
    Ok(user)
}

/// Trend rata-rata mood per hari (lokal user) dalam rentang [from, to).
pub async fn trend_range(
    pool: &PgPool,
    user_id: i64,
    tz: &str,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> anyhow::Result<Vec<DayAgg>> {
    let rows = sqlx::query_as!(
        DayAgg,
        r#"SELECT (created_at AT TIME ZONE $2::text)::date AS "day!",
                  AVG(score)::float8               AS "avg!",
                  COUNT(*)                         AS "n!"
           FROM mood_entries
           WHERE user_id = $1 AND created_at >= $3 AND created_at < $4
           GROUP BY 1
           ORDER BY 1"#,
        user_id,
        tz,
        from,
        to
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Satu sel heatmap: hari-dalam-minggu × jam (lokal user).
/// `dow`: 0=Minggu … 6=Sabtu (konvensi Postgres EXTRACT(DOW)).
#[derive(serde::Serialize)]
pub struct HeatCell {
    pub dow: i32,
    pub hour: i32,
    pub avg: f64,
    pub n: i64,
}

/// Heatmap rata-rata mood per (hari-minggu, jam) lokal, dalam rentang [from, to).
pub async fn heatmap(
    pool: &PgPool,
    user_id: i64,
    tz: &str,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> anyhow::Result<Vec<HeatCell>> {
    let rows = sqlx::query_as!(
        HeatCell,
        r#"SELECT EXTRACT(DOW  FROM created_at AT TIME ZONE $2::text)::int AS "dow!",
                  EXTRACT(HOUR FROM created_at AT TIME ZONE $2::text)::int AS "hour!",
                  AVG(score)::float8 AS "avg!",
                  COUNT(*)           AS "n!"
           FROM mood_entries
           WHERE user_id = $1 AND created_at >= $3 AND created_at < $4
           GROUP BY 1, 2
           ORDER BY 1, 2"#,
        user_id,
        tz,
        from,
        to
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Korelasi tag: rata-rata mood per tag dalam rentang [from, to), urut avg turun.
/// `HAVING COUNT(*) >= 5` — tag minim sampel jangan jadi insight palsu
/// (aturan CLAUDE.md).
pub async fn tag_correlation(
    pool: &PgPool,
    user_id: i64,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> anyhow::Result<Vec<TagAgg>> {
    let rows = sqlx::query_as!(
        TagAgg,
        r#"SELECT t.name               AS "name!",
                  COUNT(*)             AS "n!",
                  AVG(m.score)::float8 AS "avg!"
           FROM entry_tags et
           JOIN mood_entries m ON m.id = et.entry_id
           JOIN tags t         ON t.id = et.tag_id
           WHERE m.user_id = $1 AND m.created_at >= $2 AND m.created_at < $3
           GROUP BY t.name
           HAVING COUNT(*) >= 5
           ORDER BY AVG(m.score) DESC, COUNT(*) DESC"#,
        user_id,
        from,
        to
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}
