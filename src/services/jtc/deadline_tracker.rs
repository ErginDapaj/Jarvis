use std::sync::Arc;
use std::time::Duration;

use serenity::all::{ChannelId, CreateMessage, Http, UserId};
use tokio::time::interval;
use tracing::{debug, error, info, warn};

use crate::bot::data::Data;
use crate::constants::embeds;
use crate::db::queries::{user_vc_preference, voice_channel};

/// Interval for checking expired deadlines (in seconds)
const CHECK_INTERVAL_SECONDS: u64 = 10;

/// Start the deadline checker background task
pub fn spawn_deadline_checker(http: Arc<Http>, data: Arc<Data>) {
    tokio::spawn(async move {
        let mut ticker = interval(Duration::from_secs(CHECK_INTERVAL_SECONDS));

        loop {
            ticker.tick().await;

            if let Err(e) = check_expired_deadlines(&http, &data).await {
                error!("Error checking expired deadlines: {:?}", e);
            }
        }
    });
}

/// Check for expired deadlines and delete unconfigured channels
async fn check_expired_deadlines(
    http: &Http,
    data: &Arc<Data>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let expired = user_vc_preference::get_expired_deadlines(&data.pool).await?;

    for deadline in expired {
        let channel_id = ChannelId::new(deadline.channel_id as u64);

        debug!(
            "Deadline expired for channel {} owned by {}",
            channel_id, deadline.owner_id
        );

        // Check if channel still exists and is still unconfigured
        // (user might have already configured it or left)
        if let Some(vc) = voice_channel::get(&data.pool, deadline.channel_id).await? {
            // Check if it still has the default name (unconfigured)
            if vc.topic.is_none() {
                let owner_id = UserId::new(deadline.owner_id as u64);

                // Delete the channel
                match channel_id.delete(http).await {
                    Ok(_) => {
                        info!(
                            "Deleted unconfigured channel {} (deadline expired)",
                            channel_id
                        );

                        // Try to DM the user explaining why
                        notify_user_channel_deleted(http, owner_id).await;
                    }
                    Err(e) => {
                        warn!("Failed to delete channel {}: {:?}", channel_id, e);
                    }
                }

                // Clean up database
                voice_channel::delete(&data.pool, deadline.channel_id).await?;
                data.remove_channel(deadline.channel_id as u64);
            }
        }

        // Remove the deadline entry
        user_vc_preference::remove_deadline(&data.pool, deadline.channel_id).await?;
    }

    Ok(())
}

/// Notify user via DM that their channel was deleted due to timeout
async fn notify_user_channel_deleted(http: &Http, user_id: UserId) {
    let embed = embeds::info_embed()
        .title("Voice Channel Deleted")
        .description(
            "Your voice channel was automatically deleted because it wasn't configured in time.\n\n\
            When you create a new channel, you have 60 seconds to set a name. \
            If you don't configure it, the channel is removed to keep things tidy.\n\n\
            Simply join the channel again to create a new one."
        );

    let message = CreateMessage::new().embed(embed);

    // Try to DM the user - fail silently if DMs are closed
    match user_id.create_dm_channel(http).await {
        Ok(dm_channel) => {
            if let Err(e) = dm_channel.send_message(http, message).await {
                debug!("Could not DM user {} about channel deletion: {:?}", user_id, e);
            }
        }
        Err(e) => {
            debug!("Could not create DM channel for user {}: {:?}", user_id, e);
        }
    }
}
