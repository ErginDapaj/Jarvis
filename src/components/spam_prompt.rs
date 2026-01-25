use std::sync::Arc;

use serenity::all::{
    ButtonStyle, ChannelId, ComponentInteraction, Context, CreateActionRow, CreateButton,
    CreateInteractionResponse, CreateInteractionResponseMessage, CreateMessage, UserId,
};
use tracing::debug;

use crate::bot::data::Data;
use crate::bot::error::Error;
use crate::constants::embeds;
use crate::handlers::interaction::send_component_error;
use crate::services::moderation::ban_service;

/// Send a spam prompt to the channel owner
pub async fn send_prompt(
    ctx: &Context,
    _data: &Arc<Data>,
    channel_id: ChannelId,
    _owner_id: UserId,
    suspicious_user_id: UserId,
) -> Result<(), Error> {
    let embed = embeds::warning_embed()
        .title("Spam Detection Alert")
        .description(format!(
            "<@{}> has been joining and leaving rapidly.\n\n\
            Would you like to ban them from this channel?",
            suspicious_user_id
        ));

    let buttons = CreateActionRow::Buttons(vec![
        CreateButton::new(format!("spam_ban_{}_{}", channel_id, suspicious_user_id))
            .label("Ban User")
            .style(ButtonStyle::Danger),
        CreateButton::new(format!("spam_ignore_{}_{}", channel_id, suspicious_user_id))
            .label("Ignore")
            .style(ButtonStyle::Secondary),
    ]);

    let message = CreateMessage::new().embed(embed).components(vec![buttons]);

    // Send to the voice channel's text chat
    channel_id.send_message(ctx, message).await?;

    Ok(())
}

/// Handle spam prompt response
pub async fn handle_response(
    ctx: &Context,
    data: &Arc<Data>,
    component: &ComponentInteraction,
) -> Result<(), Error> {
    let custom_id = &component.data.custom_id;

    // Parse custom_id: spam_{action}_{channel_id}_{user_id}
    let parts: Vec<&str> = custom_id.split('_').collect();
    if parts.len() < 4 {
        send_component_error(ctx, component, "Invalid button state").await?;
        return Ok(());
    }

    let action = parts[1];
    let channel_id: u64 = match parts[2].parse() {
        Ok(id) => id,
        Err(_) => {
            send_component_error(ctx, component, "Invalid channel ID").await?;
            return Ok(());
        }
    };
    let user_id: u64 = match parts[3].parse() {
        Ok(id) => id,
        Err(_) => {
            send_component_error(ctx, component, "Invalid user ID").await?;
            return Ok(());
        }
    };

    // Verify the responder is the channel owner
    let owner_id = match data.get_channel_owner(channel_id) {
        Some(id) => id,
        None => {
            send_component_error(ctx, component, "This channel is not managed by the bot").await?;
            return Ok(());
        }
    };

    if component.user.id.get() != owner_id {
        send_component_error(ctx, component, "Only the channel owner can respond to this")
            .await?;
        return Ok(());
    }

    let guild_id = match component.guild_id {
        Some(id) => id,
        None => {
            send_component_error(ctx, component, "This command only works in a server").await?;
            return Ok(());
        }
    };

    debug!(
        "Spam prompt response: action={}, channel={}, user={}",
        action, channel_id, user_id
    );

    let embed = match action {
        "ban" => {
            // Ban the user
            ban_service::ban_user(
                ctx,
                data,
                guild_id,
                ChannelId::new(channel_id),
                UserId::new(user_id),
                UserId::new(owner_id),
                Some("Spam detected"),
            )
            .await?;

            embeds::success_embed()
                .title("User Banned")
                .description(format!("<@{}> has been banned for spam.", user_id))
        }
        "ignore" => {
            embeds::standard_embed()
                .title("Ignored")
                .description("The spam alert has been dismissed.")
        }
        _ => {
            send_component_error(ctx, component, "Unknown action").await?;
            return Ok(());
        }
    };

    component
        .create_response(
            ctx,
            CreateInteractionResponse::UpdateMessage(
                CreateInteractionResponseMessage::new()
                    .embed(embed)
                    .components(vec![]),
            ),
        )
        .await?;

    Ok(())
}
