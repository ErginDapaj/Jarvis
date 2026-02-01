use poise::serenity_prelude::User;

use crate::bot::data::Context;
use crate::bot::error::Error;
use crate::constants::embeds;
use crate::db::queries::voice_channel;
use crate::services::moderation::mute_service;

/// Check if a user is in a specific voice channel
fn is_user_in_channel(ctx: Context<'_>, guild_id: u64, channel_id: u64, user_id: u64) -> bool {
    ctx.serenity_context()
        .cache
        .guild(serenity::all::GuildId::new(guild_id))
        .map(|guild| {
            guild
                .voice_states
                .get(&serenity::all::UserId::new(user_id))
                .and_then(|vs| vs.channel_id)
                .map(|cid| cid.get() == channel_id)
                .unwrap_or(false)
        })
        .unwrap_or(false)
}

/// Mute a user in your voice channel
#[poise::command(slash_command, guild_only)]
pub async fn mute(
    ctx: Context<'_>,
    #[description = "User to mute"] user: User,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or(Error::custom("Not in a guild"))?;
    let author_id = ctx.author().id;

    // Find the channel the author is in and owns
    let channel_id = find_owned_channel(ctx, guild_id.get(), author_id.get()).await?;

    // Verify the target user is in the owner's channel
    if !is_user_in_channel(ctx, guild_id.get(), channel_id.get(), user.id.get()) {
        let embed = embeds::error_embed()
            .title("Not In Your Channel")
            .description(format!(
                "<@{}> is not in your voice channel. You can only mute people who are actually in your room — shocking concept, right?",
                user.id
            ));
        ctx.send(poise::CreateReply::default().embed(embed).ephemeral(true))
            .await?;
        return Ok(());
    }

    // Room owner mutes are always local (not admin) - they auto-unmute when user leaves
    // This is intentional: even if the owner has admin perms, using /mute in their own
    // channel should create a local mute, not a permanent admin mute
    let is_admin_mute = false;

    // Perform the mute
    mute_service::mute_user(
        ctx.serenity_context(),
        ctx.data(),
        guild_id,
        channel_id,
        user.id,
        author_id,
        is_admin_mute,
    )
    .await?;

    let embed = embeds::success_embed()
        .title("User Muted")
        .description(format!("<@{}> has been server muted.", user.id));

    ctx.send(poise::CreateReply::default().embed(embed).ephemeral(true))
        .await?;

    Ok(())
}

/// Unmute a user in your voice channel
#[poise::command(slash_command, guild_only)]
pub async fn unmute(
    ctx: Context<'_>,
    #[description = "User to unmute"] user: User,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or(Error::custom("Not in a guild"))?;
    let author_id = ctx.author().id;

    // Find the channel the author is in and owns
    let channel_id = find_owned_channel(ctx, guild_id.get(), author_id.get()).await?;

    // Verify the target user is in the owner's channel
    if !is_user_in_channel(ctx, guild_id.get(), channel_id.get(), user.id.get()) {
        let embed = embeds::error_embed()
            .title("Not In Your Channel")
            .description(format!(
                "<@{}> is not in your voice channel. You can only unmute people who are actually in your room.",
                user.id
            ));
        ctx.send(poise::CreateReply::default().embed(embed).ephemeral(true))
            .await?;
        return Ok(());
    }

    // Perform the unmute
    let unmuted = mute_service::unmute_user(
        ctx.serenity_context(),
        ctx.data(),
        guild_id,
        channel_id,
        user.id,
    )
    .await?;

    let embed = if unmuted {
        embeds::success_embed()
            .title("User Unmuted")
            .description(format!("<@{}> has been unmuted.", user.id))
    } else {
        embeds::error_embed()
            .title("Not Muted")
            .description(format!("<@{}> was not muted in this channel.", user.id))
    };

    ctx.send(poise::CreateReply::default().embed(embed).ephemeral(true))
        .await?;

    Ok(())
}

/// Find a channel owned by the user
async fn find_owned_channel(
    ctx: Context<'_>,
    guild_id: u64,
    user_id: u64,
) -> Result<serenity::all::ChannelId, Error> {
    use serenity::all::ChannelId;

    // First check cache
    for entry in ctx.data().channel_owners.iter() {
        if *entry.value() == user_id {
            return Ok(ChannelId::new(*entry.key()));
        }
    }

    // Check database
    if let Some(vc) =
        voice_channel::get_by_owner(&ctx.data().pool, guild_id as i64, user_id as i64).await?
    {
        let channel_id = ChannelId::new(vc.channel_id as u64);
        // Update cache
        ctx.data().set_channel_owner(vc.channel_id as u64, user_id);
        return Ok(channel_id);
    }

    Err(Error::custom(
        "You don't own a voice channel. Create one by joining a Join-to-Create channel.",
    ))
}
