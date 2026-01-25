use chrono::{DateTime, Utc};
use sqlx::PgPool;

/// Command type for rate limiting
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandType {
    Rename,
    Retag,
}

impl CommandType {
    pub fn as_str(&self) -> &'static str {
        match self {
            CommandType::Rename => "rename",
            CommandType::Retag => "retag",
        }
    }
}

/// Get the last time a user used a command
pub async fn get_last_used(
    pool: &PgPool,
    user_id: i64,
    guild_id: i64,
    command_type: CommandType,
) -> Result<Option<DateTime<Utc>>, sqlx::Error> {
    let result: Option<(DateTime<Utc>,)> = sqlx::query_as(
        "SELECT last_used_at FROM channel_command_rate_limits WHERE user_id = $1 AND guild_id = $2 AND command_type = $3"
    )
    .bind(user_id)
    .bind(guild_id)
    .bind(command_type.as_str())
    .fetch_optional(pool)
    .await?;

    Ok(result.map(|r| r.0))
}

/// Update the last used timestamp for a command
pub async fn update_last_used(
    pool: &PgPool,
    user_id: i64,
    guild_id: i64,
    command_type: CommandType,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO channel_command_rate_limits (user_id, guild_id, command_type, last_used_at)
        VALUES ($1, $2, $3, NOW())
        ON CONFLICT (user_id, guild_id, command_type)
        DO UPDATE SET last_used_at = NOW()
        "#
    )
    .bind(user_id)
    .bind(guild_id)
    .bind(command_type.as_str())
    .execute(pool)
    .await?;

    Ok(())
}
