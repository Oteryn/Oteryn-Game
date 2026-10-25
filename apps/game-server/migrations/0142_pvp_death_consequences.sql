-- PVP-1b: only the victim's death transaction may raise a contributor's skull
-- or kill block. Existing 0081 guards retain monotonic deadlines and revisions.
-- Runtime classification and the live-PvP activation gate are unchanged.
CREATE FUNCTION game_character_pvp_death_update_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.skull IS NOT DISTINCT FROM OLD.skull
       AND NEW.skull_until IS NOT DISTINCT FROM OLD.skull_until
       AND NEW.kill_block_until IS NOT DISTINCT FROM OLD.kill_block_until THEN
        RETURN NEW;
    END IF;
    -- A scoped World expiry pass can normalize precisely an expired skull.
    IF OLD.skull > 0 AND OLD.skull_until <= clock_timestamp()
       AND NEW.skull = 0 AND NEW.skull_until IS NULL
       AND NEW.kill_block_until IS NOT DISTINCT FROM OLD.kill_block_until THEN
        RETURN NEW;
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM game_character_death_receipts d
        JOIN game_character_pvp_ledger l ON l.victim_character_id = d.character_id
        WHERE d.created_xact_id = pg_current_xact_id()
          AND d.death_world_id = NEW.world_id
          AND l.contributor_character_id = NEW.character_id
          AND l.last_at >= clock_timestamp() - interval '5 minutes'
          AND l.last_at <= clock_timestamp()
    ) THEN
        RAISE EXCEPTION 'a skull or kill block changes only in its contributor death transaction'
            USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER game_character_pvp_death_update_guard BEFORE UPDATE
    ON game_character_pvp_state FOR EACH ROW
    EXECUTE FUNCTION game_character_pvp_death_update_guard();
CREATE INDEX game_character_pvp_skull_expiry
    ON game_character_pvp_state (world_id, skull_until, character_id) WHERE skull > 0;

DO $$
BEGIN
    EXECUTE format('ALTER FUNCTION game_character_pvp_death_update_guard() SET search_path = %I, pg_temp', current_schema());
END $$;
REVOKE ALL ON FUNCTION game_character_pvp_death_update_guard() FROM PUBLIC;
GRANT UPDATE (skull, skull_until, kill_block_until) ON game_character_pvp_state
    TO oteryn_game_runtime;
GRANT INSERT ON game_character_unjustified_points, game_character_revenge_marks
    TO oteryn_game_runtime;
