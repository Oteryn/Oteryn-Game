-- STORE-DUR-1: retry and delivery evidence; never a Game purchase or coin ledger.
CREATE TABLE IF NOT EXISTS store_purchase_requests (
    purchase_request_id UUID PRIMARY KEY
        CHECK (game_character_is_uuid_v7(purchase_request_id)),
    account_id UUID NOT NULL CHECK (game_character_is_uuid_v7(account_id)),
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    binding_digest BYTEA NOT NULL CHECK (octet_length(binding_digest) = 32),
    state SMALLINT NOT NULL CHECK (state IN (1, 2)),
    answer BYTEA CHECK (octet_length(answer) BETWEEN 1 AND 8192),
    created_at TIMESTAMPTZ NOT NULL DEFAULT statement_timestamp(),
    CHECK ((state = 1) = (answer IS NULL))
);

CREATE TABLE IF NOT EXISTS store_delivery_receipts (
    operation_id UUID PRIMARY KEY CHECK (game_character_is_uuid_v7(operation_id)),
    account_id UUID NOT NULL CHECK (game_character_is_uuid_v7(account_id)),
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    world_id UUID NOT NULL CHECK (game_character_is_uuid_v7(world_id)),
    fulfilment_unit_id TEXT NOT NULL
        CHECK (octet_length(fulfilment_unit_id) BETWEEN 1 AND 512),
    result TEXT NOT NULL CHECK (result = 'COMMITTED'),
    recovery_generation NUMERIC(20,0) NOT NULL
        CHECK (recovery_generation BETWEEN 1 AND 18446744073709551615),
    decided_at BIGINT NOT NULL CHECK (decided_at > 0),
    item_instance_id UUID NOT NULL UNIQUE REFERENCES game_item_instances(item_instance_id),
    receipt BYTEA NOT NULL CHECK (octet_length(receipt) BETWEEN 1 AND 4096)
);

CREATE TABLE IF NOT EXISTS store_receipt_outbox (
    operation_id UUID PRIMARY KEY REFERENCES store_delivery_receipts(operation_id),
    publish_attempts BIGINT NOT NULL DEFAULT 0 CHECK (publish_attempts >= 0),
    acknowledged_at TIMESTAMPTZ
);

DROP TRIGGER IF EXISTS store_purchase_requests_no_delete ON store_purchase_requests;
CREATE TRIGGER store_purchase_requests_no_delete BEFORE DELETE ON store_purchase_requests
    FOR EACH ROW EXECUTE FUNCTION game_character_immutable();
DROP TRIGGER IF EXISTS store_purchase_requests_no_truncate ON store_purchase_requests;
CREATE TRIGGER store_purchase_requests_no_truncate BEFORE TRUNCATE ON store_purchase_requests
    EXECUTE FUNCTION game_character_reject_truncate();
DROP TRIGGER IF EXISTS store_delivery_receipts_immutable ON store_delivery_receipts;
CREATE TRIGGER store_delivery_receipts_immutable BEFORE UPDATE OR DELETE ON store_delivery_receipts
    FOR EACH ROW EXECUTE FUNCTION game_character_immutable();
DROP TRIGGER IF EXISTS store_delivery_receipts_no_truncate ON store_delivery_receipts;
CREATE TRIGGER store_delivery_receipts_no_truncate BEFORE TRUNCATE ON store_delivery_receipts
    EXECUTE FUNCTION game_character_reject_truncate();

REVOKE ALL ON store_purchase_requests, store_delivery_receipts, store_receipt_outbox FROM PUBLIC;
GRANT SELECT, INSERT, UPDATE(answer,state) ON store_purchase_requests TO oteryn_game_runtime;
GRANT SELECT, INSERT ON store_delivery_receipts TO oteryn_game_runtime;
GRANT SELECT, INSERT, UPDATE(publish_attempts,acknowledged_at) ON store_receipt_outbox
    TO oteryn_game_runtime;
GRANT SELECT ON store_purchase_requests, store_delivery_receipts, store_receipt_outbox
    TO oteryn_game_control;

-- No CharacterRevision mutation: the native Inbox mint owns its own placement,
-- counters, audit and consistency guards. Store contributes no progression-chain
-- arm and never reopens acknowledged receipts using a caller-chosen generation.
CREATE OR REPLACE FUNCTION game_store_purchase_transition() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF ROW(NEW.purchase_request_id, NEW.account_id, NEW.character_id,
           NEW.binding_digest, NEW.created_at)
       IS DISTINCT FROM ROW(OLD.purchase_request_id, OLD.account_id, OLD.character_id,
                            OLD.binding_digest, OLD.created_at)
       OR OLD.state <> 1 OR NEW.state <> 2 OR NEW.answer IS NULL THEN
        RAISE EXCEPTION 'immutable Store request binding or answer' USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END $$;
DROP TRIGGER IF EXISTS store_purchase_transition ON store_purchase_requests;
CREATE TRIGGER store_purchase_transition BEFORE UPDATE ON store_purchase_requests
    FOR EACH ROW EXECUTE FUNCTION game_store_purchase_transition();

CREATE OR REPLACE FUNCTION game_store_delivery_proven() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM game_reward_claim_inbox_mint_receipts r
        JOIN game_reward_claim_inbox_mint_lines l USING(transaction_id)
        JOIN game_character_roots c ON c.character_id=r.character_id
        WHERE r.transaction_id=NEW.operation_id AND l.item_instance_id=NEW.item_instance_id
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
DROP TRIGGER IF EXISTS store_delivery_proven ON store_delivery_receipts;
CREATE CONSTRAINT TRIGGER store_delivery_proven AFTER INSERT ON store_delivery_receipts
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_store_delivery_proven();

DROP TRIGGER IF EXISTS store_outbox_no_delete ON store_receipt_outbox;
CREATE TRIGGER store_outbox_no_delete BEFORE DELETE ON store_receipt_outbox
    FOR EACH ROW EXECUTE FUNCTION game_character_immutable();
DROP TRIGGER IF EXISTS store_outbox_no_truncate ON store_receipt_outbox;
CREATE TRIGGER store_outbox_no_truncate BEFORE TRUNCATE ON store_receipt_outbox
    EXECUTE FUNCTION game_character_reject_truncate();
DO $$ BEGIN
    EXECUTE format('ALTER FUNCTION game_store_purchase_transition() SET search_path=pg_catalog,%I,pg_temp',current_schema());
    EXECUTE format('ALTER FUNCTION game_store_delivery_proven() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_store_purchase_transition(),game_store_delivery_proven() FROM PUBLIC;
CREATE INDEX IF NOT EXISTS store_receipt_outbox_pending ON store_receipt_outbox(operation_id)
    WHERE acknowledged_at IS NULL;
