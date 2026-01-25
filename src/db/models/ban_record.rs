use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct BanRecord {
    pub id: Uuid,
    pub guild_id: i64,
    pub channel_id: i64,
    pub banned_user_id: i64,
    pub banned_by_user_id: i64,
    pub reason: Option<String>,
    pub banned_at: DateTime<Utc>,
}
