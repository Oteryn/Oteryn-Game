-- Field schedules created by the scoped spell Item writer retain their original
-- receipt generation across assignment replacement, just like corpse schedules.
-- Extend both existing custody predicates; their callers still prove the live
-- holder and the temporal writer still checks due time and receipt succession.
DO $migration$
DECLARE body TEXT; function_name TEXT; scope_kind INTEGER; scope_tag TEXT;
        scope_column TEXT; scope_parameter TEXT; proof TEXT;
BEGIN
 FOR scope_kind IN 1..2 LOOP
  IF scope_kind=1 THEN
   function_name:='game_corpse_stage_ground_current'; scope_tag:='01';
   scope_column:='channel_id'; scope_parameter:='p_channel';
  ELSE
   function_name:='game_instance_corpse_stage_current'; scope_tag:='03';
   scope_column:='instance_id'; scope_parameter:='p_instance';
  END IF;
  body:=pg_get_functiondef((function_name||'(uuid,uuid,uuid,numeric,bigint)')::regprocedure);
  IF position('game_spell_item_receipts source' IN body)>0 THEN CONTINUE; END IF;
  IF position('SELECT EXISTS(' IN body)=0 THEN
   RAISE EXCEPTION 'field schedule custody predicate changed: %',function_name;
  END IF;
  proof:=format($proof$SELECT EXISTS(SELECT 1 FROM game_item_ground_locations g
   JOIN game_spell_field_temporal_schedules schedule ON schedule.item_instance_id=g.item_instance_id AND schedule.source_stage=1
   JOIN game_spell_item_receipts source ON source.transaction_id=schedule.source_transaction_id
   JOIN game_spell_item_lines line ON line.transaction_id=source.transaction_id AND line.ordinal=schedule.source_ordinal
    AND line.item_instance_id=g.item_instance_id AND line.operation_kind IN(1,7)
   JOIN game_runtime_scope_assignments assignment
    ON assignment.scope_key=decode('%1$s','hex')||uuid_send(g.world_id)||uuid_send(g.%2$I)
   WHERE g.item_instance_id=p_item AND g.world_id=p_world AND g.scope_kind=%4$s
    AND g.%2$I=%3$I AND g.stack_ordinal=p_stack
    AND source.world_id=g.world_id AND source.runtime_scope_kind=g.scope_kind
    AND source.channel_id IS NOT DISTINCT FROM g.channel_id
    AND source.instance_id IS NOT DISTINCT FROM g.instance_id
    AND source.ownership_generation=g.runtime_scope_ownership_generation
    AND line.world_id=g.world_id AND line.runtime_scope_kind=g.scope_kind
    AND line.channel_id IS NOT DISTINCT FROM g.channel_id
    AND line.instance_id IS NOT DISTINCT FROM g.instance_id
    AND (line.operation_kind=1 OR line.source_stack_ordinal=g.stack_ordinal)
    AND assignment.state=1 AND assignment.ownership_generation=p_generation) OR EXISTS($proof$,
    scope_tag,scope_column,scope_parameter,scope_kind);
  EXECUTE replace(body,'SELECT EXISTS(',proof);
 END LOOP;
END $migration$;
