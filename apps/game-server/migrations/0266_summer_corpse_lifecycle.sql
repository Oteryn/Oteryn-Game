-- SUMMER-1: reuse the existing durable Item chain owner for corpse transforms,
-- and the corpse owner for N+1 terminal retirement. No new timer or item store.
-- Crystal 00ce02a5 timings/edges: not verified against Global. Capacity: decision 20.
CREATE FUNCTION game_summer_corpse_chain(p_key TEXT) RETURNS INTEGER[]
LANGUAGE sql IMMUTABLE STRICT AS $$
 SELECT CASE p_key
  WHEN 'oteryn:item.tibia.i54678' THEN ARRAY[54678,54679,54680,54681]
  WHEN 'oteryn:item.tibia.i54407' THEN ARRAY[54407,54406,54405,54404]
  WHEN 'oteryn:item.tibia.i54399' THEN ARRAY[54399,54398,54397,54396]
  WHEN 'oteryn:item.tibia.i54415' THEN ARRAY[54415,54414,54413,54412]
 END
$$;

-- The original same-XID spell mint proof remains below. Corpse mint is a
-- second original-source provenance accepted by that same immutable schedule.
DO $$ DECLARE c TEXT; BEGIN
 FOR c IN SELECT conname FROM pg_constraint WHERE conrelid='game_spell_field_temporal_schedules'::regclass
  AND confrelid='game_spell_item_lines'::regclass LOOP
  EXECUTE format('ALTER TABLE game_spell_field_temporal_schedules DROP CONSTRAINT %I',c);
 END LOOP;
END $$;
CREATE OR REPLACE FUNCTION game_spell_field_temporal_schedule_proven() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE intent JSONB; operation JSONB; stage JSONB; elapsed BIGINT; corpse RECORD; chain INTEGER[];
BEGIN
 SELECT m.*,i.definition_family,i.definition_production_key,i.definition_revision_ref,g.content_revision INTO corpse FROM game_item_mint_receipts m JOIN game_item_instances i USING(item_instance_id) JOIN game_item_ground_locations g USING(item_instance_id) WHERE m.transaction_id=NEW.source_transaction_id
  AND m.item_instance_id=NEW.item_instance_id AND loot_purpose_key='CORPSE_MATERIALIZATION' AND draw_ordinal=0;
 IF FOUND THEN
  chain:=game_summer_corpse_chain(corpse.definition_production_key);
  IF chain IS NULL OR corpse.definition_family<>'Item' OR corpse.definition_revision_ref<>'definition-r1'
   OR corpse.created_xact_id<>pg_current_xact_id() OR NEW.created_xact_id<>pg_current_xact_id()
   OR NEW.source_ordinal<>0 OR NEW.source_stage NOT BETWEEN 1 AND 3
   OR NEW.definition_family<>'Item' OR NEW.definition_revision<>'definition-r1'
   OR NEW.definition_production_key<>'oteryn:item.tibia.i'||chain[NEW.source_stage]::text
   OR NEW.target_family IS DISTINCT FROM 'Item' OR NEW.target_revision IS DISTINCT FROM 'definition-r1'
   OR NEW.target_production_key IS DISTINCT FROM 'oteryn:item.tibia.i'||chain[NEW.source_stage+1]::text
   OR NEW.duration_millis<>(CASE WHEN NEW.source_stage=1 THEN 10000 ELSE 300000 END)
   OR NEW.expires_at_unix_ms<>corpse.materialized_at+(CASE NEW.source_stage WHEN 1 THEN 10000 WHEN 2 THEN 310000 ELSE 610000 END)
   OR NEW.content_digest<>sha256(convert_to(corpse.content_revision,'UTF8'))
   OR NEW.blocks_movement OR NEW.blocks_projectile OR NEW.immovable_block_solid THEN
   RAISE EXCEPTION 'corpse stage requires exact original mint provenance and policy' USING ERRCODE='23514';
  END IF;
  RETURN NULL;
 END IF;
 SELECT convert_from(r.intent,'UTF8')::jsonb INTO intent FROM game_spell_item_receipts r JOIN game_spell_item_lines l USING(transaction_id)
 WHERE r.transaction_id=NEW.source_transaction_id AND l.ordinal=NEW.source_ordinal AND l.item_instance_id=NEW.item_instance_id
   AND l.operation_kind=1 AND l.quantity_after=1 AND l.expires_at_unix_ms IS NULL
   AND l.content_generation_digest=NEW.content_digest AND NEW.created_xact_id=pg_current_xact_id()
   AND r.created_xact_id=NEW.created_xact_id AND l.created_xact_id=NEW.created_xact_id;
 IF NOT FOUND THEN RAISE EXCEPTION 'field chain requires original same-XID mint' USING ERRCODE='23514'; END IF;
 operation:=intent->'operations'->(NEW.source_ordinal-1);
 IF operation->>'kind'<>'mint_ground' OR jsonb_typeof(operation->'decay_chain')<>'array'
  OR jsonb_array_length(operation->'decay_chain') NOT BETWEEN 1 AND 32 THEN
  RAISE EXCEPTION 'field chain requires bounded complete source policy' USING ERRCODE='23514'; END IF;
 stage:=operation->'decay_chain'->(NEW.source_stage-1);
 IF stage IS DISTINCT FROM jsonb_build_object('definition',jsonb_build_object('family',NEW.definition_family,'key',NEW.definition_production_key,'revision',NEW.definition_revision),
   'duration_millis',NEW.duration_millis,'target',CASE WHEN NEW.target_family IS NULL THEN NULL ELSE jsonb_build_object('family',NEW.target_family,'key',NEW.target_production_key,'revision',NEW.target_revision) END,
   'blocks_movement',NEW.blocks_movement,'blocks_projectile',NEW.blocks_projectile,'immovable_block_solid',NEW.immovable_block_solid) THEN
  RAISE EXCEPTION 'field stage differs from original source' USING ERRCODE='23514'; END IF;
 SELECT sum((value->>'duration_millis')::bigint) INTO elapsed FROM jsonb_array_elements(operation->'decay_chain') WITH ORDINALITY t(value,ordinal) WHERE ordinal<=NEW.source_stage;
 IF NEW.expires_at_unix_ms<>(SELECT occurred_at_unix_ms FROM game_spell_item_receipts WHERE transaction_id=NEW.source_transaction_id)+elapsed
  OR (NEW.source_stage=1 AND stage->'definition' IS DISTINCT FROM operation->'definition')
  OR (NEW.source_stage>1 AND stage->'definition' IS DISTINCT FROM operation->'decay_chain'->(NEW.source_stage-2)->'target')
  OR (NEW.target_family IS NOT NULL AND NEW.source_stage<jsonb_array_length(operation->'decay_chain') AND stage->'target' IS DISTINCT FROM operation->'decay_chain'->NEW.source_stage->'definition')
  OR (NEW.target_family IS NULL AND NEW.source_stage<>jsonb_array_length(operation->'decay_chain')) THEN
  RAISE EXCEPTION 'field chain discontinuity/deadline mismatch' USING ERRCODE='23514'; END IF;
 RETURN NULL;
END $$;

CREATE FUNCTION game_summer_corpse_schedule() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE chain INTEGER[]; stage INTEGER; item game_item_instances%ROWTYPE; content TEXT;
BEGIN
 IF NEW.loot_purpose_key<>'CORPSE_MATERIALIZATION' OR NEW.draw_ordinal<>0 THEN RETURN NULL; END IF;
 SELECT * INTO STRICT item FROM game_item_instances WHERE item_instance_id=NEW.item_instance_id;
 SELECT content_revision INTO STRICT content FROM game_item_ground_locations WHERE item_instance_id=NEW.item_instance_id;
 chain:=game_summer_corpse_chain(item.definition_production_key);
 IF chain IS NULL THEN RETURN NULL; END IF;
 IF item.definition_family<>'Item' OR item.definition_revision_ref<>'definition-r1' OR NEW.materialized_at IS NULL THEN
  RAISE EXCEPTION 'unsupported Summer corpse definition' USING ERRCODE='23514'; END IF;
 FOR stage IN 1..3 LOOP
  INSERT INTO game_spell_field_temporal_schedules(item_instance_id,source_stage,source_transaction_id,source_ordinal,
   duration_millis,expires_at_unix_ms,definition_family,definition_production_key,definition_revision,
   target_family,target_production_key,target_revision,content_digest,blocks_movement,blocks_projectile,immovable_block_solid)
  VALUES(NEW.item_instance_id,stage,NEW.transaction_id,0,CASE WHEN stage=1 THEN 10000 ELSE 300000 END,
   NEW.materialized_at+CASE stage WHEN 1 THEN 10000 WHEN 2 THEN 310000 ELSE 610000 END,
   'Item','oteryn:item.tibia.i'||chain[stage]::text,'definition-r1',
   'Item','oteryn:item.tibia.i'||chain[stage+1]::text,'definition-r1',sha256(convert_to(content,'UTF8')),false,false,false);
 END LOOP;
 RETURN NULL;
END $$;
CREATE TRIGGER game_summer_corpse_schedule AFTER UPDATE OF materialized_at ON game_item_mint_receipts
 FOR EACH ROW WHEN (OLD.materialized_at IS NULL AND NEW.materialized_at IS NOT NULL)
 EXECUTE FUNCTION game_summer_corpse_schedule();

CREATE FUNCTION game_corpse_retirement_deadline(p_item UUID) RETURNS BIGINT
LANGUAGE sql STABLE STRICT AS $$
 SELECT materialized_at+CASE WHEN NOT EXISTS(SELECT 1 FROM game_spell_field_temporal_schedules p WHERE p.item_instance_id=p_item AND p.source_ordinal=0 AND p.source_stage=1) THEN 60000 ELSE 670000 END
 FROM game_item_mint_receipts WHERE item_instance_id=p_item AND loot_purpose_key='CORPSE_MATERIALIZATION' AND draw_ordinal=0
$$;
-- A recovered corpse retains its original Ground provenance. Current scope
-- authority is resolved independently; never rewrite that original generation.
CREATE FUNCTION game_corpse_stage_ground_current(p_item UUID,p_world UUID,p_channel UUID,p_generation NUMERIC,p_stack BIGINT)
RETURNS BOOLEAN LANGUAGE sql STABLE STRICT AS $$
 SELECT EXISTS(SELECT 1 FROM game_item_ground_locations g
  JOIN game_item_mint_receipts m USING(item_instance_id)
  JOIN game_spell_field_temporal_schedules p ON p.item_instance_id=m.item_instance_id AND p.source_stage=1 AND p.source_ordinal=0
  JOIN game_runtime_scope_assignments a ON a.world_id=g.world_id AND a.channel_id=g.channel_id
  WHERE g.item_instance_id=p_item AND g.world_id=p_world AND g.channel_id=p_channel AND g.stack_ordinal=p_stack
   AND m.loot_purpose_key='CORPSE_MATERIALIZATION' AND m.draw_ordinal=0 AND p.source_transaction_id=m.transaction_id
   AND m.death_world_id=g.world_id AND m.death_channel_id=g.channel_id
   AND m.death_scope_ownership_generation=g.runtime_scope_ownership_generation
   AND a.state=1 AND a.ownership_generation=p_generation)
$$;
-- Preserve both installed exact-custody guards; add only this qualified corpse
-- provenance path. The Rust writer still proves the live node and recovery seal.
DO $$ DECLARE body TEXT; f TEXT; old TEXT; replacement TEXT; BEGIN
 FOREACH f IN ARRAY ARRAY['game_spell_field_temporal_capture','game_spell_field_temporal_receipt_proven'] LOOP
  SELECT pg_get_functiondef(to_regprocedure(f||'()')) INTO STRICT body;
  IF f='game_spell_field_temporal_capture' THEN
   old:='g.runtime_scope_ownership_generation=r.ownership_generation';
   replacement:='(g.runtime_scope_ownership_generation=r.ownership_generation OR game_corpse_stage_ground_current(g.item_instance_id,r.world_id,r.channel_id,r.ownership_generation,r.source_stack_ordinal))';
  ELSE
   old:='g.runtime_scope_ownership_generation=NEW.ownership_generation';
   replacement:='(g.runtime_scope_ownership_generation=NEW.ownership_generation OR game_corpse_stage_ground_current(g.item_instance_id,NEW.world_id,NEW.channel_id,NEW.ownership_generation,NEW.source_stack_ordinal))';
  END IF;
  IF position(old IN body)=0 THEN RAISE EXCEPTION 'Item chain custody guard changed; review migration'; END IF;
  EXECUTE replace(body,old,replacement);
 END LOOP;
END $$;
-- Admission must not reserve an overdue terminal candidate before the Item
-- timer catches up: that candidate would freeze the first-stage definition.
CREATE FUNCTION game_corpse_terminal_stage_ready(p_item UUID) RETURNS BOOLEAN
LANGUAGE sql STABLE STRICT AS $$
 SELECT NOT EXISTS(SELECT 1 FROM game_spell_field_temporal_schedules p
    WHERE p.item_instance_id=p_item AND p.source_ordinal=0 AND p.source_stage=1)
  OR EXISTS(SELECT 1 FROM game_spell_field_temporal_receipts r
   JOIN game_spell_field_temporal_schedules p ON p.item_instance_id=r.item_instance_id AND p.source_stage=r.source_stage
   JOIN game_item_instances i ON i.item_instance_id=r.item_instance_id
   WHERE r.item_instance_id=p_item AND r.source_stage=3 AND p.source_ordinal=0
    AND i.lifecycle=1 AND i.quantity=1 AND i.state_revision=r.state_revision_before+1 AND i.last_transaction_id=r.transaction_id
    AND (i.definition_family,i.definition_production_key,i.definition_revision_ref)=(p.target_family,p.target_production_key,p.target_revision))
$$;
-- Preserve the currently installed guard (including WorldReset), replacing
-- only its rat deadline and adding the required exact terminal stage proof.
DO $$ DECLARE body TEXT; BEGIN
 SELECT pg_get_functiondef('game_item_decay_retire_consistency_guard()'::regprocedure) INTO body;
 IF position('NEW.deadline <> corpse_materialized + 60000' IN body)=0 THEN RAISE EXCEPTION 'corpse guard changed; review migration'; END IF;
 body:=replace(body,'NEW.deadline <> corpse_materialized + 60000',
  'NEW.deadline IS DISTINCT FROM game_corpse_retirement_deadline(NEW.corpse_item_instance_id)');
 body:=replace(body,'SELECT * INTO item FROM game_item_instances WHERE item_instance_id = NEW.item_instance_id;',
 'IF EXISTS(SELECT 1 FROM game_item_mint_receipts m WHERE m.item_instance_id=NEW.corpse_item_instance_id
   AND EXISTS(SELECT 1 FROM game_spell_field_temporal_schedules p WHERE p.item_instance_id=m.item_instance_id AND p.source_ordinal=0 AND p.source_stage=1))
  AND NOT EXISTS(SELECT 1 FROM game_spell_field_temporal_receipts r
   JOIN game_spell_field_temporal_schedules p ON p.item_instance_id=r.item_instance_id AND p.source_stage=r.source_stage
   JOIN game_item_instances i ON i.item_instance_id=r.item_instance_id
   WHERE r.item_instance_id=NEW.corpse_item_instance_id AND r.source_stage=3
    AND ((NEW.item_instance_id<>NEW.corpse_item_instance_id AND i.lifecycle=1 AND i.quantity=1
       AND i.state_revision=r.state_revision_before+1 AND i.last_transaction_id=r.transaction_id)
      OR (NEW.item_instance_id=NEW.corpse_item_instance_id AND i.lifecycle=2 AND i.quantity=0
       AND i.state_revision=r.state_revision_before+2 AND i.last_transaction_id=NEW.transaction_id))
    AND (i.definition_family,i.definition_production_key,i.definition_revision_ref)=(p.target_family,p.target_production_key,p.target_revision)) THEN
   RAISE EXCEPTION ''corpse retirement requires committed final transform'' USING ERRCODE=''23514'';
  END IF;
  SELECT * INTO item FROM game_item_instances WHERE item_instance_id = NEW.item_instance_id;');
 EXECUTE body;
END $$;
DO $$ DECLARE f TEXT; BEGIN
 FOREACH f IN ARRAY ARRAY['game_summer_corpse_chain(text)','game_summer_corpse_schedule()',
   'game_corpse_stage_ground_current(uuid,uuid,uuid,numeric,bigint)',
   'game_corpse_retirement_deadline(uuid)','game_corpse_terminal_stage_ready(uuid)','game_spell_field_temporal_schedule_proven()'] LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=%I,pg_temp',f,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC',f);
 END LOOP;
END $$;
GRANT EXECUTE ON FUNCTION game_summer_corpse_chain(text),game_corpse_retirement_deadline(uuid),game_corpse_terminal_stage_ready(uuid),game_corpse_stage_ground_current(uuid,uuid,uuid,numeric,bigint) TO oteryn_game_runtime,oteryn_game_control;
