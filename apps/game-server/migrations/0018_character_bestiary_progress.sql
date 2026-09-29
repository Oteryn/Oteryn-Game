-- CHARM-2 (`reviews/OTERYN_GAME_CHARM0_BESTIARY_CHARM_PROGRESSION_DECISION_PACKET_2026-09-29.md`
-- §4.1, owner answers 2a and 6a; DUR-02 §4.1 and §4.6): durable Bestiary kill
-- progress, written by `durability::bestiary_progress` as an independent
-- descendant of a committed creature death.
--
-- Scope of this migration:
--   * `game_character_bestiary_kill_receipts`: one immutable receipt per
--     credited kill, keyed by the (death, character) reward occurrence
--     (UUIDv7), that advances the global CharacterRevision with experience
--     and level unchanged, like a stance receipt. A kill at the saturation
--     bound is not a receipt;
--   * `game_character_bestiary_progress`: `(CharacterId, race key) ->
--     kill_count`, the projection of the race's latest kill receipt. No row
--     means zero kills; a row is never deleted. Stages are derived from
--     `kill_count` and the definition's thresholds and are not stored;
--   * the 0017 deferred consistency guard now admits a fourth receipt kind
--     (XP, death, stance or Bestiary kill), exactly one receipt per revision,
--     plus the per-race kill chain and the progress rows.
-- The 0009/0016/0017 tables, their CHECKs, the 0017 state guard (its equal
-- arm already admits a Bestiary successor) and the revision-one initializer
-- path are unchanged.

-- `before` and `after` of level and experience are equal so the cross-kind
-- chain reads uniformly. `final_kill_threshold` is the bound of the race's
-- definition revision at the kill; the count never passes it.
CREATE TABLE game_character_bestiary_kill_receipts (
    bestiary_occurrence_id UUID PRIMARY KEY
        CHECK (game_character_is_uuid_v7(bestiary_occurrence_id)),
    command_binding BYTEA NOT NULL CHECK (octet_length(command_binding) BETWEEN 1 AND 1024),
    race_digest BYTEA NOT NULL CHECK (octet_length(race_digest) = 32),
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    original_character_revision NUMERIC(20,0) NOT NULL
        CHECK (original_character_revision BETWEEN 1 AND 18446744073709551614),
    committed_character_revision NUMERIC(20,0) NOT NULL
        CHECK (committed_character_revision = original_character_revision + 1),
    level_before BIGINT NOT NULL CHECK (level_before BETWEEN 1 AND 4294967295),
    level_after BIGINT NOT NULL CHECK (level_after = level_before),
    experience_before BIGINT NOT NULL CHECK (experience_before >= 0),
    experience_after BIGINT NOT NULL CHECK (experience_after = experience_before),
    race_key TEXT NOT NULL CHECK (race_key ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'),
    race_definition_revision TEXT NOT NULL
        CHECK (race_definition_revision ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'),
    final_kill_threshold BIGINT NOT NULL CHECK (final_kill_threshold BETWEEN 1 AND 4294967295),
    kill_count_before BIGINT NOT NULL CHECK (kill_count_before >= 0),
    kill_count_after BIGINT NOT NULL
        CHECK (kill_count_after = kill_count_before + 1
               AND kill_count_after <= final_kill_threshold),
    profile_revision TEXT NOT NULL CHECK (profile_revision ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'),
    ruleset_revision TEXT NOT NULL CHECK (ruleset_revision ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'),
    content_revision TEXT NOT NULL CHECK (content_revision ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'),
    simulation_revision TEXT NOT NULL CHECK (simulation_revision ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'),
    evidence_revision TEXT NOT NULL CHECK (evidence_revision ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'),
    declaration_revision TEXT NOT NULL CHECK (declaration_revision ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'),
    policy_revision TEXT NOT NULL CHECK (policy_revision ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'),
    reward_revision TEXT NOT NULL CHECK (reward_revision ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'),
    committed_at BIGINT NOT NULL CHECK (committed_at >= 0),
    UNIQUE (character_id, original_character_revision),
    UNIQUE (character_id, committed_character_revision)
);
CREATE INDEX game_character_bestiary_kill_receipts_race
    ON game_character_bestiary_kill_receipts (character_id, race_key, committed_character_revision);

-- The deferred guard binds every column to the race's latest kill receipt.
CREATE TABLE game_character_bestiary_progress (
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    race_key TEXT NOT NULL CHECK (race_key ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'),
    kill_count BIGINT NOT NULL CHECK (kill_count BETWEEN 1 AND 4294967295),
    committed_character_revision NUMERIC(20,0) NOT NULL
        CHECK (committed_character_revision BETWEEN 2 AND 18446744073709551615),
    last_bestiary_occurrence_id UUID NOT NULL
        CHECK (game_character_is_uuid_v7(last_bestiary_occurrence_id)),
    PRIMARY KEY (character_id, race_key)
);

-- Consistency guard (deferred, at commit), over the union of all kinds. The
-- 0017 body is kept; the additions are marked CHARM-2:
--   * revision one: no receipt of any kind, no stance row and no progress row;
--   * otherwise the typed state is at the root revision, XP + death + stance
--     + Bestiary receipts = revision - 1 with exactly one receipt per
--     revision, the receipt of the current revision matches the state, and
--     each receipt's `before` equals its predecessor's `after` across kinds;
--   * the stance chain and stance row, unchanged from 0017;
--   * CHARM-2 kill chain: per race, each receipt's `kill_count_before` equals
--     the previous receipt's `kill_count_after`, and is 0 for the first;
--   * CHARM-2 progress rows: each equals its race's latest kill receipt
--     (count, revision, occurrence), and none exists without a receipt, so a
--     row-only write, or a commit of another kind that changes a row, fails;
--   * each state transition is explained by the receipt of its successor
--     revision, whose `before` is the replaced row, for every kind.
CREATE OR REPLACE FUNCTION game_character_progression_consistency_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    v_character UUID;
    v_root_revision NUMERIC(20,0);
    v_state game_character_progression_state%ROWTYPE;
BEGIN
    -- The stance and progress row triggers also fire on DELETE (rejected
    -- before they run).
    IF TG_OP = 'DELETE' THEN
        v_character := OLD.character_id;
    ELSE
        v_character := NEW.character_id;
    END IF;
    SELECT character_revision INTO v_root_revision
      FROM game_character_roots WHERE character_id = v_character;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'Character progression lost its root' USING ERRCODE = '23514';
    END IF;
    SELECT * INTO v_state FROM game_character_progression_state
     WHERE character_id = v_character;

    IF v_root_revision = 1 THEN
        IF EXISTS (SELECT 1 FROM game_character_xp_receipts WHERE character_id = v_character)
           OR EXISTS (SELECT 1 FROM game_character_death_receipts WHERE character_id = v_character)
           OR EXISTS (SELECT 1 FROM game_character_stance_receipts WHERE character_id = v_character)
           OR EXISTS (SELECT 1 FROM game_character_stance WHERE character_id = v_character)
           OR EXISTS (SELECT 1 FROM game_character_bestiary_kill_receipts WHERE character_id = v_character)
           OR EXISTS (SELECT 1 FROM game_character_bestiary_progress WHERE character_id = v_character)
           OR (v_state.character_id IS NOT NULL AND v_state.character_revision <> 1) THEN
            RAISE EXCEPTION 'initial Character progression is inconsistent' USING ERRCODE = '23514';
        END IF;
        RETURN NULL;
    END IF;

    IF v_state.character_id IS NULL
       OR v_state.character_revision <> v_root_revision
       OR EXISTS (
            WITH chain AS (
                SELECT x.original_character_revision, x.committed_character_revision,
                       x.level_before, x.level_after, x.experience_before, x.experience_after,
                       x.profile_revision, x.ruleset_revision, x.content_revision,
                       x.simulation_revision, x.evidence_revision, x.declaration_revision,
                       x.policy_revision, x.reward_revision
                  FROM game_character_xp_receipts x WHERE x.character_id = v_character
                UNION ALL
                SELECT d.original_character_revision, d.committed_character_revision,
                       d.level_before, d.level_after, d.experience_before, d.experience_after,
                       d.profile_revision, d.ruleset_revision, d.content_revision,
                       d.simulation_revision, d.evidence_revision, d.declaration_revision,
                       d.policy_revision, d.reward_revision
                  FROM game_character_death_receipts d WHERE d.character_id = v_character
                UNION ALL
                SELECT s.original_character_revision, s.committed_character_revision,
                       s.level_before, s.level_after, s.experience_before, s.experience_after,
                       s.profile_revision, s.ruleset_revision, s.content_revision,
                       s.simulation_revision, s.evidence_revision, s.declaration_revision,
                       s.policy_revision, s.reward_revision
                  FROM game_character_stance_receipts s WHERE s.character_id = v_character
                UNION ALL
                -- CHARM-2
                SELECT b.original_character_revision, b.committed_character_revision,
                       b.level_before, b.level_after, b.experience_before, b.experience_after,
                       b.profile_revision, b.ruleset_revision, b.content_revision,
                       b.simulation_revision, b.evidence_revision, b.declaration_revision,
                       b.policy_revision, b.reward_revision
                  FROM game_character_bestiary_kill_receipts b WHERE b.character_id = v_character
            )
            SELECT 1 FROM chain
            HAVING count(*)::numeric <> v_root_revision - 1
                OR count(DISTINCT committed_character_revision)::numeric <> v_root_revision - 1
            UNION ALL
            SELECT 1 WHERE NOT EXISTS (
                SELECT 1 FROM chain c
                 WHERE c.committed_character_revision = v_root_revision
                   AND c.level_after = v_state.level
                   AND c.experience_after = v_state.total_experience
                   AND c.profile_revision = v_state.profile_revision
                   AND c.ruleset_revision = v_state.ruleset_revision
                   AND c.content_revision = v_state.content_revision
                   AND c.simulation_revision = v_state.simulation_revision
                   AND c.evidence_revision = v_state.evidence_revision
                   AND c.declaration_revision = v_state.declaration_revision
                   AND c.policy_revision = v_state.policy_revision
                   AND c.reward_revision = v_state.reward_revision)
            UNION ALL
            SELECT 1 FROM chain x
             LEFT JOIN chain p ON p.committed_character_revision = x.original_character_revision
             WHERE x.committed_character_revision > v_root_revision
                OR (x.original_character_revision <> 1 AND p.committed_character_revision IS NULL)
                OR (p.committed_character_revision IS NOT NULL AND
                    (p.experience_after <> x.experience_before
                     OR p.level_after <> x.level_before))) THEN
        RAISE EXCEPTION 'Character progression revision/receipt chain is inconsistent'
            USING ERRCODE = '23514';
    END IF;

    IF EXISTS (
            WITH stance_chain AS (
                SELECT s.stance_occurrence_id AS occurrence_id, s.committed_character_revision,
                       s.stance_before, s.stance_after
                  FROM game_character_stance_receipts s WHERE s.character_id = v_character
            ), ordered AS (
                SELECT c.*,
                       row_number() OVER w AS position,
                       lag(c.stance_after) OVER w AS previous_after,
                       count(*) OVER () AS transitions
                  FROM stance_chain c
                WINDOW w AS (ORDER BY c.committed_character_revision)
            )
            SELECT 1 FROM ordered o
             WHERE o.stance_before IS DISTINCT FROM o.previous_after
                OR (o.position = o.transitions AND NOT EXISTS (
                    SELECT 1 FROM game_character_stance r
                     WHERE r.character_id = v_character
                       AND r.stance_key IS NOT DISTINCT FROM o.stance_after
                       AND r.committed_character_revision = o.committed_character_revision
                       AND r.last_stance_occurrence_id = o.occurrence_id))
            UNION ALL
            SELECT 1 FROM game_character_stance r
             WHERE r.character_id = v_character
               AND NOT EXISTS (SELECT 1 FROM stance_chain)) THEN
        RAISE EXCEPTION 'Character stance slot is inconsistent with its stance receipts'
            USING ERRCODE = '23514';
    END IF;

    -- CHARM-2: the per-race kill chain and the progress rows.
    IF EXISTS (
            WITH ordered AS (
                SELECT b.bestiary_occurrence_id, b.race_key, b.committed_character_revision,
                       b.kill_count_before, b.kill_count_after,
                       coalesce(lag(b.kill_count_after) OVER w, 0) AS previous_after,
                       row_number() OVER w AS position,
                       count(*) OVER (PARTITION BY b.race_key) AS kills
                  FROM game_character_bestiary_kill_receipts b
                 WHERE b.character_id = v_character
                WINDOW w AS (PARTITION BY b.race_key ORDER BY b.committed_character_revision)
            )
            SELECT 1 FROM ordered o
             WHERE o.kill_count_before <> o.previous_after
                OR (o.position = o.kills AND NOT EXISTS (
                    SELECT 1 FROM game_character_bestiary_progress r
                     WHERE r.character_id = v_character
                       AND r.race_key = o.race_key
                       AND r.kill_count = o.kill_count_after
                       AND r.committed_character_revision = o.committed_character_revision
                       AND r.last_bestiary_occurrence_id = o.bestiary_occurrence_id))
            UNION ALL
            SELECT 1 FROM game_character_bestiary_progress r
             WHERE r.character_id = v_character
               AND NOT EXISTS (
                    SELECT 1 FROM game_character_bestiary_kill_receipts b
                     WHERE b.character_id = v_character AND b.race_key = r.race_key)) THEN
        RAISE EXCEPTION 'Character Bestiary progress is inconsistent with its kill receipts'
            USING ERRCODE = '23514';
    END IF;

    IF TG_TABLE_NAME = 'game_character_progression_state' AND TG_OP = 'UPDATE' THEN
        IF NOT EXISTS (
                SELECT 1 FROM game_character_xp_receipts x
                 WHERE x.character_id = v_character
                   AND x.original_character_revision = OLD.character_revision
                   AND x.committed_character_revision = NEW.character_revision
                   AND x.level_before = OLD.level AND x.level_after = NEW.level
                   AND x.experience_before = OLD.total_experience
                   AND x.experience_after = NEW.total_experience
                UNION ALL
                SELECT 1 FROM game_character_death_receipts d
                 WHERE d.character_id = v_character
                   AND d.original_character_revision = OLD.character_revision
                   AND d.committed_character_revision = NEW.character_revision
                   AND d.level_before = OLD.level AND d.level_after = NEW.level
                   AND d.experience_before = OLD.total_experience
                   AND d.experience_after = NEW.total_experience
                UNION ALL
                SELECT 1 FROM game_character_stance_receipts s
                 WHERE s.character_id = v_character
                   AND s.original_character_revision = OLD.character_revision
                   AND s.committed_character_revision = NEW.character_revision
                   AND s.level_before = OLD.level AND s.level_after = NEW.level
                   AND s.experience_before = OLD.total_experience
                   AND s.experience_after = NEW.total_experience
                UNION ALL
                -- CHARM-2
                SELECT 1 FROM game_character_bestiary_kill_receipts b
                 WHERE b.character_id = v_character
                   AND b.original_character_revision = OLD.character_revision
                   AND b.committed_character_revision = NEW.character_revision
                   AND b.level_before = OLD.level AND b.level_after = NEW.level
                   AND b.experience_before = OLD.total_experience
                   AND b.experience_after = NEW.total_experience) THEN
            RAISE EXCEPTION 'Character progression transition has no matching receipt'
                USING ERRCODE = '23514';
        END IF;
    END IF;
    RETURN NULL;
END;
$$;

-- A progress row is never deleted and never moves to another Character or
-- race; its values are bound at commit by the consistency guard.
CREATE FUNCTION game_character_bestiary_progress_row_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'UPDATE'
       AND NEW.character_id = OLD.character_id
       AND NEW.race_key = OLD.race_key THEN
        RETURN NEW;
    END IF;
    RAISE EXCEPTION 'Character Bestiary progress is never deleted or reassigned'
        USING ERRCODE = '23514';
END;
$$;

CREATE TRIGGER game_character_bestiary_kill_receipt_immutable BEFORE UPDATE OR DELETE
    ON game_character_bestiary_kill_receipts FOR EACH ROW
    EXECUTE FUNCTION game_character_immutable();
CREATE CONSTRAINT TRIGGER game_character_bestiary_kill_receipt_progression_consistent
    AFTER INSERT ON game_character_bestiary_kill_receipts
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
    EXECUTE FUNCTION game_character_progression_consistency_guard();

CREATE TRIGGER game_character_bestiary_progress_row_guard BEFORE UPDATE OR DELETE
    ON game_character_bestiary_progress FOR EACH ROW
    EXECUTE FUNCTION game_character_bestiary_progress_row_guard();
CREATE CONSTRAINT TRIGGER game_character_bestiary_progress_consistent
    AFTER INSERT OR UPDATE OR DELETE ON game_character_bestiary_progress
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
    EXECUTE FUNCTION game_character_progression_consistency_guard();

CREATE TRIGGER game_character_bestiary_kill_receipts_no_truncate BEFORE TRUNCATE
    ON game_character_bestiary_kill_receipts EXECUTE FUNCTION game_character_reject_truncate();
CREATE TRIGGER game_character_bestiary_progress_no_truncate BEFORE TRUNCATE
    ON game_character_bestiary_progress EXECUTE FUNCTION game_character_reject_truncate();

-- CREATE OR REPLACE resets a function's configuration, so the replaced guard
-- gets its fixed search_path again with the new function.
DO $$
DECLARE
    v_function TEXT;
BEGIN
    FOREACH v_function IN ARRAY ARRAY[
        'game_character_progression_consistency_guard()',
        'game_character_bestiary_progress_row_guard()'
    ] LOOP
        EXECUTE format('ALTER FUNCTION %s SET search_path = %I, pg_temp', v_function, current_schema());
    END LOOP;
END $$;

REVOKE ALL ON game_character_bestiary_kill_receipts, game_character_bestiary_progress FROM PUBLIC;
REVOKE ALL ON FUNCTION game_character_bestiary_progress_row_guard() FROM PUBLIC;
-- Runtime: the CHARM-2 writer inserts a receipt and inserts or updates the
-- race's progress row in the same transaction; it never deletes a row.
GRANT SELECT, INSERT ON game_character_bestiary_kill_receipts TO oteryn_game_runtime;
GRANT SELECT, INSERT, UPDATE ON game_character_bestiary_progress TO oteryn_game_runtime;
GRANT SELECT ON game_character_bestiary_kill_receipts, game_character_bestiary_progress
    TO oteryn_game_control;
