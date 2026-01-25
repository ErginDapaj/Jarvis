use std::sync::Arc;

use serenity::all::{
    ComponentInteraction, Context, CreateInteractionResponse, CreateInteractionResponseMessage,
    Interaction, ModalInteraction,
};
use tracing::{debug, error};

use crate::bot::data::Data;
use crate::bot::error::Error;
use crate::components::{ban_selector, naming_prompt, spam_prompt, tag_selector, topic_modal};
use crate::constants::embeds;

pub async fn handle_interaction(
    ctx: &Context,
    data: &Arc<Data>,
    interaction: &Interaction,
) -> Result<(), Error> {
    match interaction {
        Interaction::Component(component) => {
            handle_component(ctx, data, component).await?;
        }
        Interaction::Modal(modal) => {
            handle_modal(ctx, data, modal).await?;
        }
        Interaction::Command(_) => {
            // Slash commands are handled by poise framework, not here
            // This should not be reached if poise is working correctly
            debug!("Received ApplicationCommand interaction - should be handled by poise");
        }
        _ => {
            debug!("Unhandled interaction type: {:?}", interaction.kind());
        }
    }

    Ok(())
}

async fn handle_component(
    ctx: &Context,
    data: &Arc<Data>,
    component: &ComponentInteraction,
) -> Result<(), Error> {
    let custom_id = &component.data.custom_id;
    debug!("Component interaction: {}", custom_id);

    // Route based on custom_id prefix
    let result = if custom_id.starts_with("tags_") {
        tag_selector::handle_selection(ctx, data, component).await
    } else if custom_id.starts_with("ban_") {
        ban_selector::handle_selection(ctx, data, component).await
    } else if custom_id.starts_with("spam_") {
        spam_prompt::handle_response(ctx, data, component).await
    } else if custom_id.starts_with("naming_") {
        naming_prompt::handle_configure_button(ctx, data, component).await
    } else if custom_id.starts_with("reconfigure_") {
        naming_prompt::handle_reconfigure_button(ctx, data, component).await
    } else {
        // Unknown component - acknowledge but do nothing
        debug!("Unknown component interaction: {}", custom_id);
        Ok(())
    };

    // If handler failed, send error response
    if let Err(e) = result {
        error!("Component interaction error for {}: {:?}", custom_id, e);
        // Try to send error response, but don't fail if it doesn't work
        let _ = send_component_error(ctx, component, &format!("An error occurred: {}", e)).await;
    }

    Ok(())
}

async fn handle_modal(
    ctx: &Context,
    data: &Arc<Data>,
    modal: &ModalInteraction,
) -> Result<(), Error> {
    let custom_id = &modal.data.custom_id;
    debug!("Modal submission: {}", custom_id);

    if custom_id.starts_with("topic_") {
        topic_modal::handle_submission(ctx, data, modal).await?;
    } else if custom_id.starts_with("naming_modal_") {
        naming_prompt::handle_naming_modal(ctx, data, modal).await?;
    }

    Ok(())
}

/// Send an ephemeral error message for a component interaction
pub async fn send_component_error(
    ctx: &Context,
    component: &ComponentInteraction,
    message: &str,
) -> Result<(), Error> {
    let embed = embeds::error_embed()
        .title("Error")
        .description(message);

    component
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

/// Send an ephemeral error message for a modal interaction
pub async fn send_modal_error(
    ctx: &Context,
    modal: &ModalInteraction,
    message: &str,
) -> Result<(), Error> {
    let embed = embeds::error_embed()
        .title("Error")
        .description(message);

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
