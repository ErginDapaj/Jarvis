use std::fmt;
use std::sync::Arc;

use dashmap::DashMap;
use sqlx::PgPool;

use crate::config::Settings;
use crate::services::spam::detector::ActivityTracker;

/// Shared data available to all commands and handlers
pub struct Data {
    pub pool: PgPool,
    pub settings: Settings,
    /// Cache of channel_id -> owner_id for quick lookups
    pub channel_owners: DashMap<u64, u64>,
    /// Activity tracker for spam detection
    pub activity_tracker: ActivityTracker,
    /// Users currently in the JTC flow (user_id -> timestamp)
    pub jtc_pending: DashMap<u64, std::time::Instant>,
    /// Track pending bot unmutes: (guild_id, user_id) -> timestamp
    /// Used to distinguish bot unmutes from manual owner unmutes
    pub pending_bot_unmutes: DashMap<(u64, u64), std::time::Instant>,
    /// Rate limit tracker for user limit changes: (user_id, channel_id) -> timestamps
    pub limit_change_timestamps: DashMap<(u64, u64), Vec<std::time::Instant>>,
}

impl Data {
    pub fn new(pool: PgPool, settings: Settings) -> Self {
        Self {
            pool,
            settings,
            channel_owners: DashMap::new(),
            activity_tracker: ActivityTracker::new(),
            jtc_pending: DashMap::new(),
            pending_bot_unmutes: DashMap::new(),
            limit_change_timestamps: DashMap::new(),
        }
    }

    /// Mark that the bot is about to unmute a user (to ignore the voice state event)
    pub fn mark_pending_unmute(&self, guild_id: u64, user_id: u64) {
        self.pending_bot_unmutes
            .insert((guild_id, user_id), std::time::Instant::now());
    }

    /// Check if this unmute was initiated by the bot (and consume the marker)
    /// Returns true if it was a bot unmute (should be ignored), false if manual
    pub fn consume_pending_unmute(&self, guild_id: u64, user_id: u64) -> bool {
        if let Some((_, timestamp)) = self.pending_bot_unmutes.remove(&(guild_id, user_id)) {
            // Only valid if within last 5 seconds
            timestamp.elapsed().as_secs() < 5
        } else {
            false
        }
    }

    /// Check if a user is the owner of a channel
    pub fn is_channel_owner(&self, channel_id: u64, user_id: u64) -> bool {
        self.channel_owners
            .get(&channel_id)
            .map(|owner| *owner == user_id)
            .unwrap_or(false)
    }

    /// Set the owner of a channel in cache
    pub fn set_channel_owner(&self, channel_id: u64, owner_id: u64) {
        self.channel_owners.insert(channel_id, owner_id);
    }

    /// Remove a channel from the cache
    pub fn remove_channel(&self, channel_id: u64) {
        self.channel_owners.remove(&channel_id);
    }

    /// Get the owner of a channel from cache
    pub fn get_channel_owner(&self, channel_id: u64) -> Option<u64> {
        self.channel_owners.get(&channel_id).map(|r| *r)
    }
}

impl fmt::Debug for Data {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Data")
            .field("channel_owners_count", &self.channel_owners.len())
            .field("jtc_pending_count", &self.jtc_pending.len())
            .finish_non_exhaustive()
    }
}

pub type Context<'a> = poise::Context<'a, Arc<Data>, crate::bot::error::Error>;
