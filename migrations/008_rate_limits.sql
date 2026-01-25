-- Rate limiting for channel rename and retag commands
CREATE TABLE IF NOT EXISTS channel_command_rate_limits (
    user_id BIGINT NOT NULL,
    guild_id BIGINT NOT NULL,
    command_type VARCHAR(20) NOT NULL, -- 'rename' or 'retag'
    last_used_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, guild_id, command_type)
);

-- Index for fast lookups
CREATE INDEX IF NOT EXISTS idx_rate_limit_lookup ON channel_command_rate_limits(user_id, guild_id, command_type);
