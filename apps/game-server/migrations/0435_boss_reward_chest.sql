-- REWARDCHEST2: personal reward-boss container projection. The death settlement owns inserts;
-- the admitted Character may only read its own live groups through the fenced durability owner.
CREATE TABLE game_character_boss_reward_chests (
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    world_id UUID NOT NULL,
    death_id UUID NOT NULL,
    boss_key TEXT NOT NULL CHECK (boss_key ~ '^[A-Za-z0-9_./-]+:[A-Za-z0-9_.:/-]+$' AND octet_length(boss_key) <= 512),
    minted_at BIGINT NOT NULL CHECK (minted_at >= 0),
    expires_at BIGINT NOT NULL CHECK (expires_at = minted_at + 604800000),
    item_count INTEGER NOT NULL CHECK (item_count BETWEEN 0 AND 1000),
    PRIMARY KEY (character_id, world_id, death_id)
);
CREATE INDEX game_character_boss_reward_chests_live
    ON game_character_boss_reward_chests(character_id, world_id, expires_at, death_id);
REVOKE ALL ON game_character_boss_reward_chests FROM PUBLIC;
GRANT SELECT, INSERT, UPDATE, DELETE ON game_character_boss_reward_chests TO oteryn_game_runtime;
GRANT SELECT ON game_character_boss_reward_chests TO oteryn_game_control;
