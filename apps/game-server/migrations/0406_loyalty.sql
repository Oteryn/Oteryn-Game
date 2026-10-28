-- LOYALTY: Platform-owned earning projected into Game. Game only consumes this account value.
-- Platform contract: upsert the absolute, non-negative spent-Premium-day total idempotently;
-- referrals and all earning eligibility remain Platform authority.
CREATE TABLE game_account_loyalty_projections (
    account_id UUID PRIMARY KEY REFERENCES game_character_account_guards(account_id),
    loyalty_points BIGINT NOT NULL CHECK (loyalty_points BETWEEN 0 AND 2147483647),
    platform_revision BIGINT NOT NULL CHECK (platform_revision > 0),
    projected_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp()
);

REVOKE INSERT, UPDATE, DELETE, TRUNCATE ON game_account_loyalty_projections FROM PUBLIC;
GRANT SELECT ON game_account_loyalty_projections TO oteryn_game_runtime, oteryn_game_control;
