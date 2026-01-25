use std::sync::Arc;

use serenity::all::{
    ButtonStyle, ChannelId, Context, CreateActionRow, CreateButton, CreateMessage, UserId,
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

    let commands = [
        "`/mute @user` - Server mute a user",
        "`/unmute @user` - Unmute a user",
        "`/vcban @user` - Ban from channel",
        "`/transfer @user` - Transfer ownership",
        "`/rename <name>` - Rename your channel",
        "`/retag <tags>` - Update channel tags",
    ];

    let tips = [
        "Mutes are removed when users leave",
        "Banned users cannot rejoin",
        "Channel deletes when empty",
    ];

    let description = format!(
        "Welcome to your {} voice channel.\n\n\
        **Owner Commands**\n{}\n\n\
        {}\n\n\
        **Notes**\n{}",
        channel_type,
        commands.iter().map(|c| format!("{} {}", BULLET, c)).collect::<Vec<_>>().join("\n"),
        DIVIDER_SHORT,
        tips.iter().map(|t| format!("{} {}", BULLET, t)).collect::<Vec<_>>().join("\n"),
    );

    let embed = embeds::standard_embed()
        .title(format!("{} Voice Channel", channel_type))
        .description(description)
        .field("Owner", format!("<@{}>", owner_id), true);

    // Build message with ping, reconfigure button, and optional donate button
    let mut buttons = Vec::new();
    
    // Add reconfigure button
    let reconfigure_button = CreateButton::new(format!("reconfigure_{}", channel_id))
        .label("Reconfigure")
        .style(ButtonStyle::Secondary);
    buttons.push(reconfigure_button);

    // Add donate button if configured
    if let Some(ref donate_link) = data.settings.donate_link {
        let donate_button = CreateButton::new_link(donate_link)
            .label("Support Us (Please!!)");
        buttons.push(donate_button);
    }

    let action_row = CreateActionRow::Buttons(buttons);
    let message = CreateMessage::new()
        .content(format!("<@{}>", owner_id)) // Ping the owner
        .embed(embed)
        .components(vec![action_row]);

    if let Err(e) = channel_id.send_message(ctx, message).await {
        error!("Failed to send welcome embed to channel {}: {:?}", channel_id, e);
    }

    Ok(())
}
