-- PVP-1b (PARTY-PVP-0 §6.2, §8.1 kill block, §9; PVP-0 §6.1 white skull, D971; D1443 windows):
-- the death-transaction writers of the skull, the kill block, the unjustified points and the
-- revenge marks, and the World job's expired-skull normalization.
--
-- Scope of this migration only:
--   * runtime grants for the writes of the victim's death transaction: the skull, its deadline and
--     the kill block of a contributor; a point row and a revenge mark (the 0081 receipt guard
--     already binds both to the victim's death receipt of the same transaction and the victim's
--     ledger);
--   * a state guard: a skull or kill block is changed only by a death transaction of the row's
--     World that names the row's Character in the victim's ledger window. The World job may
--     clear a skull whose deadline has passed (skull 0, no deadline); nothing else is relaxed.
--     The runtime holds no INSERT grant on these columns (0081), so a new row starts without
--     them and the death transaction updates it;
--   * an index for the World job's expired-skull pass.
-- No CharacterRevision advance (PARTY-PVP-0 §6.2 composition rule 1 amendment). Avenging a
-- revenge mark (an UPDATE, refused by 0081) stays with PVP-DEATH-1.
--
-- Rollback: applied migrations are immutable, so rollback is a new migration that drops the
-- trigger, its function and the index and revokes the grants below.

CREATE FUNCTION game_character_pvp_death_write_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.kill_block_until IS DISTINCT FROM OLD.kill_block_until
       OR ((NEW.skull, NEW.skull_until) IS DISTINCT FROM (OLD.skull, OLD.skull_until)
           -- The World job's normalization of an expired skull.
           AND NOT (NEW.skull = 0 AND NEW.skull_until IS NULL
                    AND OLD.skull_until <= clock_timestamp())) THEN
        IF NOT EXISTS (SELECT 1 FROM game_character_death_receipts d
                         JOIN game_character_pvp_ledger l
                           ON l.victim_character_id = d.character_id
                          AND l.contributor_character_id = NEW.character_id
                          AND l.last_at >= clock_timestamp() - INTERVAL '5 minutes'
                        WHERE d.created_xact_id = pg_current_xact_id()
                          AND d.death_world_id = NEW.world_id) THEN
            RAISE EXCEPTION 'a skull or kill block is written by the death transaction of a victim'
                USING ERRCODE = '23514';
        END IF;
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER game_character_pvp_state_death_write_guard BEFORE UPDATE
    ON game_character_pvp_state FOR EACH ROW
    EXECUTE FUNCTION game_character_pvp_death_write_guard();

DO $$
BEGIN
    EXECUTE format('ALTER FUNCTION game_character_pvp_death_write_guard() SET search_path = %I, pg_temp',
                   current_schema());
END $$;
REVOKE ALL ON FUNCTION game_character_pvp_death_write_guard() FROM PUBLIC;

CREATE INDEX game_character_pvp_state_skull_expiry
    ON game_character_pvp_state (skull_until, character_id) WHERE skull > 0;

GRANT UPDATE (skull, skull_until, kill_block_until) ON game_character_pvp_state TO oteryn_game_runtime;
GRANT INSERT (character_id, death_occurrence_id, victim_character_id, points_milli)
    ON game_character_unjustified_points TO oteryn_game_runtime;
GRANT INSERT (victim_character_id, killer_character_id, death_occurrence_id)
    ON game_character_revenge_marks TO oteryn_game_runtime;
