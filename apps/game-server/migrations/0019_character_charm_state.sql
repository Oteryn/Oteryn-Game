-- CHARM-3 (`reviews/OTERYN_GAME_CHARM0_BESTIARY_CHARM_PROGRESSION_DECISION_PACKET_2026-09-29.md`
-- §4.2 and the owner answers of §7; DUR-02 §4.6 typed Character-owned extension and §7.1 one
-- Character mutation anchor): durable Charm unlocks and assignments.
--
-- Scope of this migration:
--   * `game_character_charm_receipts`: one immutable receipt per committed Charm command
--     (unlock the next stage, or assign), keyed by the command occurrence (UUIDv7). Each
--     advances the global CharacterRevision with experience and level unchanged;
--   * `game_character_charm_unlocks`: (CharacterId, charm key) -> unlocked stage 1..3, the
--     projection of that charm's latest unlock receipt. A stage only ever rises by one;
--   * `game_character_charm_assignments`: (CharacterId, charm key) -> Bestiary race key, at most
--     one race per charm, written once. There is no unassign (owner answer 3c): it waits for
--     `GAME-ITEM-01`/`DUR-03` and ships with its gold fee, so the row is immutable here;
--   * Charm Points and Minor Charm Echoes are derived and never stored (owner answer 2a);
--   * the 0017 deferred consistency guard now admits one chain with four receipt kinds (XP,
--     death, stance or charm), exactly one receipt per revision.
-- The §4.2 race capacity is one major and one minor charm per race at the same time (TibiaWiki
-- Updates/14.10; Canary `iobestiary.cpp`): the database enforces it as one assignment per race
-- and category. The slot limit on assigned charms depends on Premium and the Charm Expansion, so
-- only the Game rule (`domain::charm::CharmSlotEntitlement`) enforces it.
-- The 0009/0016/0017 tables, their CHECKs and the state guard are unchanged: the state guard's
-- stance direction (experience and level equal) already admits a charm successor.

-- A charm key is `oteryn:charm.<name>`, a race key the Creature definition key
-- `oteryn:creature.<name>`, both at most 128 bytes (`domain::charm`).
CREATE FUNCTION game_character_is_charm_key(value TEXT) RETURNS BOOLEAN
LANGUAGE sql IMMUTABLE STRICT AS $$
    SELECT octet_length(value) <= 128 AND value ~ '^oteryn:charm\.[a-z0-9_]+$'
$$;
CREATE FUNCTION game_character_is_bestiary_race_key(value TEXT) RETURNS BOOLEAN
LANGUAGE sql IMMUTABLE STRICT AS $$
    SELECT octet_length(value) <= 128 AND value ~ '^oteryn:creature\.[a-z0-9_]+$'
$$;

-- `command_kind`: 1 unlock, 2 assign. `charm_category`: 1 major, 2 minor. An unlock records the
-- stage step and the stage cost (Charm Points for a major, Minor Charm Echoes for a minor); an
-- assign records the race. `catalogue_digest` binds the static catalogue the command was
-- validated against, `catalogue_revision` the content revision it belongs to.
CREATE TABLE game_character_charm_receipts (
    charm_occurrence_id UUID PRIMARY KEY CHECK (game_character_is_uuid_v7(charm_occurrence_id)),
    command_binding BYTEA NOT NULL CHECK (octet_length(command_binding) = 33),
    catalogue_digest BYTEA NOT NULL CHECK (octet_length(catalogue_digest) = 32),
    catalogue_revision TEXT NOT NULL
        CHECK (catalogue_revision ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'),
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    original_character_revision NUMERIC(20,0) NOT NULL
        CHECK (original_character_revision BETWEEN 1 AND 18446744073709551614),
    committed_character_revision NUMERIC(20,0) NOT NULL
        CHECK (committed_character_revision = original_character_revision + 1),
    level_before BIGINT NOT NULL CHECK (level_before BETWEEN 1 AND 4294967295),
    level_after BIGINT NOT NULL CHECK (level_after = level_before),
    experience_before BIGINT NOT NULL CHECK (experience_before >= 0),
    experience_after BIGINT NOT NULL CHECK (experience_after = experience_before),
    command_kind SMALLINT NOT NULL CHECK (command_kind IN (1, 2)),
    charm_key TEXT NOT NULL CHECK (game_character_is_charm_key(charm_key)),
    charm_category SMALLINT NOT NULL CHECK (charm_category IN (1, 2)),
    stage_before SMALLINT NULL,
    stage_after SMALLINT NULL,
    stage_cost BIGINT NULL,
    race_key TEXT NULL,
    profile_revision TEXT NOT NULL CHECK (profile_revision ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'),
    ruleset_revision TEXT NOT NULL CHECK (ruleset_revision ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'),
    content_revision TEXT NOT NULL CHECK (content_revision ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'),
    simulation_revision TEXT NOT NULL CHECK (simulation_revision ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'),
    evidence_revision TEXT NOT NULL CHECK (evidence_revision ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'),
    declaration_revision TEXT NOT NULL CHECK (declaration_revision ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'),
    policy_revision TEXT NOT NULL CHECK (policy_revision ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'),
    reward_revision TEXT NOT NULL CHECK (reward_revision ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'),
    committed_at BIGINT NOT NULL CHECK (committed_at >= 0),
    CHECK ((command_kind = 1
            AND stage_before BETWEEN 0 AND 2 AND stage_after = stage_before + 1
            AND stage_cost BETWEEN 1 AND 4294967295 AND race_key IS NULL)
        OR (command_kind = 2
            AND stage_before IS NULL AND stage_after IS NULL AND stage_cost IS NULL
            AND race_key IS NOT NULL AND game_character_is_bestiary_race_key(race_key))),
    UNIQUE (character_id, original_character_revision),
    UNIQUE (character_id, committed_character_revision)
);
-- Each stage of a charm is bought once, and a charm is assigned once.
CREATE UNIQUE INDEX game_character_charm_unlock_step
    ON game_character_charm_receipts (character_id, charm_key, stage_after)
    WHERE command_kind = 1;
CREATE UNIQUE INDEX game_character_charm_assign_once
    ON game_character_charm_receipts (character_id, charm_key)
    WHERE command_kind = 2;

CREATE TABLE game_character_charm_unlocks (
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    charm_key TEXT NOT NULL CHECK (game_character_is_charm_key(charm_key)),
    unlocked_stage SMALLINT NOT NULL CHECK (unlocked_stage BETWEEN 1 AND 3),
    committed_character_revision NUMERIC(20,0) NOT NULL
        CHECK (committed_character_revision BETWEEN 2 AND 18446744073709551615),
    last_charm_occurrence_id UUID NOT NULL
        REFERENCES game_character_charm_receipts(charm_occurrence_id),
    PRIMARY KEY (character_id, charm_key)
);

-- A race holds at most one major and one minor charm: one assignment per race and category.
CREATE TABLE game_character_charm_assignments (
    character_id UUID NOT NULL,
    charm_key TEXT NOT NULL,
    race_key TEXT NOT NULL CHECK (game_character_is_bestiary_race_key(race_key)),
    charm_category SMALLINT NOT NULL CHECK (charm_category IN (1, 2)),
    committed_character_revision NUMERIC(20,0) NOT NULL
        CHECK (committed_character_revision BETWEEN 2 AND 18446744073709551615),
    charm_occurrence_id UUID NOT NULL UNIQUE
        REFERENCES game_character_charm_receipts(charm_occurrence_id),
    PRIMARY KEY (character_id, charm_key),
    FOREIGN KEY (character_id, charm_key)
        REFERENCES game_character_charm_unlocks(character_id, charm_key),
    UNIQUE (character_id, race_key, charm_category)
);

-- The 0017 chain guard with the charm kind added to the revision-one check, the receipt chain
-- and the state-transition binding. Everything else is unchanged.
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
           OR EXISTS (SELECT 1 FROM game_character_charm_receipts WHERE character_id = v_character)
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
                SELECT c.original_character_revision, c.committed_character_revision,
                       c.level_before, c.level_after, c.experience_before, c.experience_after,
                       c.profile_revision, c.ruleset_revision, c.content_revision,
                       c.simulation_revision, c.evidence_revision, c.declaration_revision,
                       c.policy_revision, c.reward_revision
                  FROM game_character_charm_receipts c WHERE c.character_id = v_character
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
                   AND s.experience_after = NEW.total_experience
                UNION ALL
                SELECT 1 FROM game_character_charm_receipts c
                 WHERE c.character_id = v_character
                   AND c.original_character_revision = OLD.character_revision
                   AND c.committed_character_revision = NEW.character_revision
                   AND c.level_before = OLD.level AND c.level_after = NEW.level
                   AND c.experience_before = OLD.total_experience
                   AND c.experience_after = NEW.total_experience) THEN
            RAISE EXCEPTION 'Character progression transition has no matching receipt'
                USING ERRCODE = '23514';
        END IF;
    END IF;
    RETURN NULL;
END;
$$;

-- Charm projection guard (deferred, at commit), per Character:
--   * per charm, the unlock receipts step 0 -> 1 -> 2 -> 3 in revision order, and the unlock
--     row equals the latest one (stage, revision, occurrence); no unlock row without a receipt;
--   * each assign receipt has its assignment row and each row its receipt (race, category,
--     revision, occurrence), and the charm was unlocked at an earlier revision.
-- The receipt chain itself (one receipt per revision, across kinds) is the 0017 guard's.
CREATE FUNCTION game_character_charm_consistency_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    v_character UUID := NEW.character_id;
BEGIN
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
            SELECT 1 FROM game_character_charm_receipts r
             WHERE r.character_id = v_character AND r.command_kind = 2
               AND (NOT EXISTS (
                        SELECT 1 FROM game_character_charm_assignments a
                         WHERE a.character_id = v_character
                           AND a.charm_key = r.charm_key
                           AND a.race_key = r.race_key
                           AND a.charm_category = r.charm_category
                           AND a.committed_character_revision = r.committed_character_revision
                           AND a.charm_occurrence_id = r.charm_occurrence_id)
                    OR NOT EXISTS (
                        SELECT 1 FROM game_character_charm_receipts p
                         WHERE p.character_id = v_character AND p.command_kind = 1
                           AND p.charm_key = r.charm_key
                           AND p.committed_character_revision < r.committed_character_revision))
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

-- An unlock row never moves or disappears; its stage rises by exactly one.
CREATE FUNCTION game_character_charm_unlock_row_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'UPDATE'
       AND NEW.character_id = OLD.character_id
       AND NEW.charm_key = OLD.charm_key
       AND NEW.unlocked_stage = OLD.unlocked_stage + 1 THEN
        RETURN NEW;
    END IF;
    RAISE EXCEPTION 'Character charm unlock only advances by one stage and is never deleted'
        USING ERRCODE = '23514';
END;
$$;

CREATE TRIGGER game_character_charm_receipt_immutable BEFORE UPDATE OR DELETE
    ON game_character_charm_receipts FOR EACH ROW
    EXECUTE FUNCTION game_character_immutable();
CREATE CONSTRAINT TRIGGER game_character_charm_receipt_progression_consistent
    AFTER INSERT ON game_character_charm_receipts
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
    EXECUTE FUNCTION game_character_progression_consistency_guard();
CREATE CONSTRAINT TRIGGER game_character_charm_receipt_charm_consistent
    AFTER INSERT ON game_character_charm_receipts
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
    EXECUTE FUNCTION game_character_charm_consistency_guard();

CREATE TRIGGER game_character_charm_unlock_row_guard BEFORE UPDATE OR DELETE
    ON game_character_charm_unlocks FOR EACH ROW
    EXECUTE FUNCTION game_character_charm_unlock_row_guard();
CREATE CONSTRAINT TRIGGER game_character_charm_unlock_consistent
    AFTER INSERT OR UPDATE ON game_character_charm_unlocks
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
    EXECUTE FUNCTION game_character_charm_consistency_guard();

-- No unassign in this slice (owner answer 3c): an assignment row is immutable.
CREATE TRIGGER game_character_charm_assignment_immutable BEFORE UPDATE OR DELETE
    ON game_character_charm_assignments FOR EACH ROW
    EXECUTE FUNCTION game_character_immutable();
CREATE CONSTRAINT TRIGGER game_character_charm_assignment_consistent
    AFTER INSERT ON game_character_charm_assignments
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
    EXECUTE FUNCTION game_character_charm_consistency_guard();

CREATE TRIGGER game_character_charm_receipts_no_truncate BEFORE TRUNCATE
    ON game_character_charm_receipts EXECUTE FUNCTION game_character_reject_truncate();
CREATE TRIGGER game_character_charm_unlocks_no_truncate BEFORE TRUNCATE
    ON game_character_charm_unlocks EXECUTE FUNCTION game_character_reject_truncate();
CREATE TRIGGER game_character_charm_assignments_no_truncate BEFORE TRUNCATE
    ON game_character_charm_assignments EXECUTE FUNCTION game_character_reject_truncate();

-- CREATE OR REPLACE resets a function's configuration, so the replaced guard
-- gets its fixed search_path again with the new functions.
DO $$
DECLARE
    v_function TEXT;
BEGIN
    FOREACH v_function IN ARRAY ARRAY[
        'game_character_is_charm_key(text)',
        'game_character_is_bestiary_race_key(text)',
        'game_character_progression_consistency_guard()',
        'game_character_charm_consistency_guard()',
        'game_character_charm_unlock_row_guard()'
    ] LOOP
        EXECUTE format('ALTER FUNCTION %s SET search_path = %I, pg_temp', v_function, current_schema());
    END LOOP;
END $$;

REVOKE ALL ON game_character_charm_receipts, game_character_charm_unlocks,
    game_character_charm_assignments FROM PUBLIC;
REVOKE ALL ON FUNCTION
    game_character_is_charm_key(text),
    game_character_is_bestiary_race_key(text),
    game_character_charm_consistency_guard(),
    game_character_charm_unlock_row_guard()
FROM PUBLIC;
-- Runtime: the CHARM-3 writer inserts a receipt and inserts or advances the unlock row, or
-- inserts the assignment row, in the same transaction; it never deletes either.
GRANT SELECT, INSERT ON game_character_charm_receipts TO oteryn_game_runtime;
GRANT SELECT, INSERT, UPDATE ON game_character_charm_unlocks TO oteryn_game_runtime;
GRANT SELECT, INSERT ON game_character_charm_assignments TO oteryn_game_runtime;
GRANT SELECT ON game_character_charm_receipts, game_character_charm_unlocks,
    game_character_charm_assignments TO oteryn_game_control;
