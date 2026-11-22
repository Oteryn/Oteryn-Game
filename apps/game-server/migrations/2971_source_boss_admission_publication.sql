-- The exact paid admission and its party cooldowns commit together.
-- This receipt remains paid if the Channel owner is lost (BOSS-RAID-0 §6.4).
CREATE TABLE game_source_boss_admissions (
    admission UUID PRIMARY KEY CHECK (game_character_is_uuid_v7(admission)),
    scope_key BYTEA NOT NULL CHECK (octet_length(scope_key)=33),
    scope_generation NUMERIC(20,0) NOT NULL
        CHECK (scope_generation BETWEEN 1 AND 18446744073709551615),
    boss_key TEXT COLLATE "C" NOT NULL CHECK (octet_length(boss_key) BETWEEN 1 AND 512),
    -- One to five Character UUIDs, concatenated in Character lock order.
    party BYTEA NOT NULL CHECK (octet_length(party) BETWEEN 16 AND 80 AND octet_length(party)%16=0)
);

REVOKE ALL ON game_source_boss_admissions FROM PUBLIC;
GRANT SELECT, INSERT ON game_source_boss_admissions TO oteryn_game_runtime;
