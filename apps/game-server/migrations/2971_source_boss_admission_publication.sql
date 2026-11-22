-- The source lever pays the party cooldown and reserves its publication in one
-- transaction. The next ready runtime owner can compensate a reservation after
-- the original process is gone. Rows predating this migration stay paid.
CREATE TABLE game_source_boss_admissions (
    admission UUID PRIMARY KEY CHECK (game_character_is_uuid_v7(admission)),
    scope_key BYTEA NOT NULL CHECK (octet_length(scope_key)=33),
    scope_generation NUMERIC(20,0) NOT NULL
        CHECK (scope_generation BETWEEN 1 AND 18446744073709551615),
    boss_key TEXT COLLATE "C" NOT NULL CHECK (octet_length(boss_key) BETWEEN 1 AND 512),
    -- One to five Character UUIDs, concatenated in Character lock order.
    party BYTEA NOT NULL CHECK (octet_length(party) BETWEEN 16 AND 80 AND octet_length(party)%16=0),
    state SMALLINT NOT NULL CHECK (state IN (1,2,3))
);
CREATE INDEX game_source_boss_unpublished
    ON game_source_boss_admissions(scope_key,scope_generation,admission) WHERE state=1;

REVOKE ALL ON game_source_boss_admissions FROM PUBLIC;
GRANT SELECT, INSERT ON game_source_boss_admissions TO oteryn_game_runtime;
GRANT UPDATE (state) ON game_source_boss_admissions TO oteryn_game_runtime;
