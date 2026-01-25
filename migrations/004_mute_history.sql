-- Mute history table
CREATE TABLE mute_history (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    guild_id BIGINT NOT NULL,
    channel_id BIGINT NOT NULL,
    muted_user_id BIGINT NOT NULL,
    muted_by_user_id BIGINT NOT NULL,
    is_admin_mute BOOLEAN NOT NULL DEFAULT FALSE,
    muted_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    unmuted_at TIMESTAMPTZ
);

-- Indexes for lookups and stats
CREATE INDEX idx_mute_history_guild ON mute_history(guild_id);
CREATE INDEX idx_mute_history_channel ON mute_history(channel_id);
CREATE INDEX idx_mute_history_muted_user ON mute_history(muted_user_id);
CREATE INDEX idx_mute_history_active ON mute_history(channel_id, muted_user_id) WHERE unmuted_at IS NULL;
