-- Q4/Q6, Crystal 201189f potions.lua:1-31,52-54,63,125-129.
-- Evidence for the existing reserved condition owner, not a parallel condition engine.
-- A lost COMMIT acknowledgement resolves this exact receipt and original expiry.
CREATE TABLE game_item_condition_potion_receipts (
    transaction_id UUID PRIMARY KEY REFERENCES game_item_use_records(transaction_id),
    condition_key TEXT NOT NULL CHECK (condition_key IN (
        'oteryn:item.tibia.i7439', 'oteryn:item.tibia.i7440',
        'oteryn:item.tibia.i7443', 'oteryn:item.tibia.i49271')),
    definition_revision BIGINT NOT NULL CHECK (definition_revision = 1),
    applied_owner_micros NUMERIC(20,0) NOT NULL CHECK (applied_owner_micros BETWEEN 0 AND 18446744073709551615),
    expires_owner_micros NUMERIC(20,0) NOT NULL CHECK (expires_owner_micros BETWEEN 1 AND 18446744073709551615
        AND expires_owner_micros = applied_owner_micros + 600000000),
    content_digest BYTEA NOT NULL CHECK (octet_length(content_digest) = 32),
    application_binding BYTEA NOT NULL CHECK (octet_length(application_binding) = 32),
    created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id()
);

CREATE FUNCTION game_condition_potion_receipt_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    item game_item_use_records%ROWTYPE;
    receipt game_item_condition_potion_receipts%ROWTYPE;
BEGIN
    SELECT * INTO item FROM game_item_use_records WHERE transaction_id = NEW.transaction_id;
    IF item.source_definition_production_key NOT IN (
        'oteryn:item.tibia.i7439', 'oteryn:item.tibia.i7440',
        'oteryn:item.tibia.i7443', 'oteryn:item.tibia.i49271') THEN
        IF TG_TABLE_NAME = 'game_item_use_records' THEN RETURN NULL; END IF;
        RAISE EXCEPTION 'condition potion receipt needs its source' USING ERRCODE = '23514';
    END IF;
    SELECT * INTO receipt FROM game_item_condition_potion_receipts WHERE transaction_id = NEW.transaction_id;
    IF item.transaction_id IS NULL OR receipt.transaction_id IS NULL
       OR item.cause_kind <> 2 OR item.shape <> 1
       OR item.source_definition_family <> 'Item'
       OR item.source_definition_production_key <> receipt.condition_key
       OR item.created_xact_id <> pg_current_xact_id()
       OR receipt.created_xact_id <> pg_current_xact_id() THEN
        RAISE EXCEPTION 'condition potion burn and reserved effect must commit together' USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END;
$$;

CREATE CONSTRAINT TRIGGER game_condition_potion_use_consistent
    AFTER INSERT ON game_item_use_records DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW EXECUTE FUNCTION game_condition_potion_receipt_guard();
CREATE CONSTRAINT TRIGGER game_condition_potion_receipt_consistent
    AFTER INSERT ON game_item_condition_potion_receipts DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW EXECUTE FUNCTION game_condition_potion_receipt_guard();
CREATE TRIGGER game_condition_potion_receipt_immutable BEFORE UPDATE OR DELETE
    ON game_item_condition_potion_receipts FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_condition_potion_receipt_no_truncate BEFORE TRUNCATE
    ON game_item_condition_potion_receipts FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();
CREATE TRIGGER game_condition_potion_receipt_stamp_xact BEFORE INSERT
    ON game_item_condition_potion_receipts FOR EACH ROW EXECUTE FUNCTION game_item_stamp_created_xact_id();
DO $$ BEGIN
    EXECUTE format('ALTER FUNCTION game_condition_potion_receipt_guard() SET search_path = %I, pg_temp', current_schema());
END $$;
REVOKE ALL ON game_item_condition_potion_receipts FROM PUBLIC;
REVOKE ALL ON FUNCTION game_condition_potion_receipt_guard() FROM PUBLIC;
GRANT SELECT, INSERT ON game_item_condition_potion_receipts TO oteryn_game_runtime;
