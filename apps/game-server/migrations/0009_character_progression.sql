-- R7 P03 / DUR-02: typed Character-owned experience state and immutable
-- reward-occurrence receipts.  This migration deliberately does not backfill
-- progression: a bootstrap-only Character remains valid, while the XP writer
-- fails closed until an owning initializer creates the typed state.

CREATE TABLE game_character_progression_state (
    character_id UUID PRIMARY KEY REFERENCES game_character_roots(character_id),
    character_revision NUMERIC(20,0) NOT NULL
        CHECK (character_revision BETWEEN 1 AND 18446744073709551615),
    level BIGINT NOT NULL CHECK (level BETWEEN 1 AND 4294967295),
    total_experience BIGINT NOT NULL CHECK (total_experience >= 0),
    profile_revision TEXT NOT NULL CHECK (profile_revision ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'),
    ruleset_revision TEXT NOT NULL CHECK (ruleset_revision ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'),
    content_revision TEXT NOT NULL CHECK (content_revision ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'),
    simulation_revision TEXT NOT NULL CHECK (simulation_revision ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'),
    evidence_revision TEXT NOT NULL CHECK (evidence_revision ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'),
    declaration_revision TEXT NOT NULL CHECK (declaration_revision ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'),
    policy_revision TEXT NOT NULL CHECK (policy_revision ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'),
    reward_revision TEXT NOT NULL CHECK (reward_revision ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$')
);

CREATE TABLE game_character_xp_receipts (
    reward_occurrence_id UUID PRIMARY KEY CHECK (game_character_is_uuid_v7(reward_occurrence_id)),
    command_binding BYTEA NOT NULL CHECK (octet_length(command_binding) BETWEEN 1 AND 1024),
    policy_digest BYTEA NOT NULL CHECK (octet_length(policy_digest) = 32),
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    original_character_revision NUMERIC(20,0) NOT NULL
        CHECK (original_character_revision BETWEEN 1 AND 18446744073709551614),
    committed_character_revision NUMERIC(20,0) NOT NULL
        CHECK (committed_character_revision = original_character_revision + 1),
    level_before BIGINT NOT NULL CHECK (level_before BETWEEN 1 AND 4294967295),
    level_after BIGINT NOT NULL CHECK (level_after BETWEEN level_before AND 4294967295),
    experience_before BIGINT NOT NULL CHECK (experience_before >= 0),
    experience_after BIGINT NOT NULL CHECK (experience_after > experience_before),
    experience_awarded BIGINT NOT NULL CHECK (experience_awarded > 0),
    profile_revision TEXT NOT NULL CHECK (profile_revision ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'),
    ruleset_revision TEXT NOT NULL CHECK (ruleset_revision ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'),
    content_revision TEXT NOT NULL CHECK (content_revision ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'),
    simulation_revision TEXT NOT NULL CHECK (simulation_revision ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'),
    evidence_revision TEXT NOT NULL CHECK (evidence_revision ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'),
    declaration_revision TEXT NOT NULL CHECK (declaration_revision ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'),
    policy_revision TEXT NOT NULL CHECK (policy_revision ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'),
    reward_revision TEXT NOT NULL CHECK (reward_revision ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'),
    committed_at BIGINT NOT NULL CHECK (committed_at >= 0),
    CHECK (experience_after = experience_before + experience_awarded),
    UNIQUE (character_id, original_character_revision),
    UNIQUE (character_id, committed_character_revision)
);

-- 0005 froze the first-slice root because its only legal revision was one.
-- P03 keeps every identity/interpretation field immutable and permits exactly
-- one global CharacterRevision successor per semantic Character transaction.
DROP TRIGGER game_character_root_immutable ON game_character_roots;
CREATE FUNCTION game_character_root_revision_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'UPDATE'
       AND NEW.character_id = OLD.character_id
       AND NEW.account_id = OLD.account_id
       AND NEW.world_id = OLD.world_id
       AND NEW.lifecycle = OLD.lifecycle
       AND NEW.profile_revision = OLD.profile_revision
       AND NEW.ruleset_revision = OLD.ruleset_revision
       AND NEW.content_revision = OLD.content_revision
       AND NEW.starter_template_revision = OLD.starter_template_revision
       AND NEW.character_revision = OLD.character_revision + 1 THEN
        RETURN NEW;
    END IF;
    RAISE EXCEPTION 'Character root permits only one exact revision successor'
        USING ERRCODE = '23514';
END;
$$;
CREATE TRIGGER game_character_root_revision_guard BEFORE UPDATE OR DELETE
    ON game_character_roots FOR EACH ROW EXECUTE FUNCTION game_character_root_revision_guard();

-- An initialized progression row may only advance with one CharacterRevision;
-- context and policy identity changes require a separately owned transition.
CREATE FUNCTION game_character_progression_state_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'UPDATE'
       AND NEW.character_id = OLD.character_id
       AND NEW.character_revision = OLD.character_revision + 1
       AND NEW.total_experience > OLD.total_experience
       AND NEW.level >= OLD.level
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
    RAISE EXCEPTION 'Character progression permits only one typed XP successor'
        USING ERRCODE = '23514';
END;
$$;
CREATE TRIGGER game_character_progression_state_guard BEFORE UPDATE OR DELETE
    ON game_character_progression_state FOR EACH ROW
    EXECUTE FUNCTION game_character_progression_state_guard();

CREATE TRIGGER game_character_xp_receipt_immutable BEFORE UPDATE OR DELETE
    ON game_character_xp_receipts FOR EACH ROW
    EXECUTE FUNCTION game_character_immutable();

-- Commit-time cross-relation authority: no direct SQL path may advance the
-- global CharacterRevision without the exact typed state successor and one
-- immutable receipt per revision.  Initial revision one may have no progression
-- row because population belongs to a separate gate.
CREATE FUNCTION game_character_progression_consistency_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    v_character UUID := NEW.character_id;
    v_root_revision NUMERIC(20,0);
    v_state game_character_progression_state%ROWTYPE;
    v_receipts NUMERIC(20,0);
BEGIN
    SELECT character_revision INTO v_root_revision
      FROM game_character_roots WHERE character_id = v_character;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'Character progression lost its root' USING ERRCODE = '23514';
    END IF;
    SELECT * INTO v_state FROM game_character_progression_state
     WHERE character_id = v_character;
    SELECT count(*)::numeric INTO v_receipts FROM game_character_xp_receipts
     WHERE character_id = v_character;

    IF v_root_revision = 1 THEN
        IF v_receipts <> 0 OR (v_state.character_id IS NOT NULL AND v_state.character_revision <> 1) THEN
            RAISE EXCEPTION 'initial Character progression is inconsistent' USING ERRCODE = '23514';
        END IF;
        RETURN NULL;
    END IF;

    IF v_state.character_id IS NULL
       OR v_state.character_revision <> v_root_revision
       OR v_receipts <> v_root_revision - 1
       OR NOT EXISTS (
            SELECT 1 FROM game_character_xp_receipts x
             WHERE x.character_id = v_character
               AND x.committed_character_revision = v_root_revision
               AND x.level_after = v_state.level
               AND x.experience_after = v_state.total_experience
               AND x.profile_revision = v_state.profile_revision
               AND x.ruleset_revision = v_state.ruleset_revision
               AND x.content_revision = v_state.content_revision
               AND x.simulation_revision = v_state.simulation_revision
               AND x.evidence_revision = v_state.evidence_revision
               AND x.declaration_revision = v_state.declaration_revision
               AND x.policy_revision = v_state.policy_revision
               AND x.reward_revision = v_state.reward_revision)
       OR EXISTS (
            SELECT 1 FROM game_character_xp_receipts x
             LEFT JOIN game_character_xp_receipts p
               ON p.character_id = x.character_id
              AND p.committed_character_revision = x.original_character_revision
            WHERE x.character_id = v_character
              AND (x.committed_character_revision > v_root_revision
                OR (x.original_character_revision <> 1 AND p.reward_occurrence_id IS NULL)
                OR (p.reward_occurrence_id IS NOT NULL AND
                    (p.experience_after <> x.experience_before
                     OR p.level_after <> x.level_before)))) THEN
        RAISE EXCEPTION 'Character progression revision/receipt chain is inconsistent'
            USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END;
$$;

CREATE CONSTRAINT TRIGGER game_character_root_progression_consistent
    AFTER INSERT OR UPDATE ON game_character_roots
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
    EXECUTE FUNCTION game_character_progression_consistency_guard();
CREATE CONSTRAINT TRIGGER game_character_state_progression_consistent
    AFTER INSERT OR UPDATE ON game_character_progression_state
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
    EXECUTE FUNCTION game_character_progression_consistency_guard();
CREATE CONSTRAINT TRIGGER game_character_receipt_progression_consistent
    AFTER INSERT ON game_character_xp_receipts
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
    EXECUTE FUNCTION game_character_progression_consistency_guard();

DO $$ BEGIN
    EXECUTE format('ALTER FUNCTION game_character_root_revision_guard() SET search_path = %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_character_progression_state_guard() SET search_path = %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_character_progression_consistency_guard() SET search_path = %I, pg_temp', current_schema());
END $$;

CREATE TRIGGER game_character_progression_state_no_truncate BEFORE TRUNCATE
    ON game_character_progression_state EXECUTE FUNCTION game_character_reject_truncate();
CREATE TRIGGER game_character_xp_receipts_no_truncate BEFORE TRUNCATE
    ON game_character_xp_receipts EXECUTE FUNCTION game_character_reject_truncate();

REVOKE ALL ON game_character_progression_state, game_character_xp_receipts FROM PUBLIC;
REVOKE ALL ON FUNCTION
    game_character_root_revision_guard(),
    game_character_progression_state_guard(),
    game_character_progression_consistency_guard()
FROM PUBLIC;
GRANT SELECT, INSERT, UPDATE ON game_character_progression_state TO oteryn_game_runtime;
GRANT SELECT, INSERT ON game_character_xp_receipts TO oteryn_game_runtime;
GRANT SELECT ON game_character_progression_state, game_character_xp_receipts TO oteryn_game_control;
