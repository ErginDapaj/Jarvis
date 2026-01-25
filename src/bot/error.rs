use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Discord API error: {0}")]
    Serenity(#[from] serenity::Error),

    #[error("Configuration not found: {0}")]
    ConfigNotFound(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Channel not found: {0}")]
    ChannelNotFound(u64),

    #[error("User not found: {0}")]
    UserNotFound(u64),

    #[error("Invalid operation: {0}")]
    InvalidOperation(String),

    #[error("JTC not configured for this guild")]
    JtcNotConfigured,

    #[error("{0}")]
    Custom(String),
}

impl Error {
    pub fn custom<S: Into<String>>(msg: S) -> Self {
        Error::Custom(msg.into())
    }
}
