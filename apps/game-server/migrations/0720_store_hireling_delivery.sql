-- E12: reserve Game-side eligibility before the existing Platform Store debit.
CREATE TABLE game_hireling_store_reservations (
 purchase_request_id UUID PRIMARY KEY REFERENCES store_purchase_requests(purchase_request_id),
 character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
 action BYTEA NOT NULL CHECK(octet_length(action) BETWEEN 1 AND 1024),
 kind SMALLINT NOT NULL CHECK(kind BETWEEN 1 AND 4),
 entitlement_id SMALLINT,
 hireling_id UUID REFERENCES game_house_hirelings(hireling_id),
 reserved_name TEXT,
 state SMALLINT NOT NULL CHECK(state IN(1,2,3)),
 operation_id UUID UNIQUE,
 CHECK((kind=2)=(entitlement_id IS NOT NULL)),
 CHECK(entitlement_id IS NULL OR entitlement_id BETWEEN 1001 AND 1004 OR entitlement_id BETWEEN 2001 AND 2009),
 CHECK((kind IN(3,4))=(hireling_id IS NOT NULL)),
 CHECK((kind IN(1,3))=(reserved_name IS NOT NULL)),
 CHECK((state=2)=(operation_id IS NOT NULL))
);
CREATE INDEX game_hireling_store_reservation_character ON game_hireling_store_reservations(character_id,state);
CREATE INDEX game_hireling_store_reservation_target ON game_hireling_store_reservations(hireling_id);
-- Crystal schema.sql697-713 permits repeated display names; UUIDs identify Hirelings.
CREATE UNIQUE INDEX game_hireling_store_pending_unlock ON game_hireling_store_reservations(character_id,entitlement_id) WHERE state=1 AND kind=2;
CREATE UNIQUE INDEX game_hireling_store_pending_target ON game_hireling_store_reservations(hireling_id) WHERE state=1 AND kind IN(3,4);

ALTER TABLE store_delivery_receipts ALTER COLUMN item_instance_id DROP NOT NULL,
 ADD COLUMN purchase_request_id UUID REFERENCES store_purchase_requests(purchase_request_id),
 ADD COLUMN native_transaction_id UUID UNIQUE REFERENCES game_reward_claim_inbox_mint_receipts(transaction_id)
   DEFERRABLE INITIALLY DEFERRED,
 ADD CONSTRAINT store_delivery_native_attempt_shape CHECK(
   (native_transaction_id IS NOT NULL)=(purchase_request_id IS NOT NULL AND item_instance_id IS NOT NULL)
   AND (native_transaction_id IS NULL OR game_character_is_uuid_v7(native_transaction_id))),
 ADD CONSTRAINT store_delivery_has_native_effect CHECK(item_instance_id IS NOT NULL OR purchase_request_id IS NOT NULL);
CREATE INDEX store_delivery_purchase ON store_delivery_receipts(purchase_request_id);
COMMENT ON COLUMN store_delivery_receipts.native_transaction_id IS
 'Winning native Inbox attempt for one stable Hireling Store operation. Ordinary Store retains operation_id as its native transaction.';

CREATE TABLE game_store_hireling_effects (
 operation_id UUID PRIMARY KEY REFERENCES store_delivery_receipts(operation_id) DEFERRABLE INITIALLY DEFERRED,
 purchase_request_id UUID NOT NULL UNIQUE REFERENCES game_hireling_store_reservations(purchase_request_id),
 character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
 kind SMALLINT NOT NULL CHECK(kind BETWEEN 1 AND 4),
 hireling_id UUID REFERENCES game_house_hirelings(hireling_id),
 entitlement_id SMALLINT,
 name TEXT,
 sex SMALLINT,
 created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id(),
 CHECK((kind=2)=(entitlement_id IS NOT NULL)),
 CHECK((kind<>2)=(hireling_id IS NOT NULL)),
 CHECK((kind IN(1,3))=(name IS NOT NULL)),
 CHECK((kind IN(1,4))=(sex IS NOT NULL)),
 CHECK(sex IS NULL OR sex IN(1,2))
);
CREATE INDEX game_store_hireling_effect_character ON game_store_hireling_effects(character_id);
CREATE INDEX game_store_hireling_effect_target ON game_store_hireling_effects(hireling_id);
CREATE TRIGGER game_store_hireling_effect_immutable BEFORE UPDATE OR DELETE ON game_store_hireling_effects FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_store_hireling_effect_no_truncate BEFORE TRUNCATE ON game_store_hireling_effects EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_store_hireling_effect_stamp BEFORE INSERT ON game_store_hireling_effects FOR EACH ROW EXECUTE FUNCTION game_item_stamp_created_xact_id();

-- Preserve 0345's complete atomic native-mint proof. A Hireling receipt names its
-- winning native attempt; ordinary Store still identifies that mint by operation_id.
CREATE OR REPLACE FUNCTION game_store_delivery_proven() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM game_reward_claim_inbox_mint_receipts r
        JOIN game_reward_claim_inbox_mint_lines l USING(transaction_id)
        JOIN game_character_roots c ON c.character_id=r.character_id
        WHERE r.transaction_id=COALESCE(NEW.native_transaction_id,NEW.operation_id) AND l.item_instance_id=NEW.item_instance_id
          AND r.character_id=NEW.character_id AND r.world_id=NEW.world_id
          AND c.account_id=NEW.account_id AND c.world_id=NEW.world_id
          AND r.item_count=1 AND r.created_xact_id=pg_current_xact_id()
          AND l.created_xact_id=pg_current_xact_id()
    ) OR NOT EXISTS (
        SELECT 1 FROM store_receipt_outbox o WHERE o.operation_id=NEW.operation_id
    ) THEN
        RAISE EXCEPTION 'Store receipt requires atomic native mint and outbox' USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END $$;
DROP TRIGGER store_delivery_proven ON store_delivery_receipts;
CREATE CONSTRAINT TRIGGER store_delivery_proven AFTER INSERT ON store_delivery_receipts
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW WHEN(NEW.item_instance_id IS NOT NULL)
 EXECUTE FUNCTION game_store_delivery_proven();

CREATE FUNCTION game_store_hireling_delivery_proven() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 IF NOT EXISTS(
   SELECT 1 FROM game_store_hireling_effects e
   JOIN game_hireling_store_reservations r USING(purchase_request_id)
   JOIN store_purchase_requests p USING(purchase_request_id)
   JOIN game_character_roots c ON c.character_id=e.character_id
   WHERE e.operation_id=NEW.operation_id AND e.purchase_request_id=NEW.purchase_request_id
     AND e.character_id=NEW.character_id AND p.character_id=NEW.character_id
     AND r.character_id=e.character_id AND c.account_id=NEW.account_id
     AND p.account_id=NEW.account_id AND c.world_id=NEW.world_id
     AND r.state=2 AND r.operation_id=NEW.operation_id AND e.kind=r.kind
     AND e.entitlement_id IS NOT DISTINCT FROM r.entitlement_id
     AND e.name IS NOT DISTINCT FROM r.reserved_name
     AND (e.kind=1 AND e.hireling_id=NEW.operation_id AND NEW.item_instance_id IS NOT NULL
          OR e.kind=2 AND e.hireling_id IS NULL AND NEW.item_instance_id IS NULL
          OR e.kind IN(3,4) AND e.hireling_id=r.hireling_id AND NEW.item_instance_id IS NULL)
     AND e.created_xact_id=pg_current_xact_id()
     AND ((e.kind=2 AND NEW.item_instance_id IS NULL AND EXISTS(
       SELECT 1 FROM game_hireling_entitlements u WHERE u.character_id=e.character_id AND u.entitlement_id=e.entitlement_id))
       OR (e.kind<>2 AND EXISTS(
       SELECT 1 FROM game_house_hirelings h WHERE h.hireling_id=e.hireling_id
         AND h.owner_character_id=e.character_id AND NOT h.active
         AND (e.kind<>1 OR h.lamp_item_instance_id=NEW.item_instance_id)
         AND (e.name IS NULL OR e.name=h.name) AND (e.sex IS NULL OR e.sex=h.sex))))
 ) OR NOT EXISTS(SELECT 1 FROM store_receipt_outbox o WHERE o.operation_id=NEW.operation_id) THEN
   RAISE EXCEPTION 'Hireling Store delivery requires atomic grant, reservation and outbox' USING ERRCODE='23514';
 END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER store_hireling_delivery_proven AFTER INSERT ON store_delivery_receipts
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW WHEN(NEW.purchase_request_id IS NOT NULL)
 EXECUTE FUNCTION game_store_hireling_delivery_proven();

CREATE FUNCTION game_hireling_store_reservation_transition() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF ROW(NEW.purchase_request_id,NEW.character_id,NEW.action,NEW.kind,NEW.entitlement_id,NEW.hireling_id,NEW.reserved_name)
    IS DISTINCT FROM ROW(OLD.purchase_request_id,OLD.character_id,OLD.action,OLD.kind,OLD.entitlement_id,OLD.hireling_id,OLD.reserved_name)
    OR OLD.state<>1 OR NEW.state NOT IN(2,3) THEN
   RAISE EXCEPTION 'immutable hireling Store binding or terminal reservation' USING ERRCODE='23514';
 END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER game_hireling_store_reservation_transition BEFORE UPDATE ON game_hireling_store_reservations
 FOR EACH ROW EXECUTE FUNCTION game_hireling_store_reservation_transition();
CREATE TRIGGER game_hireling_store_reservation_no_delete BEFORE DELETE ON game_hireling_store_reservations FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_hireling_store_reservation_no_truncate BEFORE TRUNCATE ON game_hireling_store_reservations EXECUTE FUNCTION game_item_immutable();

DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_store_delivery_proven() SET search_path=pg_catalog,%I,pg_temp',current_schema());
 EXECUTE format('ALTER FUNCTION game_store_hireling_delivery_proven() SET search_path=pg_catalog,%I,pg_temp',current_schema());
 EXECUTE format('ALTER FUNCTION game_hireling_store_reservation_transition() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON game_hireling_store_reservations,game_store_hireling_effects FROM PUBLIC;
GRANT SELECT,INSERT,UPDATE(state,operation_id) ON game_hireling_store_reservations TO oteryn_game_runtime;
GRANT SELECT,INSERT ON game_store_hireling_effects TO oteryn_game_runtime;
REVOKE ALL ON FUNCTION game_store_hireling_delivery_proven(),game_hireling_store_reservation_transition() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_store_hireling_delivery_proven(),game_hireling_store_reservation_transition() TO oteryn_game_runtime;
