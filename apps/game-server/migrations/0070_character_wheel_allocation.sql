-- WHEEL-W1 / migration 0070 (WHEEL-0
-- `reviews/OTERYN_GAME_WHEEL0_WHEEL_OF_DESTINY_DELIVERY_DECISION_2026-09-30.md` §4, §5.1, §8 and
-- §13, on the Wheel state candidate §3.2-§3.4): the durable Wheel of Destiny allocation.
--
--   * `game_wheel_ruleset_revisions` and `game_wheel_ruleset_slot_capacities`: the Wheel ruleset
--     revisions in activation order, each declared `INITIAL` (the first, without predecessor),
--     `VALUE_ONLY` or `RESET` (WHEEL0-RST-1, §5.1), and the 36 slot capacities of each. The W-R
--     runtime piece carried by W-1 (control plane D484): seeded with the first revision of
--     `rulesets/progression/wheel-of-destiny/wheel.json`; a later revision is inserted by its
--     own migration. Rows are never updated or deleted.
--   * `game_character_wheel_state`: one row per Character with a committed change: the ruleset
--     revision the allocation is pinned to, `wheel_revision` (0 without a row, +1 per change),
--     `allocated_total`, and the CharacterRevision and occurrence of its latest receipt.
--   * `game_character_wheel_slots`: one row per non-zero slot (1..36); a missing slot is 0.
--   * `game_character_wheel_receipts`: one immutable receipt per change, a new kind of the
--     shared CharacterRevision chain (level and experience unchanged): `ALLOCATION` (a full
--     replacement, under one ruleset revision) or `RULESET_RESET` (an all-zero allocation under a
--     newer revision after a reset revision, at admission). It carries the common envelope, the
--     request-only binding, both Wheel revisions, both slot vectors and both ruleset revisions.
--
-- The guards are deferred and raise 23514. The shared consistency guard gains the Wheel kind in
-- the cross-kind chain and calls the Wheel arm (`game_character_wheel_consistency`): the full
-- chain, the ruleset chain, capacities and the row tip. Every Wheel row change also runs the
-- Wheel arm, so a row-only write fails. Rollback after receipts exist may only stop new writes:
-- the receipts and their guards stay (the 0020 pattern).

CREATE TABLE game_wheel_ruleset_revisions (
    wheel_ruleset_revision TEXT PRIMARY KEY
        CHECK (wheel_ruleset_revision ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'),
    ordinal INTEGER NOT NULL UNIQUE CHECK (ordinal >= 1),
    kind TEXT NOT NULL CHECK (kind IN ('INITIAL', 'VALUE_ONLY', 'RESET')),
    -- Only the first revision has no predecessor (§5.1).
    CONSTRAINT game_wheel_ruleset_revision_initial CHECK ((ordinal = 1) = (kind = 'INITIAL'))
);

CREATE TABLE game_wheel_ruleset_slot_capacities (
    wheel_ruleset_revision TEXT NOT NULL
        REFERENCES game_wheel_ruleset_revisions(wheel_ruleset_revision),
    slot SMALLINT NOT NULL CHECK (slot BETWEEN 1 AND 36),
    capacity SMALLINT NOT NULL CHECK (capacity IN (50, 75, 100, 150, 200)),
    PRIMARY KEY (wheel_ruleset_revision, slot)
);

CREATE TABLE game_character_wheel_state (
    character_id UUID PRIMARY KEY REFERENCES game_character_roots(character_id),
    wheel_ruleset_revision TEXT NOT NULL
        REFERENCES game_wheel_ruleset_revisions(wheel_ruleset_revision),
    wheel_revision NUMERIC(20,0) NOT NULL
        CHECK (wheel_revision BETWEEN 1 AND 18446744073709551615),
    -- WHEEL0-RL-01: 4 domains of at most 1000 points.
    allocated_total INTEGER NOT NULL CHECK (allocated_total BETWEEN 0 AND 4000),
    committed_character_revision NUMERIC(20,0) NOT NULL
        CHECK (committed_character_revision BETWEEN 2 AND 18446744073709551615),
    last_wheel_occurrence_id UUID NOT NULL
);

CREATE TABLE game_character_wheel_slots (
    character_id UUID NOT NULL REFERENCES game_character_wheel_state(character_id)
        DEFERRABLE INITIALLY DEFERRED,
    slot SMALLINT NOT NULL CHECK (slot BETWEEN 1 AND 36),
    points SMALLINT NOT NULL CHECK (points BETWEEN 1 AND 200),
    PRIMARY KEY (character_id, slot)
);

CREATE TABLE game_character_wheel_receipts (
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    wheel_occurrence_id UUID NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN ('ALLOCATION', 'RULESET_RESET')),
    request_binding BYTEA NOT NULL CHECK (octet_length(request_binding) = 33),
    before_wheel_revision NUMERIC(20,0) NOT NULL
        CHECK (before_wheel_revision BETWEEN 0 AND 18446744073709551614),
    after_wheel_revision NUMERIC(20,0) NOT NULL
        CHECK (after_wheel_revision = before_wheel_revision + 1),
    -- WHEEL0-RL-06: two vectors of 36 small integers.
    slots_before SMALLINT[] NOT NULL,
    slots_after SMALLINT[] NOT NULL,
    before_wheel_ruleset_revision TEXT NOT NULL
        REFERENCES game_wheel_ruleset_revisions(wheel_ruleset_revision),
    wheel_ruleset_revision TEXT NOT NULL
        REFERENCES game_wheel_ruleset_revisions(wheel_ruleset_revision),
    original_character_revision NUMERIC(20,0) NOT NULL
        CHECK (original_character_revision BETWEEN 1 AND 18446744073709551614),
    committed_character_revision NUMERIC(20,0) NOT NULL
        CHECK (committed_character_revision = original_character_revision + 1),
    level_before BIGINT NOT NULL CHECK (level_before BETWEEN 1 AND 4294967295),
    level_after BIGINT NOT NULL CHECK (level_after = level_before),
    experience_before BIGINT NOT NULL CHECK (experience_before >= 0),
    experience_after BIGINT NOT NULL CHECK (experience_after = experience_before),
    profile_revision TEXT NOT NULL CHECK (profile_revision ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'),
    ruleset_revision TEXT NOT NULL CHECK (ruleset_revision ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'),
    content_revision TEXT NOT NULL CHECK (content_revision ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'),
    simulation_revision TEXT NOT NULL CHECK (simulation_revision ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'),
    evidence_revision TEXT NOT NULL CHECK (evidence_revision ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'),
    declaration_revision TEXT NOT NULL CHECK (declaration_revision ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'),
    policy_revision TEXT NOT NULL CHECK (policy_revision ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'),
    reward_revision TEXT NOT NULL CHECK (reward_revision ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'),
    committed_at BIGINT NOT NULL CHECK (committed_at >= 0),
    CONSTRAINT game_character_wheel_receipt_vectors CHECK (
        array_ndims(slots_before) = 1 AND array_lower(slots_before, 1) = 1
        AND cardinality(slots_before) = 36 AND array_position(slots_before, NULL) IS NULL
        AND 0 <= ALL (slots_before) AND 200 >= ALL (slots_before)
        AND array_ndims(slots_after) = 1 AND array_lower(slots_after, 1) = 1
        AND cardinality(slots_after) = 36 AND array_position(slots_after, NULL) IS NULL
        AND 0 <= ALL (slots_after) AND 200 >= ALL (slots_after)),
    -- An allocation is a real change (WHEEL0-NOOP-1) under one ruleset revision, keyed by a
    -- UUIDv7 occurrence. A reset clears a stored allocation under a newer revision, keyed by an
    -- occurrence derived from (Character, source, destination) (WHEEL0-RST-1), version 8.
    CONSTRAINT game_character_wheel_receipt_kind_shape CHECK (CASE kind
        WHEN 'ALLOCATION' THEN
            game_character_is_uuid_v7(wheel_occurrence_id)
            AND before_wheel_ruleset_revision = wheel_ruleset_revision
            AND slots_before <> slots_after
        ELSE
            substr(wheel_occurrence_id::text, 15, 1) = '8'
            AND before_wheel_ruleset_revision <> wheel_ruleset_revision
            AND before_wheel_revision >= 1
            AND 0 = ALL (slots_after)
    END),
    PRIMARY KEY (character_id, wheel_occurrence_id),
    UNIQUE (character_id, original_character_revision),
    UNIQUE (character_id, committed_character_revision),
    UNIQUE (character_id, after_wheel_revision)
);

-- The first Wheel ruleset revision (W-R carry, D484): `wheel.json` revision, its topology's 36
-- capacities (the same for every vocation).
INSERT INTO game_wheel_ruleset_revisions VALUES ('wheel-authoring-candidate-r1', 1, 'INITIAL');
INSERT INTO game_wheel_ruleset_slot_capacities (wheel_ruleset_revision, slot, capacity)
SELECT 'wheel-authoring-candidate-r1', slot, capacity
  FROM unnest(ARRAY[200, 150, 100, 100, 150, 200, 150, 100, 75, 75, 100, 150,
                    100, 75, 50, 50, 75, 100, 100, 75, 50, 50, 75, 100,
                    150, 100, 75, 75, 100, 150, 200, 150, 100, 100, 150, 200]::smallint[])
       WITH ORDINALITY AS c(capacity, slot);

-- A ruleset revision has exactly 36 capacities, each domain at most 1000 points (WHEEL0-RL-01).
CREATE FUNCTION game_wheel_ruleset_revision_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF (SELECT count(*) FROM game_wheel_ruleset_slot_capacities
         WHERE wheel_ruleset_revision = NEW.wheel_ruleset_revision) <> 36 THEN
        RAISE EXCEPTION 'Wheel ruleset revision must have 36 slot capacities'
            USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END;
$$;

-- The Wheel arm of the shared consistency guard (WHEEL-0 §4 "Guards"), for one Character:
--   * full chain: ordered by CharacterRevision, the first receipt starts at Wheel revision 0
--     with an all-zero vector and its own ruleset revision, every later one continues from its
--     predecessor's after revision, vector and ruleset revision;
--   * ruleset chain: a reset moves to a later revision with a `RESET` revision after the source
--     up to and including the destination;
--   * every after vector is within the capacities of its ruleset revision;
--   * row tip: the state row and the slot rows equal the latest receipt; a state or slot row
--     without receipts is inconsistent.
CREATE FUNCTION game_character_wheel_consistency(p_character UUID) RETURNS void
LANGUAGE plpgsql AS $$
BEGIN
    IF EXISTS (
            WITH ordered AS (
                SELECT x.wheel_occurrence_id, x.kind, x.committed_character_revision,
                       x.before_wheel_revision, x.after_wheel_revision, x.slots_before,
                       x.slots_after, x.before_wheel_ruleset_revision, x.wheel_ruleset_revision,
                       rb.ordinal AS before_ordinal, ra.ordinal AS after_ordinal,
                       row_number() OVER w AS position,
                       count(*) OVER () AS changes,
                       lag(x.after_wheel_revision) OVER w AS previous_revision,
                       lag(x.slots_after) OVER w AS previous_after,
                       lag(x.wheel_ruleset_revision) OVER w AS previous_ruleset
                  FROM game_character_wheel_receipts x
                  JOIN game_wheel_ruleset_revisions rb
                    ON rb.wheel_ruleset_revision = x.before_wheel_ruleset_revision
                  JOIN game_wheel_ruleset_revisions ra
                    ON ra.wheel_ruleset_revision = x.wheel_ruleset_revision
                 WHERE x.character_id = p_character
                WINDOW w AS (ORDER BY x.committed_character_revision)
            )
            SELECT 1 FROM ordered o
             WHERE (o.position = 1
                    AND (o.before_wheel_revision <> 0
                         OR o.slots_before <> array_fill(0::smallint, ARRAY[36])
                         OR o.before_wheel_ruleset_revision <> o.wheel_ruleset_revision))
                OR (o.position > 1
                    AND (o.before_wheel_revision <> o.previous_revision
                         OR o.slots_before <> o.previous_after
                         OR o.before_wheel_ruleset_revision <> o.previous_ruleset))
                OR (o.kind = 'RULESET_RESET'
                    AND (o.after_ordinal <= o.before_ordinal
                         OR NOT EXISTS (
                            SELECT 1 FROM game_wheel_ruleset_revisions r
                             WHERE r.kind = 'RESET'
                               AND r.ordinal > o.before_ordinal
                               AND r.ordinal <= o.after_ordinal)))
                OR EXISTS (
                    SELECT 1 FROM unnest(o.slots_after) WITH ORDINALITY AS s(points, slot)
                      LEFT JOIN game_wheel_ruleset_slot_capacities c
                        ON c.wheel_ruleset_revision = o.wheel_ruleset_revision
                       AND c.slot = s.slot
                     WHERE c.slot IS NULL OR s.points > c.capacity)
                OR (o.position = o.changes AND NOT EXISTS (
                    SELECT 1 FROM game_character_wheel_state r
                     WHERE r.character_id = p_character
                       AND r.wheel_revision = o.after_wheel_revision
                       AND r.wheel_ruleset_revision = o.wheel_ruleset_revision
                       AND r.committed_character_revision = o.committed_character_revision
                       AND r.last_wheel_occurrence_id = o.wheel_occurrence_id
                       AND r.allocated_total =
                           (SELECT sum(p)::integer FROM unnest(o.slots_after) AS p)))
                OR (o.position = o.changes AND EXISTS (
                    SELECT 1 FROM generate_series(1, 36) AS i
                      LEFT JOIN game_character_wheel_slots s
                        ON s.character_id = p_character AND s.slot = i
                     WHERE coalesce(s.points, 0) <> o.slots_after[i]))
            UNION ALL
            SELECT 1 FROM game_character_wheel_state r
             WHERE r.character_id = p_character
               AND NOT EXISTS (SELECT 1 FROM game_character_wheel_receipts x
                                WHERE x.character_id = p_character)
            UNION ALL
            SELECT 1 FROM game_character_wheel_slots s
             WHERE s.character_id = p_character
               AND NOT EXISTS (SELECT 1 FROM game_character_wheel_receipts x
                                WHERE x.character_id = p_character)) THEN
        RAISE EXCEPTION 'Character Wheel allocation is inconsistent with its Wheel receipts'
            USING ERRCODE = '23514';
    END IF;
END;
$$;

-- A Wheel state or slot row change runs the Wheel arm (a row-only write fails).
CREATE FUNCTION game_character_wheel_row_consistent() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        PERFORM game_character_wheel_consistency(OLD.character_id);
    ELSE
        PERFORM game_character_wheel_consistency(NEW.character_id);
    END IF;
    RETURN NULL;
END;
$$;

-- The receipt commits at the current Character root revision.
CREATE FUNCTION game_character_wheel_receipt_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF NOT EXISTS (
            SELECT 1 FROM game_character_roots
             WHERE character_id = NEW.character_id
               AND character_revision = NEW.committed_character_revision) THEN
        RAISE EXCEPTION 'Wheel receipt is not at the current Character root revision'
            USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END;
$$;

-- The state row is never deleted or moved to another Character, and only moves forward.
CREATE FUNCTION game_character_wheel_state_row_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'UPDATE' AND NEW.character_id = OLD.character_id
       AND NEW.wheel_revision > OLD.wheel_revision
       AND NEW.committed_character_revision > OLD.committed_character_revision THEN
        RETURN NEW;
    END IF;
    RAISE EXCEPTION 'Wheel state cannot be deleted, rekeyed or rewritten at its revision'
        USING ERRCODE = '23514';
END;
$$;

-- A slot row is never rekeyed (its points change, or it is deleted at 0).
CREATE FUNCTION game_character_wheel_slot_row_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.character_id = OLD.character_id AND NEW.slot = OLD.slot THEN
        RETURN NEW;
    END IF;
    RAISE EXCEPTION 'Wheel slot cannot be rekeyed' USING ERRCODE = '23514';
END;
$$;

-- Full inherited 0056 shared guard follows; insertion-only WHEEL-W1 arms keep all nine kinds,
-- add the tenth (WHEEL-0 §4 "Receipt kind") and call the Wheel arm.
CREATE OR REPLACE FUNCTION game_character_progression_consistency_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    v_character UUID;
    v_root_revision NUMERIC(20,0);
    v_state game_character_progression_state%ROWTYPE;
BEGIN
    -- The stance, progress and build row triggers also fire on DELETE (rejected
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
           OR EXISTS (SELECT 1 FROM game_character_charm_receipts WHERE character_id = v_character)
           OR EXISTS (SELECT 1 FROM game_character_monk_state_receipts WHERE character_id = v_character)
           -- CHAR-BUILD-1
           OR EXISTS (SELECT 1 FROM game_character_build_receipts WHERE character_id = v_character)
           OR EXISTS (SELECT 1 FROM game_character_build_state WHERE character_id = v_character)
           -- PROF-1: revision one has no proficiency relation.
           OR EXISTS (SELECT 1 FROM game_character_proficiency_receipts WHERE character_id = v_character)
           OR EXISTS (SELECT 1 FROM game_character_proficiency_receipt_lines WHERE character_id = v_character)
           OR EXISTS (SELECT 1 FROM game_character_proficiency WHERE character_id = v_character)
           -- QUEST-STATE-1: revision one has no quest receipt, track or state (an obligation
           -- is outside the revision chain).
           OR EXISTS (SELECT 1 FROM game_character_quest_receipts WHERE character_id = v_character)
           OR EXISTS (SELECT 1 FROM game_character_quest_tracks WHERE character_id = v_character)
           OR EXISTS (SELECT 1 FROM game_character_quest_states WHERE character_id = v_character)
           -- WHEEL-W1: revision one has no Wheel receipt, state or slot.
           OR EXISTS (SELECT 1 FROM game_character_wheel_receipts WHERE character_id = v_character)
           OR EXISTS (SELECT 1 FROM game_character_wheel_state WHERE character_id = v_character)
           OR EXISTS (SELECT 1 FROM game_character_wheel_slots WHERE character_id = v_character)
           OR (v_state.character_id IS NOT NULL AND v_state.character_revision <> 1)
           -- H-1: only a monk state receipt sets Harmony or a forced Serene time.
           OR (v_state.character_id IS NOT NULL
               AND (v_state.harmony <> 0 OR v_state.serene_forced_remaining_micros <> 0)) THEN
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
                UNION ALL
                -- CHARM-3
                SELECT c.original_character_revision, c.committed_character_revision,
                       c.level_before, c.level_after, c.experience_before, c.experience_after,
                       c.profile_revision, c.ruleset_revision, c.content_revision,
                       c.simulation_revision, c.evidence_revision, c.declaration_revision,
                       c.policy_revision, c.reward_revision
                  FROM game_character_charm_receipts c WHERE c.character_id = v_character
                UNION ALL
                -- H-1
                SELECT m.original_character_revision, m.committed_character_revision,
                       m.level_before, m.level_after, m.experience_before, m.experience_after,
                       m.profile_revision, m.ruleset_revision, m.content_revision,
                       m.simulation_revision, m.evidence_revision, m.declaration_revision,
                       m.policy_revision, m.reward_revision
                  FROM game_character_monk_state_receipts m WHERE m.character_id = v_character
                UNION ALL
                -- CHAR-BUILD-1
                SELECT u.original_character_revision, u.committed_character_revision,
                       u.level_before, u.level_after, u.experience_before, u.experience_after,
                       u.profile_revision, u.ruleset_revision, u.content_revision,
                       u.simulation_revision, u.evidence_revision, u.declaration_revision,
                       u.policy_revision, u.reward_revision
                  FROM game_character_build_receipts u WHERE u.character_id = v_character
                UNION ALL
                -- PROF-1: eighth shared CharacterRevision receipt kind.
                SELECT f.original_character_revision, f.committed_character_revision,
                       f.level_before, f.level_after, f.experience_before, f.experience_after,
                       f.profile_revision, f.ruleset_revision, f.content_revision,
                       f.simulation_revision, f.evidence_revision, f.declaration_revision,
                       f.policy_revision, f.reward_revision
                  FROM game_character_proficiency_receipts f WHERE f.character_id = v_character
                UNION ALL
                -- QUEST-STATE-1: ninth shared CharacterRevision receipt kind.
                SELECT q.original_character_revision, q.committed_character_revision,
                       q.level_before, q.level_after, q.experience_before, q.experience_after,
                       q.profile_revision, q.ruleset_revision, q.content_revision,
                       q.simulation_revision, q.evidence_revision, q.declaration_revision,
                       q.policy_revision, q.reward_revision
                  FROM game_character_quest_receipts q WHERE q.character_id = v_character
                UNION ALL
                -- WHEEL-W1: tenth shared CharacterRevision receipt kind.
                SELECT w.original_character_revision, w.committed_character_revision,
                       w.level_before, w.level_after, w.experience_before, w.experience_after,
                       w.profile_revision, w.ruleset_revision, w.content_revision,
                       w.simulation_revision, w.evidence_revision, w.declaration_revision,
                       w.policy_revision, w.reward_revision
                  FROM game_character_wheel_receipts w WHERE w.character_id = v_character
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
                UNION ALL
                -- CHAR-BUILD-1 (A13 §4.2 "Stance fields"): a vocation change that prunes the
                -- stance is a stance transition.
                SELECT u.build_occurrence_id, u.committed_character_revision,
                       u.stance_before, u.stance_after
                  FROM game_character_build_receipts u
                 WHERE u.character_id = v_character
                   AND u.stance_before IS DISTINCT FROM u.stance_after
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

    -- CHAR-BUILD-1 (A13 §4.2 "Build chain" and "Row guard", SKILLS-0 §3.2): build receipts and
    -- death receipts with build fields, ordered by revision. Each `before` equals the previous
    -- `after`, or the seed for the first; the build row equals the latest one (values, revision,
    -- occurrence) and is absent when there is none. So a row-only write, a receipt without its
    -- row update, and any other kind that changes the row all fail.
    IF EXISTS (
            WITH build_chain AS (
                SELECT u.build_occurrence_id AS occurrence_id, u.committed_character_revision,
                       u.vocation_before, u.magic_level_before, u.mana_spent_before,
                       u.fist_level_before, u.fist_tries_before, u.club_level_before,
                       u.club_tries_before, u.sword_level_before, u.sword_tries_before,
                       u.axe_level_before, u.axe_tries_before, u.distance_level_before,
                       u.distance_tries_before, u.shielding_level_before,
                       u.shielding_tries_before, u.fishing_level_before, u.fishing_tries_before,
                       u.vocation_after, u.magic_level_after, u.mana_spent_after,
                       u.fist_level_after, u.fist_tries_after, u.club_level_after,
                       u.club_tries_after, u.sword_level_after, u.sword_tries_after,
                       u.axe_level_after, u.axe_tries_after, u.distance_level_after,
                       u.distance_tries_after, u.shielding_level_after, u.shielding_tries_after,
                       u.fishing_level_after, u.fishing_tries_after
                  FROM game_character_build_receipts u WHERE u.character_id = v_character
                UNION ALL
                SELECT d.death_occurrence_id, d.committed_character_revision,
                       d.vocation, d.magic_level_before, d.mana_spent_before,
                       d.fist_level_before, d.fist_tries_before, d.club_level_before,
                       d.club_tries_before, d.sword_level_before, d.sword_tries_before,
                       d.axe_level_before, d.axe_tries_before, d.distance_level_before,
                       d.distance_tries_before, d.shielding_level_before,
                       d.shielding_tries_before, d.fishing_level_before, d.fishing_tries_before,
                       d.vocation, d.magic_level_after, d.mana_spent_after,
                       d.fist_level_after, d.fist_tries_after, d.club_level_after,
                       d.club_tries_after, d.sword_level_after, d.sword_tries_after,
                       d.axe_level_after, d.axe_tries_after, d.distance_level_after,
                       d.distance_tries_after, d.shielding_level_after, d.shielding_tries_after,
                       d.fishing_level_after, d.fishing_tries_after
                  FROM game_character_death_receipts d
                 WHERE d.character_id = v_character AND d.vocation IS NOT NULL
            ), ordered AS (
                SELECT c.*,
                       row_number() OVER (ORDER BY c.committed_character_revision) AS position,
                       count(*) OVER () AS transitions
                  FROM build_chain c
            )
            SELECT 1 FROM ordered o
              LEFT JOIN ordered p ON p.position = o.position - 1
             WHERE (p.position IS NULL
                    AND (o.vocation_before, o.magic_level_before, o.mana_spent_before,
                         o.fist_level_before, o.fist_tries_before, o.club_level_before,
                         o.club_tries_before, o.sword_level_before, o.sword_tries_before,
                         o.axe_level_before, o.axe_tries_before, o.distance_level_before,
                         o.distance_tries_before, o.shielding_level_before,
                         o.shielding_tries_before, o.fishing_level_before,
                         o.fishing_tries_before)
                        IS DISTINCT FROM ('none', 0, 0, 10, 0, 10, 0, 10, 0, 10, 0, 10, 0, 10, 0,
                                          10, 0))
                OR (p.position IS NOT NULL
                    AND (o.vocation_before, o.magic_level_before, o.mana_spent_before,
                         o.fist_level_before, o.fist_tries_before, o.club_level_before,
                         o.club_tries_before, o.sword_level_before, o.sword_tries_before,
                         o.axe_level_before, o.axe_tries_before, o.distance_level_before,
                         o.distance_tries_before, o.shielding_level_before,
                         o.shielding_tries_before, o.fishing_level_before,
                         o.fishing_tries_before)
                        IS DISTINCT FROM (p.vocation_after, p.magic_level_after,
                         p.mana_spent_after, p.fist_level_after, p.fist_tries_after,
                         p.club_level_after, p.club_tries_after, p.sword_level_after,
                         p.sword_tries_after, p.axe_level_after, p.axe_tries_after,
                         p.distance_level_after, p.distance_tries_after,
                         p.shielding_level_after, p.shielding_tries_after,
                         p.fishing_level_after, p.fishing_tries_after))
                OR (o.position = o.transitions AND NOT EXISTS (
                    SELECT 1 FROM game_character_build_state r
                     WHERE r.character_id = v_character
                       AND (r.vocation, r.magic_level, r.mana_spent, r.fist_level, r.fist_tries,
                            r.club_level, r.club_tries, r.sword_level, r.sword_tries,
                            r.axe_level, r.axe_tries, r.distance_level, r.distance_tries,
                            r.shielding_level, r.shielding_tries, r.fishing_level,
                            r.fishing_tries)
                         = (o.vocation_after, o.magic_level_after, o.mana_spent_after,
                            o.fist_level_after, o.fist_tries_after, o.club_level_after,
                            o.club_tries_after, o.sword_level_after, o.sword_tries_after,
                            o.axe_level_after, o.axe_tries_after, o.distance_level_after,
                            o.distance_tries_after, o.shielding_level_after,
                            o.shielding_tries_after, o.fishing_level_after,
                            o.fishing_tries_after)
                       AND r.committed_character_revision = o.committed_character_revision
                       AND r.last_build_occurrence_id = o.occurrence_id))
            UNION ALL
            SELECT 1 FROM game_character_build_state r
             WHERE r.character_id = v_character
               AND NOT EXISTS (SELECT 1 FROM build_chain)) THEN
        RAISE EXCEPTION 'Character build state is inconsistent with its build receipts'
            USING ERRCODE = '23514';
    END IF;

    -- WHEEL-W1 (WHEEL-0 §4 "Guards"): the Wheel full-chain, ruleset-chain, capacity and
    -- row-tip arm.
    PERFORM game_character_wheel_consistency(v_character);

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
                   AND b.experience_after = NEW.total_experience
                UNION ALL
                -- CHARM-3
                SELECT 1 FROM game_character_charm_receipts c
                 WHERE c.character_id = v_character
                   AND c.original_character_revision = OLD.character_revision
                   AND c.committed_character_revision = NEW.character_revision
                   AND c.level_before = OLD.level AND c.level_after = NEW.level
                   AND c.experience_before = OLD.total_experience
                   AND c.experience_after = NEW.total_experience
                UNION ALL
                -- H-1
                SELECT 1 FROM game_character_monk_state_receipts m
                 WHERE m.character_id = v_character
                   AND m.original_character_revision = OLD.character_revision
                   AND m.committed_character_revision = NEW.character_revision
                   AND m.level_before = OLD.level AND m.level_after = NEW.level
                   AND m.experience_before = OLD.total_experience
                   AND m.experience_after = NEW.total_experience
                   AND m.harmony_before = OLD.harmony AND m.harmony_after = NEW.harmony
                   AND m.serene_forced_remaining_micros_before = OLD.serene_forced_remaining_micros
                   AND m.serene_forced_remaining_micros_after = NEW.serene_forced_remaining_micros
                UNION ALL
                -- PROF-1: a proficiency successor explains unchanged Character XP/level.
                SELECT 1 FROM game_character_proficiency_receipts f
                 WHERE f.character_id = v_character
                   AND f.original_character_revision = OLD.character_revision
                   AND f.committed_character_revision = NEW.character_revision
                   AND f.level_before = OLD.level AND f.level_after = NEW.level
                   AND f.experience_before = OLD.total_experience
                   AND f.experience_after = NEW.total_experience
                UNION ALL
                -- CHAR-BUILD-1 (#1271 F3)
                SELECT 1 FROM game_character_build_receipts u
                 WHERE u.character_id = v_character
                   AND u.original_character_revision = OLD.character_revision
                   AND u.committed_character_revision = NEW.character_revision
                   AND u.level_before = OLD.level AND u.level_after = NEW.level
                   AND u.experience_before = OLD.total_experience
                   AND u.experience_after = NEW.total_experience
                UNION ALL
                -- QUEST-STATE-1: a quest successor explains unchanged Character XP/level.
                SELECT 1 FROM game_character_quest_receipts q
                 WHERE q.character_id = v_character
                   AND q.original_character_revision = OLD.character_revision
                   AND q.committed_character_revision = NEW.character_revision
                   AND q.level_before = OLD.level AND q.level_after = NEW.level
                   AND q.experience_before = OLD.total_experience
                   AND q.experience_after = NEW.total_experience
                UNION ALL
                -- WHEEL-W1: a Wheel successor explains unchanged Character XP/level.
                SELECT 1 FROM game_character_wheel_receipts w
                 WHERE w.character_id = v_character
                   AND w.original_character_revision = OLD.character_revision
                   AND w.committed_character_revision = NEW.character_revision
                   AND w.level_before = OLD.level AND w.level_after = NEW.level
                   AND w.experience_before = OLD.total_experience
                   AND w.experience_after = NEW.total_experience)
                THEN
            RAISE EXCEPTION 'Character progression transition has no matching receipt'
                USING ERRCODE = '23514';
        END IF;
        -- H-1: a death transition empties Harmony and the forced Serene time; any other
        -- transition but a monk state receipt keeps both unchanged.
        IF EXISTS (
                SELECT 1 FROM game_character_death_receipts d
                 WHERE d.character_id = v_character
                   AND d.original_character_revision = OLD.character_revision
                   AND d.committed_character_revision = NEW.character_revision) THEN
            IF NEW.harmony <> 0 OR NEW.serene_forced_remaining_micros <> 0 THEN
                RAISE EXCEPTION 'a Character death must empty Harmony and the forced Serene time'
                    USING ERRCODE = '23514';
            END IF;
        ELSIF (NEW.harmony <> OLD.harmony
               OR NEW.serene_forced_remaining_micros <> OLD.serene_forced_remaining_micros)
              AND NOT EXISTS (
                SELECT 1 FROM game_character_monk_state_receipts m
                 WHERE m.character_id = v_character
                   AND m.original_character_revision = OLD.character_revision
                   AND m.committed_character_revision = NEW.character_revision) THEN
            RAISE EXCEPTION 'Harmony and the forced Serene time change only with a monk state receipt'
                USING ERRCODE = '23514';
        END IF;
    END IF;
    RETURN NULL;
END;
$$;

CREATE TRIGGER game_wheel_ruleset_revision_immutable BEFORE UPDATE OR DELETE
    ON game_wheel_ruleset_revisions FOR EACH ROW EXECUTE FUNCTION game_character_immutable();
CREATE TRIGGER game_wheel_ruleset_slot_capacity_immutable BEFORE UPDATE OR DELETE
    ON game_wheel_ruleset_slot_capacities FOR EACH ROW EXECUTE FUNCTION game_character_immutable();
CREATE CONSTRAINT TRIGGER game_wheel_ruleset_revision_complete
    AFTER INSERT ON game_wheel_ruleset_revisions
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
    EXECUTE FUNCTION game_wheel_ruleset_revision_guard();
CREATE TRIGGER game_wheel_ruleset_revisions_no_truncate BEFORE TRUNCATE
    ON game_wheel_ruleset_revisions EXECUTE FUNCTION game_character_reject_truncate();
CREATE TRIGGER game_wheel_ruleset_slot_capacities_no_truncate BEFORE TRUNCATE
    ON game_wheel_ruleset_slot_capacities EXECUTE FUNCTION game_character_reject_truncate();

CREATE TRIGGER game_character_wheel_receipt_immutable BEFORE UPDATE OR DELETE
    ON game_character_wheel_receipts FOR EACH ROW
    EXECUTE FUNCTION game_character_immutable();
CREATE CONSTRAINT TRIGGER game_character_wheel_receipt_consistent
    AFTER INSERT ON game_character_wheel_receipts
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
    EXECUTE FUNCTION game_character_wheel_receipt_guard();
CREATE CONSTRAINT TRIGGER game_character_wheel_receipt_progression_consistent
    AFTER INSERT ON game_character_wheel_receipts
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
    EXECUTE FUNCTION game_character_progression_consistency_guard();
CREATE TRIGGER game_character_wheel_receipts_no_truncate BEFORE TRUNCATE
    ON game_character_wheel_receipts EXECUTE FUNCTION game_character_reject_truncate();

CREATE TRIGGER game_character_wheel_state_row_guard BEFORE UPDATE OR DELETE
    ON game_character_wheel_state FOR EACH ROW
    EXECUTE FUNCTION game_character_wheel_state_row_guard();
CREATE CONSTRAINT TRIGGER game_character_wheel_state_consistent
    AFTER INSERT OR UPDATE ON game_character_wheel_state
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
    EXECUTE FUNCTION game_character_wheel_row_consistent();
CREATE TRIGGER game_character_wheel_state_no_truncate BEFORE TRUNCATE
    ON game_character_wheel_state EXECUTE FUNCTION game_character_reject_truncate();

CREATE TRIGGER game_character_wheel_slot_row_guard BEFORE UPDATE
    ON game_character_wheel_slots FOR EACH ROW
    EXECUTE FUNCTION game_character_wheel_slot_row_guard();
CREATE CONSTRAINT TRIGGER game_character_wheel_slot_consistent
    AFTER INSERT OR UPDATE OR DELETE ON game_character_wheel_slots
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
    EXECUTE FUNCTION game_character_wheel_row_consistent();
CREATE TRIGGER game_character_wheel_slots_no_truncate BEFORE TRUNCATE
    ON game_character_wheel_slots EXECUTE FUNCTION game_character_reject_truncate();

-- Candidate grants match 0056; publication/application remains separately gated.
DO $$
DECLARE
    v_function TEXT;
BEGIN
    FOREACH v_function IN ARRAY ARRAY[
        'game_character_progression_consistency_guard()',
        'game_wheel_ruleset_revision_guard()',
        'game_character_wheel_consistency(uuid)',
        'game_character_wheel_row_consistent()',
        'game_character_wheel_receipt_guard()',
        'game_character_wheel_state_row_guard()',
        'game_character_wheel_slot_row_guard()'
    ] LOOP
        EXECUTE format('ALTER FUNCTION %s SET search_path = %I, pg_temp', v_function, current_schema());
    END LOOP;
END $$;

REVOKE ALL ON game_wheel_ruleset_revisions, game_wheel_ruleset_slot_capacities,
    game_character_wheel_state, game_character_wheel_slots, game_character_wheel_receipts
    FROM PUBLIC;
REVOKE ALL ON FUNCTION game_character_progression_consistency_guard(),
    game_wheel_ruleset_revision_guard(), game_character_wheel_consistency(uuid),
    game_character_wheel_row_consistent(), game_character_wheel_receipt_guard(),
    game_character_wheel_state_row_guard(), game_character_wheel_slot_row_guard()
    FROM PUBLIC;
GRANT SELECT ON game_wheel_ruleset_revisions, game_wheel_ruleset_slot_capacities
    TO oteryn_game_runtime;
-- The shared guard and the Wheel row trigger call the Wheel arm as the writing role.
GRANT EXECUTE ON FUNCTION game_character_wheel_consistency(uuid) TO oteryn_game_runtime;
GRANT SELECT, INSERT ON game_character_wheel_receipts TO oteryn_game_runtime;
GRANT SELECT, INSERT, UPDATE ON game_character_wheel_state TO oteryn_game_runtime;
GRANT SELECT, INSERT, UPDATE, DELETE ON game_character_wheel_slots TO oteryn_game_runtime;
GRANT SELECT ON game_wheel_ruleset_revisions, game_wheel_ruleset_slot_capacities,
    game_character_wheel_state, game_character_wheel_slots, game_character_wheel_receipts
    TO oteryn_game_control;
