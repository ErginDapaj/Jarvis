use sqlx::{postgres::PgPoolOptions, PgPool};
use tracing::info;

pub async fn create_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
    info!("Connecting to database...");

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(database_url)
        .await?;

    info!("Database connection established");

    Ok(pool)
}

pub async fn run_migrations(pool: &PgPool) -> Result<(), sqlx::Error> {
    info!("Running database migrations...");

    // Read and execute migrations in order
    let migrations = [
        include_str!("../../migrations/001_initial_schema.sql"),
        include_str!("../../migrations/002_guild_configs.sql"),
        include_str!("../../migrations/003_channels.sql"),
        include_str!("../../migrations/004_mute_history.sql"),
        include_str!("../../migrations/005_ban_history.sql"),
        include_str!("../../migrations/006_spam_tracking.sql"),
        include_str!("../../migrations/007_user_vc_preferences.sql"),
        include_str!("../../migrations/008_rate_limits.sql"),
        include_str!("../../migrations/009_global_mutes.sql"),
    ];

    for (i, migration) in migrations.iter().enumerate() {
        info!("Running migration {}", i + 1);
        // Split migration by semicolons and execute each statement
        for statement in migration.split(';') {
            let statement = statement.trim();
            if !statement.is_empty() {
                // Skip if the object already exists (idempotent migrations)
                if let Err(e) = sqlx::query(statement).execute(pool).await {
                    // Ignore "already exists" errors
                    let err_str = e.to_string();
                    if !err_str.contains("already exists")
                        && !err_str.contains("duplicate key")
                    {
                        return Err(e);
                    }
                }
            }
        }
    }

    info!("Migrations completed successfully");
    Ok(())
}
