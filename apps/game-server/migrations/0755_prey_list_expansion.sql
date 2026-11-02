-- PREY-DIFF Store entitlement. A public wire command cannot grant it.
CREATE TABLE IF NOT EXISTS game_prey_expansion_deliveries (
 operation_id UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(operation_id)),
 character_id UUID NOT NULL REFERENCES game_character_roots(character_id) ON DELETE CASCADE,
 world_id UUID NOT NULL,
 occurrence UUID NOT NULL,
 fulfilment_unit_id TEXT NOT NULL CHECK(octet_length(fulfilment_unit_id) BETWEEN 1 AND 512),
 created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id(),
 UNIQUE(character_id,operation_id),
 FOREIGN KEY(character_id,occurrence) REFERENCES game_character_prey_receipts(character_id,occurrence) DEFERRABLE INITIALLY DEFERRED,
 FOREIGN KEY(operation_id) REFERENCES store_delivery_receipts(operation_id) DEFERRABLE INITIALLY DEFERRED
);
CREATE INDEX IF NOT EXISTS game_prey_expansion_delivery_receipt ON game_prey_expansion_deliveries(character_id,occurrence);
CREATE TABLE IF NOT EXISTS game_character_prey_expansions (
 character_id UUID PRIMARY KEY REFERENCES game_character_roots(character_id) ON DELETE CASCADE,
 world_id UUID NOT NULL,
 delivery_operation_id UUID NOT NULL UNIQUE REFERENCES game_prey_expansion_deliveries(operation_id),
 created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id()
);
CREATE TABLE IF NOT EXISTS game_prey_expansion_purchases (
 character_id UUID PRIMARY KEY REFERENCES game_character_roots(character_id) ON DELETE CASCADE,
 purchase_id UUID NOT NULL UNIQUE REFERENCES store_purchase_requests(purchase_request_id),
 world_id UUID NOT NULL,
 created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id()
);
REVOKE ALL ON game_prey_expansion_deliveries,game_character_prey_expansions,game_prey_expansion_purchases FROM PUBLIC;
GRANT SELECT,INSERT ON game_prey_expansion_deliveries,game_character_prey_expansions TO oteryn_game_runtime;
GRANT SELECT,INSERT,DELETE ON game_prey_expansion_purchases TO oteryn_game_runtime;

ALTER TABLE store_delivery_receipts ALTER COLUMN item_instance_id DROP NOT NULL;
ALTER TABLE store_delivery_receipts ADD COLUMN IF NOT EXISTS entitlement_key TEXT;
ALTER TABLE store_delivery_receipts DROP CONSTRAINT IF EXISTS store_delivery_payload_kind;
ALTER TABLE store_delivery_receipts ADD CONSTRAINT store_delivery_payload_kind CHECK(
 (item_instance_id IS NOT NULL AND entitlement_key IS NULL)
 OR (item_instance_id IS NULL AND entitlement_key IS NOT NULL AND entitlement_key='prey-list-expansion')
);
-- Retain the exact existing item-mint proof for every item delivery.
DROP TRIGGER IF EXISTS store_delivery_proven ON store_delivery_receipts;
CREATE CONSTRAINT TRIGGER store_delivery_proven AFTER INSERT ON store_delivery_receipts
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW WHEN(NEW.item_instance_id IS NOT NULL) EXECUTE FUNCTION game_store_delivery_proven();

CREATE OR REPLACE FUNCTION game_prey_expansion_guard() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE operation UUID; character UUID; world UUID;
BEGIN
 IF TG_TABLE_NAME='game_character_prey_expansions' THEN operation:=NEW.delivery_operation_id; character:=NEW.character_id; world:=NEW.world_id;
 ELSIF TG_TABLE_NAME='game_prey_expansion_deliveries' THEN operation:=NEW.operation_id; character:=NEW.character_id; world:=NEW.world_id;
 ELSE operation:=NEW.operation_id; character:=NEW.character_id; world:=NEW.world_id; END IF;
 IF NOT EXISTS(
  SELECT 1 FROM game_prey_expansion_deliveries d
  JOIN game_character_prey_expansions e ON e.character_id=d.character_id AND e.delivery_operation_id=d.operation_id
  JOIN game_character_prey_receipts r ON r.character_id=d.character_id AND r.occurrence=d.occurrence
  JOIN game_character_roots c ON c.character_id=d.character_id
  JOIN store_delivery_receipts s ON s.operation_id=d.operation_id
  JOIN store_receipt_outbox o ON o.operation_id=s.operation_id
  WHERE d.operation_id=operation AND d.character_id=character AND d.world_id=world AND e.world_id=world
   AND s.character_id=character AND s.world_id=world AND c.world_id=world AND c.account_id=s.account_id
   AND s.entitlement_key='prey-list-expansion' AND s.item_instance_id IS NULL
   AND s.fulfilment_unit_id=d.fulfilment_unit_id AND r.store_expansion
   AND d.created_xact_id=pg_current_xact_id() AND e.created_xact_id=pg_current_xact_id() AND r.created_xact_id=pg_current_xact_id()
 ) THEN RAISE EXCEPTION 'Prey expansion requires atomic fenced receipt, Store delivery and outbox' USING ERRCODE='23514'; END IF;
 RETURN NULL;
END $$;
CREATE OR REPLACE FUNCTION game_prey_purchase_release_guard() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE answer JSONB;
BEGIN
 SELECT convert_from(r.answer,'UTF8')::jsonb INTO answer FROM store_purchase_requests r
 WHERE r.purchase_request_id=OLD.purchase_id AND r.character_id=OLD.character_id AND r.state=2;
 IF answer IS NULL
  OR answer->'order_id' IS DISTINCT FROM 'null'::jsonb
  OR answer->'operation_ids' IS DISTINCT FROM '[]'::jsonb
  OR COALESCE(answer->>'result','') NOT IN('STALE_QUOTE','OFFER_UNAVAILABLE','PURCHASE_DISABLED','INSUFFICIENT_BALANCE','ACCOUNT_RESTRICTED')
 THEN RAISE EXCEPTION 'Prey purchase reservation requires a definitive Store refusal' USING ERRCODE='23514'; END IF;
 RETURN OLD;
END $$;
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_prey_expansion_guard() SET search_path=pg_catalog,%I,pg_temp',current_schema());
 EXECUTE format('ALTER FUNCTION game_prey_purchase_release_guard() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_prey_expansion_guard(),game_prey_purchase_release_guard() FROM PUBLIC;
DROP TRIGGER IF EXISTS game_prey_expansion_delivery_proven ON game_prey_expansion_deliveries;
CREATE CONSTRAINT TRIGGER game_prey_expansion_delivery_proven AFTER INSERT ON game_prey_expansion_deliveries
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_prey_expansion_guard();
DROP TRIGGER IF EXISTS game_prey_expansion_proven ON game_character_prey_expansions;
CREATE CONSTRAINT TRIGGER game_prey_expansion_proven AFTER INSERT ON game_character_prey_expansions
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_prey_expansion_guard();
DROP TRIGGER IF EXISTS store_prey_expansion_proven ON store_delivery_receipts;
CREATE CONSTRAINT TRIGGER store_prey_expansion_proven AFTER INSERT ON store_delivery_receipts
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW WHEN(NEW.entitlement_key='prey-list-expansion') EXECUTE FUNCTION game_prey_expansion_guard();
DROP TRIGGER IF EXISTS game_prey_purchase_release_proven ON game_prey_expansion_purchases;
CREATE TRIGGER game_prey_purchase_release_proven BEFORE DELETE ON game_prey_expansion_purchases FOR EACH ROW EXECUTE FUNCTION game_prey_purchase_release_guard();
DO $$ DECLARE tab TEXT; BEGIN
 FOREACH tab IN ARRAY ARRAY['game_prey_expansion_deliveries','game_character_prey_expansions'] LOOP
  EXECUTE format('DROP TRIGGER IF EXISTS prey_expansion_immutable ON %I',tab);
  EXECUTE format('CREATE TRIGGER prey_expansion_immutable BEFORE UPDATE OR DELETE ON %I FOR EACH ROW EXECUTE FUNCTION game_character_immutable()',tab);
 END LOOP;
 FOREACH tab IN ARRAY ARRAY['game_prey_expansion_deliveries','game_character_prey_expansions','game_prey_expansion_purchases'] LOOP
  EXECUTE format('DROP TRIGGER IF EXISTS prey_expansion_no_truncate ON %I',tab);
  EXECUTE format('CREATE TRIGGER prey_expansion_no_truncate BEFORE TRUNCATE ON %I EXECUTE FUNCTION game_character_reject_truncate()',tab);
 END LOOP;
END $$;
