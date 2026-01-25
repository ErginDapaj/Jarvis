-- User VC preferences for remembered naming settings
CREATE TABLE IF NOT EXISTS user_vc_preferences (
    id SERIAL PRIMARY KEY,
    guild_id BIGINT NOT NULL,
    user_id BIGINT NOT NULL,
    preferred_name VARCHAR(100),
    preferred_tags TEXT[] NOT NULL DEFAULT '{}',
    channel_type VARCHAR(20) NOT NULL DEFAULT 'casual',
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    UNIQUE(guild_id, user_id, channel_type)
);

-- Index for fast lookups
CREATE INDEX IF NOT EXISTS idx_user_vc_prefs_lookup ON user_vc_preferences(guild_id, user_id, channel_type);

-- Pending VC naming deadlines (for channels awaiting configuration)
CREATE TABLE IF NOT EXISTS pending_vc_deadlines (
    channel_id BIGINT PRIMARY KEY,
    guild_id BIGINT NOT NULL,
    owner_id BIGINT NOT NULL,
    deadline_at TIMESTAMP WITH TIME ZONE NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Index for deadline checking
CREATE INDEX IF NOT EXISTS idx_pending_vc_deadline ON pending_vc_deadlines(deadline_at);
