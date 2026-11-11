-- WU26 T188/T197 (capability 492): Moonsilver first-drop pity, per character and per boss
-- under the Boss Difficulty System. The boss reward settlement transaction writes it beside
-- the personal reward rows it settles; the rule is oteryn-gameplay-encounter-rules boss::pity
-- (owner decision 2026-11-11; no official figures published).
CREATE TABLE game_character_boss_pity (
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    boss_key TEXT NOT NULL CHECK (octet_length(boss_key) BETWEEN 1 AND 512),
    kills_without_drop SMALLINT NOT NULL DEFAULT 0 CHECK (kills_without_drop >= 0),
    first_drop_received BOOLEAN NOT NULL DEFAULT false,
    PRIMARY KEY (character_id, boss_key)
);
REVOKE ALL ON game_character_boss_pity FROM PUBLIC;
GRANT SELECT, INSERT, UPDATE ON game_character_boss_pity TO oteryn_game_runtime;
GRANT SELECT ON game_character_boss_pity TO oteryn_game_control;
