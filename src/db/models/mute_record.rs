use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct MuteRecord {
    pub id: Uuid,
    pub guild_id: i64,
    pub channel_id: i64,
    pub muted_user_id: i64,
    pub muted_by_user_id: i64,
    pub is_admin_mute: bool,
    pub muted_at: DateTime<Utc>,
    pub unmuted_at: Option<DateTime<Utc>>,
}

impl MuteRecord {
    pub fn is_active(&self) -> bool {
        self.unmuted_at.is_none()
    }
}
