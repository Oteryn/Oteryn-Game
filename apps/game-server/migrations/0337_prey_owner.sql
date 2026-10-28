-- PREY-LIVE-1. Transactional and deliberately re-runnable.
CREATE TABLE IF NOT EXISTS game_character_prey_receipts (
    character_id UUID NOT NULL,
    occurrence UUID NOT NULL,
    request_sha256 BYTEA NOT NULL CHECK (octet_length(request_sha256) = 32),
    result BYTEA NOT NULL CHECK (octet_length(result) BETWEEN 1 AND 8192),
    wildcards_spent SMALLINT NOT NULL CHECK(wildcards_spent BETWEEN 0 AND 15),
    fee_gold_units BIGINT NOT NULL CHECK(fee_gold_units >= 0),
    CHECK(fee_gold_units=0 OR fee_gold_units=level_before*200),
    original_character_revision NUMERIC(20,0) NOT NULL CHECK (original_character_revision >= 1),
    committed_character_revision NUMERIC(20,0) NOT NULL
        CHECK (committed_character_revision = original_character_revision + 1
               AND committed_character_revision <= 18446744073709551615),
    level_before BIGINT NOT NULL,
    level_after BIGINT NOT NULL CHECK (level_after = level_before),
    experience_before BIGINT NOT NULL,
    experience_after BIGINT NOT NULL CHECK (experience_after = experience_before),
    profile_revision TEXT NOT NULL, ruleset_revision TEXT NOT NULL, content_revision TEXT NOT NULL,
    simulation_revision TEXT NOT NULL, evidence_revision TEXT NOT NULL,
    declaration_revision TEXT NOT NULL, policy_revision TEXT NOT NULL, reward_revision TEXT NOT NULL,
    created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id(),
    UNIQUE (character_id, original_character_revision),
    UNIQUE (character_id, committed_character_revision),
    PRIMARY KEY (character_id, occurrence),
    CONSTRAINT game_character_prey_receipts_character_fk
        FOREIGN KEY (character_id) REFERENCES game_character_roots(character_id) ON DELETE CASCADE
);

GRANT SELECT, INSERT, UPDATE ON game_character_prey_slots TO oteryn_game_runtime;
GRANT SELECT, INSERT ON game_character_prey_receipts TO oteryn_game_runtime;

CREATE OR REPLACE VIEW game_character_prey_revision_chain WITH (security_invoker=true) AS
 SELECT * FROM game_character_hazard_revision_chain
 UNION ALL
 SELECT character_id,original_character_revision,committed_character_revision,
 level_before,level_after,experience_before,experience_after,profile_revision,ruleset_revision,
 content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision
 FROM game_character_prey_receipts;
DO $$ DECLARE body TEXT; BEGIN
 SELECT pg_get_functiondef('game_character_progression_consistency_guard()'::regprocedure) INTO STRICT body;
 IF position('game_character_prey_revision_chain' IN body)=0 THEN
  IF position('game_character_hazard_revision_chain' IN body)=0 THEN RAISE EXCEPTION 'missing Character receipt chain'; END IF;
  EXECUTE replace(body,'game_character_hazard_revision_chain','game_character_prey_revision_chain');
 END IF;
END $$;
GRANT SELECT ON game_character_prey_revision_chain TO oteryn_game_runtime;
REVOKE ALL ON game_character_prey_receipts FROM PUBLIC;
DROP TRIGGER IF EXISTS game_prey_receipt_stamp ON game_character_prey_receipts;
CREATE TRIGGER game_prey_receipt_stamp BEFORE INSERT ON game_character_prey_receipts
 FOR EACH ROW EXECUTE FUNCTION game_item_stamp_created_xact_id();
DROP TRIGGER IF EXISTS game_prey_receipt_immutable ON game_character_prey_receipts;
CREATE TRIGGER game_prey_receipt_immutable BEFORE UPDATE OR DELETE ON game_character_prey_receipts
 FOR EACH ROW EXECUTE FUNCTION game_character_immutable();
DROP TRIGGER IF EXISTS game_prey_receipt_no_truncate ON game_character_prey_receipts;
CREATE TRIGGER game_prey_receipt_no_truncate BEFORE TRUNCATE ON game_character_prey_receipts
 EXECUTE FUNCTION game_character_reject_truncate();
DROP TRIGGER IF EXISTS game_prey_character_chain ON game_character_prey_receipts;
CREATE CONSTRAINT TRIGGER game_prey_character_chain AFTER INSERT ON game_character_prey_receipts
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_character_progression_consistency_guard();
CREATE TABLE IF NOT EXISTS game_prey_decision_root (
 singleton BOOLEAN PRIMARY KEY CHECK(singleton), secret BYTEA NOT NULL CHECK(octet_length(secret)=32)
);
INSERT INTO game_prey_decision_root VALUES(true,sha256(uuid_send(gen_random_uuid())||uuid_send(gen_random_uuid()))) ON CONFLICT DO NOTHING;
REVOKE ALL ON game_prey_decision_root FROM PUBLIC;
GRANT SELECT ON game_prey_decision_root TO oteryn_game_runtime;

-- Extend the sole DAILYFX Account+World wildcard ledger with receipt-bound spends.
ALTER TABLE game_account_prey_wildcard_entries DROP CONSTRAINT IF EXISTS game_account_prey_wildcard_entries_claim_id_fkey;
DO $$ DECLARE c RECORD; BEGIN
 FOR c IN SELECT conname FROM pg_constraint WHERE conrelid='game_account_prey_wildcard_entries'::regclass AND contype='c' LOOP
  EXECUTE format('ALTER TABLE game_account_prey_wildcard_entries DROP CONSTRAINT %I',c.conname);
 END LOOP;
END $$;
ALTER TABLE game_account_prey_wildcard_entries ADD CONSTRAINT prey_wildcard_entry_values CHECK(
 balance_before BETWEEN 0 AND 52 AND balance_after BETWEEN 0 AND 52 AND (previous_claim_id IS NOT NULL OR balance_before=0)
 AND ((kind='DAILY_REWARD' AND requested IN(1,3) AND credited=least(requested,52-balance_before) AND balance_after=balance_before+credited)
 OR (kind='SPEND' AND requested BETWEEN 1 AND 15 AND credited=0 AND balance_after=balance_before-requested)));
DO $$ DECLARE body TEXT; BEGIN
 SELECT pg_get_functiondef('game_daily_wildcard_guard()'::regprocedure) INTO STRICT body;
 IF position('NEW.kind=''DAILY_REWARD''' IN body)=0 THEN
  body:=replace(body,'IF NOT EXISTS(SELECT 1 FROM public.game_character_daily_reward_claims c','IF NEW.kind=''DAILY_REWARD'' AND NOT EXISTS(SELECT 1 FROM public.game_character_daily_reward_claims c');
  body:=replace(body,'    SELECT l.* INTO STRICT e',
  '    IF TG_TABLE_NAME=''game_account_prey_wildcard_entries'' THEN IF NEW.kind=''SPEND'' AND NOT EXISTS(SELECT 1 FROM public.game_character_prey_receipts p JOIN public.game_character_roots r USING(character_id) WHERE p.occurrence=NEW.claim_id AND r.account_id=NEW.account_id AND r.world_id=NEW.world_id AND p.wildcards_spent=NEW.requested AND p.created_xact_id=pg_current_xact_id()) THEN RAISE EXCEPTION ''prey spend requires same-transaction receipt'' USING ERRCODE=''23514''; END IF; END IF;
    SELECT l.* INTO STRICT e');
  EXECUTE body;
 END IF;
END $$;

-- Closed gold fee cause 12; retain every previous source check.
DO $$ DECLARE body TEXT; BEGIN
 IF NOT EXISTS(SELECT 1 FROM pg_constraint WHERE conrelid='game_item_fee_burns'::regclass AND conname='game_prey_fee_cause_installed') THEN
  SELECT pg_get_constraintdef(oid) INTO STRICT body FROM pg_constraint WHERE conrelid='game_item_fee_burns'::regclass AND conname='game_item_fee_burns_cause_kind_check';
  ALTER TABLE game_item_fee_burns DROP CONSTRAINT game_item_fee_burns_cause_kind_check;
  EXECUTE 'ALTER TABLE game_item_fee_burns ADD CONSTRAINT game_item_fee_burns_cause_kind_check CHECK (('||substr(body,7)||') OR cause_kind=12)';
  SELECT regexp_replace(pg_get_constraintdef(oid),' NOT VALID$','') INTO STRICT body FROM pg_constraint WHERE conrelid='game_item_fee_burns'::regclass AND conname='game_item_fee_burns_purchase_cause';
  ALTER TABLE game_item_fee_burns DROP CONSTRAINT game_item_fee_burns_purchase_cause;
  EXECUTE 'ALTER TABLE game_item_fee_burns ADD CONSTRAINT game_item_fee_burns_purchase_cause CHECK (('||substr(body,7)||') OR (cause_kind=12 AND charm_key IS NULL AND purchase_service_key IN (''oteryn:prey.list_reroll.1'',''oteryn:prey.list_reroll.2'',''oteryn:prey.list_reroll.3'') AND fee_gold_units>0 AND fee_gold_units%200=0)) NOT VALID';
  ALTER TABLE game_item_fee_burns ADD CONSTRAINT game_prey_fee_cause_installed CHECK(cause_kind<>12 OR fee_gold_units>0);
 END IF;
 SELECT pg_get_functiondef('game_item_fee_burn_consistency_guard()'::regprocedure) INTO STRICT body;
 IF position('NEW.cause_kind=12' IN body)=0 THEN
  IF position('(NEW.cause_kind=5 AND EXISTS' IN body)=0 THEN RAISE EXCEPTION 'missing fee Character proof'; END IF;
  EXECUTE replace(body,'(NEW.cause_kind=5 AND EXISTS','(NEW.cause_kind=12 AND EXISTS(SELECT 1 FROM game_character_prey_receipts p WHERE p.character_id=NEW.character_id AND p.occurrence=NEW.cause_occurrence_id AND p.committed_character_revision=NEW.committed_character_revision AND p.fee_gold_units=NEW.fee_gold_units AND p.created_xact_id=pg_current_xact_id())) OR (NEW.cause_kind=5 AND EXISTS');
 END IF;
END $$;

CREATE OR REPLACE FUNCTION game_prey_slot_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 -- Administrative seeds remain available to migrations and isolated test fixtures.
 IF current_user='oteryn_game_runtime' AND NOT EXISTS(SELECT 1 FROM game_character_prey_receipts r
  WHERE r.character_id=NEW.character_id AND r.created_xact_id=pg_current_xact_id()) THEN
  RAISE EXCEPTION 'Prey slot requires fenced same-transaction receipt' USING ERRCODE='23514';
 END IF;
 RETURN NULL;
END $$;
DROP TRIGGER IF EXISTS game_prey_slot_consistent ON game_character_prey_slots;
CREATE CONSTRAINT TRIGGER game_prey_slot_consistent AFTER INSERT OR UPDATE ON game_character_prey_slots
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_prey_slot_guard();
CREATE OR REPLACE FUNCTION game_prey_receipt_resources_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NEW.wildcards_spent>0 AND NOT EXISTS(SELECT 1 FROM game_account_prey_wildcard_entries e
  WHERE e.claim_id=NEW.occurrence AND e.kind='SPEND' AND e.requested=NEW.wildcards_spent) THEN
  RAISE EXCEPTION 'Prey receipt requires wildcard debit' USING ERRCODE='23514';
 END IF;
 IF NEW.fee_gold_units>0 AND NOT EXISTS(SELECT 1 FROM game_item_fee_burns f WHERE f.cause_kind=12
  AND f.character_id=NEW.character_id AND f.cause_occurrence_id=NEW.occurrence AND f.fee_gold_units=NEW.fee_gold_units
  AND f.created_xact_id=pg_current_xact_id()) THEN
  RAISE EXCEPTION 'Prey receipt requires gold fee burn' USING ERRCODE='23514';
 END IF;
 RETURN NULL;
END $$;
DROP TRIGGER IF EXISTS game_prey_receipt_resources_consistent ON game_character_prey_receipts;
CREATE CONSTRAINT TRIGGER game_prey_receipt_resources_consistent AFTER INSERT ON game_character_prey_receipts
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_prey_receipt_resources_guard();
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_prey_slot_guard() SET search_path=pg_catalog,%I,pg_temp',current_schema());
 EXECUTE format('ALTER FUNCTION game_prey_receipt_resources_guard() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_prey_slot_guard(),game_prey_receipt_resources_guard() FROM PUBLIC;
