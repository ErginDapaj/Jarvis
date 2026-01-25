use chrono::{DateTime, Utc};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow)]
pub struct GlobalMute {
    pub id: Uuid,
    pub guild_id: i64,
    pub user_id: i64,
    pub detected_at: DateTime<Utc>,
    pub unmuted_at: Option<DateTime<Utc>>,
}
