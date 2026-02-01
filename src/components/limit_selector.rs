use std::sync::Arc;
use std::time::{Duration, Instant};

use serenity::all::{
    ActionRowComponent, ChannelId, ComponentInteraction, Context, CreateActionRow,
    CreateInputText, CreateInteractionResponse, CreateInteractionResponseMessage, CreateModal,
    EditChannel, InputTextStyle, ModalInteraction,
};
use tracing::{debug, error};

use crate::bot::data::Data;
use crate::bot::error::Error;
use crate::constants::embeds;
use crate::handlers::interaction::{send_component_error, send_modal_error};

const MAX_USER_LIMIT: u32 = 69;
const LIMIT_RATE_WINDOW: Duration = Duration::from_secs(60 * 60); // 1 hour
const LIMIT_RATE_MAX_USES: usize = 3;

/// Handle the "Set Limit" button — open a modal for the owner to type a number
pub async fn handle_button(
    ctx: &Context,
    data: &Arc<Data>,
    component: &ComponentInteraction,
) -> Result<(), Error> {
    let custom_id = &component.data.custom_id;

    // Parse custom_id: limit_btn_{channel_id}
    let parts: Vec<&str> = custom_id.split('_').collect();
    if parts.len() < 3 {
        send_component_error(ctx, component, "Invalid button state").await?;
        return Ok(());
    }

    let channel_id: u64 = match parts[2].parse() {
        Ok(id) => id,
        Err(_) => {
            send_component_error(ctx, component, "Invalid channel ID").await?;
            return Ok(());
        }
    };

    // Verify the user is the channel owner
    let owner_id = match data.get_channel_owner(channel_id) {
        Some(id) => id,
        None => {
            send_component_error(ctx, component, "This channel is not managed by the bot")
                .await?;
            return Ok(());
        }
    };

    if component.user.id.get() != owner_id {
        send_component_error(
            ctx,
            component,
            "You don't own this channel. Setting limits on other people's rooms is not a personality trait.",
        )
        .await?;
        return Ok(());
    }

    // Open a modal with a text input
    let modal = CreateModal::new(
        format!("limit_modal_{}", channel_id),
        "Set User Limit",
    )
    .components(vec![CreateActionRow::InputText(
        CreateInputText::new(InputTextStyle::Short, "User Limit", "limit_value")
            .placeholder("Enter a number (1-69) or 0 for unlimited")
            .required(true)
            .min_length(1)
            .max_length(2),
    )]);

    component
        .create_response(ctx, CreateInteractionResponse::Modal(modal))
        .await?;

    Ok(())
}

/// Handle the limit modal submission — validate and apply the user limit
pub async fn handle_modal(
    ctx: &Context,
    data: &Arc<Data>,
    modal: &ModalInteraction,
) -> Result<(), Error> {
    let custom_id = &modal.data.custom_id;

    // Parse custom_id: limit_modal_{channel_id}
    let parts: Vec<&str> = custom_id.split('_').collect();
    if parts.len() < 3 {
        send_modal_error(ctx, modal, "Invalid modal state").await?;
        return Ok(());
    }

    let channel_id: u64 = match parts[2].parse() {
        Ok(id) => id,
        Err(_) => {
            send_modal_error(ctx, modal, "Invalid channel ID").await?;
            return Ok(());
        }
    };

    // Verify the user is the channel owner
    let owner_id = match data.get_channel_owner(channel_id) {
        Some(id) => id,
        None => {
            send_modal_error(ctx, modal, "This channel is not managed by the bot").await?;
            return Ok(());
        }
    };

    if modal.user.id.get() != owner_id {
        send_modal_error(
            ctx,
            modal,
            "You don't own this channel. Setting limits on other people's rooms is not a personality trait.",
        )
        .await?;
        return Ok(());
    }

    // Extract the value from the modal
    let raw_value = modal
        .data
        .components
        .iter()
        .flat_map(|row| row.components.iter())
        .find_map(|component| {
            if let ActionRowComponent::InputText(input) = component {
                if input.custom_id == "limit_value" {
                    return input.value.clone();
                }
            }
            None
        })
        .unwrap_or_default();

    let trimmed = raw_value.trim();

    // Parse and validate
    let limit: u32 = match trimmed.parse::<u32>() {
        Ok(n) if n <= MAX_USER_LIMIT => n,
        Ok(_) => {
            send_modal_error(
                ctx,
                modal,
                &format!(
                    "The limit must be between 0 and {}. You typed \"{}\". Nice try.",
                    MAX_USER_LIMIT, trimmed
                ),
            )
            .await?;
            return Ok(());
        }
        Err(_) => {
            send_modal_error(
                ctx,
                modal,
                &format!(
                    "\"{}\" is not a number. Were you expecting me to guess what you meant?",
                    trimmed
                ),
            )
            .await?;
            return Ok(());
        }
    };

    // Rate limit: max 3 changes per hour
    let key = (modal.user.id.get(), channel_id);
    let now = Instant::now();
    {
        let mut entry = data.limit_change_timestamps.entry(key).or_insert_with(Vec::new);
        // Prune old timestamps outside the window
        entry.retain(|ts| now.duration_since(*ts) < LIMIT_RATE_WINDOW);

        if entry.len() >= LIMIT_RATE_MAX_USES {
            let oldest = entry[0];
            let reset_in = LIMIT_RATE_WINDOW - now.duration_since(oldest);
            let mins = reset_in.as_secs() / 60;
            send_modal_error(
                ctx,
                modal,
                &format!(
                    "You've already changed the user limit 3 times this hour. Try again in {} minute{}.",
                    mins + 1,
                    if mins == 0 { "" } else { "s" }
                ),
            )
            .await?;
            return Ok(());
        }

        entry.push(now);
    }

    debug!("Setting user limit for channel {} to {}", channel_id, limit);

    // Apply the user limit to the Discord channel (0 = unlimited)
    let channel_id_obj = ChannelId::new(channel_id);
    if let Err(e) = channel_id_obj
        .edit(ctx, EditChannel::new().user_limit(limit))
        .await
    {
        error!(
            "Failed to set user limit on channel {}: {:?}",
            channel_id, e
        );
        send_modal_error(ctx, modal, &format!("Failed to set user limit: {}", e)).await?;
        return Ok(());
    }

    let description = if limit == 0 {
        "User limit removed — your channel is now unlimited.".to_string()
    } else {
        format!("User limit set to **{}**.", limit)
    };

    let embed = embeds::success_embed()
        .title("User Limit Updated")
        .description(description);

    modal
        .create_response(
            ctx,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .embed(embed)
                    .ephemeral(true),
            ),
        )
        .await?;

    Ok(())
}
