use std::sync::Arc;
use std::time::Duration;

use serenity::all::{ChannelId, Context, GuildId, UserId, VoiceState};
use tracing::{debug, error, info};

use crate::bot::data::Data;
use crate::bot::error::Error;
use crate::db::queries::{global_mute, guild_config, mute, voice_channel};
use crate::services::jtc::{channel_creator, channel_deleter};
use crate::services::moderation::mute_service;
use crate::services::spam::detector;

/// Delay before unmuting a user after they leave a channel (in seconds)
const UNMUTE_DELAY_SECONDS: u64 = 3;

pub async fn handle_voice_state_update(
    ctx: &Context,
    data: &Arc<Data>,
    old: Option<&VoiceState>,
    new: &VoiceState,
) -> Result<(), Error> {
    let guild_id = match new.guild_id {
        Some(id) => id,
        None => return Ok(()), // DM voice states are not supported
    };

    let user_id = new.user_id;
    let old_channel = old.and_then(|o| o.channel_id);
    let new_channel = new.channel_id;

    // Check for mute state changes (manual server mutes/unmutes)
    // We only detect changes when user STAYS in the same channel (not joining/leaving)
    // This avoids false positives from channel hopping
    if let Some(channel_id) = new_channel {
        let was_muted = old.map(|o| o.mute).unwrap_or(false);
        let is_muted = new.mute;

        // Debug: always log mute state for troubleshooting
        if was_muted != is_muted {
            debug!(
                "Mute state change: user={}, channel={}, old_channel={:?}, was_muted={}, is_muted={}, old_exists={}",
                user_id, channel_id, old_channel, was_muted, is_muted, old.is_some()
            );
        }

        // Only process if user stayed in the same channel
        if old_channel == Some(channel_id) {
            if !was_muted && is_muted {
                // User was just muted in this channel
                info!(
                    "Manual mute detected for user {} in channel {}",
                    user_id, channel_id
                );
                handle_mute_detected(data, guild_id, user_id, channel_id).await?;
            } else if was_muted && !is_muted {
                // User was just unmuted in this channel
                info!(
                    "Manual unmute detected for user {} in channel {}",
                    user_id, channel_id
                );
                handle_unmute_detected(data, guild_id, user_id, channel_id).await?;
            }
        }
    }

    // User joined a channel
    if let Some(channel_id) = new_channel {
        // Only handle join if channel actually changed
        if old_channel != Some(channel_id) {
            handle_channel_join(ctx, data, guild_id, user_id, channel_id).await?;
        }
    }

    // User left a channel
    if let Some(channel_id) = old_channel {
        // Only process if they actually left (moved to different channel or disconnected)
        if new_channel != Some(channel_id) {
            handle_channel_leave(ctx, data, guild_id, user_id, channel_id).await?;
        }
    }

    Ok(())
}

async fn handle_channel_join(
    ctx: &Context,
    data: &Arc<Data>,
    guild_id: GuildId,
    user_id: UserId,
    channel_id: ChannelId,
) -> Result<(), Error> {
    // Check if this is a JTC channel
    if let Some((_config, is_casual)) =
        guild_config::find_by_jtc_channel(&data.pool, channel_id.get() as i64).await?
    {
        info!(
            "User {} joined JTC channel {} ({})",
            user_id,
            channel_id,
            if is_casual { "casual" } else { "debate" }
        );

        // Start the JTC flow
        channel_creator::start_jtc_flow(ctx, data, guild_id, user_id, channel_id, is_casual)
            .await?;
        return Ok(());
    }

    // Check if this is a managed voice channel
    if let Some(vc) = voice_channel::get(&data.pool, channel_id.get() as i64).await? {
        debug!(
            "User {} joined managed VC {} owned by {}",
            user_id, channel_id, vc.owner_id
        );

        // Track activity for spam detection
        data.activity_tracker
            .record_join(channel_id.get(), user_id.get(), data.settings.spam_window_seconds);

        // Check for spam and potentially prompt owner
        detector::check_spam(ctx, data, guild_id, channel_id, UserId::new(vc.owner_id as u64))
            .await?;

        // Check if user has an active mute for this channel
        if let Some(_mute_record) =
            mute::get_active_mute(&data.pool, channel_id.get() as i64, user_id.get() as i64).await?
        {
            // Re-apply mute
            mute_service::apply_server_mute(ctx, guild_id, user_id, true).await?;
            debug!("Re-applied mute to user {} in channel {}", user_id, channel_id);
        }
    }

    // NOTE: We do NOT detect "joining already muted" as a global mute here
    // because the user might be hopping between channels during the 5-second unmute delay.
    // Global mutes are ONLY recorded when we see the actual mute EVENT fire
    // (state change from unmuted to muted) while the user is NOT in a managed VC.

    Ok(())
}

/// Handle when a mute is detected on a user
/// Determines if it's a VC-owner mute (tracked per-channel) or a global mute (never unmuted by bot)
async fn handle_mute_detected(
    data: &Arc<Data>,
    guild_id: GuildId,
    user_id: UserId,
    channel_id: ChannelId,
) -> Result<(), Error> {
    // Check if user is in a managed voice channel
    let vc = voice_channel::get(&data.pool, channel_id.get() as i64).await?;

    match vc {
        Some(vc) => {
            // User is in a managed VC - this is a VC owner mute
            // Check if already tracked
            if mute::get_active_mute(&data.pool, channel_id.get() as i64, user_id.get() as i64)
                .await?
                .is_some()
            {
                return Ok(()); // Already tracked
            }

            let owner_id = vc.owner_id as u64;
            info!(
                "Recording VC owner mute of user {} in channel {} (owner: {})",
                user_id, channel_id, owner_id
            );

            mute::create(
                &data.pool,
                guild_id.get() as i64,
                channel_id.get() as i64,
                user_id.get() as i64,
                owner_id as i64,
                false, // not an admin mute
            )
            .await?;
        }
        None => {
            // User is NOT in a managed VC - this is a global/admin mute
            info!(
                "Recording GLOBAL mute of user {} in guild {} (not in managed VC)",
                user_id, guild_id
            );

            global_mute::record_global_mute(
                &data.pool,
                guild_id.get() as i64,
                user_id.get() as i64,
            )
            .await?;
        }
    }

    Ok(())
}

/// Handle when an unmute is detected on a user
async fn handle_unmute_detected(
    data: &Arc<Data>,
    guild_id: GuildId,
    user_id: UserId,
    channel_id: ChannelId,
) -> Result<(), Error> {
    // Check if this was a bot-initiated unmute (should be ignored)
    if data.consume_pending_unmute(guild_id.get(), user_id.get()) {
        debug!(
            "Ignoring bot-initiated unmute for user {} in guild {}",
            user_id, guild_id
        );
        return Ok(());
    }

    // This is a manual unmute by the channel owner - only clear THIS channel's mute record
    let cleared = mute::unmute_by_channel_user(
        &data.pool,
        channel_id.get() as i64,
        user_id.get() as i64,
    )
    .await?;

    if cleared {
        info!(
            "Cleared mute record for user {} in channel {} (manual owner unmute)",
            user_id, channel_id
        );
    }

    // Also clear any global mute record (if someone with server perms unmuted them)
    if global_mute::record_global_unmute(&data.pool, guild_id.get() as i64, user_id.get() as i64)
        .await?
    {
        info!(
            "Cleared GLOBAL mute for user {} in guild {}",
            user_id, guild_id
        );
    }

    Ok(())
}

async fn handle_channel_leave(
    ctx: &Context,
    data: &Arc<Data>,
    guild_id: GuildId,
    user_id: UserId,
    channel_id: ChannelId,
) -> Result<(), Error> {
    // Check if this is a managed voice channel
    let vc = match voice_channel::get(&data.pool, channel_id.get() as i64).await? {
        Some(vc) => vc,
        None => return Ok(()), // Not a managed channel
    };

    // Track activity for spam detection
    data.activity_tracker
        .record_leave(channel_id.get(), user_id.get(), data.settings.spam_window_seconds);

    // Check if the owner left
    let owner_id = vc.owner_id as u64;
    if user_id.get() == owner_id {
        debug!("Owner {} left channel {}", user_id, channel_id);

        // Check if channel should be deleted
        channel_deleter::handle_owner_leave(ctx, data, guild_id, channel_id).await?;
    } else {
        // Non-owner left - check if they should be unmuted
        if let Some(mute_record) =
            mute::get_active_mute(&data.pool, channel_id.get() as i64, user_id.get() as i64).await?
        {
            // Only unmute if it wasn't an admin mute
            if !mute_record.is_admin_mute {
                // Delay unmute to allow for channel hopping
                // If user jumps to another channel where they're also muted, they stay muted
                let http = ctx.http.clone();
                let cache = ctx.cache.clone();
                let pool = data.pool.clone();
                let data = data.clone();

                tokio::spawn(async move {
                    tokio::time::sleep(Duration::from_secs(UNMUTE_DELAY_SECONDS)).await;

                    // First check: Is user globally muted? Never unmute them.
                    let is_globally_muted = match global_mute::is_globally_muted(
                        &pool,
                        guild_id.get() as i64,
                        user_id.get() as i64,
                    )
                    .await
                    {
                        Ok(is_muted) => is_muted,
                        Err(e) => {
                            error!("Failed to check global mute status: {:?}", e);
                            true // Assume globally muted on error to be safe
                        }
                    };

                    if is_globally_muted {
                        debug!(
                            "User {} has a global mute, not unmuting",
                            user_id
                        );
                        return;
                    }

                    // Second check: Did user hop to a channel where they're muted?
                    // Get their CURRENT voice channel (after the delay)
                    let current_channel = cache
                        .guild(guild_id)
                        .and_then(|g| g.voice_states.get(&user_id).and_then(|vs| vs.channel_id));

                    if let Some(current_channel_id) = current_channel {
                        // User is in a channel - check if they have a mute for THIS specific channel
                        let is_muted_in_current = match mute::get_active_mute(
                            &pool,
                            current_channel_id.get() as i64,
                            user_id.get() as i64,
                        )
                        .await
                        {
                            Ok(Some(_)) => true,
                            Ok(None) => false,
                            Err(e) => {
                                error!("Failed to check mute in current channel: {:?}", e);
                                false // Unmute on error to be safe
                            }
                        };

                        if is_muted_in_current {
                            debug!(
                                "User {} is in channel {} where they're muted, keeping server mute",
                                user_id, current_channel_id
                            );
                            // Don't remove the server mute - they're in a channel where they're muted
                            // Don't clear any records - records are only cleared by owner unmute
                            return;
                        }
                    }

                    // User is either not in any channel, or in a channel where they're NOT muted
                    // Remove the Discord server mute so they can talk elsewhere
                    // BUT DO NOT clear the mute record - that persists until owner unmutes them
                    data.mark_pending_unmute(guild_id.get(), user_id.get());

                    // Remove the Discord server mute (but keep the database record!)
                    if let Err(e) =
                        mute_service::apply_server_mute(&http, guild_id, user_id, false).await
                    {
                        error!("Failed to unmute user {} after delay: {:?}", user_id, e);
                    } else {
                        debug!(
                            "Removed server mute from user {} after {} second delay",
                            user_id, UNMUTE_DELAY_SECONDS
                        );
                    }
                });
            }
        }
    }

    Ok(())
}
