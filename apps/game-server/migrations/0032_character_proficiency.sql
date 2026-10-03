-- PROF-1 / migration 0032 LOCAL CANDIDATE ONLY; no runtime activation.
-- Owning model: PROFICIENCY0 section4.2-4.3, architect5936454476 and D281 reservation.
-- One track per Character/canonical Item; no row is progress0/unfilled selections.
-- These storage spellings do not prove Item/Proficiency admission, actual perk shape, PZ,
-- authority or migration mapping. The future writer/resolver owns those checks and active N.
-- Training/selection strictly keep definition revision; compatible refresh remains blocked.
-- No shaping, catalyst, Orb, economy, wire or alternate authority relation is introduced.
-- Rollback after receipts exist may only stop new writes: preserve the receipt chain.

CREATE TABLE game_character_proficiency_receipts (
    proficiency_occurrence_id UUID PRIMARY KEY
        CHECK (game_character_is_uuid_v7(proficiency_occurrence_id)),
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    original_character_revision NUMERIC(20,0) NOT NULL
        CHECK (original_character_revision BETWEEN 1 AND 18446744073709551614),
    committed_character_revision NUMERIC(20,0) NOT NULL
        CHECK (committed_character_revision = original_character_revision + 1),
    cause TEXT NOT NULL CHECK (cause IN ('training', 'perk_selection', 'migration')),
    command_binding BYTEA NOT NULL CHECK (octet_length(command_binding) BETWEEN 1 AND 1024),
    policy_digest BYTEA NOT NULL CHECK (octet_length(policy_digest) = 32),
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
    UNIQUE (character_id, original_character_revision),
    UNIQUE (character_id, committed_character_revision),
    UNIQUE (proficiency_occurrence_id, character_id, committed_character_revision, cause)
);

-- Cause is redundantly retained to make direction a CHECK, and tied to the immutable header.
CREATE TABLE game_character_proficiency_receipt_lines (
    proficiency_occurrence_id UUID NOT NULL,
    character_id UUID NOT NULL,
    committed_character_revision NUMERIC(20,0) NOT NULL,
    cause TEXT NOT NULL CHECK (cause IN ('training', 'perk_selection', 'migration')),
    item_key TEXT NOT NULL CHECK (item_key ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'
        AND item_key LIKE 'oteryn:item.%' AND length(item_key) > 12),
    definition_key_before TEXT NOT NULL CHECK (definition_key_before ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'
        AND definition_key_before LIKE 'oteryn:proficiency.%' AND length(definition_key_before) > 19),
    definition_revision_before TEXT NOT NULL CHECK (definition_revision_before ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'),
    definition_key_after TEXT NOT NULL CHECK (definition_key_after ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'
        AND definition_key_after LIKE 'oteryn:proficiency.%' AND length(definition_key_after) > 19),
    definition_revision_after TEXT NOT NULL CHECK (definition_revision_after ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'),
    progress_before BIGINT NOT NULL CHECK (progress_before >= 0),
    progress_after BIGINT NOT NULL CHECK (progress_after >= progress_before),
    selections_before SMALLINT[] NOT NULL,
    selections_after SMALLINT[] NOT NULL,
    CONSTRAINT selections_before_canonical CHECK (
        array_ndims(selections_before) = 1 AND array_lower(selections_before, 1) = 1
        AND cardinality(selections_before) BETWEEN 1 AND 7
        AND array_remove(selections_before, NULL::SMALLINT) <@ ARRAY[0, 1, 2]::SMALLINT[]),
    CONSTRAINT selections_after_canonical CHECK (
        array_ndims(selections_after) = 1 AND array_lower(selections_after, 1) = 1
        AND cardinality(selections_after) BETWEEN 1 AND 7
        AND array_remove(selections_after, NULL::SMALLINT) <@ ARRAY[0, 1, 2]::SMALLINT[]),
    CONSTRAINT game_character_proficiency_line_same_definition_key
        CHECK (definition_key_before = definition_key_after),
    CONSTRAINT game_character_proficiency_line_cause_direction CHECK (CASE cause
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
    END),
    PRIMARY KEY (proficiency_occurrence_id, item_key),
    -- The per-track previous/latest lookups use this same unique B-tree in reverse order.
    UNIQUE (character_id, item_key, committed_character_revision),
    FOREIGN KEY (proficiency_occurrence_id, character_id, committed_character_revision, cause)
        REFERENCES game_character_proficiency_receipts
            (proficiency_occurrence_id, character_id, committed_character_revision, cause)
);

CREATE TABLE game_character_proficiency (
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    item_key TEXT NOT NULL CHECK (item_key ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'
        AND item_key LIKE 'oteryn:item.%' AND length(item_key) > 12),
    definition_key TEXT NOT NULL CHECK (definition_key ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'
        AND definition_key LIKE 'oteryn:proficiency.%' AND length(definition_key) > 19),
    definition_revision TEXT NOT NULL CHECK (definition_revision ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'),
    progress BIGINT NOT NULL CHECK (progress >= 0),
    selections SMALLINT[] NOT NULL,
    CONSTRAINT selections_canonical CHECK (
        array_ndims(selections) = 1 AND array_lower(selections, 1) = 1
        AND cardinality(selections) BETWEEN 1 AND 7
        AND array_remove(selections, NULL::SMALLINT) <@ ARRAY[0, 1, 2]::SMALLINT[]),
    committed_character_revision NUMERIC(20,0) NOT NULL
        CHECK (committed_character_revision BETWEEN 2 AND 18446744073709551615),
    last_proficiency_occurrence_id UUID NOT NULL
        CHECK (game_character_is_uuid_v7(last_proficiency_occurrence_id)),
    PRIMARY KEY (character_id, item_key)
);

-- Header/line cardinality is checked once per receipt, never by scanning the whole chain
-- from every changed track. Active-content upper bound N is a writer invariant.
CREATE FUNCTION game_character_proficiency_receipt_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    v_count BIGINT;
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
    IF v_count < 1 OR (NEW.cause = 'perk_selection' AND v_count <> 1) THEN
        RAISE EXCEPTION 'proficiency receipt has an invalid changed-track line count'
            USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END;
$$;

-- Index probes touch a bounded number of rows per changed track. Do not call the shared
-- whole-chain guard from this function or any line/track-row trigger.
CREATE FUNCTION game_character_proficiency_track_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    v_root_revision NUMERIC(20,0);
    v_previous game_character_proficiency_receipt_lines%ROWTYPE;
    v_latest game_character_proficiency_receipt_lines%ROWTYPE;
    v_row game_character_proficiency%ROWTYPE;
BEGIN
    SELECT character_revision INTO v_root_revision
      FROM game_character_roots WHERE character_id = NEW.character_id;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'proficiency track lost its Character root' USING ERRCODE = '23514';
    END IF;
    IF TG_TABLE_NAME = 'game_character_proficiency_receipt_lines' THEN
        IF NEW.committed_character_revision <> v_root_revision THEN
            RAISE EXCEPTION 'proficiency line cannot be added to an older or future receipt'
                USING ERRCODE = '23514';
        END IF;
        SELECT * INTO v_previous FROM game_character_proficiency_receipt_lines
         WHERE character_id = NEW.character_id AND item_key = NEW.item_key
           AND committed_character_revision < NEW.committed_character_revision
         ORDER BY committed_character_revision DESC LIMIT 1;
        IF FOUND THEN
            IF (NEW.definition_key_before, NEW.definition_revision_before,
                NEW.progress_before, NEW.selections_before) IS DISTINCT FROM
               (v_previous.definition_key_after, v_previous.definition_revision_after,
                v_previous.progress_after, v_previous.selections_after) THEN
                RAISE EXCEPTION 'proficiency line before values do not follow its previous line'
                    USING ERRCODE = '23514';
            END IF;
        ELSIF NEW.progress_before <> 0
              OR cardinality(array_remove(NEW.selections_before, NULL::SMALLINT)) <> 0 THEN
            RAISE EXCEPTION 'first proficiency line must start from zero and unfilled choices'
                USING ERRCODE = '23514';
        END IF;
        -- First definition revision is supplied by the line, not guessed from SQL content.
        -- The generic seed applies to every cause, including a candidate first migration.
    END IF;
    SELECT * INTO v_latest FROM game_character_proficiency_receipt_lines
     WHERE character_id = NEW.character_id AND item_key = NEW.item_key
     ORDER BY committed_character_revision DESC LIMIT 1;
    IF NOT FOUND OR v_latest.committed_character_revision > v_root_revision THEN
        RAISE EXCEPTION 'proficiency row/line has no committed latest track line'
            USING ERRCODE = '23514';
    END IF;
    SELECT * INTO v_row FROM game_character_proficiency
     WHERE character_id = NEW.character_id AND item_key = NEW.item_key;
    IF NOT FOUND OR
       (v_row.definition_key, v_row.definition_revision, v_row.progress, v_row.selections,
        v_row.committed_character_revision, v_row.last_proficiency_occurrence_id)
       IS DISTINCT FROM
       (v_latest.definition_key_after, v_latest.definition_revision_after,
        v_latest.progress_after, v_latest.selections_after,
        v_latest.committed_character_revision, v_latest.proficiency_occurrence_id) THEN
        RAISE EXCEPTION 'proficiency row does not equal its latest committed track line'
            USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END;
$$;

CREATE FUNCTION game_character_proficiency_row_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'UPDATE' AND NEW.character_id = OLD.character_id
       AND NEW.item_key = OLD.item_key AND NEW.definition_key = OLD.definition_key
       AND NEW.progress >= OLD.progress THEN
        RETURN NEW;
    END IF;
    RAISE EXCEPTION 'proficiency track cannot be deleted, rekeyed or lose cumulative progress'
        USING ERRCODE = '23514';
END;
$$;

-- Full inherited0030 shared guard follows; insertion-only PROF-1 arms preserve all seven kinds.
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
                   AND u.experience_after = NEW.total_experience)
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

CREATE TRIGGER game_character_proficiency_receipt_immutable BEFORE UPDATE OR DELETE
    ON game_character_proficiency_receipts FOR EACH ROW
    EXECUTE FUNCTION game_character_immutable();
CREATE CONSTRAINT TRIGGER game_character_proficiency_receipt_cardinality
    AFTER INSERT ON game_character_proficiency_receipts
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
    EXECUTE FUNCTION game_character_proficiency_receipt_guard();
CREATE CONSTRAINT TRIGGER game_character_proficiency_receipt_progression_consistent
    AFTER INSERT ON game_character_proficiency_receipts
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
    EXECUTE FUNCTION game_character_progression_consistency_guard();
CREATE TRIGGER game_character_proficiency_receipts_no_truncate BEFORE TRUNCATE
    ON game_character_proficiency_receipts EXECUTE FUNCTION game_character_reject_truncate();

CREATE TRIGGER game_character_proficiency_line_immutable BEFORE UPDATE OR DELETE
    ON game_character_proficiency_receipt_lines FOR EACH ROW
    EXECUTE FUNCTION game_character_immutable();
CREATE CONSTRAINT TRIGGER game_character_proficiency_line_track_consistent
    AFTER INSERT ON game_character_proficiency_receipt_lines
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
    EXECUTE FUNCTION game_character_proficiency_track_guard();
CREATE TRIGGER game_character_proficiency_lines_no_truncate BEFORE TRUNCATE
    ON game_character_proficiency_receipt_lines EXECUTE FUNCTION game_character_reject_truncate();

CREATE TRIGGER game_character_proficiency_row_guard BEFORE UPDATE OR DELETE
    ON game_character_proficiency FOR EACH ROW
    EXECUTE FUNCTION game_character_proficiency_row_guard();
CREATE CONSTRAINT TRIGGER game_character_proficiency_row_track_consistent
    AFTER INSERT OR UPDATE OR DELETE ON game_character_proficiency
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
    EXECUTE FUNCTION game_character_proficiency_track_guard();
CREATE TRIGGER game_character_proficiency_no_truncate BEFORE TRUNCATE
    ON game_character_proficiency EXECUTE FUNCTION game_character_reject_truncate();

-- Candidate grants match0017/0030; publication/application remains separately gated.
DO $$
DECLARE
    v_function TEXT;
BEGIN
    FOREACH v_function IN ARRAY ARRAY[
        'game_character_progression_consistency_guard()',
        'game_character_proficiency_receipt_guard()',
        'game_character_proficiency_track_guard()',
        'game_character_proficiency_row_guard()'
    ] LOOP
        EXECUTE format('ALTER FUNCTION %s SET search_path = %I, pg_temp', v_function, current_schema());
    END LOOP;
END $$;

REVOKE ALL ON game_character_proficiency_receipts, game_character_proficiency_receipt_lines,
    game_character_proficiency FROM PUBLIC;
REVOKE ALL ON FUNCTION game_character_progression_consistency_guard(),
    game_character_proficiency_receipt_guard(), game_character_proficiency_track_guard(),
    game_character_proficiency_row_guard() FROM PUBLIC;
GRANT SELECT, INSERT ON game_character_proficiency_receipts,
    game_character_proficiency_receipt_lines TO oteryn_game_runtime;
GRANT SELECT, INSERT, UPDATE ON game_character_proficiency TO oteryn_game_runtime;
GRANT SELECT ON game_character_proficiency_receipts, game_character_proficiency_receipt_lines,
    game_character_proficiency TO oteryn_game_control;
