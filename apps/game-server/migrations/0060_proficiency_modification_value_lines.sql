-- PROF-SHAPE-1b / migration 0060 (lease 0060). PROFICIENCY-1B §7.1, §7.2 and §9: the dust
-- SPEND shape of a perk modification line.
--
-- 0055 pinned every line's dust and orb costs to 0 (`..._line_no_ledger`). This migration
-- replaces that constraint:
--   * dust: a line with `dust_spent > 0` has exactly one forge dust SPEND entry (0059) with cause
--     `proficiency`, the line's Character and occurrence and `amount = dust_spent`; a line with
--     `dust_spent = 0` has none; and every `proficiency` SPEND entry has its line. Both sides are
--     checked by one deferred guard, so the receipt, its lines and the entry commit together or
--     not at all, whatever order the writer inserts them in;
--   * orb: the orb BURN shape is not admitted yet, so `orb_cost = 0` stays pinned
--     (`..._line_no_orb_burn`); ORB_RANK stays closed until a later slice replaces it.
-- Index probes only: the line's occurrence (the entry's unique (character, cause, occurrence)
-- key and the line's primary key prefix).
--
-- Rollback: applied migrations are immutable, so rollback is a new migration. Before any line
-- with `dust_spent > 0` exists it restores `..._line_no_ledger` and drops the guard. After such
-- lines exist they and their entries are retained evidence of destroyed value: a rollback keeps
-- them and may only stop new dust spends by restoring the constraint `NOT VALID`.

ALTER TABLE game_character_proficiency_modification_lines
    DROP CONSTRAINT game_character_proficiency_modification_line_no_ledger,
    -- PROF-SHAPE-1b: no orb BURN shape exists yet.
    ADD CONSTRAINT game_character_proficiency_modification_line_no_orb_burn CHECK (orb_cost = 0);

-- Deferred: a line's dust and its SPEND entry agree at commit, from either side.
CREATE FUNCTION game_character_proficiency_modification_dust_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    v_character_id UUID;
    v_occurrence_id UUID;
    v_lines INTEGER;
    v_line_dust BIGINT;
    v_entries INTEGER;
    v_entry_amount BIGINT;
BEGIN
    IF TG_TABLE_NAME = 'game_character_forge_dust_entries' THEN
        IF NEW.cause <> 'proficiency' THEN
            RETURN NULL;
        END IF;
        IF NEW.kind <> 'SPEND' THEN
            RAISE EXCEPTION 'proficiency forge dust entry must be a SPEND' USING ERRCODE = '23514';
        END IF;
    END IF;
    v_character_id := NEW.character_id;
    v_occurrence_id := CASE TG_TABLE_NAME
        WHEN 'game_character_forge_dust_entries' THEN NEW.cause_occurrence_id
        ELSE NEW.proficiency_occurrence_id END;

    -- One receipt has at most one modification line per slot; a dust cost is spent once per
    -- receipt, so a receipt with dust carries exactly one line.
    SELECT count(*), coalesce(sum(dust_spent), 0) INTO v_lines, v_line_dust
      FROM game_character_proficiency_modification_lines
     WHERE proficiency_occurrence_id = v_occurrence_id AND character_id = v_character_id;
    SELECT count(*), coalesce(sum(amount), 0) INTO v_entries, v_entry_amount
      FROM game_character_forge_dust_entries
     WHERE character_id = v_character_id AND cause = 'proficiency'
       AND cause_occurrence_id = v_occurrence_id;

    IF v_line_dust > 0 AND v_lines <> 1 THEN
        RAISE EXCEPTION 'proficiency modification with dust must have exactly one line'
            USING ERRCODE = '23514';
    END IF;
    IF (v_line_dust = 0 AND v_entries <> 0)
       OR (v_line_dust > 0 AND (v_entries <> 1 OR v_entry_amount <> v_line_dust)) THEN
        RAISE EXCEPTION 'proficiency modification dust does not equal its forge dust SPEND'
            USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END;
$$;

CREATE CONSTRAINT TRIGGER game_character_proficiency_modification_line_dust_spent
    AFTER INSERT ON game_character_proficiency_modification_lines
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
    EXECUTE FUNCTION game_character_proficiency_modification_dust_guard();
CREATE CONSTRAINT TRIGGER game_character_forge_dust_entry_proficiency_line
    AFTER INSERT ON game_character_forge_dust_entries
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
    EXECUTE FUNCTION game_character_proficiency_modification_dust_guard();

DO $$
BEGIN
    EXECUTE format('ALTER FUNCTION game_character_proficiency_modification_dust_guard() '
        'SET search_path = %I, pg_temp', current_schema());
END $$;

REVOKE ALL ON FUNCTION game_character_proficiency_modification_dust_guard() FROM PUBLIC;
