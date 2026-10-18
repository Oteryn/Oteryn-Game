-- TUTORIAL-1: the durable Newhaven tutorial progress of a Character (CP D1426; scenario brief in
-- `docs/agents/tasks/TUTORIAL-1.md`; rules in `apps/game-server/src/domain/tutorial.rs`).
--
-- Scope of this migration only:
--   * `game_character_tutorial_progress`: at most one row per Character, written only for a
--     Character whose start is on the Newhaven start island. A Character without a row never
--     entered the tutorial (every StartThais Character). It is a Character runtime-state
--     projection: no CharacterRevision, no cause, no receipt; it is never deleted;
--   * `tutorial_step`: 1 islet, 2 island, 3 departed; a departed row has no pending track;
--   * `progress_version`: the compare-and-set version. The writer inserts version 1 or updates
--     only the stored version it read, so of two concurrent writers one loses with a typed
--     conflict; a trigger refuses an UPDATE that does not raise the version by exactly one, or
--     that changes the Character;
--   * `character_lease_generation`: the lease of the last write (diagnostic only);
--   * grants: the runtime role may SELECT, INSERT and UPDATE, never DELETE; the control role may
--     SELECT.
--
-- Rollback: applied migrations are immutable, so rollback is a new migration that drops the
-- table, its triggers and its function. Nothing else reads the table.

CREATE TABLE game_character_tutorial_progress (
    character_id UUID PRIMARY KEY REFERENCES game_character_roots (character_id),
    tutorial_step SMALLINT NOT NULL CHECK (tutorial_step BETWEEN 1 AND 3),
    hunting_hint_pending BOOLEAN NOT NULL,
    corruptor_loot_pending BOOLEAN NOT NULL,
    progress_version NUMERIC(20,0) NOT NULL
        CHECK (progress_version BETWEEN 1 AND 18446744073709551615),
    character_lease_generation NUMERIC(20,0) NOT NULL
        CHECK (character_lease_generation BETWEEN 1 AND 18446744073709551615),
    CHECK (tutorial_step <> 3 OR NOT (hunting_hint_pending OR corruptor_loot_pending))
);

-- An UPDATE keeps the Character and raises the version by exactly one.
CREATE FUNCTION game_character_tutorial_progress_version_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.character_id IS DISTINCT FROM OLD.character_id THEN
        RAISE EXCEPTION 'a tutorial progress keeps its Character'
            USING ERRCODE = '23514';
    END IF;
    IF NEW.progress_version IS DISTINCT FROM OLD.progress_version + 1 THEN
        RAISE EXCEPTION 'a tutorial progress write must follow the stored version'
            USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER game_character_tutorial_progress_version BEFORE UPDATE
    ON game_character_tutorial_progress FOR EACH ROW
    EXECUTE FUNCTION game_character_tutorial_progress_version_guard();
CREATE TRIGGER game_character_tutorial_progress_no_truncate BEFORE TRUNCATE
    ON game_character_tutorial_progress EXECUTE FUNCTION game_character_reject_truncate();

DO $$
BEGIN
    EXECUTE format(
        'ALTER FUNCTION game_character_tutorial_progress_version_guard() SET search_path = %I, pg_temp',
        current_schema());
END $$;

REVOKE ALL ON game_character_tutorial_progress FROM PUBLIC;
REVOKE ALL ON FUNCTION game_character_tutorial_progress_version_guard() FROM PUBLIC;
GRANT SELECT, INSERT, UPDATE ON game_character_tutorial_progress TO oteryn_game_runtime;
GRANT SELECT ON game_character_tutorial_progress TO oteryn_game_control;
