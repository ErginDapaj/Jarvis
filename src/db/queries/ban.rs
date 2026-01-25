use sqlx::PgPool;

use crate::db::models::BanRecord;

pub async fn create(
    pool: &PgPool,
    guild_id: i64,
    channel_id: i64,
    banned_user_id: i64,
    banned_by_user_id: i64,
    reason: Option<&str>,
) -> Result<BanRecord, sqlx::Error> {
    sqlx::query_as::<_, BanRecord>(
        r#"
        INSERT INTO vc_ban_history (guild_id, channel_id, banned_user_id, banned_by_user_id, reason)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING *
        "#
    )
    .bind(guild_id)
    .bind(channel_id)
    .bind(banned_user_id)
    .bind(banned_by_user_id)
    .bind(reason)
    .fetch_one(pool)
    .await
}

pub async fn is_banned(
    pool: &PgPool,
    channel_id: i64,
    user_id: i64,
) -> Result<bool, sqlx::Error> {
    let row: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM vc_ban_history WHERE channel_id = $1 AND banned_user_id = $2"
    )
    .bind(channel_id)
    .bind(user_id)
    .fetch_one(pool)
    .await?;

    Ok(row.0 > 0)
}

pub async fn get_bans_for_channel(
    pool: &PgPool,
    channel_id: i64,
) -> Result<Vec<BanRecord>, sqlx::Error> {
    sqlx::query_as::<_, BanRecord>(
        r#"
        SELECT * FROM vc_ban_history
        WHERE channel_id = $1
        ORDER BY banned_at DESC
        "#
    )
    .bind(channel_id)
    .fetch_all(pool)
    .await
}

pub async fn get_user_ban_count(
    pool: &PgPool,
    guild_id: i64,
    user_id: i64,
) -> Result<i64, sqlx::Error> {
    let row: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM vc_ban_history WHERE guild_id = $1 AND banned_user_id = $2"
    )
    .bind(guild_id)
    .bind(user_id)
    .fetch_one(pool)
    .await?;

    Ok(row.0)
}

pub async fn get_user_bans_given(
    pool: &PgPool,
    guild_id: i64,
    user_id: i64,
) -> Result<i64, sqlx::Error> {
    let row: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM vc_ban_history WHERE guild_id = $1 AND banned_by_user_id = $2"
    )
    .bind(guild_id)
    .bind(user_id)
    .fetch_one(pool)
    .await?;

    Ok(row.0)
}
