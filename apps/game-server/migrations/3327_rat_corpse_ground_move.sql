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
             AND stage.source_stage BETWEEN 1 AND 3
             AND (stage.target_family,stage.target_production_key,stage.target_revision)
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

-- The current Channel assignment independently fences the reader. Its
-- predecessor's exact whole-item move remains valid Ground provenance.
CREATE OR REPLACE FUNCTION game_dragged_rat_ground_current(
 p_item UUID,p_world UUID,p_channel UUID,p_generation NUMERIC,p_stack BIGINT
) RETURNS BOOLEAN LANGUAGE sql STABLE STRICT SECURITY DEFINER AS $$
 SELECT EXISTS (
  SELECT 1 FROM game_item_ground_locations g
  JOIN game_item_instances i ON i.item_instance_id=g.item_instance_id
  JOIN game_runtime_scope_assignments a
    ON a.scope_key=decode('01','hex')||uuid_send(g.world_id)||uuid_send(g.channel_id)
  JOIN game_item_loop_move_lines l ON l.item_instance_id=g.item_instance_id
  JOIN game_item_loop_move_receipts move USING(transaction_id)
  WHERE g.item_instance_id=p_item AND g.world_id=p_world AND g.channel_id=p_channel
    AND g.scope_kind=1 AND g.stack_ordinal=p_stack
    AND g.runtime_scope_ownership_generation>0 AND g.runtime_scope_ownership_generation<p_generation
    AND a.state=1 AND a.ownership_generation=p_generation
    AND game_item_loop_native_corpse_source(p_item)
    AND i.definition_production_key IN('oteryn:item.tibia.i3994','oteryn:item.tibia.i3995','oteryn:item.tibia.i3996')
    AND i.state_revision>l.state_revision_before AND l.created_xact_id=move.created_xact_id
    AND l.definition_production_key IN('oteryn:item.tibia.i3994','oteryn:item.tibia.i3995','oteryn:item.tibia.i3996')
    AND l.from_kind=3 AND l.to_kind=3 AND l.quantity_before=1 AND l.quantity_after=1
    AND l.to_spatial_position=g.spatial_position
    AND g.corpse_ref=convert_to('OTERYN_ITEM_LOOP_DROP/v1:','UTF8')||uuid_send(move.transaction_id)
    AND move.scope_kind=1 AND move.world_id=g.world_id AND move.channel_id=g.channel_id
    AND move.scope_generation=g.runtime_scope_ownership_generation
 )
$$;
REVOKE ALL ON FUNCTION game_dragged_rat_ground_current(uuid,uuid,uuid,numeric,bigint) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_dragged_rat_ground_current(uuid,uuid,uuid,numeric,bigint)
 TO oteryn_game_runtime,oteryn_game_control;

DO $migration$
DECLARE
 name TEXT;
 body TEXT;
 anchor TEXT;
 proof TEXT;
 updated TEXT;
BEGIN
 EXECUTE format('ALTER FUNCTION game_dragged_rat_ground_current(uuid,uuid,uuid,numeric,bigint) SET search_path=pg_catalog,%I,pg_temp',current_schema());
 FOR name, anchor, proof IN VALUES
  ('game_corpse_stage_ground_current(uuid,uuid,uuid,numeric,bigint)',
   'm.death_scope_ownership_generation=g.runtime_scope_ownership_generation',
   'game_dragged_rat_ground_current(p_item,p_world,p_channel,p_generation,p_stack)'),
  ('game_item_loop_line_admit()',
   'cg.runtime_scope_ownership_generation=r.scope_generation',
   'game_dragged_rat_ground_current(cg.item_instance_id,r.world_id,r.channel_id,r.scope_generation,cg.stack_ordinal)'),
  ('game_spell_item_capture_custody()',
   'g.runtime_scope_ownership_generation=r.ownership_generation',
   'game_dragged_rat_ground_current(g.item_instance_id,r.world_id,r.channel_id,r.ownership_generation,g.stack_ordinal)')
 LOOP
  body:=pg_get_functiondef(name::regprocedure);
  updated:='('||anchor||' OR '||proof||')';
  IF (length(body)-length(replace(body,anchor,'')))/length(anchor)<>1 THEN
   RAISE EXCEPTION 'missing exact corpse custody guard in %',name;
  END IF;
  IF strpos(body,updated)=0 THEN
   EXECUTE replace(body,anchor,updated);
  END IF;
 END LOOP;
END $migration$;

-- The existing handoff trigger proves the latest custody-only revision and
-- carries forward the absolute terminal stage receipt through repeated moves.
DO $migration$
DECLARE
 body TEXT;
 name TEXT;
 live CONSTANT TEXT := 'i.state_revision=r.state_revision_before+1 AND i.last_transaction_id=r.transaction_id';
 retired CONSTANT TEXT := 'i.state_revision=r.state_revision_before+2 AND i.last_transaction_id=NEW.transaction_id';
 live_guard TEXT;
 retired_guard TEXT;
 handoff CONSTANT TEXT := $proof$
   i.definition_production_key='oteryn:item.tibia.i3996'
   AND EXISTS(SELECT 1 FROM game_item_decay_handoffs h
       WHERE h.item_instance_id=i.item_instance_id AND h.source_stage=r.source_stage
         AND h.stage_transaction_id=r.transaction_id AND
$proof$;
BEGIN
 live_guard:='('||live||' OR ('||handoff||
    'h.result_revision=i.state_revision AND h.custody_transaction_id=i.last_transaction_id)))';
 retired_guard:='('||retired||' OR ('||handoff||
    'h.result_revision=i.state_revision-1 AND i.last_transaction_id=NEW.transaction_id)))';
 FOREACH name IN ARRAY ARRAY['game_corpse_terminal_stage_ready(uuid)','game_item_decay_retire_consistency_guard()'] LOOP
  body:=pg_get_functiondef(name::regprocedure);
  IF (length(body)-length(replace(body,live,'')))/length(live)<>1 THEN
   RAISE EXCEPTION 'missing live terminal stage guard in %',name;
  END IF;
  IF strpos(body,live_guard)=0 THEN
   body:=replace(body,live,live_guard);
  END IF;
  IF name='game_item_decay_retire_consistency_guard()' THEN
   IF (length(body)-length(replace(body,retired,'')))/length(retired)<>1 THEN
    RAISE EXCEPTION 'missing retired terminal stage guard';
   END IF;
   IF strpos(body,retired_guard)=0 THEN
    body:=replace(body,retired,retired_guard);
   END IF;
  END IF;
  EXECUTE body;
 END LOOP;
END $migration$;
