use std::sync::Arc;

use serenity::all::{
    ChannelId, Context, GuildId, PermissionOverwrite, PermissionOverwriteType,
    Permissions, UserId,
};
use tracing::{debug, info};

use crate::bot::data::Data;
use crate::bot::error::Error;
use crate::db::queries::ban;

/// Ban a user from a voice channel
pub async fn ban_user(
    ctx: &Context,
    data: &Arc<Data>,
    guild_id: GuildId,
    channel_id: ChannelId,
    banned_user_id: UserId,
    banned_by_user_id: UserId,
    reason: Option<&str>,
) -> Result<(), Error> {
    // Add permission deny for the banned user
    apply_channel_ban(ctx, channel_id, banned_user_id).await?;

    // Store in database
    ban::create(
        &data.pool,
        guild_id.get() as i64,
        channel_id.get() as i64,
        banned_user_id.get() as i64,
        banned_by_user_id.get() as i64,
        reason,
    )
    .await?;

    // Disconnect the user from the channel if they're in it
    disconnect_user(ctx, guild_id, banned_user_id).await?;

    info!(
        "User {} banned user {} from channel {} (reason: {:?})",
        banned_by_user_id, banned_user_id, channel_id, reason
    );

    Ok(())
}

/// Apply a channel permission deny for a banned user
async fn apply_channel_ban(
    ctx: &Context,
    channel_id: ChannelId,
    user_id: UserId,
) -> Result<(), Error> {
    let permission_overwrite = PermissionOverwrite {
        allow: Permissions::empty(),
        deny: Permissions::CONNECT | Permissions::VIEW_CHANNEL,
        kind: PermissionOverwriteType::Member(user_id),
    };

    channel_id
        .create_permission(ctx, permission_overwrite)
        .await?;

    debug!(
        "Applied ban permission for user {} on channel {}",
        user_id, channel_id
    );

    Ok(())
}

/// Disconnect a user from their current voice channel
async fn disconnect_user(ctx: &Context, guild_id: GuildId, user_id: UserId) -> Result<(), Error> {
    // Move user to no channel (disconnect)
    match guild_id
        .disconnect_member(ctx, user_id)
        .await
    {
        Ok(_) => {
            debug!("Disconnected user {} from voice", user_id);
        }
        Err(e) => {
            // User might not be in a voice channel
            debug!("Could not disconnect user {}: {:?}", user_id, e);
        }
    }

    Ok(())
}

/// Check if a user is banned from a channel
pub async fn is_banned(
    data: &Arc<Data>,
    channel_id: ChannelId,
    user_id: UserId,
) -> Result<bool, Error> {
    let banned = ban::is_banned(&data.pool, channel_id.get() as i64, user_id.get() as i64).await?;

    Ok(banned)
}
