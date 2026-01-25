use poise::serenity_prelude::User;

use crate::bot::data::Context;
use crate::bot::error::Error;
use crate::constants::embeds;
use crate::db::queries::voice_channel;

/// Transfer ownership of your voice channel to another user
#[poise::command(slash_command, guild_only)]
pub async fn transfer(
    ctx: Context<'_>,
    #[description = "User to transfer ownership to"] user: User,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or(Error::custom("Not in a guild"))?;
    let author_id = ctx.author().id;

    // Find the channel the author owns
    let channel_id = find_owned_channel(ctx, guild_id.get(), author_id.get()).await?;

    // Prevent transferring to self
    if user.id == author_id {
        return Err(Error::custom("You cannot transfer ownership to yourself."));
    }

    // Prevent transferring to bots
    if user.bot {
        return Err(Error::custom("You cannot transfer ownership to a bot."));
    }

    // Update database
    voice_channel::update_owner(&ctx.data().pool, channel_id.get() as i64, user.id.get() as i64)
        .await?;

    // Update cache
    ctx.data()
        .set_channel_owner(channel_id.get(), user.id.get());

    // Update channel permissions
    // Remove manage permission from old owner, add to new owner
    use serenity::all::{PermissionOverwrite, PermissionOverwriteType, Permissions};

    // Remove old owner's permissions
    let _ = channel_id
        .delete_permission(ctx.serenity_context(), PermissionOverwriteType::Member(author_id))
        .await;

    // Add new owner's permissions
    let _ = channel_id
        .create_permission(
            ctx.serenity_context(),
            PermissionOverwrite {
                allow: Permissions::MANAGE_CHANNELS
                    | Permissions::MUTE_MEMBERS
                    | Permissions::MOVE_MEMBERS,
                deny: Permissions::empty(),
                kind: PermissionOverwriteType::Member(user.id),
            },
        )
        .await;

    let embed = embeds::success_embed()
        .title("Ownership Transferred")
        .description(format!(
            "Ownership of the voice channel has been transferred to <@{}>.",
            user.id
        ));

    ctx.send(poise::CreateReply::default().embed(embed).ephemeral(true))
        .await?;

    // Notify the new owner in the channel
    let notify_embed = embeds::standard_embed()
        .title("New Channel Owner")
        .description(format!(
            "<@{}> is now the owner of this voice channel.",
            user.id
        ));

    let _ = channel_id
        .send_message(
            ctx.serenity_context(),
            serenity::all::CreateMessage::new().embed(notify_embed),
        )
        .await;

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
