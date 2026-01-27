use std::sync::Arc;

use chrono::Utc;
use serenity::all::{
    ChannelId, ChannelType as SerenityChannelType, Context, CreateChannel,
    EditChannel, EditMember, GuildId, PermissionOverwrite, PermissionOverwriteType, Permissions,
    UserId, VideoQualityMode,
};
use tracing::{debug, error, info};

use crate::bot::data::Data;
use crate::bot::error::Error;
use crate::components::naming_prompt;
use crate::constants::timeouts::VC_NAMING_DEADLINE_SECONDS;
use crate::db::models::ChannelType;
use crate::db::queries::{guild_config, user_vc_preference, voice_channel};
use crate::services::jtc::welcome_embed;

/// Start the JTC flow - check preferences or prompt for naming
pub async fn start_jtc_flow(
    ctx: &Context,
    data: &Arc<Data>,
    guild_id: GuildId,
    user_id: UserId,
    _jtc_channel_id: ChannelId,
    is_casual: bool,
) -> Result<(), Error> {
    // Mark user as pending JTC flow
    data.jtc_pending.insert(user_id.get(), std::time::Instant::now());

    let channel_type = if is_casual { "casual" } else { "debate" };

    // Check if user has saved preferences
    let preferences = user_vc_preference::get(
        &data.pool,
        guild_id.get() as i64,
        user_id.get() as i64,
        channel_type,
    )
    .await?;

    if let Some(prefs) = preferences {
        // User has preferences - auto-apply them
        debug!(
            "Applying saved preferences for user {}: name={:?}",
            user_id, prefs.preferred_name
        );

        create_channel(
            ctx,
            data,
            guild_id,
            user_id,
            is_casual,
            prefs.preferred_name,
            prefs.preferred_tags,
        )
        .await?;
    } else {
        // No preferences - create with default name and show naming prompt
        let channel_id = create_channel(ctx, data, guild_id, user_id, is_casual, None, vec![]).await?;

        // Create deadline for configuration
        let deadline_at = Utc::now() + chrono::Duration::seconds(VC_NAMING_DEADLINE_SECONDS as i64);
        user_vc_preference::create_deadline(
            &data.pool,
            channel_id.get() as i64,
            guild_id.get() as i64,
            user_id.get() as i64,
            deadline_at,
        )
        .await?;

        // Send naming prompt
        naming_prompt::send_prompt(ctx, channel_id, user_id).await?;
    }

    Ok(())
}

/// Create a new voice channel for the user
pub async fn create_channel(
    ctx: &Context,
    data: &Arc<Data>,
    guild_id: GuildId,
    user_id: UserId,
    is_casual: bool,
    topic: Option<String>,
    tags: Vec<String>,
) -> Result<ChannelId, Error> {
    let config = guild_config::get(&data.pool, guild_id.get() as i64)
        .await?
        .ok_or(Error::JtcNotConfigured)?;

    let category_id = config.category_id(is_casual).ok_or(Error::JtcNotConfigured)?;

    let channel_type = if is_casual {
        ChannelType::Casual
    } else {
        ChannelType::Debate
    };

    // Generate channel name
    let channel_name = if let Some(ref t) = topic {
        t.clone()
    } else if is_casual {
        format!("Casual VC")
    } else {
        format!("Debate VC")
    };

    // Create the voice channel with highest quality settings
    // Bitrate: 96kbps (universal max - higher requires server boosts)
    // Video quality: Full 720p
    let channel = guild_id
        .create_channel(
            ctx,
            CreateChannel::new(&channel_name)
                .kind(SerenityChannelType::Voice)
                .category(ChannelId::new(category_id as u64))
                .bitrate(96_000) // 96kbps - max for all servers
                .video_quality_mode(VideoQualityMode::Full) // 720p video
                .permissions(vec![
                    // Owner can only mute members
                    PermissionOverwrite {
                        allow: Permissions::MUTE_MEMBERS,
                        deny: Permissions::empty(),
                        kind: PermissionOverwriteType::Member(user_id),
                    },
                ]),
        )
        .await?;

    info!(
        "Created {} voice channel {} for user {}",
        channel_type, channel.id, user_id
    );

    // Store in database
    voice_channel::create(
        &data.pool,
        channel.id.get() as i64,
        guild_id.get() as i64,
        user_id.get() as i64,
        channel_type,
        topic.as_deref(),
        &tags,
    )
    .await?;

    // Update cache
    data.set_channel_owner(channel.id.get(), user_id.get());

    // Move user to the new channel
    if let Err(e) = guild_id
        .edit_member(ctx, user_id, EditMember::new().voice_channel(channel.id))
        .await
    {
        error!("Failed to move user {} to channel {}: {:?}", user_id, channel.id, e);
    }

    // Set channel status with tags if available
    if !tags.is_empty() {
        let status_text = tags.iter().map(|t| format!("`{}`", t)).collect::<Vec<_>>().join(" ");
        if let Err(e) = channel.id.edit(ctx, EditChannel::new().status(&status_text)).await {
            debug!("Could not set channel status during creation (may not be available): {:?}", e);
        } else {
            info!("Set channel status for {}: {}", channel.id, status_text);
        }
    }

    // Send welcome embed in the text-in-voice channel
    welcome_embed::send(ctx, data, channel.id, user_id, is_casual).await?;

    // Clean up pending status
    data.jtc_pending.remove(&user_id.get());

    Ok(channel.id)
}

/// Update channel topic and name
pub async fn update_channel_topic(
    ctx: &Context,
    data: &Arc<Data>,
    channel_id: ChannelId,
    topic: &str,
) -> Result<(), Error> {
    // Update database
    voice_channel::update_topic(&data.pool, channel_id.get() as i64, Some(topic)).await?;

    // Update Discord channel name
    channel_id
        .edit(ctx, EditChannel::new().name(topic))
        .await?;

    Ok(())
}

/// Update channel tags
pub async fn update_channel_tags(
    ctx: &Context,
    data: &Arc<Data>,
    channel_id: ChannelId,
    tags: Vec<String>,
) -> Result<(), Error> {
    // Update database
    voice_channel::update_tags(&data.pool, channel_id.get() as i64, &tags).await?;

    // Set tags in channel status if available
    if !tags.is_empty() {
        let status_text = tags.iter().map(|t| format!("`{}`", t)).collect::<Vec<_>>().join(" ");
        
        // Try to set channel status (activity status)
        // This may not be available on all Discord servers/plans
        if let Err(e) = channel_id.edit(ctx, EditChannel::new().status(&status_text)).await {
            // Status not available or failed - that's okay, just log it
            tracing::debug!("Could not set channel status (may not be available): {:?}", e);
        } else {
            tracing::info!("Set channel status for {}: {}", channel_id, status_text);
        }
    } else {
        // Clear status if no tags
        if let Err(e) = channel_id.edit(ctx, EditChannel::new().status("")).await {
            tracing::debug!("Could not clear channel status: {:?}", e);
        }
    }

    Ok(())
}
