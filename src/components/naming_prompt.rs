use std::sync::Arc;

use serenity::all::{
    ActionRowComponent, ButtonStyle, ChannelId, ComponentInteraction, Context, CreateActionRow,
    CreateButton, CreateInputText, CreateInteractionResponse, CreateInteractionResponseMessage,
    CreateMessage, CreateModal, InputTextStyle, ModalInteraction, UserId,
};
use tracing::{debug, error};

use crate::bot::data::Data;
use crate::bot::error::Error;
use crate::components::tag_selector;
use crate::constants::embeds::{self, BULLET};
use crate::constants::timeouts::VC_NAMING_DEADLINE_SECONDS;
use crate::db::queries::{user_vc_preference, voice_channel};
use crate::handlers::interaction::send_component_error;
use crate::services::jtc::channel_creator;
use crate::utils::profanity;

/// Send a naming prompt to the channel
pub async fn send_prompt(
    ctx: &Context,
    channel_id: ChannelId,
    owner_id: UserId,
) -> Result<(), Error> {
    let embed = embeds::secondary_embed()
        .title("Configure Your Channel")
        .description(format!(
            "Welcome <@{}>.\n\n\
            {} You have **{} seconds** to set up your channel\n\
            {} Click the button below to choose a name\n\
            {} Your preference will be saved for next time\n\n\
            If not configured, this channel will be deleted.",
            owner_id, BULLET, VC_NAMING_DEADLINE_SECONDS, BULLET, BULLET
        ));

    let button = CreateButton::new(format!("naming_configure_{}", channel_id))
        .label("Configure Channel")
        .style(ButtonStyle::Primary);

    let action_row = CreateActionRow::Buttons(vec![button]);
    let message = CreateMessage::new()
        .embed(embed)
        .components(vec![action_row]);

    if let Err(e) = channel_id.send_message(ctx, message).await {
        error!("Failed to send naming prompt to channel {}: {:?}", channel_id, e);
    }

    Ok(())
}

/// Handle the configure button click - show modal
pub async fn handle_configure_button(
    ctx: &Context,
    data: &Arc<Data>,
    component: &ComponentInteraction,
) -> Result<(), Error> {
    let custom_id = &component.data.custom_id;

    // Parse channel_id from custom_id: naming_configure_{channel_id}
    let channel_id: u64 = match custom_id.strip_prefix("naming_configure_").and_then(|s| s.parse().ok()) {
        Some(id) => id,
        None => {
            send_component_error(ctx, component, "Invalid button state").await?;
            return Ok(());
        }
    };

    // Verify the user is the channel owner
    let owner_id = match data.get_channel_owner(channel_id) {
        Some(id) => id,
        None => {
            send_component_error(ctx, component, "This channel is not managed by the bot").await?;
            return Ok(());
        }
    };

    if component.user.id.get() != owner_id {
        send_component_error(ctx, component, "Only the channel owner can configure the channel")
            .await?;
        return Ok(());
    }

    // Extend the deadline by another minute since user is actively configuring
    let guild_id = component.guild_id.unwrap_or_default();
    let new_deadline = chrono::Utc::now() + chrono::Duration::seconds(VC_NAMING_DEADLINE_SECONDS as i64);
    let _ = user_vc_preference::create_deadline(
        &data.pool,
        channel_id as i64,
        guild_id.get() as i64,
        owner_id as i64,
        new_deadline,
    ).await;

    debug!("Extended deadline for channel {} by {} seconds", channel_id, VC_NAMING_DEADLINE_SECONDS);

    // Show the naming modal
    let modal = CreateModal::new(
        format!("naming_modal_{}", channel_id),
        "Configure Your Channel",
    )
    .components(vec![CreateActionRow::InputText(
        CreateInputText::new(InputTextStyle::Short, "Channel Name", "naming_channel_name")
            .placeholder("Enter a name for your channel")
            .max_length(100)
            .required(true),
    )]);

    component
        .create_response(ctx, CreateInteractionResponse::Modal(modal))
        .await?;

    Ok(())
}

/// Handle the reconfigure button click - show modal for already configured channels
pub async fn handle_reconfigure_button(
    ctx: &Context,
    data: &Arc<Data>,
    component: &ComponentInteraction,
) -> Result<(), Error> {
    let custom_id = &component.data.custom_id;

    // Parse channel_id from custom_id: reconfigure_{channel_id}
    let channel_id: u64 = match custom_id.strip_prefix("reconfigure_").and_then(|s| s.parse().ok()) {
        Some(id) => id,
        None => {
            send_component_error(ctx, component, "Invalid button state").await?;
            return Ok(());
        }
    };

    // Verify the user is the channel owner
    let owner_id = match data.get_channel_owner(channel_id) {
        Some(id) => id,
        None => {
            send_component_error(ctx, component, "This channel is not managed by the bot").await?;
            return Ok(());
        }
    };

    if component.user.id.get() != owner_id {
        send_component_error(ctx, component, "Only the channel owner can reconfigure the channel")
            .await?;
        return Ok(());
    }

    // Get current channel name to pre-fill the modal
    let current_name = match crate::db::queries::voice_channel::get(&data.pool, channel_id as i64).await? {
        Some(vc) => vc.topic.unwrap_or_default(),
        None => {
            send_component_error(ctx, component, "Channel not found in database").await?;
            return Ok(());
        }
    };

    // Show the naming modal (same as configure, but for reconfigure)
    let modal = CreateModal::new(
        format!("naming_modal_{}", channel_id),
        "Reconfigure Your Channel",
    )
    .components(vec![CreateActionRow::InputText(
        CreateInputText::new(InputTextStyle::Short, "Channel Name", "naming_channel_name")
            .placeholder("Enter a name for your channel")
            .max_length(100)
            .value(current_name)
            .required(true),
    )]);

    component
        .create_response(ctx, CreateInteractionResponse::Modal(modal))
        .await?;

    Ok(())
}

/// Handle the naming modal submission
pub async fn handle_naming_modal(
    ctx: &Context,
    data: &Arc<Data>,
    modal: &ModalInteraction,
) -> Result<(), Error> {
    let custom_id = &modal.data.custom_id;

    // Parse channel_id from custom_id: naming_modal_{channel_id}
    let channel_id: u64 = match custom_id.strip_prefix("naming_modal_").and_then(|s| s.parse().ok()) {
        Some(id) => id,
        None => {
            return Ok(());
        }
    };

    // Extract the channel name from the modal
    let channel_name = modal
        .data
        .components
        .iter()
        .flat_map(|row| &row.components)
        .find_map(|comp| {
            if let ActionRowComponent::InputText(input) = comp {
                if input.custom_id == "naming_channel_name" {
                    return input.value.clone();
                }
            }
            None
        })
        .unwrap_or_default();

    if channel_name.is_empty() {
        let embed = embeds::error_embed()
            .title("Error")
            .description("Channel name cannot be empty.");

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
        return Ok(());
    }

    // Get guild_id
    let guild_id = match modal.guild_id {
        Some(id) => id,
        None => return Ok(()),
    };

    // Check for profanity
    if let Err(reason) = profanity::validate_channel_name(&channel_name) {
        // Extend deadline to give user another chance
        let new_deadline = chrono::Utc::now() + chrono::Duration::seconds(VC_NAMING_DEADLINE_SECONDS as i64);
        let _ = user_vc_preference::create_deadline(
            &data.pool,
            channel_id as i64,
            guild_id.get() as i64,
            modal.user.id.get() as i64,
            new_deadline,
        ).await;

        let embed = embeds::error_embed()
            .title("Inappropriate Name")
            .description(format!(
                "{}\n\n\
                You have been given an extra **{} seconds** to choose a different name.\n\
                Click the Configure button again to try a new name.",
                reason, VC_NAMING_DEADLINE_SECONDS
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
        return Ok(());
    }

    // Get channel info to determine type
    let vc = match voice_channel::get(&data.pool, channel_id as i64).await? {
        Some(vc) => vc,
        None => return Ok(()),
    };

    let channel_type_str = vc.channel_type.to_string();
    let is_casual = channel_type_str == "casual";

    // Update the channel name
    channel_creator::update_channel_topic(ctx, data, ChannelId::new(channel_id), &channel_name)
        .await?;

    // Save user preferences for next time (without tags for now - they'll be added via tag selector)
    user_vc_preference::upsert(
        &data.pool,
        guild_id.get() as i64,
        modal.user.id.get() as i64,
        &channel_type_str,
        Some(&channel_name),
        &[],
    )
    .await?;

    // Remove the deadline since user configured the channel
    user_vc_preference::remove_deadline(&data.pool, channel_id as i64).await?;

    debug!(
        "User {} configured channel {} with name '{}'",
        modal.user.id, channel_id, channel_name
    );

    // Acknowledge the modal with ephemeral message
    let embed = embeds::success_embed()
        .title("Name Set")
        .description(format!(
            "Your channel has been renamed to **{}**.",
            channel_name
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

    // Send tag selector to the channel
    let tag_message = tag_selector::create_selector(ChannelId::new(channel_id), is_casual);
    if let Err(e) = ChannelId::new(channel_id).send_message(ctx, tag_message).await {
        error!("Failed to send tag selector: {:?}", e);
    }

    Ok(())
}
