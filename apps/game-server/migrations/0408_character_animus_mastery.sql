-- ANIMUS-1: monotonic, per-character Animus Mastery completions.
CREATE TABLE game_character_animus_masteries (
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    species_key TEXT NOT NULL CHECK (
        length(species_key) BETWEEN 1 AND 160
        AND species_key = lower(species_key)
        AND species_key ~ '^[a-z0-9:_.-]+$'
    ),
    completed_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    PRIMARY KEY (character_id, species_key)
);

CREATE FUNCTION game_character_animus_mastery_immutable() RETURNS trigger
LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'animus mastery completions are immutable'; END $$;

CREATE TRIGGER game_character_animus_mastery_immutable
    BEFORE UPDATE OR DELETE ON game_character_animus_masteries
    FOR EACH ROW EXECUTE FUNCTION game_character_animus_mastery_immutable();
CREATE TRIGGER game_character_animus_mastery_no_truncate
    BEFORE TRUNCATE ON game_character_animus_masteries
    EXECUTE FUNCTION game_character_reject_truncate();

DO $$ BEGIN
    EXECUTE format(
        'ALTER FUNCTION game_character_animus_mastery_immutable() SET search_path=pg_catalog,%I,pg_temp',
        current_schema()
    );
END $$;

REVOKE ALL ON game_character_animus_masteries FROM PUBLIC;
REVOKE ALL ON FUNCTION game_character_animus_mastery_immutable() FROM PUBLIC;
GRANT SELECT, INSERT ON game_character_animus_masteries TO oteryn_game_runtime;
GRANT SELECT ON game_character_animus_masteries TO oteryn_game_control;
