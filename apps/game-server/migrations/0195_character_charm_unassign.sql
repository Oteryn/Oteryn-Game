-- PARITY-PROGRESSION-QUERIES-1 (parity inventory §7 rank 15): remove an assigned charm for its
-- gold fee (Canary `iobestiary.cpp` `sendBuyCharmRune` action 2: `fee = level * 100`, paid by
-- `removeMoney`, coins then bank). This is the unassign 0020 deferred (owner answer 3c) until
-- the gold fee owner existed; GOLD-FEE-1a (0023) built that owner with `CharmUnassign` as its
-- cause kind 1.
--
-- Scope of this migration:
--   * `game_character_charm_receipts` admits `command_kind` 3 (remove). A remove receipt records
--     the race it frees, its fee and the fee transaction that paid it; it advances the
--     CharacterRevision with experience and level unchanged, like the other charm kinds, so the
--     progression chain guard needs no change;
--   * `assignment_ordinal` numbers the assignments of one charm: the n-th assign and the n-th
--     remove of a charm carry n, and each (charm, kind, n) exists once. It replaces the 0020
--     `game_character_charm_assign_once` index (which is n = 1 only). Receipts written before
--     this migration take the default 1, which is exact: a charm was assigned at most once;
--   * an assignment row may now be deleted, only as the projection of a remove receipt: the
--     charm guard proves, at commit, that per charm the assign and remove receipts alternate
--     from an assign, that a remove names the race of the assign before it, and that the row
--     exists exactly when the charm's latest such receipt is an assign. An UPDATE stays refused;
--   * a remove receipt commits only with its fee record: the same occurrence, charm, Character,
--     fee and committed revision (the fee owner's own composition proof binds that revision to
--     this transaction's root advance). The fee side is not tightened: cause kind 1 already
--     requires the root advance of its source.
-- The slot limit and the major-charm Bestiary threshold stay Game rules (0020).
--
-- Rollback: applied migrations are immutable, so rollback is a new migration. Before any remove
-- receipt exists it restores the 0020 kind CHECKs, the assign-once index, the immutability
-- trigger and the 0020 guard body, drops the three columns and revokes the DELETE grant. After
-- remove receipts exist they are part of the CharacterRevision chain: a rollback keeps them and
-- may only stop new removes by revoking the DELETE grant.

ALTER TABLE game_character_charm_receipts
    ADD COLUMN assignment_ordinal BIGINT NOT NULL DEFAULT 1
        CHECK (assignment_ordinal BETWEEN 1 AND 4294967295),
    ADD COLUMN fee_transaction_id UUID NULL
        REFERENCES game_item_fee_burns (transaction_id) DEFERRABLE INITIALLY DEFERRED,
    ADD COLUMN fee_gold_units BIGINT NULL CHECK (fee_gold_units >= 1);

-- The 0020 kind CHECKs are unnamed; drop them by definition.
DO $$
DECLARE
    v_constraint TEXT;
BEGIN
    FOR v_constraint IN
        SELECT c.conname FROM pg_constraint c
         WHERE c.conrelid = 'game_character_charm_receipts'::regclass AND c.contype = 'c'
           AND (pg_get_constraintdef(c.oid) LIKE '%command_kind = ANY%'
                OR pg_get_constraintdef(c.oid) LIKE '%command_kind = 2%')
    LOOP
        EXECUTE format('ALTER TABLE game_character_charm_receipts DROP CONSTRAINT %I',
                       v_constraint);
    END LOOP;
END $$;

ALTER TABLE game_character_charm_receipts
    ADD CONSTRAINT game_character_charm_receipts_command_kind_check
        CHECK (command_kind IN (1, 2, 3)),
    ADD CONSTRAINT game_character_charm_receipts_kind_shape CHECK (
        (command_kind = 1
            AND stage_before BETWEEN 0 AND 2 AND stage_after = stage_before + 1
            AND stage_cost BETWEEN 1 AND 4294967295 AND race_key IS NULL
            AND fee_transaction_id IS NULL AND fee_gold_units IS NULL)
        OR (command_kind = 2
            AND stage_before IS NULL AND stage_after IS NULL AND stage_cost IS NULL
            AND race_key IS NOT NULL AND game_character_is_bestiary_race_key(race_key)
            AND fee_transaction_id IS NULL AND fee_gold_units IS NULL)
        OR (command_kind = 3
            AND stage_before IS NULL AND stage_after IS NULL AND stage_cost IS NULL
            AND race_key IS NOT NULL AND game_character_is_bestiary_race_key(race_key)
            AND fee_transaction_id IS NOT NULL AND fee_gold_units IS NOT NULL)),
    ADD CONSTRAINT game_character_charm_receipts_fee_once UNIQUE (fee_transaction_id);

DROP INDEX game_character_charm_assign_once;
CREATE UNIQUE INDEX game_character_charm_assignment_step
    ON game_character_charm_receipts (character_id, charm_key, command_kind, assignment_ordinal)
    WHERE command_kind IN (2, 3);

-- The 0020 guard with the assignment arm generalised to assign/remove cycles and the remove fee
-- link added. The unlock arms and the category arm are unchanged.
CREATE OR REPLACE FUNCTION game_character_charm_consistency_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    v_character UUID;
BEGIN
    -- The assignment trigger also fires on DELETE (a remove).
    IF TG_OP = 'DELETE' THEN
        v_character := OLD.character_id;
    ELSE
        v_character := NEW.character_id;
    END IF;
    IF EXISTS (
            WITH ordered AS (
                SELECT r.charm_key, r.charm_occurrence_id, r.committed_character_revision,
                       r.stage_before, r.stage_after,
                       lag(r.stage_after) OVER w AS previous_after,
                       row_number() OVER w AS position,
                       count(*) OVER (PARTITION BY r.charm_key) AS steps
                  FROM game_character_charm_receipts r
                 WHERE r.character_id = v_character AND r.command_kind = 1
                WINDOW w AS (PARTITION BY r.charm_key ORDER BY r.committed_character_revision)
            ), cycles AS (
                SELECT r.charm_key, r.charm_occurrence_id, r.committed_character_revision,
                       r.command_kind, r.race_key, r.charm_category, r.assignment_ordinal,
                       lag(r.command_kind) OVER w AS previous_kind,
                       lag(r.race_key) OVER w AS previous_race,
                       row_number() OVER w AS position,
                       count(*) OVER (PARTITION BY r.charm_key) AS steps
                  FROM game_character_charm_receipts r
                 WHERE r.character_id = v_character AND r.command_kind IN (2, 3)
                WINDOW w AS (PARTITION BY r.charm_key ORDER BY r.committed_character_revision)
            )
            SELECT 1 FROM ordered o
             WHERE o.stage_before <> coalesce(o.previous_after, 0)
                OR (o.position = o.steps AND NOT EXISTS (
                    SELECT 1 FROM game_character_charm_unlocks u
                     WHERE u.character_id = v_character
                       AND u.charm_key = o.charm_key
                       AND u.unlocked_stage = o.stage_after
                       AND u.committed_character_revision = o.committed_character_revision
                       AND u.last_charm_occurrence_id = o.charm_occurrence_id))
            UNION ALL
            SELECT 1 FROM game_character_charm_unlocks u
             WHERE u.character_id = v_character
               AND NOT EXISTS (
                    SELECT 1 FROM game_character_charm_receipts r
                     WHERE r.character_id = v_character AND r.command_kind = 1
                       AND r.charm_key = u.charm_key)
            UNION ALL
            -- Per charm: assign, remove, assign, ... with the n-th of each numbered n; a remove
            -- frees the race of the assign before it; only the latest assign has its row, and
            -- the charm was unlocked before each assign.
            SELECT 1 FROM cycles c
             WHERE c.command_kind = coalesce(c.previous_kind, 3)
                OR c.assignment_ordinal <> (c.position + 1) / 2
                OR (c.command_kind = 3 AND c.race_key IS DISTINCT FROM c.previous_race)
                OR (c.command_kind = 2 AND NOT EXISTS (
                        SELECT 1 FROM game_character_charm_receipts p
                         WHERE p.character_id = v_character AND p.command_kind = 1
                           AND p.charm_key = c.charm_key
                           AND p.committed_character_revision < c.committed_character_revision))
                OR (c.position = c.steps AND c.command_kind = 2 AND NOT EXISTS (
                        SELECT 1 FROM game_character_charm_assignments a
                         WHERE a.character_id = v_character
                           AND a.charm_key = c.charm_key
                           AND a.race_key = c.race_key
                           AND a.charm_category = c.charm_category
                           AND a.committed_character_revision = c.committed_character_revision
                           AND a.charm_occurrence_id = c.charm_occurrence_id))
                OR (c.position = c.steps AND c.command_kind = 3 AND EXISTS (
                        SELECT 1 FROM game_character_charm_assignments a
                         WHERE a.character_id = v_character AND a.charm_key = c.charm_key))
            UNION ALL
            -- A remove commits with its fee: same occurrence, charm, Character, fee, revision.
            SELECT 1 FROM game_character_charm_receipts r
             WHERE r.character_id = v_character AND r.command_kind = 3
               AND NOT EXISTS (
                    SELECT 1 FROM game_item_fee_burns f
                     WHERE f.transaction_id = r.fee_transaction_id
                       AND f.cause_kind = 1
                       AND f.cause_occurrence_id = r.charm_occurrence_id
                       AND f.charm_key = r.charm_key
                       AND f.character_id = v_character
                       AND f.fee_gold_units = r.fee_gold_units
                       AND f.committed_character_revision = r.committed_character_revision)
            UNION ALL
            SELECT 1 FROM game_character_charm_receipts r
             WHERE r.character_id = v_character
             GROUP BY r.charm_key
            HAVING count(DISTINCT r.charm_category) <> 1
            UNION ALL
            SELECT 1 FROM game_character_charm_assignments a
             WHERE a.character_id = v_character
               AND NOT EXISTS (
                    SELECT 1 FROM game_character_charm_receipts r
                     WHERE r.charm_occurrence_id = a.charm_occurrence_id
                       AND r.character_id = v_character AND r.command_kind = 2
                       AND r.charm_key = a.charm_key)) THEN
        RAISE EXCEPTION 'Character charm state is inconsistent with its charm receipts'
            USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END;
$$;

-- A row is never rewritten in place; a delete is proven by the deferred guard.
DROP TRIGGER game_character_charm_assignment_immutable ON game_character_charm_assignments;
CREATE TRIGGER game_character_charm_assignment_immutable BEFORE UPDATE
    ON game_character_charm_assignments FOR EACH ROW
    EXECUTE FUNCTION game_character_immutable();
CREATE CONSTRAINT TRIGGER game_character_charm_assignment_removed_consistent
    AFTER DELETE ON game_character_charm_assignments
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
    EXECUTE FUNCTION game_character_charm_consistency_guard();

-- CREATE OR REPLACE resets a function's configuration.
DO $$
BEGIN
    EXECUTE format('ALTER FUNCTION game_character_charm_consistency_guard() SET search_path = %I, pg_temp',
                   current_schema());
END $$;
REVOKE ALL ON FUNCTION game_character_charm_consistency_guard() FROM PUBLIC;

-- Runtime: the remove writer inserts its receipt, deletes the assignment row and burns the fee
-- in one transaction.
GRANT DELETE ON game_character_charm_assignments TO oteryn_game_runtime;
