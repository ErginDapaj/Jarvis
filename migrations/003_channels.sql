-- Active voice channels table
CREATE TABLE active_voice_channels (
    channel_id BIGINT PRIMARY KEY,
    guild_id BIGINT NOT NULL REFERENCES guild_configs(guild_id) ON DELETE CASCADE,
    owner_id BIGINT NOT NULL,
    channel_type channel_type NOT NULL,
    topic VARCHAR(500),
    tags TEXT[] DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes for common queries
CREATE INDEX idx_active_vc_guild ON active_voice_channels(guild_id);
CREATE INDEX idx_active_vc_owner ON active_voice_channels(owner_id);
