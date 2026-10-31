-- W22a: a loot entry with a fluid sub-type mints the filled fluid container.
--
-- Crystal (201189f8) data/libs/functions/container.lua:48-52 transforms a looted non-stackable,
-- uncharged item to the entry's sub-type (`tmpItem:transform(itemId, item.subType)`), and an
-- unset sub-type on a fluid container to 0 (empty). Monster files name the fluid through
-- `subType` (monstertype.lua:119 copies it into the roll result).
--
-- The minted fluid is the item's first state row (0560). A state row is otherwise proven by a
-- player fluid transition; a minted one is proven by this immutable mint record, written in the
-- same transaction as the item's mint receipt for the same item and transaction.
-- Rollback: a successor restores the 0560 proof; the mint records stay as history.

CREATE TABLE game_item_fluid_loot_mints (
    item_instance_id UUID PRIMARY KEY REFERENCES game_item_instances (item_instance_id),
    transaction_id UUID NOT NULL CHECK (game_character_is_uuid_v7(transaction_id)),
    fluid_subtype SMALLINT NOT NULL CHECK (fluid_subtype BETWEEN 1 AND 255),
    created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id()
);

CREATE TRIGGER game_item_fluid_loot_mints_stamp
    BEFORE INSERT ON game_item_fluid_loot_mints
    FOR EACH ROW EXECUTE FUNCTION game_item_fluid_transition_stamp();
CREATE TRIGGER game_item_fluid_loot_mints_immutable
    BEFORE UPDATE OR DELETE ON game_item_fluid_loot_mints
    FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_item_fluid_loot_mints_no_truncate
    BEFORE TRUNCATE ON game_item_fluid_loot_mints
    EXECUTE FUNCTION game_item_immutable();

-- 0560's proof, plus: a first state row is proven by this transaction's loot mint of the item.
CREATE OR REPLACE FUNCTION game_item_fluid_state_proven() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE
    v_item UUID := CASE WHEN TG_OP = 'DELETE' THEN OLD.item_instance_id ELSE NEW.item_instance_id END;
    v_transaction UUID := CASE WHEN TG_OP = 'DELETE' THEN NULL ELSE NEW.last_transaction_id END;
    v_after SMALLINT := CASE WHEN TG_OP = 'DELETE' THEN 0 ELSE NEW.fluid_subtype END;
BEGIN
    IF TG_OP = 'UPDATE' AND (NEW.item_instance_id, NEW.world_id) IS DISTINCT FROM
                            (OLD.item_instance_id, OLD.world_id) THEN
        RAISE EXCEPTION 'ITEM_FLUID_IDENTITY' USING ERRCODE = '23514';
    END IF;
    IF TG_OP = 'INSERT' AND EXISTS (
        SELECT 1 FROM game_item_fluid_loot_mints m
          JOIN game_item_mint_receipts r
            ON r.item_instance_id = m.item_instance_id
           AND r.transaction_id = m.transaction_id
          JOIN game_item_instances i
            ON i.item_instance_id = m.item_instance_id
           AND i.world_id = NEW.world_id
           AND i.minted_transaction_id = m.transaction_id
         WHERE m.item_instance_id = v_item
           AND m.transaction_id = v_transaction
           AND m.fluid_subtype = v_after
           AND m.created_xact_id = pg_current_xact_id()
    ) THEN
        RETURN NEW;
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM game_item_fluid_transitions t
         WHERE t.item_instance_id = v_item
           AND (v_transaction IS NULL OR t.transaction_id = v_transaction)
           AND t.fluid_after = v_after
           AND t.created_xact_id = pg_current_xact_id()
    ) THEN
        RAISE EXCEPTION 'ITEM_FLUID_UNPROVEN' USING ERRCODE = '23514';
    END IF;
    RETURN CASE WHEN TG_OP = 'DELETE' THEN OLD ELSE NEW END;
END $$;

DO $$
BEGIN
    EXECUTE format('ALTER FUNCTION game_item_fluid_state_proven() SET search_path = pg_catalog, %I, pg_temp', current_schema());
END $$;

REVOKE ALL ON game_item_fluid_loot_mints FROM PUBLIC;
REVOKE ALL ON FUNCTION game_item_fluid_state_proven() FROM PUBLIC;
GRANT SELECT, INSERT ON game_item_fluid_loot_mints TO oteryn_game_runtime;
GRANT SELECT ON game_item_fluid_loot_mints TO oteryn_game_control;
