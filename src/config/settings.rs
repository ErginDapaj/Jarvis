use std::env;

use crate::constants::timeouts::{
    DEFAULT_SPAM_PROMPT_THRESHOLD, DEFAULT_SPAM_TIMEOUT_THRESHOLD, DEFAULT_SPAM_WINDOW_SECONDS,
};

#[derive(Debug, Clone)]
pub struct Settings {
    pub discord_token: String,
    pub database_url: String,
    pub donate_link: Option<String>,
    pub guild_id: Option<u64>,
    /// Spam detection: events before prompting owner
    pub spam_prompt_threshold: u32,
    /// Spam detection: events before auto-timeout
    pub spam_timeout_threshold: u32,
    /// Spam detection: rolling window in seconds
    pub spam_window_seconds: u64,
}

impl Settings {
    pub fn from_env() -> Result<Self, String> {
        let discord_token = env::var("DISCORD_TOKEN")
            .map_err(|_| "DISCORD_TOKEN environment variable not set")?;

        let database_url = env::var("DATABASE_URL")
            .map_err(|_| "DATABASE_URL environment variable not set")?;

        let donate_link = env::var("DONATE_LINK").ok().filter(|s| !s.is_empty());

        let guild_id = env::var("GUILD_ID")
            .ok()
            .and_then(|s| s.parse::<u64>().ok());

        let spam_prompt_threshold = env::var("SPAM_PROMPT_THRESHOLD")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(DEFAULT_SPAM_PROMPT_THRESHOLD);

        let spam_timeout_threshold = env::var("SPAM_TIMEOUT_THRESHOLD")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(DEFAULT_SPAM_TIMEOUT_THRESHOLD);

        let spam_window_seconds = env::var("SPAM_WINDOW_SECONDS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(DEFAULT_SPAM_WINDOW_SECONDS);

        Ok(Self {
            discord_token,
            database_url,
            donate_link,
            guild_id,
            spam_prompt_threshold,
            spam_timeout_threshold,
            spam_window_seconds,
        })
    }
}
