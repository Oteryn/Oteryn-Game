-- Global Dead Rat: the three visible decay stages move whole on the floor.
-- The immovable death appearance and ordinary timed Items stay excluded.
CREATE OR REPLACE FUNCTION game_item_loop_native_corpse_source(p_item UUID) RETURNS boolean
LANGUAGE sql STABLE SECURITY DEFINER AS $$
 SELECT EXISTS(SELECT 1 FROM game_item_instances i
   JOIN game_item_mint_receipts m ON m.item_instance_id=i.item_instance_id AND m.transaction_id=i.minted_transaction_id
   JOIN game_item_ground_locations g ON g.item_instance_id=i.item_instance_id
   WHERE i.item_instance_id=p_item AND i.lifecycle=1 AND i.quantity=1 AND i.definition_family='Item'
     AND i.definition_revision_ref='definition-r1'
     AND m.loot_purpose_key='CORPSE_MATERIALIZATION' AND m.draw_ordinal=0
     AND i.world_id=m.death_world_id AND g.world_id=m.death_world_id
     AND g.scope_kind=1 AND g.channel_id=m.death_channel_id
     AND (i.definition_production_key IN('oteryn:item.tibia.i8139','oteryn:item.tibia.i8140','oteryn:item.tibia.i8141','oteryn:item.tibia.i8106')
       OR (i.definition_production_key IN('oteryn:item.tibia.i3994','oteryn:item.tibia.i3995','oteryn:item.tibia.i3996')
         AND m.death_scope_kind=1 AND m.materialized_at IS NOT NULL AND EXISTS(
           SELECT 1 FROM game_spell_field_temporal_schedules root
            JOIN game_spell_field_temporal_schedules stage
              ON stage.item_instance_id=root.item_instance_id
             AND stage.source_transaction_id=root.source_transaction_id
             AND stage.source_ordinal=root.source_ordinal
           WHERE root.item_instance_id=i.item_instance_id AND root.source_transaction_id=m.transaction_id
             AND root.source_stage=1 AND root.source_ordinal=0
             AND root.definition_family='Item' AND root.definition_production_key='oteryn:item.tibia.i5964'
             AND root.definition_revision='definition-r1'
             AND stage.source_stage BETWEEN 2 AND 4
             AND (stage.definition_family,stage.definition_production_key,stage.definition_revision)
                 =(i.definition_family,i.definition_production_key,i.definition_revision_ref)))))
$$;
DO $migration$
DECLARE
 body TEXT := pg_get_functiondef('game_item_loop_line_admit()'::regprocedure);
 anchor CONSTANT TEXT := 'AND game_item_general_decay_chain(NEW.item_instance_id))';
 rat_guard CONSTANT TEXT := $arm$
      OR (NEW.from_kind=3 AND NEW.to_kind=3 AND NEW.quantity_before=1 AND NEW.quantity_after=1
          AND NEW.definition_production_key IN('oteryn:item.tibia.i3994','oteryn:item.tibia.i3995','oteryn:item.tibia.i3996')
          AND game_item_loop_native_corpse_source(NEW.item_instance_id))$arm$;
BEGIN
 IF (length(body)-length(replace(body,anchor,'')))/length(anchor)<>1 THEN
  RAISE EXCEPTION 'missing exact timed-item move guard';
 END IF;
 EXECUTE format('ALTER FUNCTION game_item_loop_native_corpse_source(uuid) SET search_path=pg_catalog,%I,pg_temp',current_schema());
 IF strpos(body,anchor||rat_guard)=0 THEN
  EXECUTE replace(body,anchor,anchor||rat_guard);
 END IF;
END $migration$;
