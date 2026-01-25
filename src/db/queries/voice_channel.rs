use sqlx::PgPool;

use crate::db::models::{ChannelType, VoiceChannel};

pub async fn create(
    pool: &PgPool,
    channel_id: i64,
    guild_id: i64,
    owner_id: i64,
    channel_type: ChannelType,
    topic: Option<&str>,
    tags: &[String],
) -> Result<VoiceChannel, sqlx::Error> {
    sqlx::query_as::<_, VoiceChannel>(
        r#"
        INSERT INTO active_voice_channels (channel_id, guild_id, owner_id, channel_type, topic, tags)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING *
        "#
    )
    .bind(channel_id)
    .bind(guild_id)
    .bind(owner_id)
    .bind(channel_type)
    .bind(topic)
    .bind(tags)
    .fetch_one(pool)
    .await
}

pub async fn get(pool: &PgPool, channel_id: i64) -> Result<Option<VoiceChannel>, sqlx::Error> {
    sqlx::query_as::<_, VoiceChannel>(
        "SELECT * FROM active_voice_channels WHERE channel_id = $1"
    )
    .bind(channel_id)
    .fetch_optional(pool)
    .await
}

pub async fn get_by_owner(
    pool: &PgPool,
    guild_id: i64,
    owner_id: i64,
) -> Result<Option<VoiceChannel>, sqlx::Error> {
    sqlx::query_as::<_, VoiceChannel>(
        "SELECT * FROM active_voice_channels WHERE guild_id = $1 AND owner_id = $2"
    )
    .bind(guild_id)
    .bind(owner_id)
    .fetch_optional(pool)
    .await
}

pub async fn update_owner(
    pool: &PgPool,
    channel_id: i64,
    new_owner_id: i64,
) -> Result<Option<VoiceChannel>, sqlx::Error> {
    sqlx::query_as::<_, VoiceChannel>(
        r#"
        UPDATE active_voice_channels
        SET owner_id = $2
        WHERE channel_id = $1
        RETURNING *
        "#
    )
    .bind(channel_id)
    .bind(new_owner_id)
    .fetch_optional(pool)
    .await
}

pub async fn update_topic(
    pool: &PgPool,
    channel_id: i64,
    topic: Option<&str>,
) -> Result<Option<VoiceChannel>, sqlx::Error> {
    sqlx::query_as::<_, VoiceChannel>(
        r#"
        UPDATE active_voice_channels
        SET topic = $2
        WHERE channel_id = $1
        RETURNING *
        "#
    )
    .bind(channel_id)
    .bind(topic)
    .fetch_optional(pool)
    .await
}

pub async fn update_tags(
    pool: &PgPool,
    channel_id: i64,
    tags: &[String],
) -> Result<Option<VoiceChannel>, sqlx::Error> {
    sqlx::query_as::<_, VoiceChannel>(
        r#"
        UPDATE active_voice_channels
        SET tags = $2
        WHERE channel_id = $1
        RETURNING *
        "#
    )
    .bind(channel_id)
    .bind(tags)
    .fetch_optional(pool)
    .await
}

pub async fn delete(pool: &PgPool, channel_id: i64) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM active_voice_channels WHERE channel_id = $1")
        .bind(channel_id)
        .execute(pool)
        .await?;

    Ok(result.rows_affected() > 0)
}

pub async fn list_by_guild(pool: &PgPool, guild_id: i64) -> Result<Vec<VoiceChannel>, sqlx::Error> {
    sqlx::query_as::<_, VoiceChannel>(
        "SELECT * FROM active_voice_channels WHERE guild_id = $1 ORDER BY created_at DESC"
    )
    .bind(guild_id)
    .fetch_all(pool)
    .await
}

pub async fn count_by_guild(pool: &PgPool, guild_id: i64) -> Result<i64, sqlx::Error> {
    let row: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM active_voice_channels WHERE guild_id = $1"
    )
    .bind(guild_id)
    .fetch_one(pool)
    .await?;

    Ok(row.0)
}

/// List all active voice channels (for cleanup on startup)
pub async fn list_all(pool: &PgPool) -> Result<Vec<VoiceChannel>, sqlx::Error> {
    sqlx::query_as::<_, VoiceChannel>(
        "SELECT * FROM active_voice_channels ORDER BY created_at DESC"
    )
    .fetch_all(pool)
    .await
}
