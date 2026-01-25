use std::sync::Arc;

use serenity::all::{CacheHttp, ChannelId, Context, EditMember, GuildId, UserId};
use tracing::{debug, error, info};

use crate::bot::data::Data;
use crate::bot::error::Error;
use crate::db::queries::mute;

/// Mute a user in a voice channel
pub async fn mute_user(
    ctx: &Context,
    data: &Arc<Data>,
    guild_id: GuildId,
    channel_id: ChannelId,
    muted_user_id: UserId,
    muted_by_user_id: UserId,
    is_admin_mute: bool,
) -> Result<(), Error> {
    // Apply server mute
    apply_server_mute(ctx, guild_id, muted_user_id, true).await?;

    // Store in database
    mute::create(
        &data.pool,
        guild_id.get() as i64,
        channel_id.get() as i64,
        muted_user_id.get() as i64,
        muted_by_user_id.get() as i64,
        is_admin_mute,
    )
    .await?;

    info!(
        "User {} muted user {} in channel {} (admin: {})",
        muted_by_user_id, muted_user_id, channel_id, is_admin_mute
    );

    Ok(())
}

/// Unmute a user in a voice channel
pub async fn unmute_user(
    ctx: &Context,
    data: &Arc<Data>,
    guild_id: GuildId,
    channel_id: ChannelId,
    user_id: UserId,
) -> Result<bool, Error> {
    // Check if user has an active mute
    let had_mute =
        mute::unmute_by_channel_user(&data.pool, channel_id.get() as i64, user_id.get() as i64)
            .await?;

    if had_mute {
        // Remove server mute
        apply_server_mute(ctx, guild_id, user_id, false).await?;
        info!("User {} unmuted in channel {}", user_id, channel_id);
    }

    Ok(had_mute)
}

/// Apply or remove server mute from a user
pub async fn apply_server_mute(
    http: impl CacheHttp,
    guild_id: GuildId,
    user_id: UserId,
    mute: bool,
) -> Result<(), Error> {
    let edit = EditMember::new().mute(mute);

    match guild_id.edit_member(&http, user_id, edit).await {
        Ok(_) => {
            debug!(
                "Server {} user {} in guild {}",
                if mute { "muted" } else { "unmuted" },
                user_id,
                guild_id
            );
            Ok(())
        }
        Err(e) => {
            error!(
                "Failed to {} user {}: {:?}",
                if mute { "mute" } else { "unmute" },
                user_id,
                e
            );
            Err(Error::Serenity(e))
        }
    }
}

/// Check if a user should be muted when joining a channel
pub async fn should_remute(
    data: &Arc<Data>,
    channel_id: ChannelId,
    user_id: UserId,
) -> Result<bool, Error> {
    let active_mute =
        mute::get_active_mute(&data.pool, channel_id.get() as i64, user_id.get() as i64).await?;

    Ok(active_mute.is_some())
}
