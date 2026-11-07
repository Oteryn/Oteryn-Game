-- KEY-1 (WORLD-INTERACTION-0 §4.4, ACCEPTED D1019): the key capability's instance value.
--
-- Scope of this migration only:
--   * `game_item_keys`: the one immutable typed `key_number` of a key instance, in 1..65,535
--     (`WORLDINT0-RL-10`). It is written once, by the reward-claim MINT that creates the
--     instance (the CHEST-1 path, D40), in the same physical transaction as that MINT's
--     receipt; no other path writes it and it never changes or goes away.
-- A key is not stackable, so the keyed instance is minted at quantity 1. TRANSFER reads the
-- row and carries the number unchanged in its typed evidence; it writes nothing here. No
-- existing item gains a row, so no existing item changes.

CREATE TABLE game_item_keys (
    item_instance_id UUID PRIMARY KEY REFERENCES game_item_instances (item_instance_id),
    -- WORLDINT0-RL-10.
    key_number INTEGER NOT NULL CHECK (key_number BETWEEN 1 AND 65535),
    -- The reward-claim MINT that created the instance and wrote this row.
    minted_transaction_id UUID NOT NULL UNIQUE,
    -- Stamped below, never caller supplied.
    created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id()
);

-- Commit-time: a key row is written only with the reward-claim MINT of its instance, in the
-- same physical transaction, and that instance is one item.
CREATE FUNCTION game_item_key_proven() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM game_item_instances i
          JOIN game_reward_claim_mint_receipts r
            ON r.item_instance_id = i.item_instance_id
           AND r.transaction_id = i.minted_transaction_id
         WHERE i.item_instance_id = NEW.item_instance_id
           AND i.minted_transaction_id = NEW.minted_transaction_id
           AND i.quantity = 1
           AND r.quantity = 1
           AND r.created_xact_id = pg_current_xact_id()
           AND NEW.created_xact_id = pg_current_xact_id()) THEN
        RAISE EXCEPTION 'a key number is set only by the reward-claim MINT of its one item'
            USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END;
$$;

CREATE TRIGGER game_item_keys_stamp_xact BEFORE INSERT
    ON game_item_keys FOR EACH ROW EXECUTE FUNCTION game_item_stamp_created_xact_id();
CREATE CONSTRAINT TRIGGER game_item_key_proven
    AFTER INSERT ON game_item_keys
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
    EXECUTE FUNCTION game_item_key_proven();
CREATE TRIGGER game_item_keys_immutable BEFORE UPDATE OR DELETE
    ON game_item_keys FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_item_keys_no_truncate BEFORE TRUNCATE
    ON game_item_keys
    FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();

DO $$ BEGIN
    EXECUTE format('ALTER FUNCTION game_item_key_proven() SET search_path = %I, pg_temp', current_schema());
END $$;

REVOKE ALL ON game_item_keys FROM PUBLIC;
REVOKE ALL ON FUNCTION game_item_key_proven() FROM PUBLIC;
-- Least privilege: the runtime role inserts a key row with its MINT and reads it on TRANSFER.
GRANT SELECT, INSERT ON game_item_keys TO oteryn_game_runtime;
GRANT SELECT ON game_item_keys TO oteryn_game_control;
