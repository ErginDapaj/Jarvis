use jarvis::{bot, config::Settings, db};
use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[tokio::main]
async fn main() {
    // Load .env file if present
    dotenvy::dotenv().ok();

    // Initialize tracing
    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env())
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting Jarvis Discord Bot");

    // Load settings
    let settings = match Settings::from_env() {
        Ok(s) => s,
        Err(e) => {
            error!("Failed to load settings: {}", e);
            std::process::exit(1);
        }
    };

    // Initialize database pool
    let pool = match db::pool::create_pool(&settings.database_url).await {
        Ok(p) => p,
        Err(e) => {
            error!("Failed to create database pool: {}", e);
            std::process::exit(1);
        }
    };

    // Run migrations
    if let Err(e) = db::pool::run_migrations(&pool).await {
        error!("Failed to run migrations: {}", e);
        std::process::exit(1);
    }

    info!("Database initialized successfully");

    // Start the bot
    if let Err(e) = bot::framework::run(settings, pool).await {
        error!("Bot error: {}", e);
        std::process::exit(1);
    }
}
