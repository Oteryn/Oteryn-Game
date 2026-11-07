-- W11: Quest-owned Soul War flags, first-taint clock, final completion and receipts.
-- Final failure reconstruction is qualified separately by PR2784 ANSWER6547488765.
CREATE TABLE IF NOT EXISTS game_character_soul_war (
 character_id UUID PRIMARY KEY REFERENCES game_character_roots(character_id) ON DELETE CASCADE,
 teleport_access BOOLEAN NOT NULL DEFAULT false,
 defeated_bosses SMALLINT NOT NULL DEFAULT 0 CHECK (defeated_bosses BETWEEN 0 AND 31),
 first_taint_at BIGINT CHECK(first_taint_at >= 0),
 megalomania_completed BOOLEAN NOT NULL DEFAULT false,
 final_reward_claimed BOOLEAN NOT NULL DEFAULT false,
 CHECK ((defeated_bosses=0)=(first_taint_at IS NULL)),
 CHECK (teleport_access OR defeated_bosses=0),
 CHECK (NOT final_reward_claimed OR megalomania_completed)
);
CREATE TABLE IF NOT EXISTS game_character_soul_war_receipts (
 character_id UUID NOT NULL REFERENCES game_character_roots(character_id) ON DELETE CASCADE,
 occurrence BYTEA NOT NULL CHECK(octet_length(occurrence)=32),
 request_binding BYTEA NOT NULL CHECK(octet_length(request_binding) BETWEEN 1 AND 1024),
 result BYTEA NOT NULL CHECK(octet_length(result) BETWEEN 1 AND 2048),
 source_death_key BYTEA CHECK(octet_length(source_death_key)=52),
 source_attempt BYTEA CHECK(octet_length(source_attempt)=32),
 CHECK(source_death_key IS NULL OR source_attempt IS NULL),
 boss_key TEXT CHECK(boss_key IN ('malice','hatred','spite','cruelty','greed','megalomania')),
 teleport_access BOOLEAN NOT NULL,
 defeated_bosses SMALLINT NOT NULL CHECK (defeated_bosses BETWEEN 0 AND 31),
 first_taint_at BIGINT CHECK(first_taint_at >= 0),
 megalomania_completed BOOLEAN NOT NULL,
 final_reward_claimed BOOLEAN NOT NULL,
 CHECK ((defeated_bosses=0)=(first_taint_at IS NULL)),
 CHECK (teleport_access OR defeated_bosses=0),
 CHECK (NOT final_reward_claimed OR megalomania_completed),
 CHECK ((source_death_key IS NULL)=(boss_key IS NULL)),
 original_character_revision NUMERIC(20,0) NOT NULL CHECK(original_character_revision BETWEEN 1 AND 18446744073709551615),
 committed_character_revision NUMERIC(20,0)
  CHECK(committed_character_revision=original_character_revision+1 AND committed_character_revision<=18446744073709551615),
 level_before BIGINT NOT NULL, level_after BIGINT NOT NULL CHECK(level_after=level_before),
 experience_before BIGINT NOT NULL, experience_after BIGINT NOT NULL CHECK(experience_after=experience_before),
 profile_revision TEXT NOT NULL, ruleset_revision TEXT NOT NULL, content_revision TEXT NOT NULL,
 simulation_revision TEXT NOT NULL, evidence_revision TEXT NOT NULL,
 declaration_revision TEXT NOT NULL, policy_revision TEXT NOT NULL, reward_revision TEXT NOT NULL,
 created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id(),
 PRIMARY KEY(character_id,occurrence), UNIQUE(character_id,committed_character_revision),
 FOREIGN KEY(source_death_key,character_id) REFERENCES game_party_kill_xp_members(death_key,character_id)
);
CREATE INDEX IF NOT EXISTS game_soul_war_death_member_idx
 ON game_character_soul_war_receipts(source_death_key,character_id) WHERE source_death_key IS NOT NULL;
-- Migration0711 attaches the shared terminal outcome foreign key.
CREATE INDEX IF NOT EXISTS game_soul_war_terminal_attempt_idx
 ON game_character_soul_war_receipts(source_attempt) WHERE source_attempt IS NOT NULL;
CREATE OR REPLACE VIEW game_character_hazard_revision_chain_soul_war WITH (security_invoker=true) AS
 SELECT * FROM game_character_hazard_revision_chain_primal
 UNION ALL
 SELECT character_id,original_character_revision,committed_character_revision,
 level_before,level_after,experience_before,experience_after,profile_revision,ruleset_revision,
 content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision
 FROM game_character_soul_war_receipts WHERE committed_character_revision IS NOT NULL;
DO $$ DECLARE body TEXT; BEGIN
 SELECT pg_get_functiondef('game_character_progression_consistency_guard()'::regprocedure) INTO STRICT body;
 IF position('game_character_hazard_revision_chain_primal' IN body)>0 THEN
  EXECUTE replace(body,'game_character_hazard_revision_chain_primal','game_character_hazard_revision_chain_soul_war');
 END IF; -- Reapplication must preserve later additive chain wrappers, including0710.
END $$;
CREATE OR REPLACE FUNCTION game_soul_war_death_source_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NEW.source_death_key IS NOT NULL AND NOT EXISTS(
  SELECT 1 FROM game_party_kill_xp_receipts d JOIN game_party_kill_xp_members m USING(death_key)
   WHERE d.death_key=NEW.source_death_key AND m.character_id=NEW.character_id
    AND d.created_xact_id=pg_current_xact_id()
    AND ((NEW.boss_key<>'megalomania' AND d.creature_key='oteryn:creature.goshnar_s_' || NEW.boss_key)
     OR (NEW.boss_key='megalomania' AND d.creature_key IN
       ('oteryn:creature.goshnar_s_megalomania_blue','oteryn:creature.goshnar_s_megalomania_green')))
 ) THEN
  RAISE EXCEPTION 'Soul War boss gain requires its matching native death and member receipt' USING ERRCODE='23514';
 END IF;
 RETURN NEW;
END $$;
CREATE OR REPLACE FUNCTION game_soul_war_state_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF TG_OP='UPDATE' AND NEW IS NOT DISTINCT FROM OLD THEN RETURN NULL; END IF;
 IF TG_OP='UPDATE' AND ((OLD.teleport_access AND NOT NEW.teleport_access)
  OR (OLD.megalomania_completed AND NOT NEW.megalomania_completed)
  OR (OLD.final_reward_claimed AND NOT NEW.final_reward_claimed)) THEN
  RAISE EXCEPTION 'Soul War permanent access/completion/reward facts cannot regress' USING ERRCODE='23514';
 END IF;
 IF NOT EXISTS(
  SELECT 1 FROM game_character_soul_war_receipts r JOIN game_character_roots c USING(character_id)
   WHERE r.character_id=NEW.character_id AND r.created_xact_id=pg_current_xact_id()
    AND r.committed_character_revision=c.character_revision
    AND r.teleport_access=NEW.teleport_access AND r.defeated_bosses=NEW.defeated_bosses
    AND r.first_taint_at IS NOT DISTINCT FROM NEW.first_taint_at
    AND r.megalomania_completed=NEW.megalomania_completed
    AND r.final_reward_claimed=NEW.final_reward_claimed
 ) THEN
  RAISE EXCEPTION 'Soul War state requires its matching same-transaction Character receipt' USING ERRCODE='23514';
 END IF;
 RETURN NULL;
END $$;
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_soul_war_death_source_guard() SET search_path=pg_catalog,%I,pg_temp',current_schema());
 EXECUTE format('ALTER FUNCTION game_soul_war_state_guard() SET search_path=pg_catalog,%I,pg_temp',current_schema());
 IF NOT EXISTS(SELECT 1 FROM pg_trigger WHERE tgrelid='game_character_soul_war_receipts'::regclass AND tgname='game_soul_war_receipt_stamp') THEN
  CREATE TRIGGER game_soul_war_receipt_stamp BEFORE INSERT ON game_character_soul_war_receipts
   FOR EACH ROW EXECUTE FUNCTION game_item_stamp_created_xact_id();
  CREATE TRIGGER game_soul_war_death_source BEFORE INSERT ON game_character_soul_war_receipts
   FOR EACH ROW EXECUTE FUNCTION game_soul_war_death_source_guard();
  CREATE TRIGGER game_soul_war_receipt_immutable BEFORE UPDATE OR DELETE ON game_character_soul_war_receipts
   FOR EACH ROW EXECUTE FUNCTION game_character_immutable();
  CREATE TRIGGER game_soul_war_receipt_no_truncate BEFORE TRUNCATE ON game_character_soul_war_receipts
   EXECUTE FUNCTION game_character_reject_truncate();
  CREATE CONSTRAINT TRIGGER game_soul_war_character_chain AFTER INSERT ON game_character_soul_war_receipts
   DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_character_progression_consistency_guard();
 END IF;
 IF NOT EXISTS(SELECT 1 FROM pg_trigger WHERE tgrelid='game_character_soul_war'::regclass AND tgname='game_soul_war_state_consistent') THEN
  CREATE CONSTRAINT TRIGGER game_soul_war_state_consistent AFTER INSERT OR UPDATE ON game_character_soul_war
   DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_soul_war_state_guard();
  CREATE TRIGGER game_soul_war_state_no_truncate BEFORE TRUNCATE ON game_character_soul_war
   EXECUTE FUNCTION game_character_reject_truncate();
 END IF;
END $$;
REVOKE ALL ON game_character_soul_war,game_character_soul_war_receipts,game_character_hazard_revision_chain_soul_war FROM PUBLIC,oteryn_game_control;
GRANT SELECT,INSERT,UPDATE ON game_character_soul_war TO oteryn_game_runtime;
GRANT SELECT,INSERT ON game_character_soul_war_receipts TO oteryn_game_runtime;
GRANT SELECT ON game_character_hazard_revision_chain_soul_war TO oteryn_game_runtime;
REVOKE ALL ON FUNCTION game_soul_war_death_source_guard(),game_soul_war_state_guard() FROM PUBLIC;

-- The first final victory retains a single choice; the existing MINT owner
-- acknowledges delivery in the item transaction. Full bags cannot reroll it.
CREATE TABLE IF NOT EXISTS game_character_soul_war_reward_entitlements (
 character_id UUID PRIMARY KEY REFERENCES game_character_roots(character_id) ON DELETE CASCADE,
 earned_occurrence BYTEA NOT NULL CHECK(octet_length(earned_occurrence)=32),
 item_source_id INTEGER NOT NULL CHECK(item_source_id BETWEEN 34082 AND 34099),
 claim_key TEXT NOT NULL CHECK(claim_key='oteryn:reward-claim.soul-war-final'),
 content_revision TEXT NOT NULL,
 state TEXT NOT NULL DEFAULT 'PENDING' CHECK(state IN ('PENDING','DELIVERED')),
 claim_session_id UUID,
 claim_command_id NUMERIC(20,0),
 CHECK((state='PENDING')=(claim_session_id IS NULL)),
 CHECK((claim_session_id IS NULL)=(claim_command_id IS NULL)),
 FOREIGN KEY(character_id,earned_occurrence) REFERENCES game_character_soul_war_receipts(character_id,occurrence),
 FOREIGN KEY(claim_session_id,claim_command_id) REFERENCES game_reward_claim_mint_receipts(game_session_id,command_id)
);
CREATE INDEX IF NOT EXISTS game_soul_war_reward_claim_idx
 ON game_character_soul_war_reward_entitlements(claim_session_id,claim_command_id) WHERE claim_session_id IS NOT NULL;
CREATE OR REPLACE FUNCTION game_soul_war_reward_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF TG_OP='INSERT' THEN
  IF NEW.state<>'PENDING' OR NOT EXISTS(
   SELECT 1 FROM game_character_soul_war_receipts r WHERE r.character_id=NEW.character_id
    AND r.occurrence=NEW.earned_occurrence AND r.boss_key='megalomania'
    AND r.megalomania_completed AND r.created_xact_id=pg_current_xact_id()
  ) THEN RAISE EXCEPTION 'Soul War reward requires its verified final completion' USING ERRCODE='23514'; END IF;
 ELSIF TG_OP='UPDATE' THEN
  IF (NEW.character_id,NEW.earned_occurrence,NEW.item_source_id,NEW.claim_key,NEW.content_revision)
   IS DISTINCT FROM (OLD.character_id,OLD.earned_occurrence,OLD.item_source_id,OLD.claim_key,OLD.content_revision)
   OR OLD.state<>'PENDING' OR NEW.state<>'DELIVERED' OR NOT EXISTS(
    SELECT 1 FROM game_reward_claim_mint_receipts r JOIN game_character_soul_war s USING(character_id)
     WHERE r.character_id=NEW.character_id AND r.game_session_id=NEW.claim_session_id
      AND r.command_id=NEW.claim_command_id AND r.claim_production_key=NEW.claim_key
      AND r.claim_family='RewardClaim' AND s.final_reward_claimed
   ) THEN RAISE EXCEPTION 'Soul War reward is immutable except for proven MINT delivery' USING ERRCODE='23514'; END IF;
 ELSE RAISE EXCEPTION 'Soul War reward entitlement cannot be deleted' USING ERRCODE='23514';
 END IF;
 RETURN NULL;
END $$;
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_soul_war_reward_guard() SET search_path=pg_catalog,%I,pg_temp',current_schema());
 IF NOT EXISTS(SELECT 1 FROM pg_trigger WHERE tgrelid='game_character_soul_war_reward_entitlements'::regclass AND tgname='game_soul_war_reward_consistent') THEN
  CREATE CONSTRAINT TRIGGER game_soul_war_reward_consistent AFTER INSERT OR UPDATE OR DELETE ON game_character_soul_war_reward_entitlements
   DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_soul_war_reward_guard();
  CREATE TRIGGER game_soul_war_reward_no_truncate BEFORE TRUNCATE ON game_character_soul_war_reward_entitlements
   EXECUTE FUNCTION game_character_reject_truncate();
 END IF;
END $$;
REVOKE ALL ON game_character_soul_war_reward_entitlements FROM PUBLIC,oteryn_game_control;
GRANT SELECT,INSERT,UPDATE ON game_character_soul_war_reward_entitlements TO oteryn_game_runtime;
REVOKE ALL ON FUNCTION game_soul_war_reward_guard() FROM PUBLIC;

GRANT EXECUTE ON FUNCTION game_soul_war_death_source_guard(),game_soul_war_state_guard(),game_soul_war_reward_guard() TO oteryn_game_runtime;
