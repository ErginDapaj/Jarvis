use poise::serenity_prelude::Channel;

use crate::bot::data::Context;
use crate::bot::error::Error;
use crate::constants::embeds;
use crate::db::queries::guild_config;

/// Setup commands for configuring the bot
#[poise::command(
    slash_command,
    subcommands("jtc_channel", "category", "rules_channel"),
    required_permissions = "ADMINISTRATOR",
    guild_only
)]
pub async fn setup(ctx: Context<'_>) -> Result<(), Error> {
    ctx.say("Use one of the subcommands: `/setup jtc-channel`, `/setup category`, `/setup rules-channel`").await?;
    Ok(())
}

/// Set the Join-to-Create channel
#[poise::command(slash_command, rename = "jtc-channel", guild_only)]
pub async fn jtc_channel(
    ctx: Context<'_>,
    #[description = "Channel type"] channel_type: ChannelTypeChoice,
    #[description = "Voice channel to use as JTC trigger"]
    #[channel_types("Voice")]
    channel: Channel,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or(Error::custom("Not in a guild"))?;

    let is_casual = matches!(channel_type, ChannelTypeChoice::Casual);

    guild_config::set_jtc_channel(
        &ctx.data().pool,
        guild_id.get() as i64,
        is_casual,
        channel.id().get() as i64,
    )
    .await?;

    let embed = embeds::success_embed()
        .title("JTC Channel Set")
        .description(format!(
            "Set {} JTC channel to <#{}>",
            if is_casual { "casual" } else { "debate" },
            channel.id()
        ));

    ctx.send(poise::CreateReply::default().embed(embed).ephemeral(true))
        .await?;

    Ok(())
}

/// Set the category for spawned voice channels
#[poise::command(slash_command, guild_only)]
pub async fn category(
    ctx: Context<'_>,
    #[description = "Channel type"] channel_type: ChannelTypeChoice,
    #[description = "Category where voice channels will be created"]
    #[channel_types("Category")]
    category: Channel,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or(Error::custom("Not in a guild"))?;

    let is_casual = matches!(channel_type, ChannelTypeChoice::Casual);

    guild_config::set_category(
        &ctx.data().pool,
        guild_id.get() as i64,
        is_casual,
        category.id().get() as i64,
    )
    .await?;

    let embed = embeds::success_embed()
        .title("Category Set")
        .description(format!(
            "Set {} category to <#{}>",
            if is_casual { "casual" } else { "debate" },
            category.id()
        ));

    ctx.send(poise::CreateReply::default().embed(embed).ephemeral(true))
        .await?;

    Ok(())
}

/// Set the rules channel for a channel type
#[poise::command(slash_command, rename = "rules-channel", guild_only)]
pub async fn rules_channel(
    ctx: Context<'_>,
    #[description = "Channel type"] channel_type: ChannelTypeChoice,
    #[description = "Rules channel to link"]
    #[channel_types("Text")]
    channel: Channel,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or(Error::custom("Not in a guild"))?;

    let is_casual = matches!(channel_type, ChannelTypeChoice::Casual);

    guild_config::set_rules_channel(
        &ctx.data().pool,
        guild_id.get() as i64,
        is_casual,
        channel.id().get() as i64,
    )
    .await?;

    let embed = embeds::success_embed()
        .title("Rules Channel Set")
        .description(format!(
            "Set {} rules channel to <#{}>",
            if is_casual { "casual" } else { "debate" },
            channel.id()
        ));

    ctx.send(poise::CreateReply::default().embed(embed).ephemeral(true))
        .await?;

    Ok(())
}

/// Channel type choice for commands
#[derive(Debug, Clone, Copy, poise::ChoiceParameter)]
pub enum ChannelTypeChoice {
    Casual,
    Debate,
}
