-- Extend the existing closed Prey fee family, keeping every prior source proof.
DO $$ DECLARE body TEXT; BEGIN
 SELECT regexp_replace(pg_get_constraintdef(oid),' NOT VALID$','') INTO STRICT body FROM pg_constraint
  WHERE conrelid='game_item_fee_burns'::regclass AND conname='game_item_fee_burns_purchase_cause';
 IF position('oteryn:prey.pick_again.1' IN body)=0 THEN
  ALTER TABLE game_item_fee_burns DROP CONSTRAINT game_item_fee_burns_purchase_cause;
  EXECUTE 'ALTER TABLE game_item_fee_burns ADD CONSTRAINT game_item_fee_burns_purchase_cause CHECK (('||substr(body,7)||') OR (cause_kind=12 AND charm_key IS NULL AND purchase_service_key IN (''oteryn:prey.pick_again.1'',''oteryn:prey.pick_again.2'',''oteryn:prey.pick_again.3'') AND fee_gold_units>0)) NOT VALID';
 END IF;
END $$;
CREATE OR REPLACE FUNCTION game_prey_fee_kind_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NEW.cause_kind=12 AND NOT EXISTS(
  SELECT 1 FROM game_character_prey_receipts r
  WHERE r.character_id=NEW.character_id AND r.occurrence=NEW.cause_occurrence_id
   AND r.committed_character_revision=NEW.committed_character_revision
   AND r.created_xact_id=pg_current_xact_id() AND r.fee_gold_units=NEW.fee_gold_units
   AND NEW.purchase_service_key=CASE r.fee_kind
    WHEN 'LIST_REROLL' THEN 'oteryn:prey.list_reroll.'||r.fee_slot::text
    WHEN 'PICK_AGAIN' THEN 'oteryn:prey.pick_again.'||r.fee_slot::text END
 ) THEN RAISE EXCEPTION 'Prey fee requires its exact fenced action receipt' USING ERRCODE='23514'; END IF;
 RETURN NULL;
END $$;
DO $$ BEGIN EXECUTE format('ALTER FUNCTION game_prey_fee_kind_guard() SET search_path=pg_catalog,%I,pg_temp',current_schema()); END $$;
REVOKE ALL ON FUNCTION game_prey_fee_kind_guard() FROM PUBLIC;
DROP TRIGGER IF EXISTS game_prey_fee_kind_consistent ON game_item_fee_burns;
CREATE CONSTRAINT TRIGGER game_prey_fee_kind_consistent AFTER INSERT ON game_item_fee_burns
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_prey_fee_kind_guard();
