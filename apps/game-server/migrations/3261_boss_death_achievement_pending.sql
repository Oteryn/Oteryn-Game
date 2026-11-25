CREATE TABLE game_boss_death_achievement_pending (
    scope_key BYTEA NOT NULL CHECK (octet_length(scope_key) = 33),
    death BYTEA PRIMARY KEY CHECK (octet_length(death) IN (52, 53)),
    generation NUMERIC(20, 0) NOT NULL CHECK (generation BETWEEN 1 AND 18446744073709551615),
    boss SMALLINT NOT NULL CHECK (boss BETWEEN 1 AND 3),
    recipients JSONB NOT NULL CHECK (jsonb_typeof(recipients) = 'array')
);
CREATE INDEX game_boss_death_achievement_pending_scope
    ON game_boss_death_achievement_pending (scope_key, death);
REVOKE ALL ON game_boss_death_achievement_pending FROM PUBLIC;
GRANT SELECT, INSERT, DELETE ON game_boss_death_achievement_pending TO oteryn_game_runtime;
GRANT SELECT ON game_boss_death_achievement_pending TO oteryn_game_control;
