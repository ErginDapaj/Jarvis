use std::sync::Arc;

use serenity::all::{ChannelId, Context, GuildId, Http};
use sqlx::PgPool;
use tracing::{debug, info, warn};

use crate::bot::data::Data;
use crate::bot::error::Error;
use crate::db::queries::voice_channel;

/// Handle when the channel owner leaves
pub async fn handle_owner_leave(
    ctx: &Context,
    data: &Arc<Data>,
    guild_id: GuildId,
    channel_id: ChannelId,
) -> Result<(), Error> {
    // Get current member count in the channel
    let member_count = get_channel_member_count(ctx, guild_id, channel_id).await;

    debug!(
        "Owner left channel {}, {} members remaining",
        channel_id, member_count
    );

    if member_count < 2 {
        // Delete the channel
        delete_channel(ctx, data, channel_id).await?;
    } else {
        // Transfer ownership to another member
        if let Some(new_owner) = get_next_owner(ctx, guild_id, channel_id).await {
            transfer_ownership(ctx, data, channel_id, new_owner).await?;
        } else {
            // No valid owner found, delete the channel
            delete_channel(ctx, data, channel_id).await?;
        }
    }

    Ok(())
}

/// Delete a managed voice channel
pub async fn delete_channel(
    ctx: &Context,
    data: &Arc<Data>,
    channel_id: ChannelId,
) -> Result<(), Error> {
    // Delete from database first
    voice_channel::delete(&data.pool, channel_id.get() as i64).await?;

    // Remove from cache
    data.remove_channel(channel_id.get());

    // Delete the Discord channel
    match channel_id.delete(ctx).await {
        Ok(_) => {
            info!("Deleted voice channel {}", channel_id);
        }
        Err(e) => {
            // Channel might already be deleted
            warn!("Failed to delete channel {}: {:?}", channel_id, e);
        }
    }

    Ok(())
}

/// Transfer ownership to a new user
async fn transfer_ownership(
    _ctx: &Context,
    data: &Arc<Data>,
    channel_id: ChannelId,
    new_owner_id: u64,
) -> Result<(), Error> {
    // Update database
    voice_channel::update_owner(&data.pool, channel_id.get() as i64, new_owner_id as i64).await?;

    // Update cache
    data.set_channel_owner(channel_id.get(), new_owner_id);

    info!(
        "Transferred ownership of channel {} to user {}",
        channel_id, new_owner_id
    );

    // Could notify the new owner via the text channel
    // For now, just update the permissions
    // This would require editing channel permissions to give new owner manage rights

    Ok(())
}

/// Get the number of members in a voice channel
async fn get_channel_member_count(ctx: &Context, guild_id: GuildId, channel_id: ChannelId) -> usize {
    // Get guild from cache
    if let Some(guild) = ctx.cache.guild(guild_id) {
        return guild
            .voice_states
            .values()
            .filter(|vs| vs.channel_id == Some(channel_id))
            .count();
    }

    0
}

/// Get the next suitable owner from the channel members
async fn get_next_owner(ctx: &Context, guild_id: GuildId, channel_id: ChannelId) -> Option<u64> {
    if let Some(guild) = ctx.cache.guild(guild_id) {
        // Find a member in the channel (not a bot)
        for vs in guild.voice_states.values() {
            if vs.channel_id == Some(channel_id) {
                // Check if user is not a bot
                if let Some(member) = guild.members.get(&vs.user_id) {
                    if !member.user.bot {
                        return Some(vs.user_id.get());
                    }
                }
            }
        }
    }

    None
}

/// Check for empty channels and delete them (runs after cache is populated)
pub async fn cleanup_empty_channels(
    ctx: &Context,
    data: &Arc<Data>,
) -> Result<usize, Error> {
    let channels = voice_channel::list_all(&data.pool).await?;
    let mut deleted = 0;

    for vc in channels {
        let channel_id = ChannelId::new(vc.channel_id as u64);
        let guild_id = GuildId::new(vc.guild_id as u64);

        // Check if channel is empty using cache
        let member_count = get_channel_member_count(ctx, guild_id, channel_id).await;

        if member_count == 0 {
            info!("Deleting empty channel {} on startup check", channel_id);
            delete_channel(ctx, data, channel_id).await?;
            deleted += 1;
        }
    }

    Ok(deleted)
}

/// Restore channels to cache on bot startup
/// Only removes DB entries for channels that no longer exist in Discord
/// Does NOT delete any Discord channels - let normal handlers manage that
/// Returns (cleaned_count, restored_count)
pub async fn cleanup_orphaned_channels(
    http: &Http,
    pool: &PgPool,
    data: &Arc<Data>,
) -> Result<(usize, usize), Error> {
    let channels = voice_channel::list_all(pool).await?;
    let mut cleaned = 0;
    let mut restored = 0;

    info!("Restoring {} voice channels from database...", channels.len());

    for vc in channels {
        let channel_id = ChannelId::new(vc.channel_id as u64);

        // Try to get the channel from Discord
        match http.get_channel(channel_id).await {
            Ok(channel) => {
                // Channel exists in Discord
                if channel.guild().is_some() {
                    // Restore to cache - normal voice state handlers will manage cleanup
                    // Empty channels will be cleaned up when users join/leave triggers voice state updates
                    // Note: We can't easily check member count via HTTP API without cache on startup,
                    // so we restore all existing channels. The voice state handler will delete empty
                    // channels when the owner leaves or when the channel becomes empty.
                    data.set_channel_owner(vc.channel_id as u64, vc.owner_id as u64);
                    restored += 1;
                    debug!("Restored channel {} to cache (owner: {})", channel_id, vc.owner_id);
                } else {
                    // Not a guild channel (shouldn't happen) - clean DB
                    voice_channel::delete(pool, vc.channel_id).await?;
                    cleaned += 1;
                }
            }
            Err(_) => {
                // Channel doesn't exist in Discord anymore - clean DB entry
                voice_channel::delete(pool, vc.channel_id).await?;
                info!("Removed non-existent channel {} from database", channel_id);
                cleaned += 1;
            }
        }
    }

    Ok((cleaned, restored))
}
