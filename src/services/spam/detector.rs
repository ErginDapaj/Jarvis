use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};

use dashmap::DashMap;
use serenity::all::{ChannelId, Context, GuildId, UserId};
use tracing::{info, warn};

use crate::bot::data::Data;
use crate::bot::error::Error;
use crate::components::spam_prompt;
use crate::db::queries::spam;
use crate::services::spam::timeout_calculator;

/// Tracks join/leave activity for spam detection
pub struct ActivityTracker {
    /// channel_id -> user_id -> timestamps of activity
    activity: DashMap<u64, DashMap<u64, VecDeque<Instant>>>,
    /// channel_id -> set of users who have been prompted (to avoid duplicate prompts)
    prompted: DashMap<u64, DashMap<u64, Instant>>,
}

impl ActivityTracker {
    pub fn new() -> Self {
        Self {
            activity: DashMap::new(),
            prompted: DashMap::new(),
        }
    }

    /// Record a join event
    pub fn record_join(&self, channel_id: u64, user_id: u64, window_seconds: u64) {
        self.record_event(channel_id, user_id, window_seconds);
    }

    /// Record a leave event
    pub fn record_leave(&self, channel_id: u64, user_id: u64, window_seconds: u64) {
        self.record_event(channel_id, user_id, window_seconds);
    }

    fn record_event(&self, channel_id: u64, user_id: u64, window_seconds: u64) {
        let channel_map = self.activity.entry(channel_id).or_insert_with(DashMap::new);
        let mut user_events = channel_map.entry(user_id).or_insert_with(VecDeque::new);

        let now = Instant::now();
        let window = Duration::from_secs(window_seconds);

        // Remove old events outside the window
        while let Some(front) = user_events.front() {
            if now.duration_since(*front) > window {
                user_events.pop_front();
            } else {
                break;
            }
        }

        // Add new event
        user_events.push_back(now);
    }

    /// Get the activity count for a user in a channel
    pub fn get_activity_count(&self, channel_id: u64, user_id: u64, window_seconds: u64) -> u32 {
        let channel_map = match self.activity.get(&channel_id) {
            Some(m) => m,
            None => return 0,
        };

        let user_events = match channel_map.get(&user_id) {
            Some(e) => e,
            None => return 0,
        };

        let now = Instant::now();
        let window = Duration::from_secs(window_seconds);

        user_events
            .iter()
            .filter(|t| now.duration_since(**t) <= window)
            .count() as u32
    }

    /// Check if a user has been recently prompted
    pub fn was_recently_prompted(&self, channel_id: u64, user_id: u64) -> bool {
        if let Some(channel_prompts) = self.prompted.get(&channel_id) {
            if let Some(time_ref) = channel_prompts.get(&user_id) {
                let elapsed = time_ref.elapsed();
                drop(time_ref);
                // Consider a prompt recent if within the last 5 minutes
                return elapsed < Duration::from_secs(300);
            }
        }
        false
    }

    /// Mark a user as prompted
    pub fn mark_prompted(&self, channel_id: u64, user_id: u64) {
        let channel_prompts = self.prompted.entry(channel_id).or_insert_with(DashMap::new);
        channel_prompts.insert(user_id, Instant::now());
    }

    /// Clean up data for a deleted channel
    pub fn cleanup_channel(&self, channel_id: u64) {
        self.activity.remove(&channel_id);
        self.prompted.remove(&channel_id);
    }
}

impl Default for ActivityTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Check for spam activity and take action if needed
pub async fn check_spam(
    ctx: &Context,
    data: &Arc<Data>,
    guild_id: GuildId,
    channel_id: ChannelId,
    owner_id: UserId,
) -> Result<(), Error> {
    let prompt_threshold = data.settings.spam_prompt_threshold;
    let timeout_threshold = data.settings.spam_timeout_threshold;
    let window_seconds = data.settings.spam_window_seconds;

    // Get all users with high activity in this channel
    let channel_map = match data.activity_tracker.activity.get(&channel_id.get()) {
        Some(m) => m,
        None => return Ok(()),
    };

    for entry in channel_map.iter() {
        let user_id = *entry.key();
        let count = data.activity_tracker.get_activity_count(channel_id.get(), user_id, window_seconds);

        if count >= timeout_threshold {
            // Apply progressive timeout
            handle_spam_timeout(ctx, data, guild_id, UserId::new(user_id)).await?;
        } else if count >= prompt_threshold {
            // Prompt owner if not already prompted
            if !data.activity_tracker.was_recently_prompted(channel_id.get(), user_id) {
                spam_prompt::send_prompt(ctx, data, channel_id, owner_id, UserId::new(user_id))
                    .await?;
                data.activity_tracker.mark_prompted(channel_id.get(), user_id);
            }
        }
    }

    Ok(())
}

/// Apply a progressive timeout to a spam user
async fn handle_spam_timeout(
    ctx: &Context,
    data: &Arc<Data>,
    guild_id: GuildId,
    user_id: UserId,
) -> Result<(), Error> {
    // Get or create spam record and increment level
    let record = spam::increment_infraction(&data.pool, guild_id.get() as i64, user_id.get() as i64)
        .await?;

    // Calculate timeout duration
    let duration = timeout_calculator::get_timeout_duration(record.current_timeout_level as u32);

    info!(
        "Applying timeout level {} ({:?}) to user {} for spam",
        record.current_timeout_level, duration, user_id
    );

    // Apply Discord timeout
    let timeout_until = chrono::Utc::now() + chrono::Duration::from_std(duration).unwrap();
    // Format as ISO 8601 timestamp string for Discord API
    let timestamp_str = timeout_until.to_rfc3339();

    if let Err(e) = guild_id
        .edit_member(
            ctx,
            user_id,
            serenity::all::EditMember::new().disable_communication_until(timestamp_str),
        )
        .await
    {
        warn!("Failed to apply timeout to user {}: {:?}", user_id, e);
    }

    Ok(())
}
