use std::sync::Arc;

use serenity::all::{
    ActionRowComponent, Context, CreateActionRow, CreateInputText, CreateInteractionResponse,
    CreateInteractionResponseMessage, CreateModal, InputTextStyle, ModalInteraction,
};
use tracing::debug;

use crate::bot::data::Data;
use crate::bot::error::Error;
use crate::constants::embeds;
use crate::handlers::interaction::send_modal_error;

/// Create a topic input modal
pub fn create_modal(is_casual: bool, channel_id: u64) -> CreateModal {
    let channel_type = if is_casual { "Casual" } else { "Debate" };

    CreateModal::new(
        format!("topic_{}_{}", if is_casual { "casual" } else { "debate" }, channel_id),
        format!("{} Voice Channel", channel_type),
    )
    .components(vec![CreateActionRow::InputText(
        CreateInputText::new(InputTextStyle::Short, "Topic", "topic")
            .placeholder("Enter a topic for your channel (optional)")
            .required(false)
            .max_length(100),
    )])
}

/// Handle topic modal submission
pub async fn handle_submission(
    ctx: &Context,
    _data: &Arc<Data>,
    modal: &ModalInteraction,
) -> Result<(), Error> {
    let custom_id = &modal.data.custom_id;

    // Parse custom_id: topic_{type}_{channel_id}
    let parts: Vec<&str> = custom_id.split('_').collect();
    if parts.len() < 3 {
        send_modal_error(ctx, modal, "Invalid modal state").await?;
        return Ok(());
    }

    let is_casual = parts[1] == "casual";
    let _channel_id: u64 = match parts[2].parse() {
        Ok(id) => id,
        Err(_) => {
            send_modal_error(ctx, modal, "Invalid channel ID").await?;
            return Ok(());
        }
    };

    // Extract topic from modal
    let topic = modal
        .data
        .components
        .iter()
        .flat_map(|row| row.components.iter())
        .find_map(|component| {
            if let ActionRowComponent::InputText(input) = component {
                if input.custom_id == "topic" {
                    if let Some(ref val) = input.value {
                        if !val.is_empty() {
                            return Some(val.clone());
                        }
                    }
                }
            }
            None
        });

    debug!(
        "Topic modal submitted: is_casual={}, topic={:?}",
        is_casual, topic
    );

    // Send acknowledgment
    let embed = embeds::success_embed()
        .title("Channel Created")
        .description(format!(
            "Your {} voice channel has been created!",
            if is_casual { "casual" } else { "debate" }
        ));

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
