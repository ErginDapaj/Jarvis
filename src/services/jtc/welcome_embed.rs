use std::sync::Arc;

use serenity::all::{
    ButtonStyle, ChannelId, Context, CreateActionRow, CreateButton, CreateMessage,
    CreateSelectMenu, CreateSelectMenuKind, UserId,
};
use tracing::error;

use crate::bot::data::Data;
use crate::bot::error::Error;
use crate::constants::embeds::{self, BULLET, DIVIDER_SHORT};

/// Send a welcome embed to the voice channel's text chat
pub async fn send(
    ctx: &Context,
    data: &Arc<Data>,
    channel_id: ChannelId,
    owner_id: UserId,
    is_casual: bool,
) -> Result<(), Error> {
    let channel_type = if is_casual { "Casual" } else { "Debate" };

    let tips = [
        "Use the menus below to manage users",
        "Muted users are unmuted when they leave",
        "Banned users cannot rejoin",
        "Channel deletes when empty",
    ];

    let description = format!(
        "Welcome to your {} voice channel.\n\n\
        {}\n\n\
        **Tips**\n{}",
        channel_type,
        DIVIDER_SHORT,
        tips.iter().map(|t| format!("{} {}", BULLET, t)).collect::<Vec<_>>().join("\n"),
    );

    let embed = embeds::standard_embed()
        .title(format!("{} Voice Channel", channel_type))
        .description(description)
        .field("Owner", format!("<@{}>", owner_id), true);

    // Build components - user select menus for owner actions
    let mut components = Vec::new();

    // Row 1: Mute user select
    let mute_select = CreateSelectMenu::new(
        format!("vc_mute_{}", channel_id),
        CreateSelectMenuKind::User { default_users: None },
    )
    .placeholder("Mute a user");
    components.push(CreateActionRow::SelectMenu(mute_select));

    // Row 2: Unmute user select
    let unmute_select = CreateSelectMenu::new(
        format!("vc_unmute_{}", channel_id),
        CreateSelectMenuKind::User { default_users: None },
    )
    .placeholder("Unmute a user");
    components.push(CreateActionRow::SelectMenu(unmute_select));

    // Row 3: Transfer ownership select
    let transfer_select = CreateSelectMenu::new(
        format!("vc_transfer_{}", channel_id),
        CreateSelectMenuKind::User { default_users: None },
    )
    .placeholder("Transfer ownership");
    components.push(CreateActionRow::SelectMenu(transfer_select));

    // Row 4: Ban user select
    let ban_select = CreateSelectMenu::new(
        format!("vc_ban_{}", channel_id),
        CreateSelectMenuKind::User { default_users: None },
    )
    .placeholder("Ban a user");
    components.push(CreateActionRow::SelectMenu(ban_select));

    // Row 5: Buttons (Reconfigure + Support)
    let mut buttons = Vec::new();

    let reconfigure_button = CreateButton::new(format!("reconfigure_{}", channel_id))
        .label("Reconfigure")
        .style(ButtonStyle::Secondary);
    buttons.push(reconfigure_button);

    if let Some(ref donate_link) = data.settings.donate_link {
        let donate_button = CreateButton::new_link(donate_link)
            .label("❤️ Support Us");
        buttons.push(donate_button);
    }
    components.push(CreateActionRow::Buttons(buttons));

    let message = CreateMessage::new()
        .content(format!("<@{}>", owner_id))
        .embed(embed)
        .components(components);

    if let Err(e) = channel_id.send_message(ctx, message).await {
        error!("Failed to send welcome embed to channel {}: {:?}", channel_id, e);
    }

    Ok(())
}
