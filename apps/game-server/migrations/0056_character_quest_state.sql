-- QUEST-STATE-1 / migration 0056 (QUEST-STATE-0
-- `reviews/OTERYN_GAME_QUEST_STATE0_QUEST_PROGRESS_STORE_DECISION_2026-09-30.md` §3-§6, §9 and
-- §13, decision QUEST-STATE0-CHARACTER-TRACK-STORE-V1): durable quest progress.
--
--   * `game_character_quest_tracks`: one integer per (Character, track key), owned by one quest.
--     No row reads as the track's declared initial value. Rows are never deleted or rekeyed.
--   * `game_character_quest_states`: one row per (Character, quest), written by its first
--     transition: the pinned content revision and `definition_hash` (§6), and the revision of
--     the receipt that completed it, if any. Never deleted; the pin never changes here (a
--     DUR-04 migration is a later child).
--   * `game_character_quest_receipts`: one immutable receipt per committed transition, keyed by
--     (Character, cause occurrence, transition key) (§5.1), that advances the shared
--     CharacterRevision with level and experience unchanged (§5.2), the ninth receipt kind of
--     the shared consistency guard (§13.1). It keeps the request-only binding, the quest's pin
--     and every effect's track, value before and value after.
--   * `game_character_quest_obligations`: the reward-claim obligation (§5.4), an item-only
--     claim's companion row outside the revision chain: inserted `PENDING` only with its claim's
--     MINT receipt, set to `CONSUMED` (terminal) only with the quest receipt that names it, set
--     to `REFUSED` (terminal) or `WAITING_MIGRATION` by a refused attempt. Never deleted, so a
--     `claim_obligation` receipt is proven against the row it consumed.
--
-- A cause is (kind, id, ordinal): a CommandRef is (GameSessionId, CommandId) under `command`,
-- `use` or `claim_obligation`; a creature-death reward occurrence is (occurrence, 0). The 0012
-- claim guards are extended by a new body of `game_reward_claim_mint_consistency_guard`, never by
-- editing 0012. Rollback after receipts exist may only stop new writes: the receipts, their
-- chain arm and the obligation guards stay (the 0020 pattern).

CREATE TABLE game_character_quest_receipts (
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    cause_kind TEXT NOT NULL
        CHECK (cause_kind IN ('command', 'use', 'claim_obligation', 'creature_death')),
    cause_id UUID NOT NULL CHECK (game_character_is_uuid_v7(cause_id)),
    cause_ordinal NUMERIC(20,0) NOT NULL
        CHECK (cause_ordinal BETWEEN 0 AND 18446744073709551615),
    transition_key TEXT NOT NULL
        CHECK (transition_key ~ '^oteryn:[A-Za-z0-9._:/-]+$' AND octet_length(transition_key) <= 128),
    quest_key TEXT NOT NULL
        CHECK (quest_key ~ '^oteryn:[A-Za-z0-9._:/-]+$' AND octet_length(quest_key) <= 128),
    request_binding BYTEA NOT NULL CHECK (octet_length(request_binding) BETWEEN 1 AND 1024),
    pinned_content_revision TEXT NOT NULL
        CHECK (pinned_content_revision ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'),
    definition_hash BYTEA NOT NULL CHECK (octet_length(definition_hash) = 32),
    completes BOOLEAN NOT NULL,
    track_keys TEXT[] NOT NULL,
    values_before BIGINT[] NOT NULL,
    values_after BIGINT[] NOT NULL,
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
    -- Stamped by the 0011 trigger, never caller-supplied: the obligation consume guard proves
    -- its receipt was inserted by the same physical transaction.
    created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id(),
    -- A CommandRef has a CommandId of at least 1; a creature-death occurrence has none.
    CONSTRAINT game_character_quest_receipt_cause_shape
        CHECK ((cause_kind = 'creature_death') = (cause_ordinal = 0)),
    -- One to eight effects (RL-02), or none for a transition that only completes its quest;
    -- one value before and after per track, in track order, no NULL.
    CONSTRAINT game_character_quest_receipt_effects_shape CHECK (
        cardinality(track_keys) BETWEEN 0 AND 8
        AND (cardinality(track_keys) > 0 OR completes)
        AND (cardinality(track_keys) = 0
             OR (array_ndims(track_keys) = 1 AND array_lower(track_keys, 1) = 1
                 AND array_ndims(values_before) = 1 AND array_lower(values_before, 1) = 1
                 AND array_ndims(values_after) = 1 AND array_lower(values_after, 1) = 1))
        AND cardinality(values_before) = cardinality(track_keys)
        AND cardinality(values_after) = cardinality(track_keys)
        AND array_position(track_keys, NULL) IS NULL
        AND array_position(values_before, NULL) IS NULL
        AND array_position(values_after, NULL) IS NULL),
    PRIMARY KEY (character_id, cause_id, cause_ordinal, transition_key),
    UNIQUE (character_id, original_character_revision),
    UNIQUE (character_id, committed_character_revision)
);

CREATE TABLE game_character_quest_tracks (
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    track_key TEXT NOT NULL
        CHECK (track_key ~ '^oteryn:[A-Za-z0-9._:/-]+$' AND octet_length(track_key) <= 128),
    quest_key TEXT NOT NULL
        CHECK (quest_key ~ '^oteryn:[A-Za-z0-9._:/-]+$' AND octet_length(quest_key) <= 128),
    value BIGINT NOT NULL,
    -- The receipt that last wrote this track (§3 `last_receipt`).
    committed_character_revision NUMERIC(20,0) NOT NULL
        CHECK (committed_character_revision BETWEEN 2 AND 18446744073709551615),
    PRIMARY KEY (character_id, track_key)
);

CREATE TABLE game_character_quest_states (
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    quest_key TEXT NOT NULL
        CHECK (quest_key ~ '^oteryn:[A-Za-z0-9._:/-]+$' AND octet_length(quest_key) <= 128),
    pinned_content_revision TEXT NOT NULL
        CHECK (pinned_content_revision ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'),
    definition_hash BYTEA NOT NULL CHECK (octet_length(definition_hash) = 32),
    started_character_revision NUMERIC(20,0) NOT NULL
        CHECK (started_character_revision BETWEEN 2 AND 18446744073709551615),
    completed_character_revision NUMERIC(20,0) NULL
        CHECK (completed_character_revision >= started_character_revision),
    -- The receipt that last moved this quest.
    committed_character_revision NUMERIC(20,0) NOT NULL
        CHECK (committed_character_revision >= started_character_revision),
    CHECK (completed_character_revision <= committed_character_revision),
    PRIMARY KEY (character_id, quest_key)
);

CREATE TABLE game_character_quest_obligations (
    claim_game_session_id UUID NOT NULL CHECK (game_character_is_uuid_v7(claim_game_session_id)),
    claim_command_id NUMERIC(20,0) NOT NULL
        CHECK (claim_command_id BETWEEN 1 AND 18446744073709551615),
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    transition_key TEXT NOT NULL
        CHECK (transition_key ~ '^oteryn:[A-Za-z0-9._:/-]+$' AND octet_length(transition_key) <= 128),
    state TEXT NOT NULL CHECK (state IN ('PENDING', 'CONSUMED', 'REFUSED', 'WAITING_MIGRATION')),
    result_code TEXT NULL,
    created_at BIGINT NOT NULL CHECK (created_at >= 0),
    updated_at BIGINT NOT NULL CHECK (updated_at >= created_at),
    created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id(),
    CONSTRAINT game_character_quest_obligation_result CHECK (CASE state
        WHEN 'PENDING' THEN result_code IS NULL
        WHEN 'CONSUMED' THEN result_code IS NULL
        WHEN 'WAITING_MIGRATION' THEN result_code = 'REVISION_MISMATCH'
        ELSE result_code IN ('STAGE_MISMATCH', 'OUT_OF_RANGE', 'NOT_SUPPORTED', 'CAPACITY_EXCEEDED')
    END),
    PRIMARY KEY (claim_game_session_id, claim_command_id)
);
CREATE INDEX game_character_quest_obligations_by_character
    ON game_character_quest_obligations (character_id, state);

-- The receipt at commit: at the root revision, its effects distinct well-formed keys of its own
-- quest, each track row written by it with its value after, its quest state written by it with
-- its pin (and completed when it completes), and an obligation cause proven by the claim and
-- consumed. Index probes only; never a scan of the Character's whole quest history.
CREATE FUNCTION game_character_quest_receipt_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    v_root_revision NUMERIC(20,0);
    v_state game_character_quest_states%ROWTYPE;
    v_index INTEGER;
BEGIN
    SELECT character_revision INTO v_root_revision
      FROM game_character_roots WHERE character_id = NEW.character_id;
    IF NOT FOUND OR NEW.committed_character_revision <> v_root_revision THEN
        RAISE EXCEPTION 'quest receipt is not at the current Character root revision'
            USING ERRCODE = '23514';
    END IF;
    IF (SELECT count(DISTINCT k) FROM unnest(NEW.track_keys) AS k)
           <> cardinality(NEW.track_keys)
       OR EXISTS (SELECT 1 FROM unnest(NEW.track_keys) AS k
                   WHERE NOT (k ~ '^oteryn:[A-Za-z0-9._:/-]+$' AND octet_length(k) <= 128)) THEN
        RAISE EXCEPTION 'quest receipt effects must name distinct track keys'
            USING ERRCODE = '23514';
    END IF;
    FOR v_index IN 1 .. cardinality(NEW.track_keys) LOOP
        IF NOT EXISTS (
                SELECT 1 FROM game_character_quest_tracks t
                 WHERE t.character_id = NEW.character_id
                   AND t.track_key = NEW.track_keys[v_index]
                   AND t.quest_key = NEW.quest_key
                   AND t.value = NEW.values_after[v_index]
                   AND t.committed_character_revision = NEW.committed_character_revision) THEN
            RAISE EXCEPTION 'quest receipt effect has no track row of its quest written by it'
                USING ERRCODE = '23514';
        END IF;
    END LOOP;
    SELECT * INTO v_state FROM game_character_quest_states
     WHERE character_id = NEW.character_id AND quest_key = NEW.quest_key;
    IF NOT FOUND
       OR v_state.committed_character_revision <> NEW.committed_character_revision
       OR v_state.pinned_content_revision <> NEW.pinned_content_revision
       OR v_state.definition_hash <> NEW.definition_hash
       OR (NEW.completes AND v_state.completed_character_revision IS NULL) THEN
        RAISE EXCEPTION 'quest receipt has no quest state of its pin written by it'
            USING ERRCODE = '23514';
    END IF;
    IF NEW.cause_kind = 'claim_obligation' AND (
           NOT EXISTS (
               SELECT 1 FROM game_reward_claim_mint_receipts r
                WHERE r.game_session_id = NEW.cause_id
                  AND r.command_id = NEW.cause_ordinal
                  AND r.character_id = NEW.character_id)
           OR NOT EXISTS (
               SELECT 1 FROM game_character_quest_obligations o
                WHERE o.claim_game_session_id = NEW.cause_id
                  AND o.claim_command_id = NEW.cause_ordinal
                  AND o.character_id = NEW.character_id
                  AND o.transition_key = NEW.transition_key
                  AND o.state = 'CONSUMED')) THEN
        RAISE EXCEPTION 'quest obligation receipt must consume the obligation of its claim'
            USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END;
$$;

-- A track row equals the effect of the receipt that last wrote it; an update continues from the
-- previous value. Rows are never deleted, rekeyed or moved to another quest, and each write is a
-- later revision.
CREATE FUNCTION game_character_quest_track_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    v_receipt game_character_quest_receipts%ROWTYPE;
    v_index INTEGER;
BEGIN
    SELECT * INTO v_receipt FROM game_character_quest_receipts
     WHERE character_id = NEW.character_id
       AND committed_character_revision = NEW.committed_character_revision;
    v_index := array_position(v_receipt.track_keys, NEW.track_key);
    IF NOT FOUND OR v_index IS NULL
       OR v_receipt.quest_key <> NEW.quest_key
       OR v_receipt.values_after[v_index] <> NEW.value
       OR (TG_OP = 'UPDATE' AND v_receipt.values_before[v_index] <> OLD.value) THEN
        RAISE EXCEPTION 'quest track row does not equal the effect of its receipt'
            USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END;
$$;

CREATE FUNCTION game_character_quest_track_row_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'UPDATE' AND NEW.character_id = OLD.character_id
       AND NEW.track_key = OLD.track_key AND NEW.quest_key = OLD.quest_key
       AND NEW.committed_character_revision > OLD.committed_character_revision THEN
        RETURN NEW;
    END IF;
    RAISE EXCEPTION 'quest track cannot be deleted, rekeyed or rewritten at its revision'
        USING ERRCODE = '23514';
END;
$$;

-- A quest state row is written only with a receipt of its quest at its revision and pin; the
-- completion is the revision of a completing receipt.
CREATE FUNCTION game_character_quest_state_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF NOT EXISTS (
            SELECT 1 FROM game_character_quest_receipts r
             WHERE r.character_id = NEW.character_id
               AND r.committed_character_revision = NEW.committed_character_revision
               AND r.quest_key = NEW.quest_key
               AND r.pinned_content_revision = NEW.pinned_content_revision
               AND r.definition_hash = NEW.definition_hash)
       OR (TG_OP = 'INSERT'
           AND NEW.started_character_revision <> NEW.committed_character_revision)
       OR (NEW.completed_character_revision IS NOT NULL AND NOT EXISTS (
            SELECT 1 FROM game_character_quest_receipts r
             WHERE r.character_id = NEW.character_id
               AND r.committed_character_revision = NEW.completed_character_revision
               AND r.quest_key = NEW.quest_key
               AND r.completes)) THEN
        RAISE EXCEPTION 'quest state is not written by a receipt of its quest'
            USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END;
$$;

CREATE FUNCTION game_character_quest_state_row_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'UPDATE' AND NEW.character_id = OLD.character_id
       AND NEW.quest_key = OLD.quest_key
       AND NEW.pinned_content_revision = OLD.pinned_content_revision
       AND NEW.definition_hash = OLD.definition_hash
       AND NEW.started_character_revision = OLD.started_character_revision
       AND (OLD.completed_character_revision IS NULL
            OR NEW.completed_character_revision = OLD.completed_character_revision)
       AND NEW.committed_character_revision > OLD.committed_character_revision THEN
        RETURN NEW;
    END IF;
    RAISE EXCEPTION 'quest state cannot be deleted, rekeyed, repinned or uncompleted'
        USING ERRCODE = '23514';
END;
$$;

-- An obligation is born PENDING; it moves only PENDING -> REFUSED (terminal),
-- PENDING -> WAITING_MIGRATION and back (a DUR-04 migration), and PENDING -> CONSUMED (terminal)
-- only with the quest receipt naming it, inserted by the same physical transaction. It is never
-- deleted: the CONSUMED row is the receipt guard's proof that the obligation existed.
CREATE FUNCTION game_character_quest_obligation_row_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'INSERT' THEN
        IF NEW.state = 'PENDING' THEN
            RETURN NEW;
        END IF;
    ELSIF TG_OP = 'UPDATE' THEN
        IF (NEW.claim_game_session_id, NEW.claim_command_id, NEW.character_id,
            NEW.transition_key, NEW.created_at, NEW.created_xact_id)
           = (OLD.claim_game_session_id, OLD.claim_command_id, OLD.character_id,
              OLD.transition_key, OLD.created_at, OLD.created_xact_id)
           AND NEW.updated_at >= OLD.updated_at
           AND ((OLD.state = 'PENDING' AND NEW.state IN ('REFUSED', 'WAITING_MIGRATION'))
                OR (OLD.state = 'WAITING_MIGRATION' AND NEW.state = 'PENDING')
                OR (OLD.state = 'PENDING' AND NEW.state = 'CONSUMED' AND EXISTS (
                    SELECT 1 FROM game_character_quest_receipts r
                     WHERE r.character_id = OLD.character_id
                       AND r.cause_kind = 'claim_obligation'
                       AND r.cause_id = OLD.claim_game_session_id
                       AND r.cause_ordinal = OLD.claim_command_id
                       AND r.transition_key = OLD.transition_key
                       AND r.created_xact_id = pg_current_xact_id()))) THEN
            RETURN NEW;
        END IF;
    END IF;
    RAISE EXCEPTION 'quest obligation transition is not allowed' USING ERRCODE = '23514';
END;
$$;

-- An obligation exists only as the companion of its claim's MINT receipt, for the same
-- Character, inserted by the same physical transaction.
CREATE FUNCTION game_character_quest_obligation_proven() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM game_reward_claim_mint_receipts r
         WHERE r.game_session_id = NEW.claim_game_session_id
           AND r.command_id = NEW.claim_command_id
           AND r.character_id = NEW.character_id
           AND r.created_xact_id = pg_current_xact_id()) THEN
        RAISE EXCEPTION 'quest obligation must commit with its reward-claim MINT receipt'
            USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END;
$$;

-- 0012's body plus one arm: an obligation naming this claim is its companion, for the same
-- Character, PENDING and inserted by the same physical transaction.
CREATE OR REPLACE FUNCTION game_reward_claim_mint_consistency_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    item game_item_instances%ROWTYPE;
    slot_item UUID;
    item_places INTEGER;
BEGIN
    SELECT * INTO item FROM game_item_instances WHERE item_instance_id = NEW.item_instance_id;
    SELECT s.item_instance_id INTO slot_item FROM game_item_container_slots s
     WHERE s.character_id = NEW.character_id;
    SELECT (SELECT count(*) FROM game_item_ground_locations g
             WHERE g.item_instance_id = NEW.item_instance_id)
         + (SELECT count(*) FROM game_item_container_slots s
             WHERE s.item_instance_id = NEW.item_instance_id)
         + (SELECT count(*) FROM game_item_container_entries e
             WHERE e.item_instance_id = NEW.item_instance_id)
      INTO item_places;
    IF NOT EXISTS (
           SELECT 1 FROM game_reward_claim_mint_reservations v
            WHERE (v.game_session_id, v.command_id, v.character_id, v.claim_family,
                   v.claim_production_key, v.claim_revision_ref, v.intent_binding,
                   v.transaction_id, v.event_id, v.item_instance_id, v.occurred_at)
                = (NEW.game_session_id, NEW.command_id, NEW.character_id, NEW.claim_family,
                   NEW.claim_production_key, NEW.claim_revision_ref, NEW.intent_binding,
                   NEW.transaction_id, NEW.event_id, NEW.item_instance_id, NEW.occurred_at)
              AND v.world_id = item.world_id)
       OR NOT EXISTS (
           SELECT 1 FROM game_item_audit_outbox a
            WHERE a.event_id = NEW.event_id AND a.transaction_id = NEW.transaction_id
              AND a.item_instance_id = NEW.item_instance_id
              AND a.occurred_at = NEW.occurred_at
              AND a.envelope_sha256 = NEW.envelope_sha256
              AND a.created_xact_id = pg_current_xact_id()
              AND a.publication_state = 1
              AND a.published_at IS NULL)
       -- The claiming Character must be rooted in the minted item's World.
       OR NOT EXISTS (
           SELECT 1 FROM game_character_roots cr
            WHERE cr.character_id = NEW.character_id AND cr.world_id = item.world_id)
       OR item.item_instance_id IS NULL
       OR item.minted_transaction_id IS DISTINCT FROM NEW.transaction_id
       OR item.last_transaction_id IS NOT NULL
       OR item.lifecycle <> 1
       OR item.quantity <> NEW.quantity
       OR item_places <> 1
       OR slot_item IS NULL
       OR NOT EXISTS (
           SELECT 1 FROM game_item_container_entries e
            WHERE e.item_instance_id = NEW.item_instance_id
              AND e.character_id = NEW.character_id
              AND e.parent_item_instance_id = NEW.destination_parent_item_instance_id
              AND e.parent_item_instance_id = slot_item
              AND e.placement_ordinal = NEW.destination_ordinal
              AND e.placed_transaction_id = NEW.transaction_id)
       OR NOT EXISTS (
           SELECT 1 FROM game_reward_claims c
            WHERE c.character_id = NEW.character_id
              AND c.claim_family = NEW.claim_family
              AND c.claim_production_key = NEW.claim_production_key
              AND c.claim_revision_ref = NEW.claim_revision_ref
              AND c.claimed_transaction_id = NEW.transaction_id
              AND c.next_allowed_at IS NULL)
       -- QUEST-STATE-1 (§5.4): the claim's optional quest obligation.
       OR EXISTS (
           SELECT 1 FROM game_character_quest_obligations o
            WHERE o.claim_game_session_id = NEW.game_session_id
              AND o.claim_command_id = NEW.command_id
              AND (o.character_id <> NEW.character_id
                   OR o.state <> 'PENDING'
                   OR o.created_xact_id <> pg_current_xact_id())) THEN
        RAISE EXCEPTION 'reward-claim MINT must commit its reservation, audit event, item, backpack entry and RewardClaim together'
            USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END;
$$;

-- Full inherited 0032 shared guard follows; insertion-only QUEST-STATE-1 arms keep all eight
-- kinds and add the ninth (§13.1).
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
                   AND u.experience_after = NEW.total_experience
                UNION ALL
                -- QUEST-STATE-1: a quest successor explains unchanged Character XP/level.
                SELECT 1 FROM game_character_quest_receipts q
                 WHERE q.character_id = v_character
                   AND q.original_character_revision = OLD.character_revision
                   AND q.committed_character_revision = NEW.character_revision
                   AND q.level_before = OLD.level AND q.level_after = NEW.level
                   AND q.experience_before = OLD.total_experience
                   AND q.experience_after = NEW.total_experience)
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

CREATE TRIGGER game_character_quest_receipts_stamp_xact BEFORE INSERT
    ON game_character_quest_receipts FOR EACH ROW
    EXECUTE FUNCTION game_item_stamp_created_xact_id();
CREATE TRIGGER game_character_quest_receipt_immutable BEFORE UPDATE OR DELETE
    ON game_character_quest_receipts FOR EACH ROW
    EXECUTE FUNCTION game_character_immutable();
CREATE CONSTRAINT TRIGGER game_character_quest_receipt_consistent
    AFTER INSERT ON game_character_quest_receipts
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
    EXECUTE FUNCTION game_character_quest_receipt_guard();
CREATE CONSTRAINT TRIGGER game_character_quest_receipt_progression_consistent
    AFTER INSERT ON game_character_quest_receipts
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
    EXECUTE FUNCTION game_character_progression_consistency_guard();
CREATE TRIGGER game_character_quest_receipts_no_truncate BEFORE TRUNCATE
    ON game_character_quest_receipts EXECUTE FUNCTION game_character_reject_truncate();

CREATE TRIGGER game_character_quest_track_row_guard BEFORE UPDATE OR DELETE
    ON game_character_quest_tracks FOR EACH ROW
    EXECUTE FUNCTION game_character_quest_track_row_guard();
CREATE CONSTRAINT TRIGGER game_character_quest_track_consistent
    AFTER INSERT OR UPDATE ON game_character_quest_tracks
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
    EXECUTE FUNCTION game_character_quest_track_guard();
CREATE TRIGGER game_character_quest_tracks_no_truncate BEFORE TRUNCATE
    ON game_character_quest_tracks EXECUTE FUNCTION game_character_reject_truncate();

CREATE TRIGGER game_character_quest_state_row_guard BEFORE UPDATE OR DELETE
    ON game_character_quest_states FOR EACH ROW
    EXECUTE FUNCTION game_character_quest_state_row_guard();
CREATE CONSTRAINT TRIGGER game_character_quest_state_consistent
    AFTER INSERT OR UPDATE ON game_character_quest_states
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
    EXECUTE FUNCTION game_character_quest_state_guard();
CREATE TRIGGER game_character_quest_states_no_truncate BEFORE TRUNCATE
    ON game_character_quest_states EXECUTE FUNCTION game_character_reject_truncate();

CREATE TRIGGER game_character_quest_obligations_stamp_xact BEFORE INSERT
    ON game_character_quest_obligations FOR EACH ROW
    EXECUTE FUNCTION game_item_stamp_created_xact_id();
CREATE TRIGGER game_character_quest_obligation_row_guard BEFORE INSERT OR UPDATE OR DELETE
    ON game_character_quest_obligations FOR EACH ROW
    EXECUTE FUNCTION game_character_quest_obligation_row_guard();
CREATE CONSTRAINT TRIGGER game_character_quest_obligation_proven
    AFTER INSERT ON game_character_quest_obligations
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
    EXECUTE FUNCTION game_character_quest_obligation_proven();
CREATE TRIGGER game_character_quest_obligations_no_truncate BEFORE TRUNCATE
    ON game_character_quest_obligations EXECUTE FUNCTION game_character_reject_truncate();

-- Candidate grants match 0030/0032; publication/application remains separately gated.
DO $$
DECLARE
    v_function TEXT;
BEGIN
    FOREACH v_function IN ARRAY ARRAY[
        'game_character_progression_consistency_guard()',
        'game_reward_claim_mint_consistency_guard()',
        'game_character_quest_receipt_guard()',
        'game_character_quest_track_guard()',
        'game_character_quest_track_row_guard()',
        'game_character_quest_state_guard()',
        'game_character_quest_state_row_guard()',
        'game_character_quest_obligation_row_guard()',
        'game_character_quest_obligation_proven()'
    ] LOOP
        EXECUTE format('ALTER FUNCTION %s SET search_path = %I, pg_temp', v_function, current_schema());
    END LOOP;
END $$;

REVOKE ALL ON game_character_quest_receipts, game_character_quest_tracks,
    game_character_quest_states, game_character_quest_obligations FROM PUBLIC;
REVOKE ALL ON FUNCTION game_character_progression_consistency_guard(),
    game_reward_claim_mint_consistency_guard(), game_character_quest_receipt_guard(),
    game_character_quest_track_guard(), game_character_quest_track_row_guard(),
    game_character_quest_state_guard(), game_character_quest_state_row_guard(),
    game_character_quest_obligation_row_guard(), game_character_quest_obligation_proven()
    FROM PUBLIC;
GRANT SELECT, INSERT ON game_character_quest_receipts TO oteryn_game_runtime;
GRANT SELECT, INSERT, UPDATE ON game_character_quest_tracks, game_character_quest_states
    TO oteryn_game_runtime;
GRANT SELECT, INSERT ON game_character_quest_obligations TO oteryn_game_runtime;
GRANT UPDATE (state, result_code, updated_at) ON game_character_quest_obligations
    TO oteryn_game_runtime;
GRANT SELECT ON game_character_quest_receipts, game_character_quest_tracks,
    game_character_quest_states, game_character_quest_obligations TO oteryn_game_control;
