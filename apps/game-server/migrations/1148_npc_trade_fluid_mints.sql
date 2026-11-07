-- Explicit filled-container purchases bind one physical mint and its fluid subtype.
ALTER TABLE game_npc_trades ADD COLUMN fluid_subtype SMALLINT
    CHECK (fluid_subtype BETWEEN 1 AND 255 AND side=1 AND quantity=1 AND item_units=1);
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
        SELECT 1 FROM game_npc_trades t
        JOIN game_item_instances i ON i.item_instance_id=t.item_instance_id
          AND i.world_id=t.world_id AND i.minted_transaction_id=t.transaction_id
        JOIN game_npc_trade_lines l ON l.transaction_id=t.transaction_id
          AND l.item_instance_id=t.item_instance_id
        WHERE t.item_instance_id=v_item AND t.transaction_id=v_transaction
          AND t.world_id=NEW.world_id AND t.fluid_subtype=v_after
          AND t.side=1 AND t.quantity=1 AND t.item_units=1
          AND i.quantity=1 AND i.definition_family=t.item_definition_family
          AND i.definition_production_key=t.item_definition_production_key
          AND i.definition_revision_ref=t.item_definition_revision_ref
          AND l.line_kind=4 AND l.quantity_before=0 AND l.quantity_after=1
          AND t.created_xact_id = pg_current_xact_id()
    ) THEN RETURN NEW; END IF;
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
