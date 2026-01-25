use poise::serenity_prelude::{CreateAttachment, User};

use crate::bot::data::Context;
use crate::bot::error::Error;
use crate::constants::embeds::{self, BULLET, DIVIDER};
use crate::services::stats::{aggregator, chart_generator};

/// View statistics for a user or the server
#[poise::command(slash_command, guild_only)]
pub async fn stats(
    ctx: Context<'_>,
    #[description = "User to view stats for (defaults to yourself)"] user: Option<User>,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or(Error::custom("Not in a guild"))?;
    let target_user = user.as_ref().unwrap_or_else(|| ctx.author());

    // Defer since this might take a moment
    ctx.defer_ephemeral().await?;

    // Get user stats
    let stats = aggregator::get_user_stats(
        &ctx.data().pool,
        guild_id.get() as i64,
        target_user.id.get() as i64,
    )
    .await?;

    // Build a clean embed - chart shows the visual data, embed shows summary
    let description = format!(
        "{}\n\n\
        **Moderation Summary**\n\
        {} Mutes received: **{}** | given: **{}**\n\
        {} Bans received: **{}** | given: **{}**\n\
        {} Spam infractions: **{}**\n\n\
        {}\n\n\
        **Timeout Status**\n\
        {} Current level: **{}**/5",
        DIVIDER,
        BULLET, stats.mutes_received, stats.mutes_given,
        BULLET, stats.bans_received, stats.bans_given,
        BULLET, stats.spam_infractions,
        DIVIDER,
        BULLET, stats.current_timeout_level
    );

    let mut embed = embeds::standard_embed()
        .title(format!("Stats for {}", target_user.name))
        .thumbnail(target_user.face())
        .description(description);

    let mut reply = poise::CreateReply::default();

    // Try to generate chart
    match chart_generator::generate_user_stats_chart(&stats, &target_user.name) {
        Ok(chart_data) => {
            embed = embed.image("attachment://stats.png");
            reply = reply.attachment(CreateAttachment::bytes(chart_data, "stats.png"));
        }
        Err(e) => {
            tracing::warn!("Failed to generate chart: {:?}", e);
            // Continue without chart
        }
    }

    reply = reply.embed(embed).ephemeral(true);
    ctx.send(reply).await?;

    Ok(())
}
