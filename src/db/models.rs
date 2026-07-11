use chrono::{DateTime, NaiveTime, Utc};
use sqlx::FromRow;

#[derive(Debug, Clone, FromRow)]
pub struct User {
    pub id: i64,
    pub telegram_id: i64,
    pub timezone: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct CheckinSchedule {
    pub id: i64,
    pub user_id: i64,
    pub local_time: NaiveTime,
    pub enabled: bool,
}

/// Enum Postgres `entry_type`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "entry_type", rename_all = "lowercase")]
pub enum EntryType {
    Scheduled,
    Spontaneous,
}

#[derive(Debug, Clone, FromRow)]
pub struct MoodEntry {
    pub id: i64,
    pub user_id: i64,
    pub score: i16,
    pub entry_type: EntryType,
    pub note: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct Tag {
    pub id: i64,
    /// NULL = tag bawaan sistem.
    pub user_id: Option<i64>,
    pub name: String,
}
