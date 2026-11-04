-- Crystal iologindata_save_player.cpp:264 and iologindata_load_player.cpp:287:
-- persistent conditions with remaining ticks survive logout and fresh login.
CREATE TABLE game_character_condition_snapshots (
    character_id uuid PRIMARY KEY REFERENCES game_character_roots(character_id) ON DELETE CASCADE,
    character_lease_generation numeric(20, 0) NOT NULL CHECK (character_lease_generation BETWEEN 1 AND 18446744073709551615),
    content_digest bytea NOT NULL CHECK (octet_length(content_digest) = 32),
    snapshot jsonb NOT NULL CHECK ((jsonb_typeof(snapshot) = 'object'
        AND snapshot->'schema' = '1'::jsonb
        AND jsonb_typeof(snapshot->'conditions') = 'array'
        AND jsonb_array_length(snapshot->'conditions') <= 32) IS TRUE),
    saved_at timestamptz NOT NULL DEFAULT clock_timestamp()
);
REVOKE ALL ON TABLE game_character_condition_snapshots FROM PUBLIC;
GRANT SELECT, INSERT, UPDATE, DELETE ON TABLE game_character_condition_snapshots TO oteryn_game_runtime;
GRANT SELECT, INSERT, UPDATE, DELETE ON TABLE game_character_condition_snapshots TO oteryn_game_control;
