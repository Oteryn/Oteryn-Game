-- CHAR-POSITION-1: the durable last position of a Character (decision CHAR-POSITION-0,
-- `reviews/OTERYN_GAME_CHAR_POSITION0_LOGOUT_POSITION_DECISION_2026-09-30.md` §3.1 and §3.2).
--
-- Scope of this migration only:
--   * `game_character_last_positions`: one row per Character. It is a Character runtime-state
--     projection: it has no CharacterRevision, no cause and no receipt, and is never deleted;
--   * the order key (`character_lease_generation`, `write_sequence`). The writer applies an
--     upsert only when its key is greater than the stored key; a trigger refuses any UPDATE that
--     does not raise the key, or that changes the Character or its World;
--   * the World guard: `world_id` must be the Character root's World (no World table exists, so
--     the root stands in for it, as the death guard does);
--   * grants: the runtime role may SELECT, INSERT and UPDATE, never DELETE; the control role may
--     SELECT.
--
-- The position is the Channel movement cell (`x`, `y`, legacy `floor`), the same encoding the
-- pending respawn records. The frame and map digests and the server artifact digest it was
-- written under are kept so admission can refuse a cell of another coordinate frame; the map and
-- artifact digests are diagnostics only. No channel is stored (§3.1).
--
-- Rollback: applied migrations are immutable, so rollback is a new migration that drops the
-- table, its triggers and its functions. Nothing else reads the table.

CREATE TABLE game_character_last_positions (
    character_id UUID PRIMARY KEY REFERENCES game_character_roots (character_id),
    world_id UUID NOT NULL CHECK (game_character_is_uuid_v7(world_id)),
    tile_x INTEGER NOT NULL,
    tile_y INTEGER NOT NULL,
    tile_floor SMALLINT NOT NULL,
    coordinate_frame_digest BYTEA NOT NULL CHECK (octet_length(coordinate_frame_digest) = 32),
    map_revision_digest BYTEA NOT NULL CHECK (octet_length(map_revision_digest) = 32),
    server_artifact_digest BYTEA NOT NULL CHECK (octet_length(server_artifact_digest) = 32),
    character_lease_generation NUMERIC(20,0) NOT NULL
        CHECK (character_lease_generation BETWEEN 1 AND 18446744073709551615),
    write_sequence NUMERIC(20,0) NOT NULL
        CHECK (write_sequence BETWEEN 1 AND 18446744073709551615)
);

-- §3.1: the row's World is the Character's World.
CREATE FUNCTION game_character_last_position_world_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM game_character_roots r
                    WHERE r.character_id = NEW.character_id
                      AND r.world_id = NEW.world_id) THEN
        RAISE EXCEPTION 'a last position names the World of its Character'
            USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END;
$$;

-- §3.2 order: an UPDATE keeps the Character and its World and strictly raises the key.
CREATE FUNCTION game_character_last_position_order_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.character_id IS DISTINCT FROM OLD.character_id
       OR NEW.world_id IS DISTINCT FROM OLD.world_id THEN
        RAISE EXCEPTION 'a last position keeps its Character and World'
            USING ERRCODE = '23514';
    END IF;
    IF (NEW.character_lease_generation, NEW.write_sequence)
       <= (OLD.character_lease_generation, OLD.write_sequence) THEN
        RAISE EXCEPTION 'a last position write must order after the stored one'
            USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER game_character_last_position_world BEFORE INSERT OR UPDATE
    ON game_character_last_positions FOR EACH ROW
    EXECUTE FUNCTION game_character_last_position_world_guard();
CREATE TRIGGER game_character_last_position_order BEFORE UPDATE
    ON game_character_last_positions FOR EACH ROW
    EXECUTE FUNCTION game_character_last_position_order_guard();
CREATE TRIGGER game_character_last_positions_no_truncate BEFORE TRUNCATE
    ON game_character_last_positions EXECUTE FUNCTION game_character_reject_truncate();

DO $$
DECLARE
    v_function TEXT;
BEGIN
    FOREACH v_function IN ARRAY ARRAY[
        'game_character_last_position_world_guard()',
        'game_character_last_position_order_guard()'
    ] LOOP
        EXECUTE format('ALTER FUNCTION %s SET search_path = %I, pg_temp', v_function, current_schema());
    END LOOP;
END $$;

REVOKE ALL ON game_character_last_positions FROM PUBLIC;
REVOKE ALL ON FUNCTION
    game_character_last_position_world_guard(),
    game_character_last_position_order_guard()
FROM PUBLIC;
GRANT SELECT, INSERT, UPDATE ON game_character_last_positions TO oteryn_game_runtime;
GRANT SELECT ON game_character_last_positions TO oteryn_game_control;
