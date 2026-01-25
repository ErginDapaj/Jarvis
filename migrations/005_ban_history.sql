-- VC ban history table
CREATE TABLE vc_ban_history (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    guild_id BIGINT NOT NULL,
    channel_id BIGINT NOT NULL,
    banned_user_id BIGINT NOT NULL,
    banned_by_user_id BIGINT NOT NULL,
    reason VARCHAR(500),
    banned_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes for lookups
CREATE INDEX idx_vc_ban_guild ON vc_ban_history(guild_id);
CREATE INDEX idx_vc_ban_channel ON vc_ban_history(channel_id);
CREATE INDEX idx_vc_ban_user ON vc_ban_history(banned_user_id);
CREATE INDEX idx_vc_ban_channel_user ON vc_ban_history(channel_id, banned_user_id);
