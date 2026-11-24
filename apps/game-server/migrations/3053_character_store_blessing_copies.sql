-- Only the deferred proof on a committed Store receipt may add a copy.
CREATE TABLE game_character_store_blessing_copies (
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    blessing_key TEXT NOT NULL CHECK (blessing_key ~ '^oteryn:blessing[.][a-z_]{1,111}$'),
    copies BIGINT NOT NULL CHECK (copies > 0),
    PRIMARY KEY (character_id, blessing_key)
);

REVOKE ALL ON game_character_store_blessing_copies FROM PUBLIC;
GRANT SELECT ON game_character_store_blessing_copies TO oteryn_game_runtime;

CREATE UNIQUE INDEX store_blessing_delivery_purchase
    ON store_delivery_receipts(purchase_request_id)
    WHERE entitlement_key LIKE 'store-blessing:%';

ALTER TABLE store_delivery_receipts DROP CONSTRAINT store_delivery_payload_kind;
ALTER TABLE store_delivery_receipts ADD CONSTRAINT store_delivery_payload_kind CHECK (
    (item_instance_id IS NOT NULL AND entitlement_key IS NULL)
    OR (item_instance_id IS NULL AND entitlement_key = 'prey-list-expansion' AND purchase_request_id IS NULL)
    OR (item_instance_id IS NULL AND entitlement_key = 'charm-expansion' AND purchase_request_id IS NOT NULL)
    OR (item_instance_id IS NULL AND entitlement_key IS NULL AND purchase_request_id IS NOT NULL)
    OR (item_instance_id IS NULL AND entitlement_key LIKE 'store-cosmetic:%' AND purchase_request_id IS NOT NULL)
    OR (item_instance_id IS NULL AND entitlement_key LIKE 'store-blessing:%' AND purchase_request_id IS NOT NULL)
);

DROP TRIGGER store_hireling_delivery_proven ON store_delivery_receipts;
CREATE CONSTRAINT TRIGGER store_hireling_delivery_proven
    AFTER INSERT ON store_delivery_receipts DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW WHEN (
        NEW.purchase_request_id IS NOT NULL
        AND NEW.entitlement_key IS DISTINCT FROM 'charm-expansion'
        AND (NEW.entitlement_key IS NULL OR (
            NEW.entitlement_key NOT LIKE 'store-cosmetic:%'
            AND NEW.entitlement_key NOT LIKE 'store-blessing:%'
        ))
    ) EXECUTE FUNCTION game_store_hireling_delivery_proven();

CREATE FUNCTION game_store_blessing_delivery_proven() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, public, pg_temp AS $$
DECLARE
    v_blessing_key TEXT := substr(NEW.entitlement_key, length('store-blessing:') + 1);
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM public.store_purchase_requests p
        JOIN public.game_store_purchase_history h USING (purchase_request_id)
        JOIN public.game_character_roots c ON c.character_id = p.character_id
        JOIN public.store_receipt_outbox o ON o.operation_id = NEW.operation_id
        CROSS JOIN LATERAL (SELECT convert_from(p.answer, 'UTF8')::jsonb AS value) a
        WHERE p.purchase_request_id = NEW.purchase_request_id AND p.state = 2
          AND p.account_id = NEW.account_id AND p.character_id = NEW.character_id
          AND h.account_id = p.account_id AND h.character_id = p.character_id
          AND c.account_id = p.account_id AND c.world_id = NEW.world_id
          AND h.offer_key = 'store:blessing:' || substr(v_blessing_key, length('oteryn:blessing.') + 1)
          AND a.value->>'result' = 'ACCEPTED' AND a.value->>'order_id' = h.order_id::text
          AND a.value->'operation_ids' = jsonb_build_array(NEW.operation_id::text)
    ) THEN
        RAISE EXCEPTION 'Store blessing requires the accepted purchase and delivery outbox'
            USING ERRCODE = '23514';
    END IF;

    INSERT INTO public.game_character_store_blessing_copies (character_id, blessing_key, copies)
    VALUES (NEW.character_id, v_blessing_key, 1)
    ON CONFLICT (character_id, blessing_key) DO UPDATE
        SET copies = game_character_store_blessing_copies.copies + 1;
    RETURN NULL;
END $$;

REVOKE ALL ON FUNCTION game_store_blessing_delivery_proven() FROM PUBLIC;
CREATE CONSTRAINT TRIGGER store_blessing_delivery_proven
    AFTER INSERT ON store_delivery_receipts DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW WHEN (NEW.entitlement_key LIKE 'store-blessing:%')
    EXECUTE FUNCTION game_store_blessing_delivery_proven();
