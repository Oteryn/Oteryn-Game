-- The scope owner projects this durable intent into its local creature runtime.
CREATE TABLE game_parchment_wave_intents (
    move_transaction_id UUID PRIMARY KEY REFERENCES game_parchment_removals(move_transaction_id)
);
INSERT INTO game_parchment_wave_intents(move_transaction_id)
SELECT move_transaction_id FROM game_parchment_removals WHERE reset_at>statement_timestamp();
CREATE TRIGGER game_parchment_wave_intent_immutable
    BEFORE UPDATE OR DELETE ON game_parchment_wave_intents
    FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_parchment_wave_intent_no_truncate
    BEFORE TRUNCATE ON game_parchment_wave_intents
    EXECUTE FUNCTION game_item_reject_truncate();
REVOKE ALL ON game_parchment_wave_intents FROM PUBLIC;
GRANT SELECT ON game_parchment_wave_intents TO oteryn_game_runtime,oteryn_game_control;

CREATE OR REPLACE FUNCTION game_parchment_removal() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
    IF NEW.from_kind=3 AND NEW.definition_family='Item'
       AND NEW.definition_production_key='oteryn:item.tibia.i2819' THEN
        WITH removed AS (
            INSERT INTO game_parchment_removals(mint_transaction_id,move_transaction_id,reset_at)
            SELECT m.transaction_id,r.transaction_id,statement_timestamp() + interval '2 hours'
            FROM game_map_item_mint_receipts m
            JOIN game_item_loop_move_receipts r ON r.transaction_id=NEW.transaction_id
            JOIN game_item_ground_locations g ON g.item_instance_id=m.item_instance_id
            JOIN game_item_instances i ON i.item_instance_id=m.item_instance_id
            JOIN game_runtime_scope_assignments a
              ON a.scope_key='\x01'::bytea||uuid_send(r.world_id)||uuid_send(r.channel_id)
            WHERE m.item_instance_id=NEW.item_instance_id AND m.placement_key=142006576221954
              AND (i.definition_family,i.definition_production_key,i.definition_revision_ref)
                  =(m.definition_family,m.definition_production_key,m.definition_revision_ref)
              AND (i.definition_family,i.definition_production_key,i.definition_revision_ref)
                  =(NEW.definition_family,NEW.definition_production_key,NEW.definition_revision_ref)
              AND i.minted_transaction_id=m.transaction_id AND i.last_transaction_id IS NULL
              AND i.lifecycle=1 AND i.quantity=m.quantity AND i.quantity=NEW.quantity_before
              AND i.state_revision=NEW.state_revision_before
              AND (m.world_id,m.channel_id,m.spatial_position,m.map_revision,m.content_revision)
                  =(g.world_id,g.channel_id,g.spatial_position,g.map_revision,g.content_revision)
              AND (r.world_id,r.channel_id)=(g.world_id,g.channel_id)
              AND g.corpse_ref=convert_to('map-item-materialization-v1','UTF8')
              AND m.runtime_scope_ownership_generation<=g.runtime_scope_ownership_generation
              AND g.runtime_scope_ownership_generation<=r.scope_generation
              AND a.state=1 AND a.ownership_generation=r.scope_generation
              AND r.created_xact_id=pg_current_xact_id()
              AND NEW.created_xact_id=pg_current_xact_id()
              AND m.content_revision='sha256:'||encode(r.content_digest,'hex')
              AND game_world_reset_map_epoch_current(m.world_id,m.reset_epoch)
              AND NEW.to_spatial_position IS DISTINCT FROM m.spatial_position
            ON CONFLICT (mint_transaction_id) DO NOTHING
            RETURNING move_transaction_id
        )
        INSERT INTO game_parchment_wave_intents(move_transaction_id)
        SELECT move_transaction_id FROM removed;
    END IF;
    RETURN NULL;
END $$;
DO $migration$
BEGIN
    EXECUTE format('ALTER FUNCTION game_parchment_removal() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $migration$;
REVOKE ALL ON FUNCTION game_parchment_removal() FROM PUBLIC;
