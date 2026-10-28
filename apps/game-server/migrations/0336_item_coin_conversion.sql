-- Native carried-coin USE reuses 0317's conservation, custody and atomic lineage guards.
-- NULL origin remains the banker operation. An item origin binds one exact source forever.
ALTER TABLE game_coin_exchange_receipts ADD COLUMN IF NOT EXISTS item_source UUID;
CREATE OR REPLACE FUNCTION game_item_coin_exchange_source_guard() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE input JSONB; quantity BIGINT;
BEGIN
 IF NEW.item_source IS NULL THEN RETURN NEW; END IF;
 IF NEW.amount<>1 OR NEW.npc_identity<>uuid_send(NEW.item_source)
  OR NEW.npc_key<>'item-use' OR NEW.npc_generation<>1
  OR NEW.catalogue_revision<>1 OR NEW.conversation_revision<>1 THEN
  RAISE EXCEPTION 'invalid item coin use origin';
 END IF;
 IF NEW.result=1 THEN
  IF jsonb_array_length(NEW.inputs)<>1 THEN RAISE EXCEPTION 'coin use needs one source'; END IF;
  input:=NEW.inputs->0;
  quantity:=(input->'before'->>'quantity')::bigint;
  IF (input->>'item')::uuid IS DISTINCT FROM NEW.item_source
   OR (CASE NEW.exchange WHEN 1 THEN quantity<>100 WHEN 3 THEN quantity<>100
        WHEN 2 THEN quantity NOT BETWEEN 1 AND 99 WHEN 4 THEN quantity NOT BETWEEN 1 AND 100
        ELSE true END) THEN
   RAISE EXCEPTION 'coin use source or direction mismatch';
  END IF;
 END IF;
 RETURN NEW;
END $$;
DROP TRIGGER IF EXISTS game_item_coin_exchange_source_guard ON game_coin_exchange_receipts;
CREATE TRIGGER game_item_coin_exchange_source_guard BEFORE INSERT OR UPDATE ON game_coin_exchange_receipts
 FOR EACH ROW EXECUTE FUNCTION game_item_coin_exchange_source_guard();

DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_item_coin_exchange_source_guard() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_item_coin_exchange_source_guard() FROM PUBLIC;
