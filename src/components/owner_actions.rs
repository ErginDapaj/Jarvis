use std::sync::Arc;

use serenity::all::{
    ChannelId, ComponentInteraction, ComponentInteractionDataKind, Context,
    CreateInteractionResponse, CreateInteractionResponseMessage, GuildId, UserId,
};
use tracing::{debug, error};

use crate::bot::data::Data;
use crate::bot::error::Error;
use crate::constants::embeds;
use crate::db::queries::voice_channel;
use crate::handlers::interaction::send_component_error;
use crate::services::moderation::{ban_service, mute_service};

/// Handle owner action select menus (mute, unmute, transfer, ban)
pub async fn handle_selection(
    ctx: &Context,
    data: &Arc<Data>,
    component: &ComponentInteraction,
) -> Result<(), Error> {
    let custom_id = &component.data.custom_id;

    // Parse custom_id: vc_{action}_{channel_id}
    let parts: Vec<&str> = custom_id.split('_').collect();
    if parts.len() < 3 {
        send_component_error(ctx, component, "Invalid selector state").await?;
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

    let guild_id = match component.guild_id {
        Some(id) => id,
        None => {
            send_component_error(ctx, component, "This only works in a server").await?;
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
        send_component_error(ctx, component, "Whoa there, you don't own this channel. Did you really think you could server mute people in someone else's room? Embarrassing.").await?;
        return Ok(());
    }

    // Get selected user
    let selected_user_id = match &component.data.kind {
        ComponentInteractionDataKind::UserSelect { values } => {
            match values.first() {
                Some(id) => *id,
                None => {
                    send_component_error(ctx, component, "No user selected").await?;
                    return Ok(());
                }
            }
        }
        _ => {
            send_component_error(ctx, component, "Unexpected interaction type").await?;
            return Ok(());
        }
    };

    // Don't allow actions on self
    if selected_user_id.get() == owner_id {
        send_component_error(ctx, component, "You cannot perform this action on yourself").await?;
        return Ok(());
    }

    // Don't allow actions on bots
    let is_bot = ctx.cache.user(selected_user_id).map(|u| u.bot).unwrap_or(false);
    if is_bot {
        send_component_error(ctx, component, "You cannot perform this action on bots").await?;
        return Ok(());
    }

    // Route to appropriate handler
    match action {
        "mute" => handle_mute(ctx, data, component, guild_id, channel_id, owner_id, selected_user_id).await,
        "unmute" => handle_unmute(ctx, data, component, guild_id, channel_id, selected_user_id).await,
        "transfer" => handle_transfer(ctx, data, component, guild_id, channel_id, owner_id, selected_user_id).await,
        "ban" => handle_ban(ctx, data, component, guild_id, channel_id, owner_id, selected_user_id).await,
        _ => {
            send_component_error(ctx, component, "Unknown action").await?;
            Ok(())
        }
    }
}

async fn handle_mute(
    ctx: &Context,
    data: &Arc<Data>,
    component: &ComponentInteraction,
    guild_id: GuildId,
    channel_id: u64,
    owner_id: u64,
    target_id: UserId,
) -> Result<(), Error> {
    debug!("Mute action: owner {} muting {} in channel {}", owner_id, target_id, channel_id);

    let result = mute_service::mute_user(
        ctx,
        data,
        guild_id,
        ChannelId::new(channel_id),
        target_id,
        UserId::new(owner_id),
        false, // Not an admin mute - room owner mute
    )
    .await;

    let embed = match result {
        Ok(_) => embeds::success_embed()
            .title("User Muted")
            .description(format!("<@{}> has been server muted.", target_id)),
        Err(e) => {
            error!("Mute failed: {:?}", e);
            embeds::error_embed()
                .title("Mute Failed")
                .description(format!("Failed to mute user: {}", e))
        }
    };

    send_ephemeral_response(ctx, component, embed).await
}

async fn handle_unmute(
    ctx: &Context,
    data: &Arc<Data>,
    component: &ComponentInteraction,
    guild_id: GuildId,
    channel_id: u64,
    target_id: UserId,
) -> Result<(), Error> {
    debug!("Unmute action: unmuting {} in channel {}", target_id, channel_id);

    let result = mute_service::unmute_user(
        ctx,
        data,
        guild_id,
        ChannelId::new(channel_id),
        target_id,
    )
    .await;

    let embed = match result {
        Ok(true) => embeds::success_embed()
            .title("User Unmuted")
            .description(format!("<@{}> has been unmuted.", target_id)),
        Ok(false) => embeds::warning_embed()
            .title("Not Muted")
            .description(format!("<@{}> was not muted in this channel.", target_id)),
        Err(e) => {
            error!("Unmute failed: {:?}", e);
            embeds::error_embed()
                .title("Unmute Failed")
                .description(format!("Failed to unmute user: {}", e))
        }
    };

    send_ephemeral_response(ctx, component, embed).await
}

async fn handle_transfer(
    ctx: &Context,
    data: &Arc<Data>,
    component: &ComponentInteraction,
    guild_id: GuildId,
    channel_id: u64,
    old_owner_id: u64,
    new_owner_id: UserId,
) -> Result<(), Error> {
    debug!("Transfer action: {} transferring channel {} to {}", old_owner_id, channel_id, new_owner_id);

    // Update database
    let result = voice_channel::update_owner(
        &data.pool,
        channel_id as i64,
        new_owner_id.get() as i64,
    )
    .await;

    let embed = match result {
        Ok(_) => {
            // Update cache
            data.set_channel_owner(channel_id, new_owner_id.get());

            // Try to update channel permissions
            let channel_id_obj = ChannelId::new(channel_id);
            if let Err(e) = update_channel_permissions(ctx, guild_id, channel_id_obj, UserId::new(old_owner_id), new_owner_id).await {
                error!("Failed to update channel permissions: {:?}", e);
            }

            embeds::success_embed()
                .title("Ownership Transferred")
                .description(format!(
                    "Channel ownership has been transferred to <@{}>.",
                    new_owner_id
                ))
        }
        Err(e) => {
            error!("Transfer failed: {:?}", e);
            embeds::error_embed()
                .title("Transfer Failed")
                .description(format!("Failed to transfer ownership: {}", e))
        }
    };

    send_ephemeral_response(ctx, component, embed).await
}

async fn handle_ban(
    ctx: &Context,
    data: &Arc<Data>,
    component: &ComponentInteraction,
    guild_id: GuildId,
    channel_id: u64,
    owner_id: u64,
    target_id: UserId,
) -> Result<(), Error> {
    debug!("Ban action: owner {} banning {} from channel {}", owner_id, target_id, channel_id);

    let result = ban_service::ban_user(
        ctx,
        data,
        guild_id,
        ChannelId::new(channel_id),
        target_id,
        UserId::new(owner_id),
        None,
    )
    .await;

    let embed = match result {
        Ok(_) => embeds::success_embed()
            .title("User Banned")
            .description(format!(
                "<@{}> has been banned from this voice channel.",
                target_id
            )),
        Err(e) => {
            error!("Ban failed: {:?}", e);
            embeds::error_embed()
                .title("Ban Failed")
                .description(format!("Failed to ban user: {}", e))
        }
    };

    send_ephemeral_response(ctx, component, embed).await
}

/// Update channel permissions when ownership is transferred
async fn update_channel_permissions(
    ctx: &Context,
    _guild_id: GuildId,
    channel_id: ChannelId,
    old_owner: UserId,
    new_owner: UserId,
) -> Result<(), Error> {
    use serenity::all::{PermissionOverwrite, PermissionOverwriteType, Permissions};

    // Remove old owner's permission overwrite
    let _ = channel_id
        .delete_permission(ctx, PermissionOverwriteType::Member(old_owner))
        .await;

    // Add new owner's permission overwrite - only mute
    let new_owner_perms = PermissionOverwrite {
        allow: Permissions::MUTE_MEMBERS,
        deny: Permissions::empty(),
        kind: PermissionOverwriteType::Member(new_owner),
    };

    channel_id.create_permission(ctx, new_owner_perms).await?;

    Ok(())
}

/// Send an ephemeral response for a component interaction
async fn send_ephemeral_response(
    ctx: &Context,
    component: &ComponentInteraction,
    embed: serenity::all::CreateEmbed,
) -> Result<(), Error> {
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
