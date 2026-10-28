-- HAZARD-1: bounded Character state and immutable command receipts.
CREATE TABLE IF NOT EXISTS game_character_hazard (
    character_id UUID PRIMARY KEY REFERENCES game_character_roots(character_id) ON DELETE CASCADE,
    unlocked SMALLINT NOT NULL DEFAULT 1 CHECK (unlocked BETWEEN 1 AND 12),
    selected SMALLINT NOT NULL DEFAULT 1 CHECK (selected BETWEEN 1 AND unlocked)
);
CREATE TABLE IF NOT EXISTS game_character_hazard_receipts (
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id) ON DELETE CASCADE,
    command_session_id UUID NOT NULL,
    connection_generation NUMERIC(20,0) NOT NULL CHECK (connection_generation BETWEEN 1 AND 18446744073709551615),
    command_id NUMERIC(20,0) NOT NULL CHECK (command_id BETWEEN 1 AND 18446744073709551615),
    selected_requested SMALLINT NOT NULL CHECK (selected_requested BETWEEN 1 AND 12),
    result BYTEA NOT NULL CHECK (octet_length(result)=2),
    CHECK (get_byte(result,0) BETWEEN 1 AND 12 AND get_byte(result,1)=selected_requested
           AND get_byte(result,1)<=get_byte(result,0)),
    original_character_revision NUMERIC(20,0) NOT NULL CHECK (original_character_revision >= 1),
    committed_character_revision NUMERIC(20,0)
        CHECK (committed_character_revision = original_character_revision+1
               AND committed_character_revision <= 18446744073709551615),
    level_before BIGINT NOT NULL, level_after BIGINT NOT NULL CHECK (level_after=level_before),
    experience_before BIGINT NOT NULL, experience_after BIGINT NOT NULL CHECK (experience_after=experience_before),
    profile_revision TEXT NOT NULL, ruleset_revision TEXT NOT NULL, content_revision TEXT NOT NULL,
    simulation_revision TEXT NOT NULL, evidence_revision TEXT NOT NULL,
    declaration_revision TEXT NOT NULL, policy_revision TEXT NOT NULL, reward_revision TEXT NOT NULL,
    created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id(),
    PRIMARY KEY (character_id,command_session_id,connection_generation,command_id),
    UNIQUE (character_id,committed_character_revision)
);
CREATE OR REPLACE VIEW game_character_hazard_revision_chain WITH (security_invoker=true) AS
 SELECT * FROM game_character_atelier_revision_chain
 UNION ALL
 SELECT character_id,original_character_revision,committed_character_revision,
 level_before,level_after,experience_before,experience_after,profile_revision,ruleset_revision,
 content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision
 FROM game_character_hazard_receipts WHERE committed_character_revision IS NOT NULL;
DO $$ DECLARE body TEXT; BEGIN
 SELECT pg_get_functiondef('game_character_progression_consistency_guard()'::regprocedure) INTO STRICT body;
 IF position('game_character_hazard_revision_chain' IN body)=0 THEN
  IF position('game_character_atelier_revision_chain' IN body)=0 THEN
   RAISE EXCEPTION 'expected Atelier Character receipt chain';
  END IF;
  EXECUTE replace(body,'game_character_atelier_revision_chain','game_character_hazard_revision_chain');
 END IF;
END $$;
DO $$ BEGIN
 IF NOT EXISTS(SELECT 1 FROM pg_trigger WHERE tgrelid='game_character_hazard_receipts'::regclass AND tgname='game_hazard_receipt_stamp') THEN
  CREATE TRIGGER game_hazard_receipt_stamp BEFORE INSERT ON game_character_hazard_receipts
   FOR EACH ROW EXECUTE FUNCTION game_item_stamp_created_xact_id();
  CREATE TRIGGER game_hazard_receipt_immutable BEFORE UPDATE OR DELETE ON game_character_hazard_receipts
   FOR EACH ROW EXECUTE FUNCTION game_character_immutable();
  CREATE TRIGGER game_hazard_receipt_no_truncate BEFORE TRUNCATE ON game_character_hazard_receipts
   EXECUTE FUNCTION game_character_reject_truncate();
  CREATE CONSTRAINT TRIGGER game_hazard_character_chain AFTER INSERT ON game_character_hazard_receipts
   DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_character_progression_consistency_guard();
 END IF;
END $$;
REVOKE ALL ON game_character_hazard,game_character_hazard_receipts,game_character_hazard_revision_chain FROM PUBLIC,oteryn_game_control;
GRANT SELECT, INSERT, UPDATE ON game_character_hazard TO oteryn_game_runtime;
GRANT SELECT, INSERT ON game_character_hazard_receipts TO oteryn_game_runtime;
GRANT SELECT ON game_character_hazard_revision_chain TO oteryn_game_runtime;
CREATE OR REPLACE FUNCTION game_hazard_state_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF TG_OP='INSERT' AND NEW.unlocked=1 AND NEW.selected=1
    AND NOT EXISTS(SELECT 1 FROM game_character_hazard_receipts WHERE character_id=NEW.character_id
                   AND committed_character_revision IS NOT NULL AND created_xact_id<>pg_current_xact_id()) THEN
  RETURN NULL;
 END IF;
 IF TG_OP='UPDATE' AND NEW.unlocked=OLD.unlocked AND NEW.selected=OLD.selected THEN
  RETURN NULL;
 END IF;
 IF TG_OP='UPDATE' AND NEW.unlocked<OLD.unlocked THEN
  RAISE EXCEPTION 'Hazard unlocks cannot decrease' USING ERRCODE='23514';
 END IF;
 IF NOT EXISTS(SELECT 1 FROM game_character_hazard_receipts r
     JOIN game_character_roots c USING(character_id)
     WHERE r.character_id=NEW.character_id AND r.created_xact_id=pg_current_xact_id()
       AND r.committed_character_revision=c.character_revision
       AND get_byte(r.result,0)=NEW.unlocked AND get_byte(r.result,1)=NEW.selected) THEN
  RAISE EXCEPTION 'Hazard change requires its same-transaction Character receipt' USING ERRCODE='23514';
 END IF;
 RETURN NULL;
END $$;
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_hazard_state_guard() SET search_path = pg_catalog, %I, pg_temp',current_schema());
 IF NOT EXISTS(SELECT 1 FROM pg_trigger WHERE tgrelid='game_character_hazard'::regclass AND tgname='game_hazard_state_consistent') THEN
  CREATE CONSTRAINT TRIGGER game_hazard_state_consistent AFTER INSERT OR UPDATE ON game_character_hazard
   DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_hazard_state_guard();
  CREATE TRIGGER game_hazard_state_no_truncate BEFORE TRUNCATE ON game_character_hazard
   EXECUTE FUNCTION game_character_reject_truncate();
 END IF;
END $$;
REVOKE ALL ON FUNCTION game_hazard_state_guard() FROM PUBLIC;
