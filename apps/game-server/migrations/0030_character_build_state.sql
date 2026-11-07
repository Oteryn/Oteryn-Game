-- CHAR-BUILD-1 (A13 `reviews/OTERYN_GAME_A13_CHARACTER_BUILD_STATE_DECISION_2026-09-29.md`
-- §4.1, §4.2 and §4.6, decision A13-CHARACTER-BUILD-STATE-V1, owner decisions D150-D151, as
-- amended by SKILLS-0 `reviews/OTERYN_GAME_SKILLS0_WEAPON_SKILLS_DECISION_2026-09-30.md` §3.1,
-- §3.2 and §3.6): durable Character build state, i.e. vocation, magic level with `mana_spent`,
-- and the level and tries of the seven skills (fist, club, sword, axe, distance, shielding,
-- fishing). Migration only (CHAR-BUILD-1a): the writer, reconcile and admission load follow in
-- CHAR-BUILD-1b, as STANCE-1 followed 0017.
--
-- Scope of this migration:
--   * `game_character_build_state`: at most one row per Character, the projection of its latest
--     build-carrying receipt. No row means vocation `none`, magic level 0, `mana_spent` 0 and
--     every skill at level 10 with 0 tries (the chain seed). Character creation inserts nothing,
--     existing Characters need no backfill, and the row is never deleted;
--   * `game_character_build_receipts`: one immutable receipt per build change, keyed by the
--     build occurrence (UUIDv7), that advances the global CharacterRevision with experience and
--     level unchanged. The cause CHECK binds the direction: `training` keeps the vocation and
--     no family goes down, `vocation_choice` goes from `none` to a vocation key (the writer
--     re-levels progress, SKILLS-0 §3.3), `promotion` goes from one key to another with every
--     family unchanged. A receipt that changes nothing is not a receipt;
--   * nullable build fields on `game_character_death_receipts` (DEATH-0 §3.1 as amended): all
--     NULL (the death did not change build state; DEATH-1 writes NULL) or all set, with the
--     vocation unchanged and every family not higher, at least one strictly lower;
--   * the 0026 deferred consistency guard now admits a seventh receipt kind (XP, death, stance,
--     Bestiary kill, charm, monk state or build), exactly one receipt per revision. Its previous
--     body is kept verbatim and gains only the build arms: the revision-one check, the chain,
--     the stance chain (a build receipt that prunes the stance, A13 §4.2 "Stance fields"), the
--     build chain and build row, and the successor that explains a state transition.
-- A family is compared as (`level`, `tries`) in that order: magic level is (`magic_level`,
-- `mana_spent`). Level bounds are storage bounds, not game rules. The 0009-0026 tables, their
-- CHECKs, the 0017 state guard (its equal arm already admits a build successor) and every other
-- guard are unchanged. No CHECK calls a new function, so the runtime needs no new EXECUTE grant
-- (#1271 F2); PRIV-GUARD-1 walks the catalog for that.
--
-- Rollback: applied migrations are immutable, so rollback is a new migration. Before any build
-- receipt or death build field exists it drops the two tables and the death columns and restores
-- the 0026 guard body. After they exist they are part of the CharacterRevision chain that
-- `verify_character_integrity` checks: a rollback must keep them (and their guard arms) and may
-- only stop new writes, e.g. by revoking the runtime INSERT grant.

-- A13 §4.2 and SKILLS-0 §3.2. `before` and `after` of level and experience are equal so the
-- cross-kind chain reads uniformly.
CREATE TABLE game_character_build_receipts (
    build_occurrence_id UUID PRIMARY KEY CHECK (game_character_is_uuid_v7(build_occurrence_id)),
    command_binding BYTEA NOT NULL CHECK (octet_length(command_binding) BETWEEN 1 AND 1024),
    policy_digest BYTEA NOT NULL CHECK (octet_length(policy_digest) = 32),
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    original_character_revision NUMERIC(20,0) NOT NULL
        CHECK (original_character_revision BETWEEN 1 AND 18446744073709551614),
    committed_character_revision NUMERIC(20,0) NOT NULL
        CHECK (committed_character_revision = original_character_revision + 1),
    cause TEXT NOT NULL CHECK (cause IN ('training', 'vocation_choice', 'promotion')),
    level_before BIGINT NOT NULL CHECK (level_before BETWEEN 1 AND 4294967295),
    level_after BIGINT NOT NULL CHECK (level_after = level_before),
    experience_before BIGINT NOT NULL CHECK (experience_before >= 0),
    experience_after BIGINT NOT NULL CHECK (experience_after = experience_before),
    vocation_before TEXT NOT NULL CHECK (vocation_before ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'),
    magic_level_before INTEGER NOT NULL CHECK (magic_level_before BETWEEN 0 AND 1000),
    mana_spent_before BIGINT NOT NULL CHECK (mana_spent_before >= 0),
    fist_level_before INTEGER NOT NULL CHECK (fist_level_before BETWEEN 10 AND 1000),
    fist_tries_before BIGINT NOT NULL CHECK (fist_tries_before >= 0),
    club_level_before INTEGER NOT NULL CHECK (club_level_before BETWEEN 10 AND 1000),
    club_tries_before BIGINT NOT NULL CHECK (club_tries_before >= 0),
    sword_level_before INTEGER NOT NULL CHECK (sword_level_before BETWEEN 10 AND 1000),
    sword_tries_before BIGINT NOT NULL CHECK (sword_tries_before >= 0),
    axe_level_before INTEGER NOT NULL CHECK (axe_level_before BETWEEN 10 AND 1000),
    axe_tries_before BIGINT NOT NULL CHECK (axe_tries_before >= 0),
    distance_level_before INTEGER NOT NULL CHECK (distance_level_before BETWEEN 10 AND 1000),
    distance_tries_before BIGINT NOT NULL CHECK (distance_tries_before >= 0),
    shielding_level_before INTEGER NOT NULL CHECK (shielding_level_before BETWEEN 10 AND 1000),
    shielding_tries_before BIGINT NOT NULL CHECK (shielding_tries_before >= 0),
    fishing_level_before INTEGER NOT NULL CHECK (fishing_level_before BETWEEN 10 AND 1000),
    fishing_tries_before BIGINT NOT NULL CHECK (fishing_tries_before >= 0),
    vocation_after TEXT NOT NULL CHECK (vocation_after ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'),
    magic_level_after INTEGER NOT NULL CHECK (magic_level_after BETWEEN 0 AND 1000),
    mana_spent_after BIGINT NOT NULL CHECK (mana_spent_after >= 0),
    fist_level_after INTEGER NOT NULL CHECK (fist_level_after BETWEEN 10 AND 1000),
    fist_tries_after BIGINT NOT NULL CHECK (fist_tries_after >= 0),
    club_level_after INTEGER NOT NULL CHECK (club_level_after BETWEEN 10 AND 1000),
    club_tries_after BIGINT NOT NULL CHECK (club_tries_after >= 0),
    sword_level_after INTEGER NOT NULL CHECK (sword_level_after BETWEEN 10 AND 1000),
    sword_tries_after BIGINT NOT NULL CHECK (sword_tries_after >= 0),
    axe_level_after INTEGER NOT NULL CHECK (axe_level_after BETWEEN 10 AND 1000),
    axe_tries_after BIGINT NOT NULL CHECK (axe_tries_after >= 0),
    distance_level_after INTEGER NOT NULL CHECK (distance_level_after BETWEEN 10 AND 1000),
    distance_tries_after BIGINT NOT NULL CHECK (distance_tries_after >= 0),
    shielding_level_after INTEGER NOT NULL CHECK (shielding_level_after BETWEEN 10 AND 1000),
    shielding_tries_after BIGINT NOT NULL CHECK (shielding_tries_after >= 0),
    fishing_level_after INTEGER NOT NULL CHECK (fishing_level_after BETWEEN 10 AND 1000),
    fishing_tries_after BIGINT NOT NULL CHECK (fishing_tries_after >= 0),
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
    -- The cause direction (A13 §4.2, SKILLS-0 §3.2), so one cause cannot pass as another.
    CONSTRAINT game_character_build_receipts_cause_direction CHECK (CASE cause
        WHEN 'training' THEN
            vocation_after = vocation_before
            AND (magic_level_after, mana_spent_after) >= (magic_level_before, mana_spent_before)
            AND (fist_level_after, fist_tries_after) >= (fist_level_before, fist_tries_before)
            AND (club_level_after, club_tries_after) >= (club_level_before, club_tries_before)
            AND (sword_level_after, sword_tries_after) >= (sword_level_before, sword_tries_before)
            AND (axe_level_after, axe_tries_after) >= (axe_level_before, axe_tries_before)
            AND (distance_level_after, distance_tries_after)
                >= (distance_level_before, distance_tries_before)
            AND (shielding_level_after, shielding_tries_after)
                >= (shielding_level_before, shielding_tries_before)
            AND (fishing_level_after, fishing_tries_after)
                >= (fishing_level_before, fishing_tries_before)
            AND ((magic_level_after, mana_spent_after) > (magic_level_before, mana_spent_before)
              OR (fist_level_after, fist_tries_after) > (fist_level_before, fist_tries_before)
              OR (club_level_after, club_tries_after) > (club_level_before, club_tries_before)
              OR (sword_level_after, sword_tries_after) > (sword_level_before, sword_tries_before)
              OR (axe_level_after, axe_tries_after) > (axe_level_before, axe_tries_before)
              OR (distance_level_after, distance_tries_after)
                 > (distance_level_before, distance_tries_before)
              OR (shielding_level_after, shielding_tries_after)
                 > (shielding_level_before, shielding_tries_before)
              OR (fishing_level_after, fishing_tries_after)
                 > (fishing_level_before, fishing_tries_before))
        WHEN 'vocation_choice' THEN
            vocation_before = 'none' AND vocation_after <> 'none'
        WHEN 'promotion' THEN
            vocation_before <> 'none' AND vocation_after <> 'none'
            AND vocation_after <> vocation_before
            AND (magic_level_after, mana_spent_after, fist_level_after, fist_tries_after,
                 club_level_after, club_tries_after, sword_level_after, sword_tries_after,
                 axe_level_after, axe_tries_after, distance_level_after, distance_tries_after,
                 shielding_level_after, shielding_tries_after, fishing_level_after,
                 fishing_tries_after)
              = (magic_level_before, mana_spent_before, fist_level_before, fist_tries_before,
                 club_level_before, club_tries_before, sword_level_before, sword_tries_before,
                 axe_level_before, axe_tries_before, distance_level_before, distance_tries_before,
                 shielding_level_before, shielding_tries_before, fishing_level_before,
                 fishing_tries_before)
    END),
    -- A13 §4.2 "Stance fields": both NULL, or a prune (stored key to NULL) carried by a
    -- vocation change.
    CONSTRAINT game_character_build_receipts_stance_prune CHECK (
        (stance_before IS NULL AND stance_after IS NULL)
        OR (cause IN ('vocation_choice', 'promotion')
            AND stance_before IS NOT NULL AND stance_after IS NULL)),
    UNIQUE (character_id, original_character_revision),
    UNIQUE (character_id, committed_character_revision)
);

-- A13 §4.1 and SKILLS-0 §3.1. The deferred guard binds every column to the latest
-- build-carrying receipt: a build receipt, or a death receipt with build fields.
CREATE TABLE game_character_build_state (
    character_id UUID PRIMARY KEY REFERENCES game_character_roots(character_id),
    vocation TEXT NOT NULL CHECK (vocation ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'),
    magic_level INTEGER NOT NULL CHECK (magic_level BETWEEN 0 AND 1000),
    mana_spent BIGINT NOT NULL CHECK (mana_spent >= 0),
    fist_level INTEGER NOT NULL CHECK (fist_level BETWEEN 10 AND 1000),
    fist_tries BIGINT NOT NULL CHECK (fist_tries >= 0),
    club_level INTEGER NOT NULL CHECK (club_level BETWEEN 10 AND 1000),
    club_tries BIGINT NOT NULL CHECK (club_tries >= 0),
    sword_level INTEGER NOT NULL CHECK (sword_level BETWEEN 10 AND 1000),
    sword_tries BIGINT NOT NULL CHECK (sword_tries >= 0),
    axe_level INTEGER NOT NULL CHECK (axe_level BETWEEN 10 AND 1000),
    axe_tries BIGINT NOT NULL CHECK (axe_tries >= 0),
    distance_level INTEGER NOT NULL CHECK (distance_level BETWEEN 10 AND 1000),
    distance_tries BIGINT NOT NULL CHECK (distance_tries >= 0),
    shielding_level INTEGER NOT NULL CHECK (shielding_level BETWEEN 10 AND 1000),
    shielding_tries BIGINT NOT NULL CHECK (shielding_tries >= 0),
    fishing_level INTEGER NOT NULL CHECK (fishing_level BETWEEN 10 AND 1000),
    fishing_tries BIGINT NOT NULL CHECK (fishing_tries >= 0),
    committed_character_revision NUMERIC(20,0) NOT NULL
        CHECK (committed_character_revision BETWEEN 2 AND 18446744073709551615),
    last_build_occurrence_id UUID NOT NULL
        CHECK (game_character_is_uuid_v7(last_build_occurrence_id))
);

-- A13 §4.6 and SKILLS-0 §3.6: the loss a death takes, carried by the death receipt itself.
-- `vocation` is unchanged by a death, so it is both its before and its after value.
ALTER TABLE game_character_death_receipts
    ADD COLUMN vocation TEXT NULL
        CHECK (vocation IS NULL OR vocation ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'),
    ADD COLUMN magic_level_before INTEGER NULL CHECK (magic_level_before BETWEEN 0 AND 1000),
    ADD COLUMN mana_spent_before BIGINT NULL CHECK (mana_spent_before >= 0),
    ADD COLUMN fist_level_before INTEGER NULL CHECK (fist_level_before BETWEEN 10 AND 1000),
    ADD COLUMN fist_tries_before BIGINT NULL CHECK (fist_tries_before >= 0),
    ADD COLUMN club_level_before INTEGER NULL CHECK (club_level_before BETWEEN 10 AND 1000),
    ADD COLUMN club_tries_before BIGINT NULL CHECK (club_tries_before >= 0),
    ADD COLUMN sword_level_before INTEGER NULL CHECK (sword_level_before BETWEEN 10 AND 1000),
    ADD COLUMN sword_tries_before BIGINT NULL CHECK (sword_tries_before >= 0),
    ADD COLUMN axe_level_before INTEGER NULL CHECK (axe_level_before BETWEEN 10 AND 1000),
    ADD COLUMN axe_tries_before BIGINT NULL CHECK (axe_tries_before >= 0),
    ADD COLUMN distance_level_before INTEGER NULL
        CHECK (distance_level_before BETWEEN 10 AND 1000),
    ADD COLUMN distance_tries_before BIGINT NULL CHECK (distance_tries_before >= 0),
    ADD COLUMN shielding_level_before INTEGER NULL
        CHECK (shielding_level_before BETWEEN 10 AND 1000),
    ADD COLUMN shielding_tries_before BIGINT NULL CHECK (shielding_tries_before >= 0),
    ADD COLUMN fishing_level_before INTEGER NULL CHECK (fishing_level_before BETWEEN 10 AND 1000),
    ADD COLUMN fishing_tries_before BIGINT NULL CHECK (fishing_tries_before >= 0),
    ADD COLUMN magic_level_after INTEGER NULL CHECK (magic_level_after BETWEEN 0 AND 1000),
    ADD COLUMN mana_spent_after BIGINT NULL CHECK (mana_spent_after >= 0),
    ADD COLUMN fist_level_after INTEGER NULL CHECK (fist_level_after BETWEEN 10 AND 1000),
    ADD COLUMN fist_tries_after BIGINT NULL CHECK (fist_tries_after >= 0),
    ADD COLUMN club_level_after INTEGER NULL CHECK (club_level_after BETWEEN 10 AND 1000),
    ADD COLUMN club_tries_after BIGINT NULL CHECK (club_tries_after >= 0),
    ADD COLUMN sword_level_after INTEGER NULL CHECK (sword_level_after BETWEEN 10 AND 1000),
    ADD COLUMN sword_tries_after BIGINT NULL CHECK (sword_tries_after >= 0),
    ADD COLUMN axe_level_after INTEGER NULL CHECK (axe_level_after BETWEEN 10 AND 1000),
    ADD COLUMN axe_tries_after BIGINT NULL CHECK (axe_tries_after >= 0),
    ADD COLUMN distance_level_after INTEGER NULL CHECK (distance_level_after BETWEEN 10 AND 1000),
    ADD COLUMN distance_tries_after BIGINT NULL CHECK (distance_tries_after >= 0),
    ADD COLUMN shielding_level_after INTEGER NULL
        CHECK (shielding_level_after BETWEEN 10 AND 1000),
    ADD COLUMN shielding_tries_after BIGINT NULL CHECK (shielding_tries_after >= 0),
    ADD COLUMN fishing_level_after INTEGER NULL CHECK (fishing_level_after BETWEEN 10 AND 1000),
    ADD COLUMN fishing_tries_after BIGINT NULL CHECK (fishing_tries_after >= 0),
    ADD CONSTRAINT game_character_death_receipts_build_all_or_none CHECK (num_nulls(
        vocation, magic_level_before, mana_spent_before, fist_level_before, fist_tries_before,
        club_level_before, club_tries_before, sword_level_before, sword_tries_before,
        axe_level_before, axe_tries_before, distance_level_before, distance_tries_before,
        shielding_level_before, shielding_tries_before, fishing_level_before,
        fishing_tries_before, magic_level_after, mana_spent_after, fist_level_after,
        fist_tries_after, club_level_after, club_tries_after, sword_level_after,
        sword_tries_after, axe_level_after, axe_tries_after, distance_level_after,
        distance_tries_after, shielding_level_after, shielding_tries_after, fishing_level_after,
        fishing_tries_after) IN (0, 33)),
    -- Every family not higher, at least one strictly lower; so a death is never the first
    -- build-carrying receipt (the seed cannot go lower).
    ADD CONSTRAINT game_character_death_receipts_build_loss CHECK (vocation IS NULL OR (
        (magic_level_after, mana_spent_after) <= (magic_level_before, mana_spent_before)
        AND (fist_level_after, fist_tries_after) <= (fist_level_before, fist_tries_before)
        AND (club_level_after, club_tries_after) <= (club_level_before, club_tries_before)
        AND (sword_level_after, sword_tries_after) <= (sword_level_before, sword_tries_before)
        AND (axe_level_after, axe_tries_after) <= (axe_level_before, axe_tries_before)
        AND (distance_level_after, distance_tries_after)
            <= (distance_level_before, distance_tries_before)
        AND (shielding_level_after, shielding_tries_after)
            <= (shielding_level_before, shielding_tries_before)
        AND (fishing_level_after, fishing_tries_after)
            <= (fishing_level_before, fishing_tries_before)
        AND ((magic_level_after, mana_spent_after) < (magic_level_before, mana_spent_before)
          OR (fist_level_after, fist_tries_after) < (fist_level_before, fist_tries_before)
          OR (club_level_after, club_tries_after) < (club_level_before, club_tries_before)
          OR (sword_level_after, sword_tries_after) < (sword_level_before, sword_tries_before)
          OR (axe_level_after, axe_tries_after) < (axe_level_before, axe_tries_before)
          OR (distance_level_after, distance_tries_after)
             < (distance_level_before, distance_tries_before)
          OR (shielding_level_after, shielding_tries_after)
             < (shielding_level_before, shielding_tries_before)
          OR (fishing_level_after, fishing_tries_after)
             < (fishing_level_before, fishing_tries_before))));

-- The 0026 chain guard (XP, death, stance, Bestiary, charm and monk state kinds) with the build
-- kind added to the revision-one check, the receipt chain, the stance chain and the successor
-- that explains a state transition, plus the build chain and build row check (#1271 F1: the
-- latest merged guard, every arm kept). Everything else is unchanged.
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

-- A13 §4.1: the build row is never deleted and never moves to another Character; its values
-- are bound at commit by the consistency guard.
CREATE FUNCTION game_character_build_state_row_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'UPDATE' AND NEW.character_id = OLD.character_id THEN
        RETURN NEW;
    END IF;
    RAISE EXCEPTION 'Character build state is never deleted or reassigned'
        USING ERRCODE = '23514';
END;
$$;

CREATE TRIGGER game_character_build_receipt_immutable BEFORE UPDATE OR DELETE
    ON game_character_build_receipts FOR EACH ROW
    EXECUTE FUNCTION game_character_immutable();
CREATE CONSTRAINT TRIGGER game_character_build_receipt_progression_consistent
    AFTER INSERT ON game_character_build_receipts
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
    EXECUTE FUNCTION game_character_progression_consistency_guard();
CREATE TRIGGER game_character_build_receipts_no_truncate BEFORE TRUNCATE
    ON game_character_build_receipts EXECUTE FUNCTION game_character_reject_truncate();

CREATE TRIGGER game_character_build_state_row_guard BEFORE UPDATE OR DELETE
    ON game_character_build_state FOR EACH ROW
    EXECUTE FUNCTION game_character_build_state_row_guard();
CREATE CONSTRAINT TRIGGER game_character_build_state_progression_consistent
    AFTER INSERT OR UPDATE OR DELETE ON game_character_build_state
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
    EXECUTE FUNCTION game_character_progression_consistency_guard();
CREATE TRIGGER game_character_build_state_no_truncate BEFORE TRUNCATE
    ON game_character_build_state EXECUTE FUNCTION game_character_reject_truncate();

-- CREATE OR REPLACE resets a function's configuration, so the replaced guard gets its fixed
-- search_path again, as does the new row guard.
DO $$
DECLARE
    v_function TEXT;
BEGIN
    FOREACH v_function IN ARRAY ARRAY[
        'game_character_progression_consistency_guard()',
        'game_character_build_state_row_guard()'
    ] LOOP
        EXECUTE format('ALTER FUNCTION %s SET search_path = %I, pg_temp', v_function, current_schema());
    END LOOP;
END $$;

REVOKE ALL ON game_character_build_receipts, game_character_build_state FROM PUBLIC;
REVOKE ALL ON FUNCTION game_character_build_state_row_guard() FROM PUBLIC;
-- Runtime: the CHAR-BUILD-1b writer inserts a receipt and inserts or updates the build row in
-- the same transaction; it never deletes the row. A death with build fields (the DEATH ML loss
-- child) updates the row under the existing death receipt INSERT grant. The CHECKs call only
-- `game_character_is_uuid_v7(uuid)`, which the runtime already executes (0006).
GRANT SELECT, INSERT ON game_character_build_receipts TO oteryn_game_runtime;
GRANT SELECT, INSERT, UPDATE ON game_character_build_state TO oteryn_game_runtime;
GRANT SELECT ON game_character_build_receipts, game_character_build_state TO oteryn_game_control;
