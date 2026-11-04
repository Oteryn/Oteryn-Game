-- P01: serialize permanent Charm Expansion Store purchases before Platform debit.
CREATE TABLE game_charm_expansion_purchases (
    character_id UUID PRIMARY KEY REFERENCES game_character_roots(character_id),
    purchase_id UUID NOT NULL UNIQUE REFERENCES store_purchase_requests(purchase_request_id),
    world_id UUID NOT NULL,
    reserved_at TIMESTAMPTZ NOT NULL DEFAULT statement_timestamp()
);

REVOKE ALL ON game_charm_expansion_purchases FROM PUBLIC;
GRANT SELECT, INSERT, DELETE ON game_charm_expansion_purchases TO oteryn_game_runtime;
GRANT SELECT ON game_charm_expansion_purchases TO oteryn_game_control;

ALTER TABLE store_delivery_receipts DROP CONSTRAINT store_delivery_payload_kind;
ALTER TABLE store_delivery_receipts ADD CONSTRAINT store_delivery_payload_kind CHECK(
    (item_instance_id IS NOT NULL AND entitlement_key IS NULL)
    OR (item_instance_id IS NULL AND entitlement_key = 'prey-list-expansion' AND purchase_request_id IS NULL)
    OR (item_instance_id IS NULL AND entitlement_key = 'charm-expansion' AND purchase_request_id IS NOT NULL)
    OR (item_instance_id IS NULL AND entitlement_key IS NULL AND purchase_request_id IS NOT NULL)
);

-- Purchase-bound Charm receipts use their own atomic effect proof below.
-- The Hireling proof continues to cover every other purchase-bound receipt.
DROP TRIGGER store_hireling_delivery_proven ON store_delivery_receipts;
CREATE CONSTRAINT TRIGGER store_hireling_delivery_proven AFTER INSERT ON store_delivery_receipts
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
    WHEN(NEW.purchase_request_id IS NOT NULL AND NEW.entitlement_key IS DISTINCT FROM 'charm-expansion')
    EXECUTE FUNCTION game_store_hireling_delivery_proven();

CREATE FUNCTION game_charm_expansion_delivery_proven() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
    IF NOT EXISTS(
        SELECT 1
        FROM game_charm_expansion_purchases p
        JOIN store_purchase_requests q ON q.purchase_request_id=p.purchase_id
        JOIN game_character_charm_expansions e ON e.character_id=p.character_id
        JOIN store_receipt_outbox o ON o.operation_id=NEW.operation_id
        WHERE p.purchase_id=NEW.purchase_request_id
          AND p.character_id=NEW.character_id AND p.world_id=NEW.world_id
          AND q.character_id=NEW.character_id AND q.account_id=NEW.account_id
          AND NEW.entitlement_key='charm-expansion' AND NEW.item_instance_id IS NULL
    ) THEN
        RAISE EXCEPTION 'Charm expansion requires Store purchase proof and atomic effect'
            USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END $$;
DO $$ BEGIN
    EXECUTE format('ALTER FUNCTION game_charm_expansion_delivery_proven() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_charm_expansion_delivery_proven() FROM PUBLIC;
CREATE CONSTRAINT TRIGGER store_charm_expansion_proven
    AFTER INSERT ON store_delivery_receipts DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW WHEN(NEW.entitlement_key='charm-expansion')
    EXECUTE FUNCTION game_charm_expansion_delivery_proven();
