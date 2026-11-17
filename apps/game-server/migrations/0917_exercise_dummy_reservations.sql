CREATE TABLE game_exercise_dummy_reservations (
 world_id UUID NOT NULL,
 base_bundle_digest BYTEA NOT NULL CHECK (octet_length(base_bundle_digest)=32),
 placement_key NUMERIC(20,0) NOT NULL CHECK (placement_key BETWEEN 0 AND 18446744073709551615),
 reset_epoch NUMERIC(20,0) NOT NULL,
 character_id UUID NOT NULL REFERENCES game_character_roots(character_id) ON DELETE CASCADE,
 game_session_id UUID NOT NULL,
 lease_until_ms BIGINT NOT NULL CHECK (lease_until_ms > 0),
 PRIMARY KEY (world_id,base_bundle_digest,placement_key)
);
CREATE INDEX game_exercise_dummy_reservations_character_idx
 ON game_exercise_dummy_reservations(character_id);
REVOKE ALL ON game_exercise_dummy_reservations FROM PUBLIC;
GRANT SELECT,INSERT,UPDATE,DELETE ON game_exercise_dummy_reservations TO oteryn_game_runtime;
GRANT SELECT ON game_exercise_dummy_reservations TO oteryn_game_control;
