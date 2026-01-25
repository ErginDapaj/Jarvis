use std::sync::Arc;

use poise::serenity_prelude::{self as serenity, FullEvent};
use tracing::{debug, error, info};

use crate::bot::data::Data;
use crate::bot::error::Error;
use crate::handlers::{interaction, voice_state};

pub async fn event_handler(
    ctx: &serenity::Context,
    event: &FullEvent,
    _framework: poise::FrameworkContext<'_, Arc<Data>, Error>,
    data: &Arc<Data>,
) -> Result<(), Error> {
    match event {
        FullEvent::Ready { data_about_bot, .. } => {
            info!("Bot ready as {}", data_about_bot.user.name);
        }

        FullEvent::VoiceStateUpdate { old, new } => {
            debug!("Voice state update: {:?} -> {:?}", old, new);
            if let Err(e) = voice_state::handle_voice_state_update(ctx, data, old.as_ref(), new).await {
                error!("Voice state handler error: {:?}", e);
            }
        }

        FullEvent::InteractionCreate { interaction } => {
            // Poise handles ApplicationCommand (slash commands) automatically
            // We only handle Component and Modal interactions here for custom components
            match interaction {
                serenity::Interaction::Component(_) | serenity::Interaction::Modal(_) => {
                    if let Err(e) = interaction::handle_interaction(ctx, data, interaction).await {
                        error!("Component/Modal interaction handler error: {:?}", e);
                    }
                }
                _ => {
                    // ApplicationCommand interactions are handled by poise framework
                    // This should not be reached if poise is working correctly
                }
            }
        }

        FullEvent::ChannelDelete { channel, .. } => {
            // Clean up cache when a channel is deleted
            data.remove_channel(channel.id.get());
            debug!("Channel {} deleted, removed from cache", channel.id);
        }

        FullEvent::GuildDelete { incomplete, .. } => {
            // Could clean up guild data here if needed
            debug!("Guild {} removed", incomplete.id);
        }

        _ => {}
    }

    Ok(())
}
