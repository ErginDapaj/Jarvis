use chrono::{DateTime, Utc};
use sqlx::PgPool;

use crate::db::models::{PendingVcDeadline, UserVcPreference};

/// Get user's VC preferences for a specific channel type
pub async fn get(
    pool: &PgPool,
    guild_id: i64,
    user_id: i64,
    channel_type: &str,
) -> Result<Option<UserVcPreference>, sqlx::Error> {
    sqlx::query_as::<_, UserVcPreference>(
        "SELECT * FROM user_vc_preferences WHERE guild_id = $1 AND user_id = $2 AND channel_type = $3"
    )
    .bind(guild_id)
    .bind(user_id)
    .bind(channel_type)
    .fetch_optional(pool)
    .await
}

/// Save or update user's VC preferences
pub async fn upsert(
    pool: &PgPool,
    guild_id: i64,
    user_id: i64,
    channel_type: &str,
    preferred_name: Option<&str>,
    preferred_tags: &[String],
) -> Result<UserVcPreference, sqlx::Error> {
    sqlx::query_as::<_, UserVcPreference>(
        r#"
        INSERT INTO user_vc_preferences (guild_id, user_id, channel_type, preferred_name, preferred_tags)
        VALUES ($1, $2, $3, $4, $5)
        ON CONFLICT (guild_id, user_id, channel_type)
        DO UPDATE SET
            preferred_name = $4,
            preferred_tags = $5,
            updated_at = NOW()
        RETURNING *
        "#
    )
    .bind(guild_id)
    .bind(user_id)
    .bind(channel_type)
    .bind(preferred_name)
    .bind(preferred_tags)
    .fetch_one(pool)
    .await
}

/// Create a pending deadline for a VC
pub async fn create_deadline(
    pool: &PgPool,
    channel_id: i64,
    guild_id: i64,
    owner_id: i64,
    deadline_at: DateTime<Utc>,
) -> Result<PendingVcDeadline, sqlx::Error> {
    sqlx::query_as::<_, PendingVcDeadline>(
        r#"
        INSERT INTO pending_vc_deadlines (channel_id, guild_id, owner_id, deadline_at)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (channel_id) DO UPDATE SET deadline_at = $4
        RETURNING *
        "#
    )
    .bind(channel_id)
    .bind(guild_id)
    .bind(owner_id)
    .bind(deadline_at)
    .fetch_one(pool)
    .await
}

/// Remove a pending deadline (when user configures or channel is deleted)
pub async fn remove_deadline(pool: &PgPool, channel_id: i64) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM pending_vc_deadlines WHERE channel_id = $1")
        .bind(channel_id)
        .execute(pool)
        .await?;

    Ok(result.rows_affected() > 0)
}

/// Get all deadlines that have passed
pub async fn get_expired_deadlines(pool: &PgPool) -> Result<Vec<PendingVcDeadline>, sqlx::Error> {
    sqlx::query_as::<_, PendingVcDeadline>(
        "SELECT * FROM pending_vc_deadlines WHERE deadline_at <= NOW()"
    )
    .fetch_all(pool)
    .await
}

/// Check if a channel has a pending deadline
pub async fn has_deadline(pool: &PgPool, channel_id: i64) -> Result<bool, sqlx::Error> {
    let result: Option<(i64,)> = sqlx::query_as(
        "SELECT channel_id FROM pending_vc_deadlines WHERE channel_id = $1"
    )
    .bind(channel_id)
    .fetch_optional(pool)
    .await?;

    Ok(result.is_some())
}
