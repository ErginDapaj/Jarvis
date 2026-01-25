-- Track global server mutes (admin mutes, not VC-owner mutes)
-- These should never be unmuted by the bot

CREATE TABLE IF NOT EXISTS global_mutes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id BIGINT NOT NULL,
    user_id BIGINT NOT NULL,
    detected_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    unmuted_at TIMESTAMPTZ
);

-- Only one active global mute per user per guild (partial unique index)
CREATE UNIQUE INDEX IF NOT EXISTS idx_global_mutes_active_unique
    ON global_mutes(guild_id, user_id)
    WHERE unmuted_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_global_mutes_guild_user ON global_mutes(guild_id, user_id);
