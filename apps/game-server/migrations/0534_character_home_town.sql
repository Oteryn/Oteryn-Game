-- C20d-W23: durable citizenship, fenced by the current Character session.
CREATE TABLE game_character_home_towns (
    character_id uuid PRIMARY KEY REFERENCES game_character_roots(character_id) ON DELETE CASCADE,
    town_id integer NOT NULL CHECK (town_id > 0)
);

REVOKE ALL ON game_character_home_towns FROM PUBLIC;
GRANT SELECT, INSERT, UPDATE ON game_character_home_towns TO oteryn_game_runtime;
GRANT SELECT ON game_character_home_towns TO oteryn_game_control;

CREATE TABLE game_character_home_town_receipts (
    character_id uuid NOT NULL REFERENCES game_character_roots(character_id) ON DELETE CASCADE,
    game_session_id uuid NOT NULL,
    connection_generation numeric(20,0) NOT NULL CHECK (connection_generation BETWEEN 1 AND 18446744073709551615),
    character_lease_generation numeric(20,0) NOT NULL CHECK (character_lease_generation BETWEEN 1 AND 18446744073709551615),
    scope_ownership_generation numeric(20,0) NOT NULL CHECK (scope_ownership_generation BETWEEN 1 AND 18446744073709551615),
    town_before integer NOT NULL CHECK (town_before > 0),
    town_after integer NOT NULL CHECK (town_after > 0 AND town_after <> town_before),
    original_character_revision numeric(20,0) NOT NULL CHECK (original_character_revision >= 1),
    committed_character_revision numeric(20,0) NOT NULL CHECK (
        committed_character_revision = original_character_revision + 1
        AND committed_character_revision <= 18446744073709551615),
    level_before bigint NOT NULL,
    level_after bigint NOT NULL CHECK (level_after = level_before),
    experience_before bigint NOT NULL,
    experience_after bigint NOT NULL CHECK (experience_after = experience_before),
    profile_revision text NOT NULL, ruleset_revision text NOT NULL, content_revision text NOT NULL,
    simulation_revision text NOT NULL, evidence_revision text NOT NULL,
    declaration_revision text NOT NULL, policy_revision text NOT NULL, reward_revision text NOT NULL,
    created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id(),
    PRIMARY KEY (character_id, committed_character_revision)
);
REVOKE ALL ON game_character_home_town_receipts FROM PUBLIC;
GRANT SELECT, INSERT ON game_character_home_town_receipts TO oteryn_game_runtime;
GRANT SELECT ON game_character_home_town_receipts TO oteryn_game_control;

-- Extend the hazard chain in place, as 0337 does: the guard keeps naming it, so 0332 and
-- 0337 stay re-runnable. The body repeats 0337's chain and adds home-town receipts.
CREATE OR REPLACE VIEW game_character_hazard_revision_chain WITH (security_invoker=true) AS
 SELECT * FROM game_character_atelier_revision_chain
 UNION ALL
 SELECT character_id,original_character_revision,committed_character_revision,
 level_before,level_after,experience_before,experience_after,profile_revision,ruleset_revision,
 content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision
 FROM game_character_hazard_receipts WHERE committed_character_revision IS NOT NULL
 UNION ALL
 SELECT character_id,original_character_revision,committed_character_revision,
 level_before,level_after,experience_before,experience_after,profile_revision,ruleset_revision,
 content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision
 FROM game_character_prey_receipts
 UNION ALL
 SELECT character_id,original_character_revision,committed_character_revision,
 level_before,level_after,experience_before,experience_after,profile_revision,ruleset_revision,
 content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision
 FROM game_character_home_town_receipts;

DO $$ DECLARE body text; BEGIN
    SELECT pg_get_functiondef('game_character_progression_consistency_guard()'::regprocedure)
      INTO STRICT body;
    IF position('game_character_hazard_revision_chain' IN body) = 0 THEN
        RAISE EXCEPTION 'expected current Character receipt chain';
    END IF;
END $$;

CREATE FUNCTION game_character_home_town_state_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'INSERT' AND NOT EXISTS (
        SELECT 1 FROM game_character_home_town_receipts WHERE character_id = NEW.character_id
    ) THEN
        RETURN NULL; -- Materialization of the pinned logical default.
    END IF;
    IF TG_OP = 'UPDATE' AND NEW.town_id = OLD.town_id THEN
        RETURN NULL;
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM game_character_home_town_receipts t
        JOIN game_character_roots r USING (character_id)
        WHERE t.character_id = NEW.character_id AND t.town_after = NEW.town_id
          AND t.created_xact_id = pg_current_xact_id()
          AND t.committed_character_revision = r.character_revision
          AND (TG_OP = 'INSERT' OR t.town_before = OLD.town_id)
    ) THEN
        RAISE EXCEPTION 'citizenship requires its same-transaction Character receipt' USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END $$;
CREATE FUNCTION game_character_home_town_receipt_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM game_character_roots r JOIN game_character_home_towns t USING (character_id)
        WHERE r.character_id = NEW.character_id
          AND r.character_revision = NEW.committed_character_revision AND t.town_id = NEW.town_after
    ) THEN
        RAISE EXCEPTION 'citizenship receipt lost its state or root' USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END $$;
DO $$ BEGIN
    EXECUTE format('ALTER FUNCTION game_character_home_town_state_guard() SET search_path = pg_catalog, %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_character_home_town_receipt_guard() SET search_path = pg_catalog, %I, pg_temp', current_schema());
END $$;
REVOKE ALL ON FUNCTION game_character_home_town_state_guard(), game_character_home_town_receipt_guard() FROM PUBLIC;

CREATE TRIGGER game_home_town_receipt_stamp BEFORE INSERT ON game_character_home_town_receipts
    FOR EACH ROW EXECUTE FUNCTION game_item_stamp_created_xact_id();
CREATE TRIGGER game_home_town_receipt_immutable BEFORE UPDATE OR DELETE ON game_character_home_town_receipts
    FOR EACH ROW EXECUTE FUNCTION game_character_immutable();
CREATE TRIGGER game_home_town_receipt_no_truncate BEFORE TRUNCATE ON game_character_home_town_receipts
    EXECUTE FUNCTION game_character_reject_truncate();
CREATE CONSTRAINT TRIGGER game_home_town_character_chain AFTER INSERT ON game_character_home_town_receipts
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_character_progression_consistency_guard();
CREATE CONSTRAINT TRIGGER game_home_town_receipt_consistent AFTER INSERT ON game_character_home_town_receipts
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_character_home_town_receipt_guard();
CREATE CONSTRAINT TRIGGER game_home_town_state_consistent AFTER INSERT OR UPDATE ON game_character_home_towns
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_character_home_town_state_guard();
CREATE TRIGGER game_home_town_no_truncate BEFORE TRUNCATE ON game_character_home_towns
    EXECUTE FUNCTION game_character_reject_truncate();
