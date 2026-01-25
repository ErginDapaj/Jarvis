-- Spam user status table
CREATE TABLE spam_user_status (
    guild_id BIGINT NOT NULL,
    user_id BIGINT NOT NULL,
    current_timeout_level INT NOT NULL DEFAULT 0,
    last_infraction_at TIMESTAMPTZ,
    total_infractions INT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (guild_id, user_id)
);

-- Index for cleanup queries
CREATE INDEX idx_spam_status_last_infraction ON spam_user_status(last_infraction_at);
