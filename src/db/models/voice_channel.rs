use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "channel_type", rename_all = "lowercase")]
pub enum ChannelType {
    Casual,
    Debate,
}

impl ChannelType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ChannelType::Casual => "casual",
            ChannelType::Debate => "debate",
        }
    }

    pub fn is_casual(&self) -> bool {
        matches!(self, ChannelType::Casual)
    }
}

impl std::fmt::Display for ChannelType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct VoiceChannel {
    pub channel_id: i64,
    pub guild_id: i64,
    pub owner_id: i64,
    pub channel_type: ChannelType,
    pub topic: Option<String>,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
}

impl VoiceChannel {
    /// Generate the channel name based on topic and tags
    pub fn display_name(&self) -> String {
        if let Some(ref topic) = self.topic {
            if !topic.is_empty() {
                return topic.clone();
            }
        }

        match self.channel_type {
            ChannelType::Casual => "Casual VC".to_string(),
            ChannelType::Debate => "Debate VC".to_string(),
        }
    }
}
