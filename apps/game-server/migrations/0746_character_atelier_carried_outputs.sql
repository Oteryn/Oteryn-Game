-- Crystal 201189f player.cpp:4932-5047: ordinary carried fragment destinations.
-- Retain the existing Atelier receipt, MINT audit and item custody owners.
DO $migration$
DECLARE c RECORD;
BEGIN
 FOR c IN SELECT conname FROM pg_constraint WHERE conrelid='game_atelier_item_records'::regclass
   AND contype='c' AND pg_get_constraintdef(oid) LIKE '%jsonb_array_length(outputs)%' LOOP
  EXECUTE format('ALTER TABLE game_atelier_item_records DROP CONSTRAINT %I',c.conname);
 END LOOP;
END $migration$;
ALTER TABLE game_atelier_item_records
 ADD CONSTRAINT game_atelier_carried_outputs CHECK(jsonb_typeof(outputs)='array' AND jsonb_array_length(outputs)<=10),
 ADD CONSTRAINT game_atelier_material_shape CHECK(
  (cause='REVEAL' AND jsonb_array_length(inputs)=1 AND jsonb_array_length(outputs)=0 AND stash_input IS NULL)
  OR (cause='GRADE_UP' AND jsonb_array_length(outputs)=0 AND (jsonb_array_length(inputs)>0 OR stash_input IS NOT NULL))
  OR (cause='DISMANTLE' AND jsonb_array_length(inputs)=0 AND jsonb_array_length(outputs) BETWEEN 1 AND 10 AND stash_input IS NULL));

CREATE OR REPLACE FUNCTION game_atelier_carried_tree_valid(p_character UUID,p_world UUID)
RETURNS BOOLEAN LANGUAGE sql STABLE SECURITY DEFINER AS $$
 WITH RECURSIVE roots AS (
  SELECT item_instance_id,false AS equipment FROM game_item_container_slots WHERE character_id=p_character AND world_id=p_world
  UNION ALL SELECT item_instance_id,true FROM game_character_equipment_slots WHERE character_id=p_character AND world_id=p_world
 ), carried AS (
  SELECT r.item_instance_id,r.equipment,0 AS depth,ARRAY[r.item_instance_id] AS seen
   FROM roots r JOIN game_item_instances i USING(item_instance_id) WHERE i.world_id=p_world AND i.lifecycle=1
  UNION ALL
  SELECT e.item_instance_id,false,p.depth+1,p.seen||e.item_instance_id FROM carried p
   JOIN LATERAL (
    SELECT item_instance_id FROM game_item_container_entries WHERE character_id=p_character AND world_id=p_world AND parent_item_instance_id=p.item_instance_id
    UNION ALL SELECT item_instance_id FROM game_item_nested_entries WHERE world_id=p_world AND parent_item_instance_id=p.item_instance_id) e ON true
   JOIN game_item_instances i ON i.item_instance_id=e.item_instance_id AND i.world_id=p_world AND i.lifecycle=1
   WHERE p.depth<9 AND NOT e.item_instance_id=ANY(p.seen)
 ), bounded AS MATERIALIZED (SELECT * FROM carried LIMIT 510)
 SELECT count(*)<510 AND count(*) FILTER(WHERE NOT equipment)<=500
  AND count(*) FILTER(WHERE equipment)<=9 AND coalesce(max(depth),0)<=8
  AND count(DISTINCT item_instance_id)=count(*) FROM bounded
$$;

CREATE OR REPLACE FUNCTION game_atelier_output_admit(p_character UUID,p_world UUID,p_root UUID,p_outputs JSONB)
RETURNS void LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE line JSONB; destination UUID; kind TEXT;
BEGIN
 IF jsonb_typeof(p_outputs)<>'array' OR jsonb_array_length(p_outputs)>10
  OR (SELECT count(DISTINCT value->>'item') FROM jsonb_array_elements(p_outputs))<>jsonb_array_length(p_outputs) THEN
  RAISE EXCEPTION 'duplicate or oversized Atelier fragment outputs' USING ERRCODE='23514';
 END IF;
 FOR line IN SELECT value FROM jsonb_array_elements(p_outputs) LOOP
  destination:=coalesce((line->>'parent')::uuid,p_root); kind:=coalesce(line->>'location','main');
  IF destination IS NULL OR kind NOT IN ('main','nested')
    OR (line->>'ordinal')::numeric NOT BETWEEN 1 AND 18446744073709551615
    OR (kind='main' AND (destination IS DISTINCT FROM p_root OR NOT EXISTS(SELECT 1 FROM game_item_container_slots s
      JOIN game_item_instances i USING(item_instance_id,world_id) WHERE s.item_instance_id=destination AND s.character_id=p_character AND s.world_id=p_world AND i.lifecycle=1)))
    OR (kind='nested' AND (destination IS NOT DISTINCT FROM p_root OR NOT game_atelier_carried_parent(p_character,p_world,destination)))
    OR (SELECT count(*) FROM (
      SELECT item_instance_id FROM game_item_container_entries WHERE parent_item_instance_id=destination
      UNION ALL SELECT item_instance_id FROM game_item_nested_entries WHERE parent_item_instance_id=destination) e)
       +(SELECT count(*) FROM jsonb_array_elements(p_outputs) l WHERE NOT l ? 'before' AND coalesce((l->>'parent')::uuid,p_root)=destination)>20
    OR (line ? 'before' AND (EXISTS(SELECT 1 FROM game_item_container_entries WHERE parent_item_instance_id=(line->>'item')::uuid)
      OR EXISTS(SELECT 1 FROM game_item_nested_entries WHERE parent_item_instance_id=(line->>'item')::uuid))) THEN
   RAISE EXCEPTION 'Atelier fragment destination is not current carried room' USING ERRCODE='23514';
  END IF;
 END LOOP;
END $$;

CREATE OR REPLACE FUNCTION game_atelier_outputs_complete(p_transaction UUID)
RETURNS void LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE r game_atelier_item_records%ROWTYPE; line JSONB; destination UUID;
BEGIN
 SELECT * INTO STRICT r FROM game_atelier_item_records WHERE transaction_id=p_transaction;
 IF r.created_xact_id<>pg_current_xact_id() THEN
  RAISE EXCEPTION 'historical Atelier fragment output proof' USING ERRCODE='23514';
 END IF;
 IF jsonb_array_length(r.outputs)=0 THEN RETURN; END IF;
 IF NOT game_atelier_carried_tree_valid(r.character_id,r.world_id) THEN
  RAISE EXCEPTION 'Atelier fragment output exceeds carried graph' USING ERRCODE='23514';
 END IF;
 FOR line IN SELECT value FROM jsonb_array_elements(r.outputs) LOOP
  destination:=coalesce((line->>'parent')::uuid,r.parent_item_instance_id);
  IF NOT EXISTS(SELECT 1 FROM game_item_instances n
    WHERE n.item_instance_id=(line->>'item')::uuid AND n.world_id=r.world_id AND n.lifecycle=1
      AND n.quantity=coalesce((line->'before'->>'quantity')::bigint,0)+(line->>'count')::bigint
      AND n.quantity BETWEEN 1 AND 100 AND n.definition_family='Item' AND n.definition_production_key=line->>'key'
      AND n.definition_revision_ref=line->>'revision'
      AND ((NOT line ? 'before' AND n.minted_transaction_id=r.transaction_id AND n.last_transaction_id IS NULL AND n.state_revision=1)
        OR (line ? 'before' AND n.last_transaction_id=r.transaction_id AND n.state_revision=(line->'before'->>'state_revision')::numeric+1
          AND (to_jsonb(n)-ARRAY['quantity','last_transaction_id','state_revision'])=((line->'before')-ARRAY['quantity','last_transaction_id','state_revision'])))
      AND game_atelier_input_present(r.character_id,r.world_id,r.parent_item_instance_id,line)
      AND (line ? 'before' OR EXISTS(SELECT 1 FROM game_item_container_entries e WHERE e.item_instance_id=n.item_instance_id AND e.placed_transaction_id=r.transaction_id)
        OR EXISTS(SELECT 1 FROM game_item_nested_entries e WHERE e.item_instance_id=n.item_instance_id AND e.placed_transaction_id=r.transaction_id)))
    OR (SELECT count(*) FROM (
      SELECT item_instance_id FROM game_item_container_entries WHERE parent_item_instance_id=destination
      UNION ALL SELECT item_instance_id FROM game_item_nested_entries WHERE parent_item_instance_id=destination) e)>20 THEN
   RAISE EXCEPTION 'Atelier carried fragment output mismatch' USING ERRCODE='23514';
  END IF;
 END LOOP;
END $$;

DO $migration$
DECLARE body TEXT; old_part TEXT; new_part TEXT; at_begin INT; first_at INT; last_at INT;
BEGIN
 body:=pg_get_functiondef('game_atelier_item_admit()'::regprocedure);
 IF position('game_atelier_output_admit' IN body)=0 THEN
  old_part:=$old$(NOT line ? 'before' AND (line->>'ordinal')::numeric <= (SELECT coalesce(max(placement_ordinal),0) FROM game_item_container_entries WHERE parent_item_instance_id=NEW.parent_item_instance_id))$old$;
  new_part:=$new$(NOT line ? 'before' AND (line->>'ordinal')::numeric <= (SELECT coalesce(max(placement_ordinal),0) FROM (
    SELECT placement_ordinal FROM game_item_container_entries WHERE parent_item_instance_id=coalesce((line->>'parent')::uuid,NEW.parent_item_instance_id)
    UNION ALL SELECT placement_ordinal FROM game_item_nested_entries WHERE parent_item_instance_id=coalesce((line->>'parent')::uuid,NEW.parent_item_instance_id)) e))$new$;
  IF position(old_part IN body)=0 THEN RAISE EXCEPTION 'missing Atelier output ordinal admission anchor'; END IF;
  body:=replace(body,old_part,new_part);
  old_part:=$old$NOT EXISTS(SELECT 1 FROM game_item_container_entries e WHERE e.item_instance_id=i.item_instance_id
      AND e.character_id=NEW.character_id AND e.world_id=NEW.world_id AND e.parent_item_instance_id=NEW.parent_item_instance_id AND e.placement_ordinal=(line->>'ordinal')::numeric)$old$;
  IF position(old_part IN body)=0 THEN RAISE EXCEPTION 'missing Atelier receiver admission anchor'; END IF;
  body:=replace(body,old_part,'NOT game_atelier_input_present(NEW.character_id,NEW.world_id,NEW.parent_item_instance_id,line)');
  at_begin:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
  EXECUTE overlay(body placing E'BEGIN\n PERFORM game_atelier_output_admit(NEW.character_id,NEW.world_id,NEW.parent_item_instance_id,NEW.outputs);\n' from at_begin for 5);
 END IF;

 body:=pg_get_functiondef('game_atelier_item_complete(uuid)'::regprocedure);
 IF position('game_atelier_outputs_complete' IN body)=0 THEN
  old_part:=' FOR line IN SELECT value FROM jsonb_array_elements(r.outputs) LOOP';
  first_at:=position(old_part IN body);
  IF first_at=0 THEN RAISE EXCEPTION 'missing Atelier output completion loop'; END IF;
  last_at:=position(' END LOOP;' IN substring(body from first_at));
  IF last_at=0 THEN RAISE EXCEPTION 'missing Atelier output completion end'; END IF;
  EXECUTE overlay(body placing ' PERFORM game_atelier_outputs_complete(p_transaction);' from first_at for last_at-1+length(' END LOOP;'));
 END IF;

 body:=pg_get_functiondef('game_atelier_assets_guard()'::regprocedure);
 IF position('sum((l->>''count'')::int) FROM jsonb_array_elements(r.outputs)' IN body)=0 THEN
  old_part:='r.outputs->0->>''key'' IS DISTINCT FROM key OR (r.outputs->0->>''count'')::int NOT BETWEEN';
  new_part:='EXISTS(SELECT 1 FROM jsonb_array_elements(r.outputs) l WHERE l->>''key'' IS DISTINCT FROM key) OR (SELECT sum((l->>''count'')::int) FROM jsonb_array_elements(r.outputs) l) NOT BETWEEN';
  IF position(old_part IN body)=0 THEN RAISE EXCEPTION 'missing Atelier total fragment yield anchor'; END IF;
  EXECUTE replace(body,old_part,new_part);
 END IF;

 body:=pg_get_functiondef('game_item_nested_entry_proven()'::regprocedure);
 IF position('Atelier carried fragment output' IN body)=0 THEN
  at_begin:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
  EXECUTE overlay(body placing $arm$BEGIN
 -- Atelier carried fragment output: fresh nested placement belongs to the same receipt.
 IF TG_OP='INSERT' AND EXISTS(SELECT 1 FROM game_atelier_item_records a,LATERAL jsonb_array_elements(a.outputs) l
   WHERE a.transaction_id=NEW.placed_transaction_id AND a.created_xact_id=pg_current_xact_id() AND a.world_id=NEW.world_id
    AND l->>'location'='nested' AND NOT l ? 'before' AND (l->>'item')::uuid=NEW.item_instance_id
    AND (l->>'parent')::uuid=NEW.parent_item_instance_id AND (l->>'ordinal')::numeric=NEW.placement_ordinal) THEN
  PERFORM game_atelier_item_complete(NEW.placed_transaction_id); RETURN NULL;
 END IF;
$arm$ from at_begin for 5);
 END IF;
END $migration$;

REVOKE ALL ON FUNCTION game_atelier_carried_tree_valid(UUID,UUID),game_atelier_output_admit(UUID,UUID,UUID,JSONB),game_atelier_outputs_complete(UUID) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_atelier_carried_tree_valid(UUID,UUID),game_atelier_output_admit(UUID,UUID,UUID,JSONB),game_atelier_outputs_complete(UUID) TO oteryn_game_runtime;
DO $$ DECLARE n TEXT; BEGIN
 FOREACH n IN ARRAY ARRAY['game_atelier_carried_tree_valid(uuid,uuid)','game_atelier_output_admit(uuid,uuid,uuid,jsonb)','game_atelier_outputs_complete(uuid)',
  'game_atelier_item_admit()','game_atelier_item_complete(uuid)','game_atelier_assets_guard()','game_item_nested_entry_proven()'] LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',n,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC',n);
 END LOOP;
END $$;
