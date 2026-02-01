use std::sync::Arc;

use serenity::all::{
    ChannelId, ComponentInteraction, ComponentInteractionDataKind, Context, CreateActionRow,
    CreateInteractionResponse, CreateInteractionResponseMessage, CreateMessage, CreateSelectMenu,
    CreateSelectMenuKind, CreateSelectMenuOption, GuildId, UserId,
};
use tracing::{debug, error};

use crate::bot::data::Data;
use crate::bot::error::Error;
use crate::constants::embeds;
use crate::handlers::interaction::send_component_error;
use crate::services::moderation::ban_service;

/// Create a ban selector message for the channel owner
pub async fn create_selector(
    ctx: &Context,
    guild_id: GuildId,
    channel_id: ChannelId,
    owner_id: UserId,
) -> Result<CreateMessage, Error> {
    // Get members in the voice channel
    let members = get_channel_members(ctx, guild_id, channel_id, owner_id).await;

    if members.is_empty() {
        let embed = embeds::error_embed()
            .title("No Users to Ban")
            .description("There are no other users in the channel to ban.");

        return Ok(CreateMessage::new().embed(embed));
    }

    let options: Vec<CreateSelectMenuOption> = members
        .into_iter()
        .map(|(user_id, username)| {
            CreateSelectMenuOption::new(&username, user_id.to_string())
                .description(format!("Ban {}", username))
        })
        .collect();

    let select_menu = CreateSelectMenu::new(
        format!("ban_select_{}", channel_id),
        CreateSelectMenuKind::String { options },
    )
    .placeholder("Select a user to ban")
    .min_values(1)
    .max_values(1);

    let embed = embeds::warning_embed()
        .title("Ban User from Channel")
        .description(
            "Select a user to ban from this voice channel.\n\
            Banned users cannot rejoin this channel.",
        );

    Ok(CreateMessage::new()
        .embed(embed)
        .components(vec![CreateActionRow::SelectMenu(select_menu)]))
}

/// Handle ban selection
pub async fn handle_selection(
    ctx: &Context,
    data: &Arc<Data>,
    component: &ComponentInteraction,
) -> Result<(), Error> {
    let custom_id = &component.data.custom_id;

    // Parse custom_id: ban_select_{channel_id}
    let parts: Vec<&str> = custom_id.split('_').collect();
    if parts.len() < 3 {
        send_component_error(ctx, component, "Invalid selector state").await?;
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
            send_component_error(ctx, component, "This channel is not managed by the bot").await?;
            return Ok(());
        }
    };

    if component.user.id.get() != owner_id {
        send_component_error(ctx, component, "This isn't your channel, buddy. Trying to ban people from rooms you don't own is a bold strategy — too bad it doesn't work.").await?;
        return Ok(());
    }

    // Get selected user
    let selected_user_id: u64 = match &component.data.kind {
        ComponentInteractionDataKind::StringSelect { values } => {
            match values.first().and_then(|v| v.parse().ok()) {
                Some(id) => id,
                None => {
                    send_component_error(ctx, component, "Invalid user selection").await?;
                    return Ok(());
                }
            }
        }
        _ => {
            send_component_error(ctx, component, "Unexpected interaction type").await?;
            return Ok(());
        }
    };

    let guild_id = match component.guild_id {
        Some(id) => id,
        None => {
            send_component_error(ctx, component, "This command only works in a server").await?;
            return Ok(());
        }
    };

    debug!(
        "Ban selected: owner {} banning user {} from channel {}",
        owner_id, selected_user_id, channel_id
    );

    // Perform the ban
    let ban_result = ban_service::ban_user(
        ctx,
        data,
        guild_id,
        ChannelId::new(channel_id),
        UserId::new(selected_user_id),
        UserId::new(owner_id),
        None,
    )
    .await;

    // Create response based on result
    let embed = match &ban_result {
        Ok(_) => {
            debug!("Successfully banned user {} from channel {}", selected_user_id, channel_id);
            embeds::success_embed()
                .title("User Banned")
                .description(format!(
                    "<@{}> has been banned from this voice channel.",
                    selected_user_id
                ))
        }
        Err(e) => {
            error!("Ban failed for user {} from channel {}: {:?}", selected_user_id, channel_id, e);
            embeds::error_embed()
                .title("Ban Failed")
                .description(format!("Failed to ban user: {}", e))
        }
    };

    // Acknowledge and update message
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

/// Get members in a voice channel (excluding the owner)
async fn get_channel_members(
    ctx: &Context,
    guild_id: GuildId,
    channel_id: ChannelId,
    owner_id: UserId,
) -> Vec<(u64, String)> {
    let mut members = Vec::new();

    if let Some(guild) = ctx.cache.guild(guild_id) {
        for vs in guild.voice_states.values() {
            if vs.channel_id == Some(channel_id) && vs.user_id != owner_id {
                // Get username
                let username = if let Some(member) = guild.members.get(&vs.user_id) {
                    if member.user.bot {
                        continue; // Skip bots
                    }
                    member.display_name().to_string()
                } else {
                    format!("User {}", vs.user_id)
                };

                members.push((vs.user_id.get(), username));
            }
        }
    }

    members
}
