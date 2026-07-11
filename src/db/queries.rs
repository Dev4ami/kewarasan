// Query database — compile-time checked (query!/query_as!) lawan DB dev,
// dengan offline cache (.sqlx) supaya Docker build tidak butuh DB hidup.

use crate::db::models::{EntryType, Tag, User};
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
