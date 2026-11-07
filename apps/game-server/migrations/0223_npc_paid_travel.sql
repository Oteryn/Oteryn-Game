-- TRAVEL: extend the existing receipt/arrival owner; paid travel is item-only.
ALTER TABLE game_newhaven_travel_receipts
 DROP CONSTRAINT game_newhaven_travel_receipts_route_check,
 ADD COLUMN request_binding BYTEA CHECK (request_binding IS NULL OR octet_length(request_binding)=32),
 ADD COLUMN fee_transaction_id UUID UNIQUE REFERENCES game_item_fee_burns(transaction_id) DEFERRABLE INITIALLY DEFERRED,
 ADD COLUMN fare_gold_units BIGINT,
 ADD COLUMN base_fare_gold_units BIGINT,
 ADD COLUMN postman_discount_applied BOOLEAN,
 ADD COLUMN created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id(),
 ADD CONSTRAINT game_travel_route_shape CHECK (
   (route IN ('rookgaard','thais_peninsula') AND request_binding IS NULL AND fee_transaction_id IS NULL AND fare_gold_units IS NULL AND base_fare_gold_units IS NULL AND postman_discount_applied IS NULL)
   OR (route ~ '^oteryn:service[.]travel[.][a-z0-9_]{1,80}/[a-z0-9_]{1,80}$'
       AND request_binding IS NOT NULL AND fare_gold_units IS NOT NULL AND fare_gold_units BETWEEN 0 AND 1000000
       AND base_fare_gold_units IS NOT NULL AND base_fare_gold_units BETWEEN 0 AND 1000000
       AND postman_discount_applied IS NOT NULL
       AND fare_gold_units=GREATEST(0,base_fare_gold_units-CASE WHEN postman_discount_applied THEN 10 ELSE 0 END)
       AND ((fare_gold_units=0 AND fee_transaction_id IS NULL) OR (fare_gold_units>0 AND fee_transaction_id IS NOT NULL))));

CREATE INDEX game_npc_travel_cooldown ON game_newhaven_travel_receipts(character_id,arrived_at DESC) WHERE arrived_at IS NOT NULL;

CREATE FUNCTION game_npc_travel_receipt_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NEW.request_binding IS NOT NULL AND (
   NEW.created_xact_id<>pg_current_xact_id() OR NOT EXISTS (SELECT 1 FROM game_character_pending_arrivals a WHERE a.character_id=NEW.character_id AND a.occurrence_id=NEW.occurrence_id AND a.xmin=pg_current_xact_id()::xid)
   OR (NEW.fare_gold_units>0 AND NOT EXISTS (
      SELECT 1 FROM game_item_fee_burns f WHERE f.transaction_id=NEW.fee_transaction_id
       AND f.cause_kind=8 AND f.character_id=NEW.character_id AND f.cause_occurrence_id=NEW.occurrence_id
       AND f.fee_gold_units=NEW.fare_gold_units AND f.purchase_service_key=split_part(NEW.route,'/',1)
       AND f.created_xact_id=pg_current_xact_id() AND NEW.created_xact_id=pg_current_xact_id()))) THEN
   RAISE EXCEPTION 'paid travel requires same-transaction fee and arrival' USING ERRCODE='23514';
 END IF;
 RETURN NULL;
END;
$$;
CREATE CONSTRAINT TRIGGER game_npc_travel_receipt_consistent AFTER INSERT ON game_newhaven_travel_receipts
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_npc_travel_receipt_guard();
CREATE TRIGGER game_npc_travel_receipt_stamp BEFORE INSERT ON game_newhaven_travel_receipts
 FOR EACH ROW EXECUTE FUNCTION game_item_stamp_created_xact_id();

CREATE FUNCTION game_npc_travel_fee_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NEW.cause_kind=8 AND NOT EXISTS (
   SELECT 1 FROM game_newhaven_travel_receipts r WHERE r.fee_transaction_id=NEW.transaction_id
     AND r.character_id=NEW.character_id AND r.occurrence_id=NEW.cause_occurrence_id
     AND r.fare_gold_units=NEW.fee_gold_units AND split_part(r.route,'/',1)=NEW.purchase_service_key
     AND r.created_xact_id=pg_current_xact_id()) THEN
   RAISE EXCEPTION 'travel fee requires its own receipt' USING ERRCODE='23514';
 END IF;
 RETURN NULL;
END;
$$;
CREATE CONSTRAINT TRIGGER game_npc_travel_fee_consistent AFTER INSERT ON game_item_fee_burns
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_npc_travel_fee_guard();

DO $$
DECLARE body TEXT;
BEGIN
 SELECT pg_get_constraintdef(oid) INTO STRICT body FROM pg_constraint
  WHERE conrelid='game_item_fee_burns'::regclass AND conname='game_item_fee_burns_cause_kind_check';
 ALTER TABLE game_item_fee_burns DROP CONSTRAINT game_item_fee_burns_cause_kind_check;
 EXECUTE 'ALTER TABLE game_item_fee_burns ADD CONSTRAINT game_item_fee_burns_cause_kind_check CHECK (('||substr(body,7)||') OR cause_kind=8)';
 SELECT pg_get_constraintdef(oid) INTO STRICT body FROM pg_constraint
  WHERE conrelid='game_item_fee_burns'::regclass AND conname='game_item_fee_burns_purchase_cause';
 ALTER TABLE game_item_fee_burns DROP CONSTRAINT game_item_fee_burns_purchase_cause;
 EXECUTE 'ALTER TABLE game_item_fee_burns ADD CONSTRAINT game_item_fee_burns_purchase_cause CHECK (('||substr(body,7)||') OR (cause_kind=8 AND charm_key IS NULL AND purchase_service_key ~ ''^oteryn:service[.]travel[.][a-z0-9_]{1,80}$'' AND fee_gold_units BETWEEN 1 AND 1000000))';
 ALTER TABLE game_item_fee_burns DROP CONSTRAINT game_item_fee_burns_committed_character_revision_check;
 ALTER TABLE game_item_fee_burns ADD CONSTRAINT game_item_fee_burns_committed_character_revision_check
  CHECK (committed_character_revision BETWEEN 1 AND 18446744073709551615 AND (cause_kind IN (5,7,8) OR committed_character_revision>=2));
 SELECT pg_get_functiondef('game_item_fee_burn_consistency_guard()'::regprocedure) INTO STRICT body;
 IF position('(NEW.cause_kind=5 AND EXISTS' IN body)=0 THEN RAISE EXCEPTION 'missing item-only fee proof'; END IF;
 EXECUTE replace(body,'(NEW.cause_kind=5 AND EXISTS',
  '(NEW.cause_kind=8 AND EXISTS(SELECT 1 FROM game_newhaven_travel_receipts r WHERE r.fee_transaction_id=NEW.transaction_id AND r.character_id=NEW.character_id AND r.created_xact_id=pg_current_xact_id())) OR (NEW.cause_kind=5 AND EXISTS');
END;
$$;

DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_npc_travel_receipt_guard() SET search_path = pg_catalog, %I, pg_temp',current_schema());
 EXECUTE format('ALTER FUNCTION game_npc_travel_fee_guard() SET search_path = pg_catalog, %I, pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_npc_travel_receipt_guard(),game_npc_travel_fee_guard() FROM PUBLIC;
