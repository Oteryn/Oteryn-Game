-- I14: Crystal 201189f8 potions.lua:21-26,36-42,57,62,78-123,141-147.
-- Both special effects, the source burn and the antidote flask commit together.
ALTER TABLE game_item_condition_potion_receipts
 DROP CONSTRAINT game_item_condition_potion_receipts_condition_key_check,
 DROP CONSTRAINT game_item_condition_potion_receipts_check,
 ADD CONSTRAINT game_item_condition_potion_receipts_condition_key_check CHECK(condition_key IN (
  'oteryn:item.tibia.i7439','oteryn:item.tibia.i7440','oteryn:item.tibia.i7443',
  'oteryn:item.tibia.i49271','oteryn:item.tibia.i7644','oteryn:item.tibia.i35563')),
 ADD CONSTRAINT game_item_condition_potion_receipts_expires_owner_micros_check CHECK(
  expires_owner_micros BETWEEN 0 AND 18446744073709551615 AND
  expires_owner_micros = applied_owner_micros +
   CASE condition_key
    WHEN 'oteryn:item.tibia.i7644' THEN 0
    WHEN 'oteryn:item.tibia.i35563' THEN 60000000
    ELSE 600000000
   END);

CREATE OR REPLACE FUNCTION game_condition_potion_receipt_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE item game_item_use_records%ROWTYPE;
 receipt game_item_condition_potion_receipts%ROWTYPE;
BEGIN
 SELECT * INTO item FROM game_item_use_records WHERE transaction_id=NEW.transaction_id;
 IF item.source_definition_production_key NOT IN (
  'oteryn:item.tibia.i7439','oteryn:item.tibia.i7440','oteryn:item.tibia.i7443',
  'oteryn:item.tibia.i49271','oteryn:item.tibia.i7644','oteryn:item.tibia.i35563') THEN
  IF TG_TABLE_NAME='game_item_use_records' THEN RETURN NULL; END IF;
  RAISE EXCEPTION 'condition potion receipt needs its source' USING ERRCODE='23514';
 END IF;
 SELECT * INTO receipt FROM game_item_condition_potion_receipts
 WHERE transaction_id=NEW.transaction_id;
 IF item.transaction_id IS NULL OR receipt.transaction_id IS NULL
    OR item.cause_kind<>2 OR item.source_definition_family<>'Item'
    OR item.source_definition_production_key<>receipt.condition_key
    OR item.created_xact_id<>pg_current_xact_id()
    OR receipt.created_xact_id<>pg_current_xact_id() THEN
  RAISE EXCEPTION 'condition potion burn and reserved effect must commit together'
   USING ERRCODE='23514';
 END IF;
 IF receipt.condition_key='oteryn:item.tibia.i7644' THEN
  IF item.shape NOT IN (3,4)
     OR item.flask_definition_family IS DISTINCT FROM 'Item'
     OR item.flask_definition_production_key IS DISTINCT FROM 'oteryn:item.tibia.i285'
     OR item.flask_definition_revision_ref IS DISTINCT FROM 'definition-r1' THEN
   RAISE EXCEPTION 'antidote requires its exact empty flask' USING ERRCODE='23514';
  END IF;
 ELSIF item.shape<>1 THEN
  RAISE EXCEPTION 'this condition potion has no flask' USING ERRCODE='23514';
 END IF;
 RETURN NULL;
END $$;
DO $$ BEGIN
 EXECUTE format(
  'ALTER FUNCTION game_condition_potion_receipt_guard() '
  'SET search_path=pg_catalog,%I,pg_temp', current_schema());
END $$;
REVOKE ALL ON FUNCTION game_condition_potion_receipt_guard() FROM PUBLIC;
