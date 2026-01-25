use poise::serenity_prelude::User;

use crate::bot::data::Context;
use crate::bot::error::Error;
use crate::constants::embeds;
use crate::db::queries::voice_channel;
use crate::services::moderation::ban_service;

/// Ban a user from your voice channel
#[poise::command(slash_command, guild_only)]
pub async fn vcban(
    ctx: Context<'_>,
    #[description = "User to ban"] user: User,
    #[description = "Reason for the ban"] reason: Option<String>,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or(Error::custom("Not in a guild"))?;
    let author_id = ctx.author().id;

    // Find the channel the author owns
    let channel_id = find_owned_channel(ctx, guild_id.get(), author_id.get()).await?;

    // Perform the ban
    ban_service::ban_user(
        ctx.serenity_context(),
        ctx.data(),
        guild_id,
        channel_id,
        user.id,
        author_id,
        reason.as_deref(),
    )
    .await?;

    let mut embed = embeds::success_embed()
        .title("User Banned")
        .description(format!(
            "<@{}> has been banned from your voice channel.",
            user.id
        ));

    if let Some(ref r) = reason {
        embed = embed.field("Reason", r, false);
    }

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
