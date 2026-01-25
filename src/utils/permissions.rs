use serenity::all::{Context, GuildId, Member, Permissions, UserId};

/// Check if a member has administrator permissions
pub async fn is_admin(ctx: &Context, guild_id: GuildId, user_id: UserId) -> bool {
    if let Ok(member) = guild_id.member(ctx, user_id).await {
        return member.permissions(ctx).map(|p| p.administrator()).unwrap_or(false);
    }
    false
}

/// Check if a member has a specific permission
pub async fn has_permission(
    ctx: &Context,
    guild_id: GuildId,
    user_id: UserId,
    permission: Permissions,
) -> bool {
    if let Ok(member) = guild_id.member(ctx, user_id).await {
        return member.permissions(ctx).map(|p| p.contains(permission)).unwrap_or(false);
    }
    false
}

/// Check if a member can moderate (has kick/ban permissions)
pub async fn can_moderate(ctx: &Context, guild_id: GuildId, user_id: UserId) -> bool {
    if let Ok(member) = guild_id.member(ctx, user_id).await {
        return member.permissions(ctx)
            .map(|p| p.administrator() || p.kick_members() || p.ban_members())
            .unwrap_or(false);
    }
    false
}

/// Check if a member is in a voice channel
pub fn is_in_voice_channel(_member: &Member, _channel_id: u64) -> bool {
    // This would need voice state lookup via guild cache
    // For now, this is a placeholder that would be implemented with proper caching
    false
}
