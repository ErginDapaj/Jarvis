-- Guild configuration table
CREATE TABLE guild_configs (
    guild_id BIGINT PRIMARY KEY,
    jtc_casual_channel_id BIGINT,
    jtc_debate_channel_id BIGINT,
    category_casual_id BIGINT,
    category_debate_id BIGINT,
    rules_casual_channel_id BIGINT,
    rules_debate_channel_id BIGINT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Index for quick lookups
CREATE INDEX idx_guild_configs_jtc_channels
    ON guild_configs(jtc_casual_channel_id, jtc_debate_channel_id);
