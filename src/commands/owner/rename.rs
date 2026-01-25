use chrono::Utc;
use poise::serenity_prelude::ChannelId;

use crate::bot::data::Context;
use crate::bot::error::Error;
use crate::constants::embeds;
use crate::constants::timeouts::RENAME_RETAG_RATE_LIMIT_SECONDS;
use crate::db::queries::{rate_limit, user_vc_preference, voice_channel};
use crate::db::queries::rate_limit::CommandType;
use crate::services::jtc::channel_creator;
use crate::utils::profanity;

/// Rename your voice channel
#[poise::command(slash_command, guild_only)]
pub async fn rename(
    ctx: Context<'_>,
    #[description = "New name for your channel (max 100 characters)"] name: String,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or(Error::custom("Not in a guild"))?;
    let author_id = ctx.author().id;

    // Validate name
    if name.is_empty() {
        let embed = embeds::error_embed()
            .title("Error")
            .description("Channel name cannot be empty.");

        ctx.send(poise::CreateReply::default().embed(embed).ephemeral(true))
            .await?;

        return Ok(());
    }

    // Check for profanity and validate name
    if let Err(reason) = profanity::validate_channel_name(&name) {
        let embed = embeds::error_embed()
            .title("Inappropriate Name")
            .description(reason);

        ctx.send(poise::CreateReply::default().embed(embed).ephemeral(true))
            .await?;

        return Ok(());
    }

    // Find the channel the author owns
    let channel_id = find_owned_channel(ctx, guild_id.get(), author_id.get()).await?;

    // Check rate limit
    let last_used = rate_limit::get_last_used(
        &ctx.data().pool,
        author_id.get() as i64,
        guild_id.get() as i64,
        CommandType::Rename,
    )
    .await?;

    if let Some(last_used_time) = last_used {
        let elapsed = Utc::now() - last_used_time;
        let elapsed_secs = elapsed.num_seconds() as u64;

        if elapsed_secs < RENAME_RETAG_RATE_LIMIT_SECONDS {
            let remaining = RENAME_RETAG_RATE_LIMIT_SECONDS - elapsed_secs;
            let minutes = remaining / 60;
            let seconds = remaining % 60;

            let error_msg = if minutes > 0 {
                format!(
                    "You can only rename your channel once every 30 minutes. Please wait {} minute{} and {} second{}.",
                    minutes,
                    if minutes == 1 { "" } else { "s" },
                    seconds,
                    if seconds == 1 { "" } else { "s" }
                )
            } else {
                format!(
                    "You can only rename your channel once every 30 minutes. Please wait {} second{}.",
                    seconds,
                    if seconds == 1 { "" } else { "s" }
                )
            };

            let embed = embeds::error_embed()
                .title("Rate Limit")
                .description(error_msg);

            ctx.send(poise::CreateReply::default().embed(embed).ephemeral(true))
                .await?;

            return Ok(());
        }
    }

    // Get channel info to determine type
    let vc = voice_channel::get(&ctx.data().pool, channel_id.get() as i64)
        .await?
        .ok_or(Error::custom("Channel not found in database."))?;

    let channel_type_str = vc.channel_type.to_string();

    // Update the channel name
    channel_creator::update_channel_topic(ctx.serenity_context(), ctx.data(), channel_id, &name)
        .await?;

    // Update user preferences so next time they create a channel, it uses the new name
    // Get existing tags to preserve them
    let existing_tags = vc.tags.clone();
    user_vc_preference::upsert(
        &ctx.data().pool,
        guild_id.get() as i64,
        author_id.get() as i64,
        &channel_type_str,
        Some(&name),
        &existing_tags,
    )
    .await?;

    // Update rate limit
    rate_limit::update_last_used(
        &ctx.data().pool,
        author_id.get() as i64,
        guild_id.get() as i64,
        CommandType::Rename,
    )
    .await?;

    let embed = embeds::success_embed()
        .title("Channel Renamed")
        .description(format!("Your channel has been renamed to **{}**.", name));

    ctx.send(poise::CreateReply::default().embed(embed).ephemeral(true))
        .await?;

    Ok(())
}

/// Find a channel owned by the user
async fn find_owned_channel(
    ctx: Context<'_>,
    guild_id: u64,
    user_id: u64,
) -> Result<ChannelId, Error> {
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
