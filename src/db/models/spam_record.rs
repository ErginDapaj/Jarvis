use chrono::{DateTime, Utc};

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct SpamRecord {
    pub guild_id: i64,
    pub user_id: i64,
    pub current_timeout_level: i32,
    pub last_infraction_at: Option<DateTime<Utc>>,
    pub total_infractions: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl SpamRecord {
    /// Check if the timeout level should be reset (30 days of good behavior)
    pub fn should_reset(&self) -> bool {
        if let Some(last) = self.last_infraction_at {
            let days_since = (Utc::now() - last).num_days();
            days_since >= 30
        } else {
            true
        }
    }
}
