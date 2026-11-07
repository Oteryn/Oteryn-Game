-- P12c: bounded, server-produced observations of the existing combat/equipment owners.
-- This is a read model, not a second combat authority. A Character has at most one sample.
-- Crystal 201189f protocolgame.cpp:5016-5325 reports current stats, not damage counters.
CREATE TABLE game_character_cyclopedia_combat (
    character_id UUID PRIMARY KEY REFERENCES game_character_roots(character_id),
    game_session_id UUID NOT NULL,
    character_lease_generation BIGINT NOT NULL CHECK (character_lease_generation > 0),
    connection_generation NUMERIC(20, 0) NOT NULL CHECK (connection_generation > 0),
    observed_owner_micros NUMERIC(20, 0) NOT NULL CHECK (observed_owner_micros >= 0),
    observed_at_unix_ms BIGINT NOT NULL CHECK (observed_at_unix_ms >= 0),
    character_revision NUMERIC(20, 0) NOT NULL CHECK (character_revision > 0),
    equipment_revision NUMERIC(20, 0) NOT NULL CHECK (equipment_revision > 0),
    content_digest BYTEA NOT NULL CHECK (octet_length(content_digest) = 32),
    input_sha256 BYTEA NOT NULL CHECK (octet_length(input_sha256) = 32),
    sample_revision BIGINT NOT NULL CHECK (sample_revision > 0),
    offence BYTEA NOT NULL CHECK (octet_length(offence) BETWEEN 26 AND 16384 AND get_byte(offence, 0) = 8),
    defence BYTEA NOT NULL CHECK (octet_length(defence) BETWEEN 26 AND 16384 AND get_byte(defence, 0) = 9),
    misc BYTEA NOT NULL CHECK (octet_length(misc) BETWEEN 26 AND 16384 AND get_byte(misc, 0) = 10)
);

REVOKE ALL ON game_character_cyclopedia_combat FROM PUBLIC;
GRANT SELECT, INSERT, UPDATE ON game_character_cyclopedia_combat TO oteryn_game_runtime;
