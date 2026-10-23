-- LOOP-MAP-1 owner option A: one basic authored area, not Reference-qualified.
CREATE TABLE game_character_cyclopedia (
 character_id UUID PRIMARY KEY REFERENCES game_character_roots,
 bundle_digest BYTEA NOT NULL CHECK(octet_length(bundle_digest)=32),
 catalogue_digest BYTEA NOT NULL CHECK(octet_length(catalogue_digest)=32),
 progress JSONB NOT NULL CHECK(octet_length(progress::text)<=4096),
 occurrence UUID NOT NULL
);
CREATE TABLE game_world_cyclopedia (
 world_id UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(world_id)),
 bundle_digest BYTEA NOT NULL CHECK(octet_length(bundle_digest)=32),
 catalogue_digest BYTEA NOT NULL CHECK(octet_length(catalogue_digest)=32),
 pool BIGINT NOT NULL DEFAULT 0 CHECK(pool>=0)
);
CREATE TABLE game_world_cyclopedia_epochs (
 world_id UUID NOT NULL REFERENCES game_world_cyclopedia,
 epoch NUMERIC(20,0) NOT NULL CHECK(epoch BETWEEN 0 AND 18446744073709551615),
 improved BOOLEAN NOT NULL,
 donation_winner BOOLEAN NOT NULL,
 pool_before BIGINT NOT NULL CHECK(pool_before>=0),
 created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id(),
 PRIMARY KEY(world_id,epoch),
 CHECK(NOT donation_winner OR (improved AND pool_before>=10000000))
);
CREATE TABLE game_character_cyclopedia_receipts (
 occurrence UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(occurrence)),
 character_id UUID NOT NULL REFERENCES game_character_roots,
 world_id UUID NOT NULL CHECK(game_character_is_uuid_v7(world_id)),
 command_session_id UUID NOT NULL CHECK(game_character_is_uuid_v7(command_session_id)),
 command_id NUMERIC(20,0) NOT NULL CHECK(command_id BETWEEN 1 AND 18446744073709551615),
 binding BYTEA NOT NULL CHECK(octet_length(binding)=32),
 result BYTEA NOT NULL CHECK(octet_length(result) BETWEEN 29 AND 69),
 fee_transaction_id UUID UNIQUE REFERENCES game_item_fee_burns DEFERRABLE INITIALLY DEFERRED,
 donation BIGINT NOT NULL CHECK(donation BETWEEN 0 AND 10000000),
 pool_before BIGINT NOT NULL CHECK(pool_before>=0),
 pool_after BIGINT NOT NULL CHECK(pool_after=pool_before+donation),
 original_character_revision NUMERIC(20,0) NOT NULL CHECK(original_character_revision>=1),
 committed_character_revision NUMERIC(20,0) CHECK(committed_character_revision=original_character_revision+1 AND committed_character_revision<=18446744073709551615),
 level_before BIGINT NOT NULL,
 level_after BIGINT NOT NULL CHECK(level_after=level_before),
 experience_before BIGINT NOT NULL,
 experience_after BIGINT NOT NULL CHECK(experience_after=experience_before),
 profile_revision TEXT NOT NULL, ruleset_revision TEXT NOT NULL, content_revision TEXT NOT NULL,
 simulation_revision TEXT NOT NULL, evidence_revision TEXT NOT NULL, declaration_revision TEXT NOT NULL,
 policy_revision TEXT NOT NULL, reward_revision TEXT NOT NULL,
 created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id(),
 UNIQUE(character_id,command_session_id,command_id),
 UNIQUE(character_id,committed_character_revision),
 CHECK((donation>0)=(fee_transaction_id IS NOT NULL)),
 CHECK(donation=0 OR committed_character_revision IS NOT NULL)
);
ALTER TABLE game_character_cyclopedia ADD FOREIGN KEY(occurrence)
 REFERENCES game_character_cyclopedia_receipts DEFERRABLE INITIALLY DEFERRED;

CREATE VIEW game_character_cyclopedia_revision_chain WITH(security_invoker=true) AS
 SELECT * FROM game_character_premium_revision_chain
 UNION ALL
 SELECT character_id,original_character_revision,committed_character_revision,
 level_before,level_after,experience_before,experience_after,profile_revision,ruleset_revision,
 content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision
 FROM game_character_cyclopedia_receipts WHERE committed_character_revision IS NOT NULL;
DO $$ DECLARE body TEXT; BEGIN
 SELECT pg_get_functiondef('game_character_progression_consistency_guard()'::regprocedure) INTO body;
 IF position('game_character_premium_revision_chain' IN body)=0 THEN RAISE EXCEPTION 'expected premium receipt chain'; END IF;
 EXECUTE replace(body,'game_character_premium_revision_chain','game_character_cyclopedia_revision_chain');
END $$;
CREATE CONSTRAINT TRIGGER game_cyclopedia_revision_consistent AFTER INSERT ON game_character_cyclopedia_receipts
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_character_progression_consistency_guard();
CREATE TRIGGER game_cyclopedia_receipt_stamp BEFORE INSERT ON game_character_cyclopedia_receipts
 FOR EACH ROW EXECUTE FUNCTION game_item_stamp_created_xact_id();
CREATE TRIGGER game_cyclopedia_receipt_immutable BEFORE UPDATE OR DELETE ON game_character_cyclopedia_receipts
 FOR EACH ROW EXECUTE FUNCTION game_character_immutable();
CREATE TRIGGER game_cyclopedia_receipt_no_truncate BEFORE TRUNCATE ON game_character_cyclopedia_receipts
 EXECUTE FUNCTION game_character_reject_truncate();
CREATE TRIGGER game_cyclopedia_epoch_immutable BEFORE UPDATE OR DELETE ON game_world_cyclopedia_epochs
 FOR EACH ROW EXECUTE FUNCTION game_character_immutable();

ALTER TABLE game_item_fee_burns DROP CONSTRAINT game_item_fee_burns_cause_kind_check,
 DROP CONSTRAINT game_item_fee_burns_purchase_cause,
 ADD CONSTRAINT game_item_fee_burns_cause_kind_check CHECK(cause_kind IN(1,2,3,4)),
 ADD CONSTRAINT game_item_fee_burns_purchase_cause CHECK(
 (cause_kind=1 AND charm_key IS NOT NULL AND purchase_service_key IS NULL)
 OR (cause_kind=2 AND charm_key IS NULL AND purchase_service_key IS NOT NULL
     AND purchase_service_key ~ '^oteryn:service[.]promotion[.][a-z0-9_]{1,80}$'
     AND bank_debit_gold_units=0 AND fee_gold_units=20000)
 OR (cause_kind=3 AND charm_key IS NULL AND purchase_service_key IS NOT NULL AND purchase_service_key IN(
 'oteryn:service.blessing.the_wisdom_of_solitude','oteryn:service.blessing.blood_of_the_mountain','oteryn:service.blessing.heart_of_the_mountain'))
 OR (cause_kind=4 AND charm_key IS NULL AND purchase_service_key IS NOT NULL
     AND purchase_service_key='oteryn:area.entry_environs' AND fee_gold_units BETWEEN 1 AND 10000000));
CREATE FUNCTION game_cyclopedia_fee_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF TG_TABLE_NAME='game_item_fee_burns' THEN
  IF NEW.cause_kind=4 AND NOT EXISTS(SELECT 1 FROM game_character_cyclopedia_receipts r
    WHERE r.fee_transaction_id=NEW.transaction_id AND r.occurrence=NEW.cause_occurrence_id
      AND r.character_id=NEW.character_id AND r.world_id=NEW.world_id AND r.donation=NEW.fee_gold_units
      AND r.committed_character_revision=NEW.committed_character_revision
      AND r.created_xact_id=pg_current_xact_id()) THEN
   RAISE EXCEPTION 'area donation requires atomic map receipt' USING ERRCODE='23514';
  END IF;
 ELSE
  IF NEW.donation>0 AND NOT EXISTS(SELECT 1 FROM game_item_fee_burns f
    WHERE f.transaction_id=NEW.fee_transaction_id AND f.cause_kind=4 AND f.cause_occurrence_id=NEW.occurrence
      AND f.character_id=NEW.character_id AND f.world_id=NEW.world_id AND f.fee_gold_units=NEW.donation
      AND f.committed_character_revision=NEW.committed_character_revision
      AND f.created_xact_id=pg_current_xact_id()) THEN
   RAISE EXCEPTION 'map donation requires its atomic fee' USING ERRCODE='23514';
  END IF;
 END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_cyclopedia_fee_source AFTER INSERT ON game_item_fee_burns
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_cyclopedia_fee_guard();
CREATE CONSTRAINT TRIGGER game_cyclopedia_fee_receipt AFTER INSERT ON game_character_cyclopedia_receipts
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_cyclopedia_fee_guard();
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_cyclopedia_fee_guard() SET search_path=%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_cyclopedia_fee_guard() FROM PUBLIC;
REVOKE ALL ON game_character_cyclopedia,game_world_cyclopedia,game_world_cyclopedia_epochs,
 game_character_cyclopedia_receipts,game_character_cyclopedia_revision_chain FROM PUBLIC;
GRANT SELECT,INSERT,UPDATE ON game_character_cyclopedia,game_world_cyclopedia TO oteryn_game_runtime;
GRANT SELECT,INSERT ON game_world_cyclopedia_epochs,game_character_cyclopedia_receipts TO oteryn_game_runtime;
GRANT SELECT ON game_character_cyclopedia_revision_chain TO oteryn_game_runtime;
GRANT SELECT ON game_character_cyclopedia,game_world_cyclopedia,game_world_cyclopedia_epochs,
 game_character_cyclopedia_receipts,game_character_cyclopedia_revision_chain TO oteryn_game_control;


-- Bidirectional transaction evidence: neither pool nor progression can silently
-- change independently of the receipt/selection that proves the same transition.
CREATE FUNCTION game_cyclopedia_state_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF TG_OP='UPDATE' AND (OLD.progress->>'discovered')::boolean
    AND NOT (NEW.progress->>'discovered')::boolean THEN
  RAISE EXCEPTION 'discovery cannot regress' USING ERRCODE='23514';
 END IF;
 IF NOT EXISTS(SELECT 1 FROM game_character_cyclopedia_receipts r
   WHERE r.occurrence=NEW.occurrence AND r.character_id=NEW.character_id
     AND r.committed_character_revision IS NOT NULL AND r.created_xact_id=pg_current_xact_id()) THEN
  RAISE EXCEPTION 'map state requires this transaction Character receipt' USING ERRCODE='23514';
 END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_cyclopedia_state_consistent AFTER INSERT OR UPDATE ON game_character_cyclopedia
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_cyclopedia_state_guard();
CREATE FUNCTION game_cyclopedia_pool_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NEW.bundle_digest<>OLD.bundle_digest OR NEW.catalogue_digest<>OLD.catalogue_digest THEN
  RAISE EXCEPTION 'map bundle change needs explicit migration' USING ERRCODE='23514';
 END IF;
 IF NEW.pool<>OLD.pool AND NOT EXISTS(SELECT 1 FROM game_character_cyclopedia_receipts r
    WHERE r.world_id=NEW.world_id AND r.pool_before=OLD.pool AND r.pool_after=NEW.pool
      AND r.donation>0 AND r.created_xact_id=pg_current_xact_id())
   AND NOT EXISTS(SELECT 1 FROM game_world_cyclopedia_epochs e
    WHERE e.world_id=NEW.world_id AND e.donation_winner AND NEW.pool=0 AND e.pool_before=OLD.pool
      AND e.created_xact_id=pg_current_xact_id()) THEN
  RAISE EXCEPTION 'area pool requires atomic donation or reset selection' USING ERRCODE='23514';
 END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_cyclopedia_pool_consistent AFTER UPDATE ON game_world_cyclopedia
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_cyclopedia_pool_guard();
DO $$ DECLARE f TEXT; BEGIN
 FOREACH f IN ARRAY ARRAY['game_cyclopedia_state_guard()','game_cyclopedia_pool_guard()'] LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=%I,pg_temp',f,current_schema());
 END LOOP;
END $$;
REVOKE ALL ON FUNCTION game_cyclopedia_state_guard(),game_cyclopedia_pool_guard() FROM PUBLIC;
