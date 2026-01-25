use sqlx::PgPool;

use crate::bot::error::Error;
use crate::db::queries::{ban, mute, spam};

/// User statistics aggregated from the database
#[derive(Debug, Clone, Default)]
pub struct UserStats {
    pub mutes_received: i64,
    pub mutes_given: i64,
    pub bans_received: i64,
    pub bans_given: i64,
    pub spam_infractions: i64,
    pub current_timeout_level: i32,
}

/// Aggregate statistics for a user in a guild
pub async fn get_user_stats(
    pool: &PgPool,
    guild_id: i64,
    user_id: i64,
) -> Result<UserStats, Error> {
    let mutes_received = mute::get_user_mute_count(pool, guild_id, user_id).await?;
    let mutes_given = mute::get_user_mutes_given(pool, guild_id, user_id).await?;
    let bans_received = ban::get_user_ban_count(pool, guild_id, user_id).await?;
    let bans_given = ban::get_user_bans_given(pool, guild_id, user_id).await?;

    let (spam_infractions, current_timeout_level) =
        if let Some(spam_record) = spam::get_user_stats(pool, guild_id, user_id).await? {
            (
                spam_record.total_infractions as i64,
                spam_record.current_timeout_level,
            )
        } else {
            (0, 0)
        };

    Ok(UserStats {
        mutes_received,
        mutes_given,
        bans_received,
        bans_given,
        spam_infractions,
        current_timeout_level,
    })
}

/// Guild-wide statistics
#[derive(Debug, Clone, Default)]
pub struct GuildStats {
    pub total_mutes: i64,
    pub total_bans: i64,
    pub active_channels: i64,
}

/// Get guild-wide statistics
pub async fn get_guild_stats(pool: &PgPool, guild_id: i64) -> Result<GuildStats, Error> {
    let total_mutes: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM mute_history WHERE guild_id = $1")
            .bind(guild_id)
            .fetch_one(pool)
            .await?;

    let total_bans: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM vc_ban_history WHERE guild_id = $1")
            .bind(guild_id)
            .fetch_one(pool)
            .await?;

    let active_channels: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM active_voice_channels WHERE guild_id = $1")
            .bind(guild_id)
            .fetch_one(pool)
            .await?;

    Ok(GuildStats {
        total_mutes: total_mutes.0,
        total_bans: total_bans.0,
        active_channels: active_channels.0,
    })
}
