-- Game-owned display journal for accepted purchases made through the game client.
CREATE TABLE IF NOT EXISTS game_store_purchase_history (
    purchase_request_id UUID PRIMARY KEY REFERENCES store_purchase_requests(purchase_request_id),
    account_id UUID NOT NULL CHECK (game_character_is_uuid_v7(account_id)),
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    order_id UUID NOT NULL CHECK (game_character_is_uuid_v7(order_id)),
    offer_key TEXT NOT NULL CHECK (octet_length(offer_key) BETWEEN 1 AND 128),
    catalog_revision TEXT NOT NULL CHECK (octet_length(catalog_revision) BETWEEN 1 AND 128),
    price BIGINT NOT NULL CHECK (price > 0),
    currency TEXT NOT NULL CHECK (octet_length(currency) BETWEEN 1 AND 128),
    purchased_at_ms BIGINT NOT NULL CHECK (purchased_at_ms > 0)
);

CREATE INDEX IF NOT EXISTS game_store_purchase_history_account_page
    ON game_store_purchase_history(account_id, purchased_at_ms DESC, purchase_request_id DESC);

DROP TRIGGER IF EXISTS game_store_purchase_history_immutable ON game_store_purchase_history;
CREATE TRIGGER game_store_purchase_history_immutable
    BEFORE UPDATE OR DELETE ON game_store_purchase_history
    FOR EACH ROW EXECUTE FUNCTION game_character_immutable();
DROP TRIGGER IF EXISTS game_store_purchase_history_no_truncate ON game_store_purchase_history;
CREATE TRIGGER game_store_purchase_history_no_truncate
    BEFORE TRUNCATE ON game_store_purchase_history
    EXECUTE FUNCTION game_character_reject_truncate();

REVOKE ALL ON game_store_purchase_history FROM PUBLIC;
GRANT SELECT, INSERT ON game_store_purchase_history TO oteryn_game_runtime;
GRANT SELECT ON game_store_purchase_history TO oteryn_game_control;
