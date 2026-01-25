use std::sync::Arc;
use std::time::Duration;

use serenity::all::{ChannelId, Context, GuildId, UserId};
use tokio::sync::mpsc;
use tracing::{error, info, warn};

use crate::bot::data::Data;
use crate::bot::error::Error;
use crate::services::jtc::channel_creator;

/// Queue entry for pending JTC channel creation
#[derive(Debug, Clone)]
pub struct JtcQueueEntry {
    guild_id: GuildId,
    user_id: UserId,
    jtc_channel_id: ChannelId,
    is_casual: bool,
}

/// Check all JTC channels for users and add them to the queue
pub async fn check_jtc_channels_on_startup(
    ctx: &Context,
    data: &Arc<Data>,
    queue_tx: &mpsc::UnboundedSender<JtcQueueEntry>,
) -> Result<usize, Error> {
    // Get all guild configs
    let configs = sqlx::query_as::<_, crate::db::models::GuildConfig>(
        "SELECT * FROM guild_configs WHERE jtc_casual_channel_id IS NOT NULL OR jtc_debate_channel_id IS NOT NULL"
    )
    .fetch_all(&data.pool)
    .await?;

    let mut queued = 0;

    for config in configs {
        let guild_id = GuildId::new(config.guild_id as u64);

        // Check casual JTC channel
        if let Some(jtc_channel_id) = config.jtc_casual_channel_id {
            let channel_id = ChannelId::new(jtc_channel_id as u64);
            let users = get_users_in_channel(ctx, guild_id, channel_id).await?;
            
            for user_id in users {
                // Check if user is a bot
                if let Some(guild) = ctx.cache.guild(guild_id) {
                    if let Some(member) = guild.members.get(&user_id) {
                        if member.user.bot {
                            continue;
                        }
                    }
                }

                let entry = JtcQueueEntry {
                    guild_id,
                    user_id,
                    jtc_channel_id: channel_id,
                    is_casual: true,
                };
                
                if queue_tx.send(entry).is_ok() {
                    queued += 1;
                    info!("Queued JTC channel creation for user {} in casual channel {}", user_id, channel_id);
                }
            }
        }

        // Check debate JTC channel
        if let Some(jtc_channel_id) = config.jtc_debate_channel_id {
            let channel_id = ChannelId::new(jtc_channel_id as u64);
            let users = get_users_in_channel(ctx, guild_id, channel_id).await?;
            
            for user_id in users {
                // Check if user is a bot
                if let Some(guild) = ctx.cache.guild(guild_id) {
                    if let Some(member) = guild.members.get(&user_id) {
                        if member.user.bot {
                            continue;
                        }
                    }
                }

                let entry = JtcQueueEntry {
                    guild_id,
                    user_id,
                    jtc_channel_id: channel_id,
                    is_casual: false,
                };
                
                if queue_tx.send(entry).is_ok() {
                    queued += 1;
                    info!("Queued JTC channel creation for user {} in debate channel {}", user_id, channel_id);
                }
            }
        }
    }

    Ok(queued)
}

/// Get all users currently in a voice channel
async fn get_users_in_channel(
    ctx: &Context,
    guild_id: GuildId,
    channel_id: ChannelId,
) -> Result<Vec<UserId>, Error> {
    let mut users = Vec::new();

    // Get guild from cache
    if let Some(guild) = ctx.cache.guild(guild_id) {
        for voice_state in guild.voice_states.values() {
            if voice_state.channel_id == Some(channel_id) {
                users.push(voice_state.user_id);
            }
        }
    }

    Ok(users)
}

/// Process queue with Context
pub async fn process_jtc_queue_with_context(
    ctx: Context,
    data: Arc<Data>,
    mut queue_rx: mpsc::UnboundedReceiver<JtcQueueEntry>,
) {
    info!("Started JTC queue processor");

    // Rate limit: wait 1 second between channel creations to avoid Discord rate limits
    const RATE_LIMIT_DELAY: Duration = Duration::from_secs(1);

    while let Some(entry) = queue_rx.recv().await {
        info!(
            "Processing JTC queue entry: user={}, guild={}, channel={}, casual={}",
            entry.user_id, entry.guild_id, entry.jtc_channel_id, entry.is_casual
        );

        // Check if user is still in the JTC channel
        let still_in_channel = {
            if let Some(guild) = ctx.cache.guild(entry.guild_id) {
                guild
                    .voice_states
                    .get(&entry.user_id)
                    .and_then(|vs| vs.channel_id)
                    == Some(entry.jtc_channel_id)
            } else {
                false
            }
        };

        if !still_in_channel {
            warn!(
                "User {} is no longer in JTC channel {}, skipping",
                entry.user_id, entry.jtc_channel_id
            );
            continue;
        }

        // Create the channel
        match channel_creator::start_jtc_flow(
            &ctx,
            &data,
            entry.guild_id,
            entry.user_id,
            entry.jtc_channel_id,
            entry.is_casual,
        )
        .await
        {
            Ok(_) => {
                info!(
                    "Successfully created channel for user {} from queue",
                    entry.user_id
                );
            }
            Err(e) => {
                error!(
                    "Failed to create channel for user {} from queue: {:?}",
                    entry.user_id, e
                );
            }
        }

        // Rate limit: wait before processing next entry
        tokio::time::sleep(RATE_LIMIT_DELAY).await;
    }

    warn!("JTC queue processor stopped");
}

/// Spawn the JTC queue processor
pub fn spawn_queue_processor(
    ctx: Context,
    data: Arc<Data>,
    queue_rx: mpsc::UnboundedReceiver<JtcQueueEntry>,
) {
    tokio::spawn(async move {
        process_jtc_queue_with_context(ctx, data, queue_rx).await;
    });
}
