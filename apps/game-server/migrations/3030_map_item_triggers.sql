-- The original move from the room's retained parchment is the removal occurrence.
-- Re-arm that placement two hours after the move, leaving every other MINT one-shot.
CREATE TABLE game_parchment_removals (
    mint_transaction_id UUID PRIMARY KEY REFERENCES game_map_item_mint_receipts(transaction_id),
    move_transaction_id UUID NOT NULL UNIQUE REFERENCES game_item_loop_move_receipts(transaction_id),
    reset_at TIMESTAMPTZ NOT NULL
);
CREATE TRIGGER game_parchment_removal_immutable
    BEFORE UPDATE OR DELETE ON game_parchment_removals
    FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_parchment_removal_no_truncate
    BEFORE TRUNCATE ON game_parchment_removals
    EXECUTE FUNCTION game_item_reject_truncate();

ALTER TABLE game_map_item_mint_receipts ADD COLUMN parchment_previous UUID
    REFERENCES game_parchment_removals(mint_transaction_id)
    CHECK (parchment_previous IS NULL OR
           (placement_key=142006576221954 AND definition_production_key='oteryn:item.tibia.i2819'));
CREATE INDEX game_map_item_parchment_previous ON game_map_item_mint_receipts(parchment_previous)
    WHERE parchment_previous IS NOT NULL;
-- PostgreSQL truncates the generated name of the original five-column UNIQUE.
DO $migration$
DECLARE old_constraint TEXT;
BEGIN
    SELECT conname INTO STRICT old_constraint FROM pg_constraint
     WHERE conrelid='game_map_item_mint_receipts'::regclass AND contype='u'
       AND array_length(conkey,1)=5;
    EXECUTE format('ALTER TABLE game_map_item_mint_receipts DROP CONSTRAINT %I',old_constraint);
END $migration$;
CREATE UNIQUE INDEX game_map_item_mint_placement_cycle
    ON game_map_item_mint_receipts
    (world_id,channel_id,base_bundle_digest,reset_epoch,placement_key,parchment_previous)
    NULLS NOT DISTINCT;
CREATE UNIQUE INDEX game_parchment_mint_latest
    ON game_map_item_mint_receipts
    (world_id,channel_id,base_bundle_digest,reset_epoch,committed_at DESC,transaction_id DESC)
    WHERE placement_key=142006576221954 AND definition_production_key='oteryn:item.tibia.i2819';

CREATE FUNCTION game_parchment_mint_cycle() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE previous UUID; deadline TIMESTAMPTZ;
BEGIN
    NEW.parchment_previous := NULL;
    -- Native (33063,31624,-15), top-level ordinal 2, retains the parchment inscription.
    IF NEW.placement_key=142006576221954
        AND NEW.definition_production_key='oteryn:item.tibia.i2819' THEN
        SELECT m.transaction_id, r.reset_at INTO previous, deadline
          FROM game_map_item_mint_receipts m
          LEFT JOIN game_parchment_removals r ON r.mint_transaction_id=m.transaction_id
         WHERE (m.world_id,m.channel_id,m.base_bundle_digest,m.reset_epoch,m.placement_key)
             =(NEW.world_id,NEW.channel_id,NEW.base_bundle_digest,NEW.reset_epoch,NEW.placement_key)
         ORDER BY m.committed_at DESC,m.transaction_id DESC LIMIT 1;
        IF previous IS NOT NULL AND (deadline IS NULL OR deadline>statement_timestamp()) THEN
            RAISE EXCEPTION 'parchment seal has not reset' USING ERRCODE='23514';
        END IF;
        NEW.parchment_previous := previous;
    END IF;
    RETURN NEW;
END $$;
CREATE TRIGGER game_parchment_mint_cycle BEFORE INSERT ON game_map_item_mint_receipts
    FOR EACH ROW WHEN (NEW.placement_key=142006576221954 AND NEW.definition_production_key='oteryn:item.tibia.i2819')
    EXECUTE FUNCTION game_parchment_mint_cycle();

CREATE FUNCTION game_parchment_removal() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
    IF NEW.from_kind=3 AND NEW.definition_family='Item'
       AND NEW.definition_production_key='oteryn:item.tibia.i2819' THEN
        INSERT INTO game_parchment_removals(mint_transaction_id,move_transaction_id,reset_at)
        SELECT m.transaction_id,r.transaction_id,statement_timestamp() + interval '2 hours'
        FROM game_map_item_mint_receipts m
        JOIN game_item_loop_move_receipts r ON r.transaction_id=NEW.transaction_id
        JOIN game_item_ground_locations g ON g.item_instance_id=m.item_instance_id
        WHERE m.item_instance_id=NEW.item_instance_id AND m.placement_key=142006576221954
          AND m.definition_production_key=NEW.definition_production_key
          AND (m.world_id,m.channel_id,m.spatial_position,m.map_revision)
              =(g.world_id,g.channel_id,g.spatial_position,g.map_revision)
          AND (r.world_id,r.channel_id,r.scope_generation)
              =(g.world_id,g.channel_id,g.runtime_scope_ownership_generation)
          AND r.created_xact_id=pg_current_xact_id()
          AND m.content_revision='sha256:'||encode(r.content_digest,'hex')
          AND game_world_reset_map_epoch_current(m.world_id,m.reset_epoch)
          AND NEW.to_spatial_position IS DISTINCT FROM m.spatial_position
        ON CONFLICT (mint_transaction_id) DO NOTHING;
    END IF;
    RETURN NULL;
END $$;
CREATE TRIGGER game_parchment_removal AFTER INSERT ON game_item_loop_move_lines
    FOR EACH ROW WHEN (NEW.from_kind=3 AND NEW.definition_production_key='oteryn:item.tibia.i2819')
    EXECUTE FUNCTION game_parchment_removal();

DO $migration$
BEGIN
    EXECUTE format('ALTER FUNCTION game_parchment_mint_cycle() SET search_path=pg_catalog,%I,pg_temp',current_schema());
    EXECUTE format('ALTER FUNCTION game_parchment_removal() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $migration$;
REVOKE ALL ON game_parchment_removals FROM PUBLIC;
REVOKE ALL ON FUNCTION game_parchment_mint_cycle(),game_parchment_removal() FROM PUBLIC;
GRANT SELECT ON game_parchment_removals TO oteryn_game_runtime;
GRANT SELECT ON game_parchment_removals TO oteryn_game_control;
