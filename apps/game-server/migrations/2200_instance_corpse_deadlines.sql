-- M29: source-qualified corpse stages use the existing Item deadline owner.
-- Crystal 201189f data/items/items.xml:73328-73347: 39530 -> 39529 (10s),
-- ->39528 (300s), ->39527 (300s), then the corpse owner retires after 60s.
-- Keep every installed Summer/general-decay policy and proof unchanged.
DO $$ DECLARE body TEXT; needle TEXT; BEGIN
 body:=pg_get_functiondef('game_summer_corpse_chain(text)'::regprocedure);
 needle:='SELECT CASE p_key';
 IF position(needle IN body)=0 THEN RAISE EXCEPTION 'corpse chain dispatcher changed'; END IF;
 EXECUTE replace(body,needle,needle||E'\n WHEN ''oteryn:item.tibia.i39530'' THEN ARRAY[39530,39529,39528,39527]');
END $$;

ALTER TABLE game_spell_field_temporal_receipts
 ALTER COLUMN channel_id DROP NOT NULL,
 ADD COLUMN instance_id UUID,
 ADD CONSTRAINT game_field_timer_exact_scope CHECK (
  (channel_id IS NOT NULL AND instance_id IS NULL)
  OR (channel_id IS NULL AND instance_id IS NOT NULL AND custody_kind=1
      AND game_character_is_uuid_v7(instance_id)));

-- Original Ground generation is immutable death provenance, not current authority.
-- A resumed Instance may catch up only under its independently current assignment.
CREATE FUNCTION game_instance_corpse_stage_current(
 p_item UUID,p_world UUID,p_instance UUID,p_generation NUMERIC,p_stack BIGINT)
RETURNS BOOLEAN LANGUAGE sql STABLE STRICT AS $$
 SELECT EXISTS(SELECT 1 FROM game_item_ground_locations g
  JOIN game_item_mint_receipts m USING(item_instance_id)
  JOIN game_spell_field_temporal_schedules p ON p.item_instance_id=m.item_instance_id
   AND p.source_stage=1 AND p.source_ordinal=0 AND p.source_transaction_id=m.transaction_id
  JOIN game_runtime_scope_assignments a ON a.scope_key=m.death_scope_key
  WHERE g.item_instance_id=p_item AND g.world_id=p_world AND g.scope_kind=2
   AND g.channel_id IS NULL AND g.instance_id=p_instance AND g.stack_ordinal=p_stack
   AND m.death_scope_kind=2 AND m.death_world_id=p_world AND m.death_instance_id=p_instance
   AND m.death_scope_ownership_generation=g.runtime_scope_ownership_generation
   AND m.loot_purpose_key='CORPSE_MATERIALIZATION' AND m.draw_ordinal=0
   AND a.state=1 AND a.ownership_generation=p_generation)
$$;

-- Extend exact custody in both predecessor and deferred successor guards. All
-- source, due-clock, same-XID, previous-stage, current-assignment and audit proofs
-- remain in place; an Instance cannot borrow a Channel's same-UUID custody.
DO $$ DECLARE body TEXT; f TEXT; row_name TEXT; needle TEXT; BEGIN
 FOREACH f IN ARRAY ARRAY['game_spell_field_temporal_capture','game_spell_field_temporal_receipt_proven'] LOOP
  row_name:=CASE WHEN f='game_spell_field_temporal_capture' THEN 'r' ELSE 'NEW' END;
  body:=pg_get_functiondef((f||'()')::regprocedure);
  needle:='g.channel_id='||row_name||'.channel_id';
  IF position(needle IN body)=0 THEN RAISE EXCEPTION 'field timer custody changed: %',f; END IF;
  body:=replace(body,needle,format('((%1$s.instance_id IS NULL AND %2$s) OR game_instance_corpse_stage_current(g.item_instance_id,%1$s.world_id,%1$s.instance_id,%1$s.ownership_generation,%1$s.source_stack_ordinal))',row_name,needle));
  needle:='g.runtime_scope_ownership_generation='||row_name||'.ownership_generation';
  IF position(needle IN body)=0 THEN RAISE EXCEPTION 'field timer generation changed: %',f; END IF;
  EXECUTE replace(body,needle,format('(%2$s OR game_instance_corpse_stage_current(g.item_instance_id,%1$s.world_id,%1$s.instance_id,%1$s.ownership_generation,%1$s.source_stack_ordinal))',row_name,needle));
 END LOOP;
 EXECUTE format('ALTER FUNCTION game_instance_corpse_stage_current(UUID,UUID,UUID,NUMERIC,BIGINT) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_instance_corpse_stage_current(UUID,UUID,UUID,NUMERIC,BIGINT) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_instance_corpse_stage_current(UUID,UUID,UUID,NUMERIC,BIGINT) TO oteryn_game_runtime,oteryn_game_control;
