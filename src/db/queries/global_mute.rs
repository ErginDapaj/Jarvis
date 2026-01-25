use sqlx::PgPool;

use crate::db::models::GlobalMute;

/// Check if a user has an active global mute
pub async fn is_globally_muted(
    pool: &PgPool,
    guild_id: i64,
    user_id: i64,
) -> Result<bool, sqlx::Error> {
    let row: (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*) FROM global_mutes
        WHERE guild_id = $1 AND user_id = $2 AND unmuted_at IS NULL
        "#,
    )
    .bind(guild_id)
    .bind(user_id)
    .fetch_one(pool)
    .await?;

    Ok(row.0 > 0)
}

/// Record a global mute (admin/server-wide mute)
pub async fn record_global_mute(
    pool: &PgPool,
    guild_id: i64,
    user_id: i64,
) -> Result<GlobalMute, sqlx::Error> {
    sqlx::query_as::<_, GlobalMute>(
        r#"
        INSERT INTO global_mutes (guild_id, user_id)
        VALUES ($1, $2)
        ON CONFLICT (guild_id, user_id) WHERE unmuted_at IS NULL
        DO UPDATE SET detected_at = NOW()
        RETURNING *
        "#,
    )
    .bind(guild_id)
    .bind(user_id)
    .fetch_one(pool)
    .await
}

/// Record a global unmute
pub async fn record_global_unmute(
    pool: &PgPool,
    guild_id: i64,
    user_id: i64,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        r#"
        UPDATE global_mutes
        SET unmuted_at = NOW()
        WHERE guild_id = $1 AND user_id = $2 AND unmuted_at IS NULL
        "#,
    )
    .bind(guild_id)
    .bind(user_id)
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}
