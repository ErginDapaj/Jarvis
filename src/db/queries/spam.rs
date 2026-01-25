use sqlx::PgPool;

use crate::db::models::SpamRecord;

pub async fn get_or_create(
    pool: &PgPool,
    guild_id: i64,
    user_id: i64,
) -> Result<SpamRecord, sqlx::Error> {
    // Try to get existing record
    let existing = sqlx::query_as::<_, SpamRecord>(
        "SELECT * FROM spam_user_status WHERE guild_id = $1 AND user_id = $2"
    )
    .bind(guild_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await?;

    if let Some(record) = existing {
        return Ok(record);
    }

    // Create new record
    sqlx::query_as::<_, SpamRecord>(
        r#"
        INSERT INTO spam_user_status (guild_id, user_id)
        VALUES ($1, $2)
        RETURNING *
        "#
    )
    .bind(guild_id)
    .bind(user_id)
    .fetch_one(pool)
    .await
}

pub async fn increment_infraction(
    pool: &PgPool,
    guild_id: i64,
    user_id: i64,
) -> Result<SpamRecord, sqlx::Error> {
    // Ensure record exists
    get_or_create(pool, guild_id, user_id).await?;

    sqlx::query_as::<_, SpamRecord>(
        r#"
        UPDATE spam_user_status
        SET
            current_timeout_level = LEAST(current_timeout_level + 1, 7),
            last_infraction_at = NOW(),
            total_infractions = total_infractions + 1,
            updated_at = NOW()
        WHERE guild_id = $1 AND user_id = $2
        RETURNING *
        "#
    )
    .bind(guild_id)
    .bind(user_id)
    .fetch_one(pool)
    .await
}

pub async fn reset_timeout_level(
    pool: &PgPool,
    guild_id: i64,
    user_id: i64,
) -> Result<SpamRecord, sqlx::Error> {
    sqlx::query_as::<_, SpamRecord>(
        r#"
        UPDATE spam_user_status
        SET
            current_timeout_level = 0,
            updated_at = NOW()
        WHERE guild_id = $1 AND user_id = $2
        RETURNING *
        "#
    )
    .bind(guild_id)
    .bind(user_id)
    .fetch_one(pool)
    .await
}

pub async fn get_user_stats(
    pool: &PgPool,
    guild_id: i64,
    user_id: i64,
) -> Result<Option<SpamRecord>, sqlx::Error> {
    sqlx::query_as::<_, SpamRecord>(
        "SELECT * FROM spam_user_status WHERE guild_id = $1 AND user_id = $2"
    )
    .bind(guild_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
}
