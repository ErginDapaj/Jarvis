use chrono::{DateTime, Utc};
use sqlx::FromRow;

#[derive(Debug, Clone, FromRow)]
pub struct UserVcPreference {
    pub id: i32,
    pub guild_id: i64,
    pub user_id: i64,
    pub preferred_name: Option<String>,
    pub preferred_tags: Vec<String>,
    pub channel_type: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct PendingVcDeadline {
    pub channel_id: i64,
    pub guild_id: i64,
    pub owner_id: i64,
    pub deadline_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}
