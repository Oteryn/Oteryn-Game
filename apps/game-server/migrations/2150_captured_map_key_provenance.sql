-- M21: shared map-source key MINT uses the same immutable key custody owner.
CREATE OR REPLACE FUNCTION game_item_key_proven() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.created_xact_id = pg_current_xact_id() AND EXISTS (
        SELECT 1 FROM game_item_instances i
        JOIN game_map_item_mint_receipts r ON r.item_instance_id = i.item_instance_id
         AND r.transaction_id = i.minted_transaction_id
        WHERE i.item_instance_id = NEW.item_instance_id
          AND i.minted_transaction_id = NEW.minted_transaction_id
          AND i.quantity = 1 AND r.quantity = 1
          AND r.created_xact_id = pg_current_xact_id()) THEN
        RETURN NULL;
    END IF;
    IF NEW.created_xact_id = pg_current_xact_id()
       AND game_reward_claim_inbox_item_proven(NEW.item_instance_id, NEW.minted_transaction_id) THEN
        RETURN NULL;
    END IF;
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
DO $$ BEGIN
    EXECUTE format('ALTER FUNCTION game_item_key_proven() SET search_path = pg_catalog, %I, pg_temp', current_schema());
END $$;
