use std::sync::Arc;

use serenity::all::{
    ChannelId, ComponentInteraction, ComponentInteractionDataKind, Context, CreateActionRow,
    CreateInteractionResponse, CreateInteractionResponseMessage, CreateMessage, CreateSelectMenu,
    CreateSelectMenuKind, CreateSelectMenuOption, EditMessage,
};
use tracing::{debug, warn};

use crate::bot::data::Data;
use crate::bot::error::Error;
use crate::constants::embeds;
use crate::constants::tags::{get_tags, MAX_TAGS};
use crate::db::queries::{user_vc_preference, voice_channel};
use crate::handlers::interaction::send_component_error;
use crate::services::jtc::channel_creator;

/// Create a tag selector message
pub fn create_selector(channel_id: ChannelId, is_casual: bool) -> CreateMessage {
    let tags = get_tags(is_casual);
    let channel_type = if is_casual { "casual" } else { "debate" };

    let options: Vec<CreateSelectMenuOption> = tags
        .iter()
        .map(|tag| CreateSelectMenuOption::new(*tag, *tag))
        .collect();

    let select_menu = CreateSelectMenu::new(
        format!("tags_{}_{}", channel_type, channel_id),
        CreateSelectMenuKind::String { options },
    )
    .placeholder(format!("Select up to {} tags", MAX_TAGS))
    .min_values(0)
    .max_values(MAX_TAGS as u8);

    let embed = embeds::standard_embed()
        .title("Select Channel Tags")
        .description(format!(
            "Choose up to {} tags for your channel to help others find it.",
            MAX_TAGS
        ));

    CreateMessage::new()
        .embed(embed)
        .components(vec![CreateActionRow::SelectMenu(select_menu)])
}

/// Handle tag selection
pub async fn handle_selection(
    ctx: &Context,
    data: &Arc<Data>,
    component: &ComponentInteraction,
) -> Result<(), Error> {
    let custom_id = &component.data.custom_id;

    // Parse custom_id: tags_{type}_{channel_id}
    let parts: Vec<&str> = custom_id.split('_').collect();
    if parts.len() < 3 {
        send_component_error(ctx, component, "Invalid selector state").await?;
        return Ok(());
    }

    let _is_casual = parts[1] == "casual";
    let channel_id: u64 = match parts[2].parse() {
        Ok(id) => id,
        Err(_) => {
            send_component_error(ctx, component, "Invalid channel ID").await?;
            return Ok(());
        }
    };

    // Extract selected tags
    let selected_tags: Vec<String> = match &component.data.kind {
        ComponentInteractionDataKind::StringSelect { values } => {
            values.iter().take(MAX_TAGS).cloned().collect()
        }
        _ => vec![],
    };

    debug!(
        "Tags selected for channel {}: {:?}",
        channel_id, selected_tags
    );

    // Update channel tags
    channel_creator::update_channel_tags(ctx, data, ChannelId::new(channel_id), selected_tags.clone())
        .await?;

    // Save to user preferences for next time
    if let Some(guild_id) = component.guild_id {
        if let Ok(Some(vc)) = voice_channel::get(&data.pool, channel_id as i64).await {
            let channel_type = vc.channel_type.to_string();
            // Get existing preference to preserve the name
            if let Ok(Some(pref)) = user_vc_preference::get(
                &data.pool,
                guild_id.get() as i64,
                component.user.id.get() as i64,
                &channel_type,
            ).await {
                let _ = user_vc_preference::upsert(
                    &data.pool,
                    guild_id.get() as i64,
                    component.user.id.get() as i64,
                    &channel_type,
                    pref.preferred_name.as_deref(),
                    &selected_tags,
                ).await;
            }
        }
    }

    // Acknowledge with ephemeral message
    let embed = embeds::success_embed()
        .title("Tags Updated")
        .description(if selected_tags.is_empty() {
            "No tags selected.".to_string()
        } else {
            format!(
                "Tags set: {}",
                selected_tags
                    .iter()
                    .map(|t| format!("`{}`", t))
                    .collect::<Vec<_>>()
                    .join(" ")
            )
        });

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

    // Delete the original tag selector message
    if let Err(e) = component.message.clone().edit(ctx, EditMessage::new().components(vec![])).await {
        warn!("Failed to clear tag selector components: {:?}", e);
    }

    // Try to delete the message entirely
    if let Err(e) = component.message.delete(ctx).await {
        debug!("Could not delete tag selector message: {:?}", e);
    }

    Ok(())
}
