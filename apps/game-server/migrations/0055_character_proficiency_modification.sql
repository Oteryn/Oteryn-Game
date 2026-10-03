-- PROF-SHAPE-1a / migration 0055 (lease D353). PROFICIENCY-1B §4, §6.3, §7.1, §8 and §9.
-- Perk modification rows, their immutable lines, terminal records of refused occurrences and
-- the retained shaping revision set. `level` is the zero-based perk level of the track, as the
-- selection array index. No value ledger exists before FORGE-1: every line pins dust and orb
-- costs to 0 (`..._line_no_ledger`), and PROF-SHAPE-1b replaces that constraint with
-- the dust SPEND and orb BURN guards. The shared progression guard is not replaced: a
-- `perk_modification` receipt is a row of `game_character_proficiency_receipts`, which the 0032
-- body already counts in the CharacterRevision chain.
-- Rollback after receipts exist may only stop new writes: preserve the receipt chain.

-- §4.3: the reserved cause joins the receipt and track line CHECKs.
ALTER TABLE game_character_proficiency_receipts
    DROP CONSTRAINT game_character_proficiency_receipts_cause_check,
    ADD CONSTRAINT game_character_proficiency_receipts_cause_check
        CHECK (cause IN ('training', 'perk_selection', 'migration', 'perk_modification'));
ALTER TABLE game_character_proficiency_receipt_lines
    DROP CONSTRAINT game_character_proficiency_receipt_lines_cause_check,
    ADD CONSTRAINT game_character_proficiency_receipt_lines_cause_check
        CHECK (cause IN ('training', 'perk_selection', 'migration', 'perk_modification')),
    DROP CONSTRAINT game_character_proficiency_line_cause_direction,
    ADD CONSTRAINT game_character_proficiency_line_cause_direction CHECK (CASE cause
        WHEN 'training' THEN progress_after > progress_before
            AND definition_revision_after = definition_revision_before
            AND selections_after IS NOT DISTINCT FROM selections_before
        WHEN 'perk_selection' THEN progress_after = progress_before
            AND definition_revision_after = definition_revision_before
            AND cardinality(selections_after) = cardinality(selections_before)
            AND ((selections_before[1] IS DISTINCT FROM selections_after[1])::INTEGER
              + (selections_before[2] IS DISTINCT FROM selections_after[2])::INTEGER
              + (selections_before[3] IS DISTINCT FROM selections_after[3])::INTEGER
              + (selections_before[4] IS DISTINCT FROM selections_after[4])::INTEGER
              + (selections_before[5] IS DISTINCT FROM selections_after[5])::INTEGER
              + (selections_before[6] IS DISTINCT FROM selections_after[6])::INTEGER
              + (selections_before[7] IS DISTINCT FROM selections_after[7])::INTEGER) = 1
        WHEN 'migration' THEN progress_after = progress_before
            AND definition_revision_after <> definition_revision_before
        -- PROF-SHAPE-1a: the track line only advances the track's committed revision.
        WHEN 'perk_modification' THEN progress_after = progress_before
            AND definition_revision_after = definition_revision_before
            AND selections_after IS NOT DISTINCT FROM selections_before
        ELSE FALSE
    END);

-- §8: the retained shaping revisions. Exactly one active revision per shaping key; a revision
-- leaves the set only through game_proficiency_shaping_activate while no row references it.
CREATE TABLE game_proficiency_shaping_revisions (
    shaping_key TEXT NOT NULL CHECK (shaping_key ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'
        AND shaping_key LIKE 'oteryn:proficiency-shaping.%' AND length(shaping_key) > 27),
    shaping_revision TEXT NOT NULL
        CHECK (shaping_revision ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'),
    active BOOLEAN NOT NULL,
    PRIMARY KEY (shaping_key, shaping_revision)
);
CREATE UNIQUE INDEX game_proficiency_shaping_one_active
    ON game_proficiency_shaping_revisions (shaping_key) WHERE active;

-- §4.1. Rows are never deleted; CLEAR stores the cleared state (all value columns NULL).
CREATE TABLE game_character_proficiency_modifications (
    character_id UUID NOT NULL,
    item_key TEXT NOT NULL,
    slot SMALLINT NOT NULL CHECK (slot IN (1, 2)),
    level SMALLINT CHECK (level BETWEEN 0 AND 6),
    shaping_key TEXT CHECK (shaping_key ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'
        AND shaping_key LIKE 'oteryn:proficiency-shaping.%' AND length(shaping_key) > 27),
    shaping_revision TEXT CHECK (shaping_revision ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'),
    entry_index SMALLINT CHECK (entry_index BETWEEN 0 AND 63),
    rank SMALLINT CHECK (rank BETWEEN 1 AND 10),
    pending_offer SMALLINT[],
    committed_character_revision NUMERIC(20,0) NOT NULL
        CHECK (committed_character_revision BETWEEN 2 AND 18446744073709551615),
    last_proficiency_occurrence_id UUID NOT NULL
        CHECK (game_character_is_uuid_v7(last_proficiency_occurrence_id)),
    CONSTRAINT game_character_proficiency_modification_shape CHECK (
        (level IS NULL AND shaping_key IS NULL AND shaping_revision IS NULL
            AND entry_index IS NULL AND rank IS NULL AND pending_offer IS NULL)
        OR (level IS NOT NULL AND shaping_key IS NOT NULL AND shaping_revision IS NOT NULL
            AND entry_index IS NOT NULL AND rank IS NOT NULL
            AND (pending_offer IS NULL OR (
                array_ndims(pending_offer) = 1 AND array_lower(pending_offer, 1) = 1
                AND cardinality(pending_offer) = 3
                AND array_position(pending_offer, NULL::SMALLINT) IS NULL
                AND 0 <= ALL(pending_offer) AND 63 >= ALL(pending_offer)
                AND pending_offer[1] <> pending_offer[2] AND pending_offer[1] <> pending_offer[3]
                AND pending_offer[2] <> pending_offer[3]
                AND entry_index <> ALL(pending_offer))))),
    PRIMARY KEY (character_id, item_key, slot),
    UNIQUE (character_id, item_key, level),
    FOREIGN KEY (character_id, item_key) REFERENCES game_character_proficiency (character_id, item_key)
);
CREATE INDEX game_character_proficiency_modifications_shaping
    ON game_character_proficiency_modifications (shaping_key, shaping_revision);

-- §4.2 and §7.1. One line per changed row; before and after are each a cleared or modified
-- state with the row's shape rule. Costs are copied from the bound shaping revision.
CREATE TABLE game_character_proficiency_modification_lines (
    proficiency_occurrence_id UUID NOT NULL,
    character_id UUID NOT NULL,
    committed_character_revision NUMERIC(20,0) NOT NULL,
    cause TEXT NOT NULL CHECK (cause IN ('perk_modification', 'migration')),
    item_key TEXT NOT NULL,
    slot SMALLINT NOT NULL CHECK (slot IN (1, 2)),
    operation TEXT NOT NULL CHECK (operation IN ('MODIFY', 'RANK_UP', 'ORB_RANK', 'RESHAPE_OFFER',
        'RESHAPE_CHOOSE', 'RESHAPE_DECLINE', 'CLEAR', 'MIGRATION_CLEAR')),
    -- The shaping revision the command bound (§6.2), so its binding recomputes; NULL only for
    -- MIGRATION_CLEAR, which no command binds.
    bound_shaping_revision TEXT
        CHECK (bound_shaping_revision ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'),
    level_before SMALLINT CHECK (level_before BETWEEN 0 AND 6),
    shaping_key_before TEXT,
    shaping_revision_before TEXT,
    entry_index_before SMALLINT CHECK (entry_index_before BETWEEN 0 AND 63),
    rank_before SMALLINT CHECK (rank_before BETWEEN 1 AND 10),
    pending_offer_before SMALLINT[],
    level_after SMALLINT CHECK (level_after BETWEEN 0 AND 6),
    shaping_key_after TEXT,
    shaping_revision_after TEXT,
    entry_index_after SMALLINT CHECK (entry_index_after BETWEEN 0 AND 63),
    rank_after SMALLINT CHECK (rank_after BETWEEN 1 AND 10),
    pending_offer_after SMALLINT[],
    dust_cost BIGINT NOT NULL CHECK (dust_cost >= 0),
    dust_spent BIGINT NOT NULL,
    orb_cost SMALLINT NOT NULL CHECK (orb_cost IN (0, 1)),
    orbs_spent SMALLINT NOT NULL,
    CONSTRAINT game_character_proficiency_modification_line_before_shape CHECK (
        (level_before IS NULL AND shaping_key_before IS NULL AND shaping_revision_before IS NULL
            AND entry_index_before IS NULL AND rank_before IS NULL AND pending_offer_before IS NULL)
        OR (level_before IS NOT NULL AND shaping_key_before IS NOT NULL
            AND shaping_revision_before IS NOT NULL AND entry_index_before IS NOT NULL
            AND rank_before IS NOT NULL)),
    CONSTRAINT game_character_proficiency_modification_line_after_shape CHECK (
        (level_after IS NULL AND shaping_key_after IS NULL AND shaping_revision_after IS NULL
            AND entry_index_after IS NULL AND rank_after IS NULL AND pending_offer_after IS NULL)
        OR (level_after IS NOT NULL AND shaping_key_after IS NOT NULL
            AND shaping_revision_after IS NOT NULL AND entry_index_after IS NOT NULL
            AND rank_after IS NOT NULL)),
    CONSTRAINT game_character_proficiency_modification_line_offer_shape CHECK (
        (pending_offer_before IS NULL OR (array_ndims(pending_offer_before) = 1
            AND array_lower(pending_offer_before, 1) = 1 AND cardinality(pending_offer_before) = 3
            AND array_position(pending_offer_before, NULL::SMALLINT) IS NULL))
        AND (pending_offer_after IS NULL OR (array_ndims(pending_offer_after) = 1
            AND array_lower(pending_offer_after, 1) = 1 AND cardinality(pending_offer_after) = 3
            AND array_position(pending_offer_after, NULL::SMALLINT) IS NULL))),
    CONSTRAINT game_character_proficiency_modification_line_cause CHECK (
        (cause = 'migration') = (operation = 'MIGRATION_CLEAR')
        AND (operation = 'MIGRATION_CLEAR') = (bound_shaping_revision IS NULL)),
    -- §7.1: costs are bound and spent exactly; only ORB_RANK burns an orb; the three
    -- free operations cost nothing.
    CONSTRAINT game_character_proficiency_modification_line_cost CHECK (
        dust_spent = dust_cost AND orbs_spent = orb_cost
        AND (operation = 'ORB_RANK' OR orb_cost = 0)
        AND (operation NOT IN ('RESHAPE_CHOOSE', 'RESHAPE_DECLINE', 'MIGRATION_CLEAR')
             OR dust_cost = 0)),
    -- PROF-SHAPE-1a: no dust ledger or orb BURN shape exists yet (FORGE-1, PROF-SHAPE-1b).
    CONSTRAINT game_character_proficiency_modification_line_no_ledger CHECK (
        dust_cost = 0 AND orb_cost = 0),
    -- §7.1: per operation, only the fields it may change change.
    CONSTRAINT game_character_proficiency_modification_line_direction CHECK (CASE operation
        WHEN 'MODIFY' THEN level_before IS NULL AND level_after IS NOT NULL AND rank_after = 1
            AND pending_offer_after IS NULL AND shaping_revision_after = bound_shaping_revision
        WHEN 'RANK_UP' THEN level_before IS NOT NULL AND pending_offer_before IS NULL
            AND pending_offer_after IS NULL AND rank_after = rank_before + 1
            AND (level_after, shaping_key_after, shaping_revision_after, entry_index_after)
                IS NOT DISTINCT FROM
                (level_before, shaping_key_before, shaping_revision_before, entry_index_before)
        WHEN 'ORB_RANK' THEN level_before IS NOT NULL AND pending_offer_before IS NULL
            AND pending_offer_after IS NULL AND rank_before < 10 AND rank_after = 10
            AND (level_after, shaping_key_after, shaping_revision_after, entry_index_after)
                IS NOT DISTINCT FROM
                (level_before, shaping_key_before, shaping_revision_before, entry_index_before)
        WHEN 'RESHAPE_OFFER' THEN level_before IS NOT NULL AND pending_offer_before IS NULL
            AND pending_offer_after IS NOT NULL
            AND (level_after, shaping_key_after, shaping_revision_after, entry_index_after,
                 rank_after) IS NOT DISTINCT FROM
                (level_before, shaping_key_before, shaping_revision_before, entry_index_before,
                 rank_before)
        WHEN 'RESHAPE_CHOOSE' THEN pending_offer_before IS NOT NULL AND pending_offer_after IS NULL
            AND entry_index_after = ANY(pending_offer_before)
            AND (level_after, shaping_key_after, shaping_revision_after, rank_after)
                IS NOT DISTINCT FROM
                (level_before, shaping_key_before, shaping_revision_before, rank_before)
        WHEN 'RESHAPE_DECLINE' THEN pending_offer_before IS NOT NULL AND pending_offer_after IS NULL
            AND (level_after, shaping_key_after, shaping_revision_after, entry_index_after,
                 rank_after) IS NOT DISTINCT FROM
                (level_before, shaping_key_before, shaping_revision_before, entry_index_before,
                 rank_before)
        WHEN 'CLEAR' THEN level_before IS NOT NULL AND level_after IS NULL
        WHEN 'MIGRATION_CLEAR' THEN level_before IS NOT NULL AND level_after IS NULL
        ELSE FALSE
    END),
    PRIMARY KEY (proficiency_occurrence_id, item_key, slot),
    -- The per-(track, slot) previous/latest lookups use this unique B-tree in reverse order.
    UNIQUE (character_id, item_key, slot, committed_character_revision),
    FOREIGN KEY (proficiency_occurrence_id, character_id, committed_character_revision, cause)
        REFERENCES game_character_proficiency_receipts
            (proficiency_occurrence_id, character_id, committed_character_revision, cause)
);

-- §6.3: the receipt of a refused occurrence. No line, no value, no CharacterRevision.
CREATE TABLE game_character_proficiency_modification_terminals (
    proficiency_occurrence_id UUID NOT NULL
        CHECK (game_character_is_uuid_v7(proficiency_occurrence_id)),
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    command_binding BYTEA NOT NULL CHECK (octet_length(command_binding) = 33),
    item_key TEXT NOT NULL CHECK (item_key ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'
        AND item_key LIKE 'oteryn:item.%' AND length(item_key) > 12),
    slot SMALLINT NOT NULL CHECK (slot IN (1, 2)),
    operation TEXT NOT NULL CHECK (operation IN ('MODIFY', 'RANK_UP', 'ORB_RANK', 'RESHAPE_OFFER',
        'RESHAPE_CHOOSE', 'CLEAR')),
    bound_definition_revision TEXT NOT NULL
        CHECK (bound_definition_revision ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'),
    bound_shaping_revision TEXT NOT NULL
        CHECK (bound_shaping_revision ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'),
    bound_simulation_revision TEXT NOT NULL
        CHECK (bound_simulation_revision ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'),
    refusal TEXT NOT NULL CHECK (refusal = 'REVISION_CHANGED'),
    recorded_at BIGINT NOT NULL CHECK (recorded_at >= 0),
    PRIMARY KEY (proficiency_occurrence_id, character_id),
    UNIQUE (proficiency_occurrence_id)
);

-- 0032's cardinality guard plus §4.3: a perk_modification receipt has exactly one track line
-- and one modification line of the same track; training and selection carry none; a receipt
-- never shares its occurrence with a terminal record.
CREATE OR REPLACE FUNCTION game_character_proficiency_receipt_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    v_count BIGINT;
    v_modifications BIGINT;
    v_root_revision NUMERIC(20,0);
BEGIN
    SELECT character_revision INTO v_root_revision
      FROM game_character_roots WHERE character_id = NEW.character_id;
    IF NOT FOUND OR NEW.committed_character_revision <> v_root_revision THEN
        RAISE EXCEPTION 'proficiency receipt is not at the current Character root revision'
            USING ERRCODE = '23514';
    END IF;
    SELECT count(*) INTO v_count FROM game_character_proficiency_receipt_lines
     WHERE proficiency_occurrence_id = NEW.proficiency_occurrence_id;
    IF v_count < 1 OR (NEW.cause IN ('perk_selection', 'perk_modification') AND v_count <> 1) THEN
        RAISE EXCEPTION 'proficiency receipt has an invalid changed-track line count'
            USING ERRCODE = '23514';
    END IF;
    SELECT count(*) INTO v_modifications FROM game_character_proficiency_modification_lines
     WHERE proficiency_occurrence_id = NEW.proficiency_occurrence_id;
    IF (NEW.cause = 'perk_modification' AND (v_modifications <> 1 OR NOT EXISTS (
            SELECT 1 FROM game_character_proficiency_modification_lines m
              JOIN game_character_proficiency_receipt_lines l
                ON l.proficiency_occurrence_id = m.proficiency_occurrence_id
               AND l.item_key = m.item_key
             WHERE m.proficiency_occurrence_id = NEW.proficiency_occurrence_id)))
       OR (NEW.cause IN ('training', 'perk_selection') AND v_modifications <> 0) THEN
        RAISE EXCEPTION 'proficiency receipt has an invalid modification line count'
            USING ERRCODE = '23514';
    END IF;
    IF NEW.cause = 'migration' AND EXISTS (
            SELECT 1 FROM game_character_proficiency_modification_lines m
             WHERE m.proficiency_occurrence_id = NEW.proficiency_occurrence_id
               AND NOT EXISTS (SELECT 1 FROM game_character_proficiency_receipt_lines l
                    WHERE l.proficiency_occurrence_id = m.proficiency_occurrence_id
                      AND l.item_key = m.item_key)) THEN
        RAISE EXCEPTION 'migration modification line has no track line'
            USING ERRCODE = '23514';
    END IF;
    IF EXISTS (SELECT 1 FROM game_character_proficiency_modification_terminals t
                WHERE t.proficiency_occurrence_id = NEW.proficiency_occurrence_id) THEN
        RAISE EXCEPTION 'proficiency occurrence already has a terminal record'
            USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END;
$$;

-- §7.1 per-(track, slot) chain, as 0032's per-track guard: each line follows the previous
-- line's after values (or the cleared state) and the row equals its latest line.
CREATE FUNCTION game_character_proficiency_modification_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    v_root_revision NUMERIC(20,0);
    v_previous game_character_proficiency_modification_lines%ROWTYPE;
    v_latest game_character_proficiency_modification_lines%ROWTYPE;
    v_row game_character_proficiency_modifications%ROWTYPE;
BEGIN
    SELECT character_revision INTO v_root_revision
      FROM game_character_roots WHERE character_id = NEW.character_id;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'proficiency modification lost its Character root' USING ERRCODE = '23514';
    END IF;
    IF TG_TABLE_NAME = 'game_character_proficiency_modification_lines' THEN
        IF NEW.committed_character_revision <> v_root_revision THEN
            RAISE EXCEPTION 'modification line cannot be added to an older or future receipt'
                USING ERRCODE = '23514';
        END IF;
        SELECT * INTO v_previous FROM game_character_proficiency_modification_lines
         WHERE character_id = NEW.character_id AND item_key = NEW.item_key AND slot = NEW.slot
           AND committed_character_revision < NEW.committed_character_revision
         ORDER BY committed_character_revision DESC LIMIT 1;
        IF FOUND THEN
            IF (NEW.level_before, NEW.shaping_key_before, NEW.shaping_revision_before,
                NEW.entry_index_before, NEW.rank_before, NEW.pending_offer_before)
               IS DISTINCT FROM
               (v_previous.level_after, v_previous.shaping_key_after,
                v_previous.shaping_revision_after, v_previous.entry_index_after,
                v_previous.rank_after, v_previous.pending_offer_after) THEN
                RAISE EXCEPTION 'modification line before values do not follow its previous line'
                    USING ERRCODE = '23514';
            END IF;
        ELSIF NEW.level_before IS NOT NULL THEN
            RAISE EXCEPTION 'first modification line must start from the cleared state'
                USING ERRCODE = '23514';
        END IF;
        -- §7.1: exactly one slot changes per perk_modification receipt, also for a line added
        -- after its receipt was inserted.
        IF NEW.cause = 'perk_modification' AND (
                SELECT count(*) FROM game_character_proficiency_modification_lines m
                 WHERE m.proficiency_occurrence_id = NEW.proficiency_occurrence_id) <> 1 THEN
            RAISE EXCEPTION 'a perk_modification receipt changes exactly one modification line'
                USING ERRCODE = '23514';
        END IF;
    END IF;
    SELECT * INTO v_latest FROM game_character_proficiency_modification_lines
     WHERE character_id = NEW.character_id AND item_key = NEW.item_key AND slot = NEW.slot
     ORDER BY committed_character_revision DESC LIMIT 1;
    IF NOT FOUND OR v_latest.committed_character_revision > v_root_revision THEN
        RAISE EXCEPTION 'modification row/line has no committed latest line'
            USING ERRCODE = '23514';
    END IF;
    SELECT * INTO v_row FROM game_character_proficiency_modifications
     WHERE character_id = NEW.character_id AND item_key = NEW.item_key AND slot = NEW.slot;
    IF NOT FOUND OR
       (v_row.level, v_row.shaping_key, v_row.shaping_revision, v_row.entry_index, v_row.rank,
        v_row.pending_offer, v_row.committed_character_revision,
        v_row.last_proficiency_occurrence_id)
       IS DISTINCT FROM
       (v_latest.level_after, v_latest.shaping_key_after, v_latest.shaping_revision_after,
        v_latest.entry_index_after, v_latest.rank_after, v_latest.pending_offer_after,
        v_latest.committed_character_revision, v_latest.proficiency_occurrence_id) THEN
        RAISE EXCEPTION 'modification row does not equal its latest line'
            USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END;
$$;

-- §4.1 and §8: a row is never deleted or rekeyed, and every write that references a shaping
-- revision holds the key's shared lock until commit and finds the revision retained.
CREATE FUNCTION game_character_proficiency_modification_row_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'DELETE' OR (TG_OP = 'UPDATE' AND (NEW.character_id, NEW.item_key, NEW.slot)
            IS DISTINCT FROM (OLD.character_id, OLD.item_key, OLD.slot)) THEN
        RAISE EXCEPTION 'proficiency modification rows cannot be deleted or rekeyed'
            USING ERRCODE = '23514';
    END IF;
    IF NEW.shaping_key IS NOT NULL THEN
        PERFORM pg_advisory_xact_lock_shared(
            hashtextextended('proficiency_shaping:' || NEW.shaping_key, 0));
        IF NOT EXISTS (SELECT 1 FROM game_proficiency_shaping_revisions r
                        WHERE r.shaping_key = NEW.shaping_key
                          AND r.shaping_revision = NEW.shaping_revision) THEN
            RAISE EXCEPTION 'proficiency modification references an unretained shaping revision'
                USING ERRCODE = '23514';
        END IF;
    END IF;
    RETURN NEW;
END;
$$;

CREATE FUNCTION game_character_proficiency_modification_terminal_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF EXISTS (SELECT 1 FROM game_character_proficiency_receipts h
                WHERE h.proficiency_occurrence_id = NEW.proficiency_occurrence_id) THEN
        RAISE EXCEPTION 'proficiency occurrence already has a receipt' USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END;
$$;

CREATE FUNCTION game_proficiency_shaping_revision_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'UPDATE' AND (NEW.shaping_key, NEW.shaping_revision)
            IS DISTINCT FROM (OLD.shaping_key, OLD.shaping_revision) THEN
        RAISE EXCEPTION 'shaping revisions cannot be rekeyed' USING ERRCODE = '23514';
    END IF;
    IF TG_OP = 'DELETE' AND EXISTS (SELECT 1 FROM game_character_proficiency_modifications m
            WHERE m.shaping_key = OLD.shaping_key AND m.shaping_revision = OLD.shaping_revision) THEN
        RAISE EXCEPTION 'a referenced shaping revision cannot be dropped' USING ERRCODE = 'OTC01';
    END IF;
    IF TG_OP = 'DELETE' THEN
        RETURN OLD;
    END IF;
    RETURN NEW;
END;
$$;

-- §8: activate p_revision for p_key and retain exactly p_retained plus p_revision. Under the
-- exclusive key lock, dropping a referenced revision is refused (OTC01) and nothing changes.
CREATE FUNCTION game_proficiency_shaping_activate(
    p_key TEXT, p_revision TEXT, p_retained TEXT[]
) RETURNS VOID LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
    PERFORM pg_advisory_xact_lock(hashtextextended('proficiency_shaping:' || p_key, 0));
    DELETE FROM game_proficiency_shaping_revisions
     WHERE shaping_key = p_key AND shaping_revision <> p_revision
       AND shaping_revision <> ALL(coalesce(p_retained, ARRAY[]::TEXT[]));
    UPDATE game_proficiency_shaping_revisions SET active = FALSE
     WHERE shaping_key = p_key AND active AND shaping_revision <> p_revision;
    INSERT INTO game_proficiency_shaping_revisions (shaping_key, shaping_revision, active)
    SELECT p_key, r, r = p_revision
      FROM unnest(array_append(coalesce(p_retained, ARRAY[]::TEXT[]), p_revision)) AS r
    ON CONFLICT (shaping_key, shaping_revision) DO UPDATE SET active = EXCLUDED.active;
END;
$$;

CREATE TRIGGER game_character_proficiency_modification_line_immutable BEFORE UPDATE OR DELETE
    ON game_character_proficiency_modification_lines FOR EACH ROW
    EXECUTE FUNCTION game_character_immutable();
CREATE CONSTRAINT TRIGGER game_character_proficiency_modification_line_consistent
    AFTER INSERT ON game_character_proficiency_modification_lines
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
    EXECUTE FUNCTION game_character_proficiency_modification_guard();
CREATE TRIGGER game_character_proficiency_modification_lines_no_truncate BEFORE TRUNCATE
    ON game_character_proficiency_modification_lines
    EXECUTE FUNCTION game_character_reject_truncate();

CREATE TRIGGER game_character_proficiency_modification_row_guard
    BEFORE INSERT OR UPDATE OR DELETE ON game_character_proficiency_modifications FOR EACH ROW
    EXECUTE FUNCTION game_character_proficiency_modification_row_guard();
CREATE CONSTRAINT TRIGGER game_character_proficiency_modification_row_consistent
    AFTER INSERT OR UPDATE ON game_character_proficiency_modifications
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
    EXECUTE FUNCTION game_character_proficiency_modification_guard();
CREATE TRIGGER game_character_proficiency_modifications_no_truncate BEFORE TRUNCATE
    ON game_character_proficiency_modifications EXECUTE FUNCTION game_character_reject_truncate();

CREATE TRIGGER game_character_proficiency_modification_terminal_immutable
    BEFORE UPDATE OR DELETE ON game_character_proficiency_modification_terminals FOR EACH ROW
    EXECUTE FUNCTION game_character_immutable();
CREATE TRIGGER game_character_proficiency_modification_terminal_guard
    BEFORE INSERT ON game_character_proficiency_modification_terminals FOR EACH ROW
    EXECUTE FUNCTION game_character_proficiency_modification_terminal_guard();
CREATE TRIGGER game_character_proficiency_modification_terminals_no_truncate BEFORE TRUNCATE
    ON game_character_proficiency_modification_terminals
    EXECUTE FUNCTION game_character_reject_truncate();

CREATE TRIGGER game_proficiency_shaping_revision_guard BEFORE UPDATE OR DELETE
    ON game_proficiency_shaping_revisions FOR EACH ROW
    EXECUTE FUNCTION game_proficiency_shaping_revision_guard();
CREATE TRIGGER game_proficiency_shaping_revisions_no_truncate BEFORE TRUNCATE
    ON game_proficiency_shaping_revisions EXECUTE FUNCTION game_character_reject_truncate();

DO $$
DECLARE
    v_function TEXT;
BEGIN
    FOREACH v_function IN ARRAY ARRAY[
        'game_character_proficiency_receipt_guard()',
        'game_character_proficiency_modification_guard()',
        'game_character_proficiency_modification_row_guard()',
        'game_character_proficiency_modification_terminal_guard()',
        'game_proficiency_shaping_revision_guard()',
        'game_proficiency_shaping_activate(text, text, text[])'
    ] LOOP
        EXECUTE format('ALTER FUNCTION %s SET search_path = %I, pg_temp', v_function, current_schema());
    END LOOP;
END $$;

REVOKE ALL ON game_proficiency_shaping_revisions, game_character_proficiency_modifications,
    game_character_proficiency_modification_lines,
    game_character_proficiency_modification_terminals FROM PUBLIC;
REVOKE ALL ON FUNCTION game_character_proficiency_receipt_guard(),
    game_character_proficiency_modification_guard(),
    game_character_proficiency_modification_row_guard(),
    game_character_proficiency_modification_terminal_guard(),
    game_proficiency_shaping_revision_guard(),
    game_proficiency_shaping_activate(TEXT, TEXT, TEXT[]) FROM PUBLIC;
GRANT SELECT, INSERT ON game_character_proficiency_modification_lines,
    game_character_proficiency_modification_terminals TO oteryn_game_runtime;
GRANT SELECT, INSERT, UPDATE ON game_character_proficiency_modifications TO oteryn_game_runtime;
GRANT SELECT ON game_proficiency_shaping_revisions TO oteryn_game_runtime, oteryn_game_control;
GRANT SELECT ON game_character_proficiency_modifications,
    game_character_proficiency_modification_lines,
    game_character_proficiency_modification_terminals TO oteryn_game_control;
GRANT EXECUTE ON FUNCTION game_proficiency_shaping_activate(TEXT, TEXT, TEXT[])
    TO oteryn_game_control;
