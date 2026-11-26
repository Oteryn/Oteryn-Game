-- Reserve Inbox room before sending paid ordinary Store purchases to Platform.
CREATE TABLE game_store_item_reservations (
    purchase_request_id UUID PRIMARY KEY REFERENCES store_purchase_requests(purchase_request_id),
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id)
);
CREATE INDEX game_store_item_reservations_character ON game_store_item_reservations(character_id);
REVOKE ALL ON game_store_item_reservations FROM PUBLIC;
GRANT SELECT, INSERT, DELETE ON game_store_item_reservations TO oteryn_game_runtime;
GRANT SELECT ON game_store_item_reservations TO oteryn_game_control;

-- Ordinary Store receipts written before this migration still identify the
-- native mint by operation_id. All new Store item receipts name the attempt.
ALTER TABLE store_delivery_receipts
    DROP CONSTRAINT store_delivery_native_attempt_shape,
    ADD CONSTRAINT store_delivery_native_attempt_shape CHECK (
        (native_transaction_id IS NULL OR
            (item_instance_id IS NOT NULL AND game_character_is_uuid_v7(native_transaction_id)))
        AND (purchase_request_id IS NULL OR item_instance_id IS NULL OR native_transaction_id IS NOT NULL)
    );
COMMENT ON COLUMN store_delivery_receipts.native_transaction_id IS
    'Winning native Inbox attempt for a stable Store operation. Older ordinary Store mints used operation_id.';

-- A coin delivery follows the purchasing Account. Its immutable provenance
-- must not prevent a later owner of the Character from buying the same offer.
ALTER TABLE game_character_store_cosmetics
    DROP CONSTRAINT game_character_store_cosmetics_pkey,
    DROP CONSTRAINT game_character_store_cosmetics_purchase_id_key,
    ADD PRIMARY KEY (purchase_id);
CREATE INDEX store_cosmetic_character_offer ON game_character_store_cosmetics(character_id,world_id,offer_key);

-- A later confirmation can resume a paid operation. Preserve its idempotency
-- even when that operation finishes before the later client receives a reply.
CREATE TABLE game_store_purchase_retries (
    retry_id UUID PRIMARY KEY REFERENCES store_purchase_requests(purchase_request_id),
    original_id UUID NOT NULL REFERENCES store_purchase_requests(purchase_request_id),
    world_id UUID NOT NULL CHECK (game_character_is_uuid_v7(world_id)),
    CHECK (retry_id <> original_id)
);
CREATE INDEX game_store_purchase_retries_original ON game_store_purchase_retries(original_id);
REVOKE ALL ON game_store_purchase_retries FROM PUBLIC;
GRANT SELECT, INSERT ON game_store_purchase_retries TO oteryn_game_runtime;
GRANT SELECT ON game_store_purchase_retries TO oteryn_game_control;
