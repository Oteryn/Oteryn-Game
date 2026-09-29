-- STANCE-0 (`reviews/OTERYN_GAME_STANCE0_CHARACTER_STANCE_PERSISTENCE_DECISION_2026-09-29.md`
-- §4.1-§4.3 and §4.7, decision STANCE0-CHARACTER-STANCE-PERSISTENCE-V1, owner
-- decision D145): durable storage for the spell `standard` stance slot.
-- Migration only: no writer exists until STANCE-1.
--
-- Scope of this migration:
--   * `game_character_stance`: at most one row per Character, the projection
--     of its latest stance transition. No row means an empty slot; a row
--     exists only after the first stance receipt and is never deleted;
--   * `game_character_stance_receipts`: one immutable receipt per committed
--     toggle, keyed by the toggle occurrence (UUIDv7), that advances the global
--     CharacterRevision with experience and level unchanged;
--   * the 0009/0016 state guard and deferred consistency guard now admit one
--     chain with three receipt kinds (XP, death or stance), exactly one
--     receipt per revision, plus the stance chain and the stance row.
-- The 0009 XP and 0016 death tables, their CHECKs and the revision-one
-- initializer path are unchanged.

-- §4.2. `before` and `after` of level and experience are equal so the
-- cross-kind chain reads uniformly; a no-op toggle is not a receipt.
CREATE TABLE game_character_stance_receipts (
    stance_occurrence_id UUID PRIMARY KEY CHECK (game_character_is_uuid_v7(stance_occurrence_id)),
    command_binding BYTEA NOT NULL CHECK (octet_length(command_binding) BETWEEN 1 AND 1024),
    policy_digest BYTEA NOT NULL CHECK (octet_length(policy_digest) = 32),
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    original_character_revision NUMERIC(20,0) NOT NULL
        CHECK (original_character_revision BETWEEN 1 AND 18446744073709551614),
    committed_character_revision NUMERIC(20,0) NOT NULL
        CHECK (committed_character_revision = original_character_revision + 1),
    level_before BIGINT NOT NULL CHECK (level_before BETWEEN 1 AND 4294967295),
    level_after BIGINT NOT NULL CHECK (level_after = level_before),
    experience_before BIGINT NOT NULL CHECK (experience_before >= 0),
    experience_after BIGINT NOT NULL CHECK (experience_after = experience_before),
    stance_before TEXT NULL
        CHECK (stance_before IS NULL OR stance_before ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'),
    stance_after TEXT NULL
        CHECK (stance_after IS NULL OR stance_after ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'),
    profile_revision TEXT NOT NULL CHECK (profile_revision ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'),
    ruleset_revision TEXT NOT NULL CHECK (ruleset_revision ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'),
    content_revision TEXT NOT NULL CHECK (content_revision ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'),
    simulation_revision TEXT NOT NULL CHECK (simulation_revision ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'),
    evidence_revision TEXT NOT NULL CHECK (evidence_revision ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'),
    declaration_revision TEXT NOT NULL CHECK (declaration_revision ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'),
    policy_revision TEXT NOT NULL CHECK (policy_revision ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'),
    reward_revision TEXT NOT NULL CHECK (reward_revision ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'),
    committed_at BIGINT NOT NULL CHECK (committed_at >= 0),
    CHECK (stance_before IS DISTINCT FROM stance_after),
    UNIQUE (character_id, original_character_revision),
    UNIQUE (character_id, committed_character_revision)
);

-- §4.1. Only the `standard` slot. A NULL key is an empty slot after a toggle
-- switched the stance off. The deferred guard binds every column to the
-- latest stance receipt.
CREATE TABLE game_character_stance (
    character_id UUID PRIMARY KEY REFERENCES game_character_roots(character_id),
    stance_key TEXT NULL
        CHECK (stance_key IS NULL OR stance_key ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'),
    committed_character_revision NUMERIC(20,0) NOT NULL
        CHECK (committed_character_revision BETWEEN 2 AND 18446744073709551615),
    last_stance_occurrence_id UUID NOT NULL
        CHECK (game_character_is_uuid_v7(last_stance_occurrence_id))
);

-- §4.3 state guard: one CharacterRevision successor with unchanged revision
-- fields, in the XP direction (experience strictly larger, level not lower),
-- the death direction (experience not larger, level not higher) or the stance
-- direction (experience and level equal). The death direction already admits
-- the stance one; the deferred guard decides which receipt kind explains it.
CREATE OR REPLACE FUNCTION game_character_progression_state_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'UPDATE'
       AND NEW.character_id = OLD.character_id
       AND NEW.character_revision = OLD.character_revision + 1
       AND ((NEW.total_experience > OLD.total_experience AND NEW.level >= OLD.level)
         OR (NEW.total_experience <= OLD.total_experience AND NEW.level <= OLD.level)
         OR (NEW.total_experience = OLD.total_experience AND NEW.level = OLD.level))
       AND NEW.profile_revision = OLD.profile_revision
       AND NEW.ruleset_revision = OLD.ruleset_revision
       AND NEW.content_revision = OLD.content_revision
       AND NEW.simulation_revision = OLD.simulation_revision
       AND NEW.evidence_revision = OLD.evidence_revision
       AND NEW.declaration_revision = OLD.declaration_revision
       AND NEW.policy_revision = OLD.policy_revision
       AND NEW.reward_revision = OLD.reward_revision THEN
        RETURN NEW;
    END IF;
    RAISE EXCEPTION 'Character progression permits only one typed XP, death or stance successor'
        USING ERRCODE = '23514';
END;
$$;

-- §4.3 consistency guard (deferred, at commit), over the union of all kinds:
--   * revision one: no receipt of any kind and no stance row;
--   * otherwise the typed state is at the root revision, XP + death + stance
--     receipts = revision - 1 with exactly one receipt per revision, the
--     receipt of the current revision matches the state, and each receipt's
--     `before` equals its predecessor's `after` across kinds;
--   * the stance chain: each stance transition's `stance_before` equals the
--     previous transition's `stance_after`, and is NULL for the first. Only
--     stance receipts carry a transition today; a later combined
--     vocation-change receipt (§4.6) joins `stance_chain`;
--   * the stance row equals the latest transition (key, revision,
--     occurrence) and is absent when there is none, so a row-only write, or
--     an XP or death commit that changes the row, fails;
--   * each state transition is explained by the receipt of its successor
--     revision, whose `before` is the replaced row (DEATH-0 extra guard, kept
--     for every kind). The per-kind CHECKs keep XP strictly increasing, death
--     non-increasing and stance equal.
CREATE OR REPLACE FUNCTION game_character_progression_consistency_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    v_character UUID;
    v_root_revision NUMERIC(20,0);
    v_state game_character_progression_state%ROWTYPE;
BEGIN
    -- The stance row trigger also fires on DELETE (rejected before it runs).
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
                   AND s.experience_after = NEW.total_experience) THEN
            RAISE EXCEPTION 'Character progression transition has no matching receipt'
                USING ERRCODE = '23514';
        END IF;
    END IF;
    RETURN NULL;
END;
$$;

-- §4.1/§4.3: the stance row is never deleted and never moves to another
-- Character; its values are bound at commit by the consistency guard.
CREATE FUNCTION game_character_stance_row_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'UPDATE' AND NEW.character_id = OLD.character_id THEN
        RETURN NEW;
    END IF;
    RAISE EXCEPTION 'Character stance slot is never deleted or reassigned'
        USING ERRCODE = '23514';
END;
$$;

CREATE TRIGGER game_character_stance_receipt_immutable BEFORE UPDATE OR DELETE
    ON game_character_stance_receipts FOR EACH ROW
    EXECUTE FUNCTION game_character_immutable();
CREATE CONSTRAINT TRIGGER game_character_stance_receipt_progression_consistent
    AFTER INSERT ON game_character_stance_receipts
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
    EXECUTE FUNCTION game_character_progression_consistency_guard();

CREATE TRIGGER game_character_stance_row_guard BEFORE UPDATE OR DELETE
    ON game_character_stance FOR EACH ROW
    EXECUTE FUNCTION game_character_stance_row_guard();
CREATE CONSTRAINT TRIGGER game_character_stance_progression_consistent
    AFTER INSERT OR UPDATE OR DELETE ON game_character_stance
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
    EXECUTE FUNCTION game_character_progression_consistency_guard();

CREATE TRIGGER game_character_stance_receipts_no_truncate BEFORE TRUNCATE
    ON game_character_stance_receipts EXECUTE FUNCTION game_character_reject_truncate();
CREATE TRIGGER game_character_stance_no_truncate BEFORE TRUNCATE
    ON game_character_stance EXECUTE FUNCTION game_character_reject_truncate();

-- CREATE OR REPLACE resets a function's configuration, so the two replaced
-- guards get their fixed search_path again with the new function.
DO $$
DECLARE
    v_function TEXT;
BEGIN
    FOREACH v_function IN ARRAY ARRAY[
        'game_character_progression_state_guard()',
        'game_character_progression_consistency_guard()',
        'game_character_stance_row_guard()'
    ] LOOP
        EXECUTE format('ALTER FUNCTION %s SET search_path = %I, pg_temp', v_function, current_schema());
    END LOOP;
END $$;

REVOKE ALL ON game_character_stance_receipts, game_character_stance FROM PUBLIC;
REVOKE ALL ON FUNCTION game_character_stance_row_guard() FROM PUBLIC;
-- Runtime: the STANCE-1 writer inserts a receipt and inserts or updates the
-- slot row in the same transaction; it never deletes the row.
GRANT SELECT, INSERT ON game_character_stance_receipts TO oteryn_game_runtime;
GRANT SELECT, INSERT, UPDATE ON game_character_stance TO oteryn_game_runtime;
GRANT SELECT ON game_character_stance_receipts, game_character_stance TO oteryn_game_control;
