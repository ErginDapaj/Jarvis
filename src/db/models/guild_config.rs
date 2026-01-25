use chrono::{DateTime, Utc};

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct GuildConfig {
    pub guild_id: i64,
    pub jtc_casual_channel_id: Option<i64>,
    pub jtc_debate_channel_id: Option<i64>,
    pub category_casual_id: Option<i64>,
    pub category_debate_id: Option<i64>,
    pub rules_casual_channel_id: Option<i64>,
    pub rules_debate_channel_id: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl GuildConfig {
    /// Check if this guild has JTC fully configured for a given channel type
    pub fn is_jtc_configured(&self, casual: bool) -> bool {
        if casual {
            self.jtc_casual_channel_id.is_some() && self.category_casual_id.is_some()
        } else {
            self.jtc_debate_channel_id.is_some() && self.category_debate_id.is_some()
        }
    }

    /// Get the JTC channel ID for a given type
    pub fn jtc_channel_id(&self, casual: bool) -> Option<i64> {
        if casual {
            self.jtc_casual_channel_id
        } else {
            self.jtc_debate_channel_id
        }
    }

    /// Get the category ID for a given type
    pub fn category_id(&self, casual: bool) -> Option<i64> {
        if casual {
            self.category_casual_id
        } else {
            self.category_debate_id
        }
    }

    /// Get the rules channel ID for a given type
    pub fn rules_channel_id(&self, casual: bool) -> Option<i64> {
        if casual {
            self.rules_casual_channel_id
        } else {
            self.rules_debate_channel_id
        }
    }
}
