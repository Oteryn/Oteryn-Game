-- SUMMER / NIGHT-W1 owner decision 20: corpse containers admit up to 30 entries.
-- Keep the original migration and all receipt, lifecycle, scope and single-location
-- proofs. The corpse receipt row is still locked before counting concurrent entries.
-- The separate generated loot-plan ceiling stays 16.
CREATE OR REPLACE FUNCTION game_item_corpse_container_entry_proven() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, public, pg_temp AS $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM game_item_mint_receipts r
         WHERE r.transaction_id = NEW.placed_transaction_id
           AND r.item_instance_id = NEW.item_instance_id
           AND r.destination_parent_item_instance_id = NEW.parent_item_instance_id
           AND r.destination_ordinal = NEW.placement_ordinal
           AND r.created_xact_id = pg_current_xact_id()
           AND EXISTS (SELECT 1 FROM game_item_instances i
                        WHERE i.item_instance_id = NEW.item_instance_id AND i.lifecycle = 1)) THEN
        RAISE EXCEPTION 'corpse container placement must commit with its loot MINT receipt'
            USING ERRCODE = '23514';
    END IF;
    -- An item is never both a corpse container entry and on Ground (the
    -- single-location invariant): refuse if a Ground row for it already
    -- exists (whichever insertion lands second within the transaction).
    IF EXISTS (
        SELECT 1 FROM game_item_ground_locations g
         WHERE g.item_instance_id = NEW.item_instance_id) THEN
        RAISE EXCEPTION 'corpse container entry must not also hold a Ground location'
            USING ERRCODE = '23514';
    END IF;
    -- Lock the corpse's own receipt row (its unique CORPSE_MATERIALIZATION /
    -- draw_ordinal = 0 cause, found by the unique item_instance_id) before
    -- counting, so concurrent loot MINTs into the same corpse serialize
    -- instead of racing the count.
    PERFORM 1 FROM game_item_mint_receipts r
     WHERE r.item_instance_id = NEW.parent_item_instance_id
       AND r.loot_purpose_key = 'CORPSE_MATERIALIZATION'
       AND r.draw_ordinal = 0
     FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'corpse container entry must parent a materialized corpse'
            USING ERRCODE = '23514';
    END IF;
    IF (SELECT count(*) FROM game_item_corpse_container_entries e
         WHERE e.parent_item_instance_id = NEW.parent_item_instance_id) > 30 THEN
        RAISE EXCEPTION 'corpse container entry ceiling exceeded' USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END;
$$;

-- Preserve the schema-local function resolution used by the migration runner.
DO $$ BEGIN
    EXECUTE format('ALTER FUNCTION game_item_corpse_container_entry_proven() SET search_path = %I, pg_temp', current_schema());
END $$;

ALTER TABLE game_item_decay_retire_reservations
    DROP CONSTRAINT game_item_decay_retire_reservations_placement_ordinal_check,
    ADD CONSTRAINT game_item_decay_retire_reservations_placement_ordinal_check
        CHECK (placement_ordinal BETWEEN 1 AND 30);
ALTER TABLE game_item_decay_retire_receipts
    DROP CONSTRAINT game_item_decay_retire_receipts_placement_ordinal_check,
    ADD CONSTRAINT game_item_decay_retire_receipts_placement_ordinal_check
        CHECK (placement_ordinal BETWEEN 1 AND 30);
