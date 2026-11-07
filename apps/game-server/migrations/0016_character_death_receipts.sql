-- DEATH-0 (`reviews/OTERYN_GAME_DEATH0_CHARACTER_DEATH_RECEIPT_DECISION_2026-09-28.md`
-- §3.1-§3.4, decision DEATH0-CHARACTER-DEATH-RECEIPT-V1): storage for the
-- durable Character death transaction of the Reference first player death
-- decision (§4.3). Migration only: no writer exists until DEATH-1.
--
-- Scope of this migration:
--   * `game_character_death_receipts`: one immutable receipt per death, keyed
--     by the `PlayerDeathOccurrence` (UUIDv7), that advances the global
--     CharacterRevision exactly like an XP receipt but with non-increasing
--     experience and level;
--   * `game_character_blessings`: held blessings; a death consumes them in the
--     same transaction as its receipt, which records the set before and after;
--   * `game_character_pending_respawns`: the committed death's respawn
--     obligation, outside the revision chain;
--   * the 0009 state guard and deferred consistency guard now admit one chain
--     with two receipt kinds (XP or death), exactly one receipt per revision.
-- The 0009 XP receipt table, its CHECKs and the revision-one initializer path
-- are unchanged.

-- Canonical blessing set: a one-dimensional array of distinct bounded keys in
-- strictly ascending byte order (collation "C"), so set equality is array
-- equality. 32 is a storage bound, not a game rule.
CREATE FUNCTION game_character_is_blessing_set(value TEXT[]) RETURNS BOOLEAN
LANGUAGE sql IMMUTABLE AS $$
    SELECT value IS NOT NULL
       AND cardinality(value) <= 32
       AND (cardinality(value) = 0
            OR (array_ndims(value) = 1 AND array_lower(value, 1) = 1))
       AND NOT EXISTS (
            SELECT 1 FROM unnest(value) WITH ORDINALITY AS e(key, ordinal)
             WHERE e.key IS NULL
                OR e.key !~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'
                OR (e.ordinal > 1
                    AND value[e.ordinal - 1] COLLATE "C" >= e.key COLLATE "C"))
$$;

-- §3.1. The death cell mirrors the Ground location columns (0010) so a
-- resumed DEATH-3 item workflow can drop at it after runtime state is lost.
CREATE TABLE game_character_death_receipts (
    death_occurrence_id UUID PRIMARY KEY CHECK (game_character_is_uuid_v7(death_occurrence_id)),
    command_binding BYTEA NOT NULL CHECK (octet_length(command_binding) BETWEEN 1 AND 1024),
    policy_digest BYTEA NOT NULL CHECK (octet_length(policy_digest) = 32),
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    original_character_revision NUMERIC(20,0) NOT NULL
        CHECK (original_character_revision BETWEEN 1 AND 18446744073709551614),
    committed_character_revision NUMERIC(20,0) NOT NULL
        CHECK (committed_character_revision = original_character_revision + 1),
    level_before BIGINT NOT NULL CHECK (level_before BETWEEN 1 AND 4294967295),
    level_after BIGINT NOT NULL CHECK (level_after BETWEEN 1 AND level_before),
    experience_before BIGINT NOT NULL CHECK (experience_before >= 0),
    experience_after BIGINT NOT NULL CHECK (experience_after BETWEEN 0 AND experience_before),
    experience_lost BIGINT NOT NULL CHECK (experience_lost >= 0),
    blessings_before TEXT[] NOT NULL CHECK (game_character_is_blessing_set(blessings_before)),
    blessings_after TEXT[] NOT NULL CHECK (game_character_is_blessing_set(blessings_after)),
    amulet_of_loss_item_id UUID NULL
        CHECK (amulet_of_loss_item_id IS NULL OR game_character_is_uuid_v7(amulet_of_loss_item_id)),
    lost_item_ids UUID[] NOT NULL,
    death_world_id UUID NOT NULL CHECK (game_character_is_uuid_v7(death_world_id)),
    death_channel_id UUID NOT NULL CHECK (game_character_is_uuid_v7(death_channel_id)),
    death_spatial_position BYTEA NOT NULL
        CHECK (octet_length(death_spatial_position) BETWEEN 1 AND 128),
    death_map_revision TEXT NOT NULL CHECK (octet_length(death_map_revision) BETWEEN 1 AND 512),
    respawn_position BYTEA NOT NULL CHECK (octet_length(respawn_position) BETWEEN 1 AND 128),
    death_policy_revision TEXT NOT NULL CHECK (death_policy_revision ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'),
    profile_revision TEXT NOT NULL CHECK (profile_revision ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'),
    ruleset_revision TEXT NOT NULL CHECK (ruleset_revision ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'),
    content_revision TEXT NOT NULL CHECK (content_revision ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'),
    simulation_revision TEXT NOT NULL CHECK (simulation_revision ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'),
    evidence_revision TEXT NOT NULL CHECK (evidence_revision ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'),
    declaration_revision TEXT NOT NULL CHECK (declaration_revision ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'),
    policy_revision TEXT NOT NULL CHECK (policy_revision ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'),
    reward_revision TEXT NOT NULL CHECK (reward_revision ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'),
    committed_at BIGINT NOT NULL CHECK (committed_at >= 0),
    -- Stamped by the insert trigger below; never caller-supplied.
    created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id(),
    CHECK (experience_lost = experience_before - experience_after),
    -- A death only consumes blessings (after is empty for regular ones).
    CHECK (blessings_after <@ blessings_before),
    -- Death decision §4.4 delivery gap: until DEATH-3 is admitted a death
    -- selects no amulet and loses no item. DEATH-3's migration replaces these
    -- two named constraints with its own bound.
    CONSTRAINT game_character_death_receipts_no_amulet_until_death3
        CHECK (amulet_of_loss_item_id IS NULL),
    CONSTRAINT game_character_death_receipts_no_lost_items_until_death3
        CHECK (cardinality(lost_item_ids) = 0),
    UNIQUE (character_id, original_character_revision),
    UNIQUE (character_id, committed_character_revision),
    UNIQUE (death_occurrence_id, character_id, respawn_position)
);

-- §3.3. Held blessings. No path inserts one until DEATH-4 decides blessing
-- purchase receipts; a death deletes the ones it consumes.
CREATE TABLE game_character_blessings (
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    blessing_key TEXT NOT NULL CHECK (blessing_key ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'),
    provenance TEXT NOT NULL CHECK (provenance ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'),
    PRIMARY KEY (character_id, blessing_key)
);

-- §3.4. One pending respawn per Character, the obligation of its committed
-- death. It neither advances nor needs a CharacterRevision. The primary key
-- is what makes a second death uncommittable while a respawn is pending: every
-- death must insert its own row in its own transaction.
CREATE TABLE game_character_pending_respawns (
    character_id UUID PRIMARY KEY REFERENCES game_character_roots(character_id),
    death_occurrence_id UUID NOT NULL UNIQUE,
    respawn_position BYTEA NOT NULL CHECK (octet_length(respawn_position) BETWEEN 1 AND 128),
    FOREIGN KEY (death_occurrence_id, character_id, respawn_position)
        REFERENCES game_character_death_receipts (death_occurrence_id, character_id, respawn_position)
        DEFERRABLE INITIALLY DEFERRED
);

-- §3.2 state guard: one CharacterRevision successor with unchanged revision
-- fields, in either the XP direction (experience strictly larger, level not
-- lower) or the death direction (experience not larger, level not higher).
-- The deferred guard decides which receipt kind explains it.
CREATE OR REPLACE FUNCTION game_character_progression_state_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'UPDATE'
       AND NEW.character_id = OLD.character_id
       AND NEW.character_revision = OLD.character_revision + 1
       AND ((NEW.total_experience > OLD.total_experience AND NEW.level >= OLD.level)
         OR (NEW.total_experience <= OLD.total_experience AND NEW.level <= OLD.level))
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
    RAISE EXCEPTION 'Character progression permits only one typed XP or death successor'
        USING ERRCODE = '23514';
END;
$$;

-- §3.2 consistency guard (deferred, at commit), over the union of both kinds:
--   * revision one: no receipt of either kind;
--   * otherwise the typed state is at the root revision, XP + death receipts
--     = revision - 1 with exactly one receipt per revision, the receipt of the
--     current revision matches the state, and each receipt's `before` equals
--     its predecessor's `after` across kinds;
--   * each state transition is explained by the receipt of its successor
--     revision, whose `before` is the replaced row. This binds the first
--     receipt too (its predecessor, revision one, has no receipt), so neither
--     kind can record the other's direction. The per-kind CHECKs keep XP
--     strictly increasing and death non-increasing.
CREATE OR REPLACE FUNCTION game_character_progression_consistency_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    v_character UUID := NEW.character_id;
    v_root_revision NUMERIC(20,0);
    v_state game_character_progression_state%ROWTYPE;
BEGIN
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
                   AND d.experience_after = NEW.total_experience) THEN
            RAISE EXCEPTION 'Character progression transition has no matching receipt'
                USING ERRCODE = '23514';
        END IF;
    END IF;
    RETURN NULL;
END;
$$;

-- Death-receipt admission (before insert): stamp the inserting transaction,
-- require the recorded `blessings_before` to be the held set at that moment
-- (so the receipt is written before the blessings it consumes are deleted),
-- and refuse a death while another death's respawn is still pending.
CREATE FUNCTION game_character_death_receipt_admission() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    NEW.created_xact_id := pg_current_xact_id();
    IF EXISTS (SELECT 1 FROM game_character_pending_respawns
                WHERE character_id = NEW.character_id
                  AND death_occurrence_id <> NEW.death_occurrence_id) THEN
        RAISE EXCEPTION 'a Character death cannot commit while a respawn is pending'
            USING ERRCODE = '23514';
    END IF;
    IF NEW.blessings_before IS DISTINCT FROM ARRAY(
            SELECT b.blessing_key COLLATE "C" FROM game_character_blessings b
             WHERE b.character_id = NEW.character_id ORDER BY 1)::TEXT[] THEN
        RAISE EXCEPTION 'death receipt blessings_before is not the held blessing set'
            USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END;
$$;

-- Death-receipt outcome (deferred, at commit): the death cell is in the
-- Character's World, the held blessing set is exactly `blessings_after`, and
-- the death's own pending respawn row exists.
CREATE FUNCTION game_character_death_receipt_consistency_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM game_character_roots r
                    WHERE r.character_id = NEW.character_id
                      AND r.world_id = NEW.death_world_id)
       OR NEW.blessings_after IS DISTINCT FROM ARRAY(
            SELECT b.blessing_key COLLATE "C" FROM game_character_blessings b
             WHERE b.character_id = NEW.character_id ORDER BY 1)::TEXT[]
       OR NOT EXISTS (SELECT 1 FROM game_character_pending_respawns p
                       WHERE p.character_id = NEW.character_id
                         AND p.death_occurrence_id = NEW.death_occurrence_id
                         AND p.respawn_position = NEW.respawn_position) THEN
        RAISE EXCEPTION 'Character death outcome is inconsistent with its receipt'
            USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END;
$$;

-- A blessing is deleted only by the death receipt of the same physical
-- transaction that records it as held before and consumed after.
CREATE FUNCTION game_character_blessing_consumption_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM game_character_death_receipts d
                    WHERE d.character_id = OLD.character_id
                      AND d.created_xact_id = pg_current_xact_id()
                      AND OLD.blessing_key = ANY (d.blessings_before)
                      AND NOT (OLD.blessing_key = ANY (d.blessings_after))) THEN
        RAISE EXCEPTION 'a blessing is consumed only by a death receipt of this transaction'
            USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END;
$$;

-- A pending respawn is created only by its own death, in the same physical
-- transaction; a consumed respawn can never be recreated.
CREATE FUNCTION game_character_pending_respawn_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM game_character_death_receipts d
                    WHERE d.death_occurrence_id = NEW.death_occurrence_id
                      AND d.character_id = NEW.character_id
                      AND d.respawn_position = NEW.respawn_position
                      AND d.created_xact_id = pg_current_xact_id()) THEN
        RAISE EXCEPTION 'a pending respawn is created only by its death transaction'
            USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END;
$$;

CREATE TRIGGER game_character_death_receipt_admission BEFORE INSERT
    ON game_character_death_receipts FOR EACH ROW
    EXECUTE FUNCTION game_character_death_receipt_admission();
CREATE TRIGGER game_character_death_receipt_immutable BEFORE UPDATE OR DELETE
    ON game_character_death_receipts FOR EACH ROW
    EXECUTE FUNCTION game_character_immutable();
CREATE CONSTRAINT TRIGGER game_character_death_receipt_progression_consistent
    AFTER INSERT ON game_character_death_receipts
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
    EXECUTE FUNCTION game_character_progression_consistency_guard();
CREATE CONSTRAINT TRIGGER game_character_death_receipt_outcome_consistent
    AFTER INSERT ON game_character_death_receipts
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
    EXECUTE FUNCTION game_character_death_receipt_consistency_guard();

-- Blessings are inserted (DEATH-4) and consumed, never rewritten.
CREATE TRIGGER game_character_blessing_immutable BEFORE UPDATE
    ON game_character_blessings FOR EACH ROW
    EXECUTE FUNCTION game_character_immutable();
CREATE CONSTRAINT TRIGGER game_character_blessing_consumed_by_death
    AFTER DELETE ON game_character_blessings
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
    EXECUTE FUNCTION game_character_blessing_consumption_guard();

-- Pending respawns are inserted by the death and deleted by the respawn that
-- consumes them (DEATH-1), never rewritten.
CREATE TRIGGER game_character_pending_respawn_immutable BEFORE UPDATE
    ON game_character_pending_respawns FOR EACH ROW
    EXECUTE FUNCTION game_character_immutable();
CREATE CONSTRAINT TRIGGER game_character_pending_respawn_created_by_death
    AFTER INSERT ON game_character_pending_respawns
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
    EXECUTE FUNCTION game_character_pending_respawn_guard();

CREATE TRIGGER game_character_death_receipts_no_truncate BEFORE TRUNCATE
    ON game_character_death_receipts EXECUTE FUNCTION game_character_reject_truncate();
CREATE TRIGGER game_character_blessings_no_truncate BEFORE TRUNCATE
    ON game_character_blessings EXECUTE FUNCTION game_character_reject_truncate();
CREATE TRIGGER game_character_pending_respawns_no_truncate BEFORE TRUNCATE
    ON game_character_pending_respawns EXECUTE FUNCTION game_character_reject_truncate();

-- CREATE OR REPLACE resets a function's configuration, so the two replaced
-- 0009 guards get their fixed search_path again with the new functions.
DO $$
DECLARE
    v_function TEXT;
BEGIN
    FOREACH v_function IN ARRAY ARRAY[
        'game_character_is_blessing_set(text[])',
        'game_character_progression_state_guard()',
        'game_character_progression_consistency_guard()',
        'game_character_death_receipt_admission()',
        'game_character_death_receipt_consistency_guard()',
        'game_character_blessing_consumption_guard()',
        'game_character_pending_respawn_guard()'
    ] LOOP
        EXECUTE format('ALTER FUNCTION %s SET search_path = %I, pg_temp', v_function, current_schema());
    END LOOP;
END $$;

REVOKE ALL ON game_character_death_receipts, game_character_blessings,
    game_character_pending_respawns FROM PUBLIC;
REVOKE ALL ON FUNCTION
    game_character_is_blessing_set(text[]),
    game_character_death_receipt_admission(),
    game_character_death_receipt_consistency_guard(),
    game_character_blessing_consumption_guard(),
    game_character_pending_respawn_guard()
FROM PUBLIC;
-- Runtime: the DEATH-1 writer inserts receipts and pending respawns, consumes
-- blessings, and the respawn deletes its pending row. Blessing INSERT is
-- granted with DEATH-4's receipt kind, not here.
GRANT SELECT, INSERT ON game_character_death_receipts TO oteryn_game_runtime;
GRANT SELECT, DELETE ON game_character_blessings TO oteryn_game_runtime;
GRANT SELECT, INSERT, DELETE ON game_character_pending_respawns TO oteryn_game_runtime;
GRANT SELECT ON game_character_death_receipts, game_character_blessings,
    game_character_pending_respawns TO oteryn_game_control;
