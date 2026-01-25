use sqlx::PgPool;

use crate::db::models::GuildConfig;

pub async fn get_or_create(pool: &PgPool, guild_id: i64) -> Result<GuildConfig, sqlx::Error> {
    // Try to get existing config
    let existing = sqlx::query_as::<_, GuildConfig>(
        "SELECT * FROM guild_configs WHERE guild_id = $1"
    )
    .bind(guild_id)
    .fetch_optional(pool)
    .await?;

    if let Some(config) = existing {
        return Ok(config);
    }

    // Create new config
    sqlx::query_as::<_, GuildConfig>(
        r#"
        INSERT INTO guild_configs (guild_id)
        VALUES ($1)
        RETURNING *
        "#
    )
    .bind(guild_id)
    .fetch_one(pool)
    .await
}

pub async fn get(pool: &PgPool, guild_id: i64) -> Result<Option<GuildConfig>, sqlx::Error> {
    sqlx::query_as::<_, GuildConfig>(
        "SELECT * FROM guild_configs WHERE guild_id = $1"
    )
    .bind(guild_id)
    .fetch_optional(pool)
    .await
}

pub async fn set_jtc_channel(
    pool: &PgPool,
    guild_id: i64,
    casual: bool,
    channel_id: i64,
) -> Result<GuildConfig, sqlx::Error> {
    // Ensure config exists
    get_or_create(pool, guild_id).await?;

    let query = if casual {
        r#"
        UPDATE guild_configs
        SET jtc_casual_channel_id = $2, updated_at = NOW()
        WHERE guild_id = $1
        RETURNING *
        "#
    } else {
        r#"
        UPDATE guild_configs
        SET jtc_debate_channel_id = $2, updated_at = NOW()
        WHERE guild_id = $1
        RETURNING *
        "#
    };

    sqlx::query_as::<_, GuildConfig>(query)
        .bind(guild_id)
        .bind(channel_id)
        .fetch_one(pool)
        .await
}

pub async fn set_category(
    pool: &PgPool,
    guild_id: i64,
    casual: bool,
    category_id: i64,
) -> Result<GuildConfig, sqlx::Error> {
    // Ensure config exists
    get_or_create(pool, guild_id).await?;

    let query = if casual {
        r#"
        UPDATE guild_configs
        SET category_casual_id = $2, updated_at = NOW()
        WHERE guild_id = $1
        RETURNING *
        "#
    } else {
        r#"
        UPDATE guild_configs
        SET category_debate_id = $2, updated_at = NOW()
        WHERE guild_id = $1
        RETURNING *
        "#
    };

    sqlx::query_as::<_, GuildConfig>(query)
        .bind(guild_id)
        .bind(category_id)
        .fetch_one(pool)
        .await
}

pub async fn set_rules_channel(
    pool: &PgPool,
    guild_id: i64,
    casual: bool,
    channel_id: i64,
) -> Result<GuildConfig, sqlx::Error> {
    // Ensure config exists
    get_or_create(pool, guild_id).await?;

    let query = if casual {
        r#"
        UPDATE guild_configs
        SET rules_casual_channel_id = $2, updated_at = NOW()
        WHERE guild_id = $1
        RETURNING *
        "#
    } else {
        r#"
        UPDATE guild_configs
        SET rules_debate_channel_id = $2, updated_at = NOW()
        WHERE guild_id = $1
        RETURNING *
        "#
    };

    sqlx::query_as::<_, GuildConfig>(query)
        .bind(guild_id)
        .bind(channel_id)
        .fetch_one(pool)
        .await
}

/// Find which guild and type a JTC channel belongs to
pub async fn find_by_jtc_channel(
    pool: &PgPool,
    channel_id: i64,
) -> Result<Option<(GuildConfig, bool)>, sqlx::Error> {
    let config = sqlx::query_as::<_, GuildConfig>(
        r#"
        SELECT * FROM guild_configs
        WHERE jtc_casual_channel_id = $1 OR jtc_debate_channel_id = $1
        "#
    )
    .bind(channel_id)
    .fetch_optional(pool)
    .await?;

    Ok(config.map(|c| {
        let is_casual = c.jtc_casual_channel_id == Some(channel_id);
        (c, is_casual)
    }))
}
