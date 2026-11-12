-- W17: Drome has its own points, immutable command receipts and World rotation rankings.
CREATE TABLE game_character_tibiadrome (
    character_id UUID PRIMARY KEY REFERENCES game_character_roots(character_id),
    body JSONB NOT NULL CHECK(jsonb_typeof(body)='object')
);
CREATE TABLE game_tibiadrome_receipts (
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    game_session_id UUID NOT NULL,
    command_id NUMERIC(20,0) NOT NULL CHECK(command_id BETWEEN 1 AND 18446744073709551615),
    payload BYTEA NOT NULL CHECK(octet_length(payload)<=32),
    reply BYTEA NOT NULL CHECK(octet_length(reply) BETWEEN 1 AND 128),
    PRIMARY KEY(game_session_id,command_id)
);
CREATE INDEX game_tibiadrome_receipts_character ON game_tibiadrome_receipts(character_id);
CREATE TABLE game_tibiadrome_scores (
    world_id UUID NOT NULL,
    rotation BIGINT NOT NULL CHECK(rotation>=0),
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    level BIGINT NOT NULL CHECK(level BETWEEN 1 AND 4294967295),
    experience BIGINT NOT NULL CHECK(experience>=0),
    PRIMARY KEY(world_id,rotation,character_id)
);
CREATE INDEX game_tibiadrome_scores_character ON game_tibiadrome_scores(character_id);
CREATE INDEX game_tibiadrome_scores_rank ON game_tibiadrome_scores(world_id,rotation,level DESC,experience,character_id);
CREATE TABLE game_tibiadrome_awards (
    world_id UUID NOT NULL,
    rotation BIGINT NOT NULL,
    character_id UUID NOT NULL,
    rank BIGINT NOT NULL CHECK(rank>0),
    potions INTEGER NOT NULL CHECK(potions BETWEEN 0 AND 3),
    claimed INTEGER NOT NULL DEFAULT 0 CHECK(claimed BETWEEN 0 AND potions),
    PRIMARY KEY(world_id,rotation,character_id),
    FOREIGN KEY(world_id,rotation,character_id) REFERENCES game_tibiadrome_scores(world_id,rotation,character_id)
);
CREATE INDEX game_tibiadrome_awards_character ON game_tibiadrome_awards(character_id,rotation);
REVOKE ALL ON game_character_tibiadrome,game_tibiadrome_receipts,game_tibiadrome_scores,game_tibiadrome_awards FROM PUBLIC;
GRANT SELECT,INSERT,UPDATE ON game_character_tibiadrome,game_tibiadrome_receipts,game_tibiadrome_scores,game_tibiadrome_awards TO oteryn_game_runtime;
GRANT SELECT ON game_character_tibiadrome,game_tibiadrome_receipts,game_tibiadrome_scores,game_tibiadrome_awards TO oteryn_game_control;
CREATE TABLE game_tibiadrome_supply_receipts (
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    game_session_id UUID NOT NULL,
    command_id NUMERIC(20,0) NOT NULL CHECK(command_id BETWEEN 1 AND 18446744073709551615),
    binding BYTEA NOT NULL CHECK(octet_length(binding)=32),
    shape SMALLINT NOT NULL CHECK(shape BETWEEN 1 AND 4),
    PRIMARY KEY(game_session_id,command_id)
);
CREATE INDEX game_tibiadrome_supply_character ON game_tibiadrome_supply_receipts(character_id);
REVOKE ALL ON game_tibiadrome_supply_receipts FROM PUBLIC;
GRANT SELECT,INSERT ON game_tibiadrome_supply_receipts TO oteryn_game_runtime;
GRANT SELECT ON game_tibiadrome_supply_receipts TO oteryn_game_control;

-- Finalization never locks another participant's Character root: award provenance
-- references the already committed, indexed score tuple above.
CREATE TABLE game_tibiadrome_rotations (
    world_id UUID NOT NULL,
    rotation BIGINT NOT NULL CHECK(rotation>=0),
    PRIMARY KEY(world_id,rotation)
);
REVOKE ALL ON game_tibiadrome_rotations FROM PUBLIC;
GRANT SELECT,INSERT ON game_tibiadrome_rotations TO oteryn_game_runtime;
GRANT SELECT ON game_tibiadrome_rotations TO oteryn_game_control;
