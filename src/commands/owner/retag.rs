use chrono::Utc;
use poise::serenity_prelude::ChannelId;

use crate::bot::data::Context;
use crate::bot::error::Error;
use crate::constants::embeds;
use crate::constants::tags::{get_tags, MAX_TAGS};
use crate::constants::timeouts::RENAME_RETAG_RATE_LIMIT_SECONDS;
use crate::db::queries::{rate_limit, voice_channel};
use crate::db::queries::rate_limit::CommandType;
use crate::services::jtc::channel_creator;

/// Retag your voice channel
#[poise::command(slash_command, guild_only)]
pub async fn retag(
    ctx: Context<'_>,
    #[description = "Tags for your channel (space-separated, max 4)"] tags: String,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or(Error::custom("Not in a guild"))?;
    let author_id = ctx.author().id;

    // Find the channel the author owns
    let channel_id = find_owned_channel(ctx, guild_id.get(), author_id.get()).await?;

    // Get channel info to determine if it's casual or debate
    let vc = voice_channel::get(&ctx.data().pool, channel_id.get() as i64)
        .await?
        .ok_or(Error::custom("Channel not found in database."))?;

    let is_casual = vc.channel_type.is_casual();
    let available_tags = get_tags(is_casual);

    // Parse tags from input
    let tag_list: Vec<String> = tags
        .split_whitespace()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    // Validate tag count
    if tag_list.len() > MAX_TAGS {
        return Err(Error::custom(format!(
            "You can only select up to {} tags.",
            MAX_TAGS
        )));
    }

    // Validate tags are in the available list
    let mut valid_tags = Vec::new();
    for tag in &tag_list {
        if available_tags.contains(&tag.as_str()) {
            valid_tags.push(tag.clone());
        } else {
            return Err(Error::custom(format!(
                "Invalid tag: '{}'. Available tags: {}",
                tag,
                available_tags.join(", ")
            )));
        }
    }

    // Check rate limit
    let last_used = rate_limit::get_last_used(
        &ctx.data().pool,
        author_id.get() as i64,
        guild_id.get() as i64,
        CommandType::Retag,
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
                    "You can only retag your channel once every 30 minutes. Please wait {} minute{} and {} second{}.",
                    minutes,
                    if minutes == 1 { "" } else { "s" },
                    seconds,
                    if seconds == 1 { "" } else { "s" }
                )
            } else {
                format!(
                    "You can only retag your channel once every 30 minutes. Please wait {} second{}.",
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

    // Update the channel tags
    channel_creator::update_channel_tags(ctx.serenity_context(), ctx.data(), channel_id, valid_tags.clone())
        .await?;

    // Update rate limit
    rate_limit::update_last_used(
        &ctx.data().pool,
        author_id.get() as i64,
        guild_id.get() as i64,
        CommandType::Retag,
    )
    .await?;

    let embed = embeds::success_embed()
        .title("Channel Tags Updated")
        .description(if valid_tags.is_empty() {
            "All tags have been removed from your channel.".to_string()
        } else {
            format!(
                "Tags updated: {}",
                valid_tags
                    .iter()
                    .map(|t| format!("`{}`", t))
                    .collect::<Vec<_>>()
                    .join(" ")
            )
        });

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
