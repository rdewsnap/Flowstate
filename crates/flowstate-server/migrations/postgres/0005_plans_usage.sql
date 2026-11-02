-- Monthly allowances for hosted cleanup and summaries. Limits count per
-- seat; on a pooled plan they're multiplied by the team's member count and
-- shared. A limit of 0 means hosted models aren't included at all.
CREATE TABLE plans (
    id               TEXT PRIMARY KEY,
    name             TEXT NOT NULL,
    monthly_requests BIGINT NOT NULL,
    monthly_tokens   BIGINT NOT NULL,
    pooled           BIGINT NOT NULL DEFAULT 0
);
INSERT INTO plans (id, name, monthly_requests, monthly_tokens, pooled) VALUES
    ('free', 'Free', 0, 0, 0),
    ('pro', 'Pro', 20000, 20000000, 0),
    ('team', 'Team', 20000, 20000000, 1);

ALTER TABLE teams ADD COLUMN plan TEXT NOT NULL DEFAULT 'free';

-- One row per upstream model call. Never stores request or response text.
CREATE TABLE usage (
    id            BIGSERIAL PRIMARY KEY,
    user_id       TEXT NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    team_id       TEXT NOT NULL REFERENCES teams (id) ON DELETE CASCADE,
    kind          TEXT NOT NULL,
    model         TEXT NOT NULL,
    input_tokens  BIGINT NOT NULL,
    output_tokens BIGINT NOT NULL,
    created_at    BIGINT NOT NULL
);
CREATE INDEX usage_team_time ON usage (team_id, created_at);
CREATE INDEX usage_user_time ON usage (user_id, created_at);
