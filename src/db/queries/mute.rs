use sqlx::PgPool;
use uuid::Uuid;

use crate::db::models::MuteRecord;

pub async fn create(
    pool: &PgPool,
    guild_id: i64,
    channel_id: i64,
    muted_user_id: i64,
    muted_by_user_id: i64,
    is_admin_mute: bool,
) -> Result<MuteRecord, sqlx::Error> {
    sqlx::query_as::<_, MuteRecord>(
        r#"
        INSERT INTO mute_history (guild_id, channel_id, muted_user_id, muted_by_user_id, is_admin_mute)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING *
        "#
    )
    .bind(guild_id)
    .bind(channel_id)
    .bind(muted_user_id)
    .bind(muted_by_user_id)
    .bind(is_admin_mute)
    .fetch_one(pool)
    .await
}

pub async fn get_active_mute(
    pool: &PgPool,
    channel_id: i64,
    user_id: i64,
) -> Result<Option<MuteRecord>, sqlx::Error> {
    sqlx::query_as::<_, MuteRecord>(
        r#"
        SELECT * FROM mute_history
        WHERE channel_id = $1 AND muted_user_id = $2 AND unmuted_at IS NULL
        ORDER BY muted_at DESC
        LIMIT 1
        "#
    )
    .bind(channel_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
}

pub async fn unmute(pool: &PgPool, id: Uuid) -> Result<Option<MuteRecord>, sqlx::Error> {
    sqlx::query_as::<_, MuteRecord>(
        r#"
        UPDATE mute_history
        SET unmuted_at = NOW()
        WHERE id = $1
        RETURNING *
        "#
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

pub async fn unmute_by_channel_user(
    pool: &PgPool,
    channel_id: i64,
    user_id: i64,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        r#"
        UPDATE mute_history
        SET unmuted_at = NOW()
        WHERE channel_id = $1 AND muted_user_id = $2 AND unmuted_at IS NULL
        "#
    )
    .bind(channel_id)
    .bind(user_id)
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}

pub async fn get_user_mute_count(
    pool: &PgPool,
    guild_id: i64,
    user_id: i64,
) -> Result<i64, sqlx::Error> {
    let row: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM mute_history WHERE guild_id = $1 AND muted_user_id = $2"
    )
    .bind(guild_id)
    .bind(user_id)
    .fetch_one(pool)
    .await?;

    Ok(row.0)
}

pub async fn get_user_mutes_given(
    pool: &PgPool,
    guild_id: i64,
    user_id: i64,
) -> Result<i64, sqlx::Error> {
    let row: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM mute_history WHERE guild_id = $1 AND muted_by_user_id = $2"
    )
    .bind(guild_id)
    .bind(user_id)
    .fetch_one(pool)
    .await?;

    Ok(row.0)
}

pub async fn list_active_mutes_for_channel(
    pool: &PgPool,
    channel_id: i64,
) -> Result<Vec<MuteRecord>, sqlx::Error> {
    sqlx::query_as::<_, MuteRecord>(
        r#"
        SELECT * FROM mute_history
        WHERE channel_id = $1 AND unmuted_at IS NULL
        ORDER BY muted_at DESC
        "#
    )
    .bind(channel_id)
    .fetch_all(pool)
    .await
}

/// Clear all active mute records for a user in a guild
/// Used when a server-wide unmute is detected
pub async fn unmute_all_for_user_in_guild(
    pool: &PgPool,
    guild_id: i64,
    user_id: i64,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        r#"
        UPDATE mute_history
        SET unmuted_at = NOW()
        WHERE guild_id = $1 AND muted_user_id = $2 AND unmuted_at IS NULL
        "#
    )
    .bind(guild_id)
    .bind(user_id)
    .execute(pool)
    .await?;

    Ok(result.rows_affected())
}

/// Check if user has an active mute in any channel in the guild, except the specified one
/// Used to determine if a user hopping between channels should stay muted
pub async fn has_active_mute_in_guild_except(
    pool: &PgPool,
    guild_id: i64,
    user_id: i64,
    except_channel_id: i64,
) -> Result<bool, sqlx::Error> {
    let row: (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*) FROM mute_history
        WHERE guild_id = $1
        AND muted_user_id = $2
        AND channel_id != $3
        AND unmuted_at IS NULL
        "#
    )
    .bind(guild_id)
    .bind(user_id)
    .bind(except_channel_id)
    .fetch_one(pool)
    .await?;

    Ok(row.0 > 0)
}
