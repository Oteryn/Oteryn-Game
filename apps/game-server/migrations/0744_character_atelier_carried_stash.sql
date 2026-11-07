-- P07: Crystal 201189f player.cpp:5304-5356,5821-5844 and player_wheel.cpp:1218-1240,1514-1568.
-- Extend the existing Atelier transaction and Supply Stash ledger; no second asset store.
ALTER TABLE game_atelier_item_records
 ALTER COLUMN parent_item_instance_id DROP NOT NULL,
 ADD COLUMN IF NOT EXISTS stash_input JSONB,
 ADD COLUMN IF NOT EXISTS equipment_before JSONB;
DO $migration$
DECLARE c RECORD;
BEGIN
 FOR c IN SELECT conname FROM pg_constraint WHERE conrelid='game_atelier_item_records'::regclass
   AND contype='c' AND pg_get_constraintdef(oid) LIKE '%jsonb_array_length(inputs)%' LOOP
  EXECUTE format('ALTER TABLE game_atelier_item_records DROP CONSTRAINT %I',c.conname);
 END LOOP;
END $migration$;
ALTER TABLE game_atelier_item_records
 DROP CONSTRAINT IF EXISTS game_atelier_stash_shape,
 DROP CONSTRAINT IF EXISTS game_atelier_equipment_shape;
ALTER TABLE game_atelier_item_records
 ADD CONSTRAINT game_atelier_material_inputs CHECK(jsonb_typeof(inputs)='array' AND jsonb_array_length(inputs)<=30),
 ADD CONSTRAINT game_atelier_material_shape CHECK(
  (cause='REVEAL' AND jsonb_array_length(inputs)=1 AND jsonb_array_length(outputs)=0 AND stash_input IS NULL)
  OR (cause='GRADE_UP' AND jsonb_array_length(outputs)=0 AND (jsonb_array_length(inputs)>0 OR stash_input IS NOT NULL))
  -- Keep immutable split-output history valid when this leaf is reapplied after 0746.
  -- Before 0746, 0314's independent outputs<=1 check and completion proof still apply.
  OR (cause='DISMANTLE' AND jsonb_array_length(inputs)=0 AND jsonb_array_length(outputs) BETWEEN 1 AND 10 AND stash_input IS NULL)),
 ADD CONSTRAINT game_atelier_stash_shape CHECK(stash_input IS NULL OR
   (cause='GRADE_UP' AND jsonb_typeof(stash_input)='object' AND (stash_input->>'count')::int BETWEEN 1 AND 30)),
 ADD CONSTRAINT game_atelier_equipment_shape CHECK(equipment_before IS NULL OR jsonb_typeof(equipment_before)='object');

ALTER TABLE game_character_stash_entries
 ALTER COLUMN transaction_id DROP NOT NULL,
 ADD COLUMN IF NOT EXISTS atelier_transaction_id UUID UNIQUE REFERENCES game_atelier_item_records(transaction_id);
ALTER TABLE game_character_stash_entries DROP CONSTRAINT IF EXISTS game_stash_entry_owner;
ALTER TABLE game_character_stash_entries ADD CONSTRAINT game_stash_entry_owner
 CHECK(num_nonnulls(transaction_id,atelier_transaction_id)=1);

-- Bound ancestry is read from current carried custody, never from a client or a receipt.
CREATE OR REPLACE FUNCTION game_atelier_carried_parent(p_character UUID,p_world UUID,p_parent UUID)
RETURNS BOOLEAN LANGUAGE sql STABLE SECURITY DEFINER AS $$
 WITH RECURSIVE path(item,depth,seen) AS (
  SELECT i.item_instance_id,0,ARRAY[i.item_instance_id] FROM game_item_instances i
   WHERE i.item_instance_id=p_parent AND i.world_id=p_world AND i.lifecycle=1
  UNION ALL
  SELECT e.parent_item_instance_id,p.depth+1,p.seen||e.parent_item_instance_id FROM path p
   JOIN (SELECT item_instance_id,parent_item_instance_id,world_id FROM game_item_container_entries WHERE character_id=p_character
     UNION ALL SELECT item_instance_id,parent_item_instance_id,world_id FROM game_item_nested_entries) e ON e.item_instance_id=p.item AND e.world_id=p_world
   JOIN game_item_instances i ON i.item_instance_id=e.parent_item_instance_id AND i.world_id=p_world AND i.lifecycle=1
   WHERE p.depth<7 AND NOT e.parent_item_instance_id=ANY(p.seen))
 SELECT EXISTS(SELECT 1 FROM path p WHERE EXISTS(SELECT 1 FROM game_item_container_slots s
   WHERE s.item_instance_id=p.item AND s.character_id=p_character AND s.world_id=p_world)
  OR EXISTS(SELECT 1 FROM game_character_equipment_slots s WHERE s.item_instance_id=p.item AND s.character_id=p_character AND s.world_id=p_world))
$$;
CREATE OR REPLACE FUNCTION game_atelier_input_present(p_character UUID,p_world UUID,p_root UUID,p_line JSONB)
RETURNS BOOLEAN LANGUAGE sql STABLE SECURITY DEFINER AS $$
 SELECT CASE coalesce(p_line->>'location','main')
  WHEN 'main' THEN EXISTS(SELECT 1 FROM game_item_container_entries e
   WHERE e.item_instance_id=(p_line->>'item')::uuid AND e.character_id=p_character AND e.world_id=p_world
    AND e.parent_item_instance_id=p_root AND e.placement_ordinal=(p_line->>'ordinal')::numeric)
  WHEN 'nested' THEN EXISTS(SELECT 1 FROM game_item_nested_entries e
   WHERE e.item_instance_id=(p_line->>'item')::uuid AND e.world_id=p_world
    AND e.parent_item_instance_id=(p_line->>'parent')::uuid AND e.placement_ordinal=(p_line->>'ordinal')::numeric
    AND game_atelier_carried_parent(p_character,p_world,e.parent_item_instance_id))
  WHEN 'equipment' THEN EXISTS(SELECT 1 FROM game_character_equipment_slots s
   WHERE s.item_instance_id=(p_line->>'item')::uuid AND s.character_id=p_character AND s.world_id=p_world
    AND s.slot=(p_line->>'slot')::smallint AND p_line->>'parent' IS NULL)
  ELSE false END
$$;

CREATE OR REPLACE FUNCTION game_atelier_supplement_complete(p_transaction UUID) RETURNS void LANGUAGE plpgsql AS $$
DECLARE r game_atelier_item_records%ROWTYPE; e game_character_stash_entries%ROWTYPE;
 b game_character_stash_balances%ROWTYPE; s game_character_equipment_state%ROWTYPE;
BEGIN
 SELECT * INTO STRICT r FROM game_atelier_item_records WHERE transaction_id=p_transaction;
 IF r.created_xact_id<>pg_current_xact_id() THEN RAISE EXCEPTION 'historical Atelier supplement proof' USING ERRCODE='23514'; END IF;
 SELECT * INTO e FROM game_character_stash_entries WHERE atelier_transaction_id=p_transaction;
 IF r.stash_input IS NULL THEN
  IF e.entry_id IS NOT NULL THEN RAISE EXCEPTION 'unrecorded Atelier stash debit' USING ERRCODE='23514'; END IF;
 ELSE
  SELECT * INTO b FROM game_character_stash_balances WHERE character_id=r.character_id AND world_id=r.world_id
   AND definition_key=r.stash_input->>'key' AND definition_revision=r.stash_input->>'revision';
  IF r.cause<>'GRADE_UP' OR e.entry_id IS NULL OR e.created_xact_id<>pg_current_xact_id() OR e.transaction_id IS NOT NULL
   OR (e.entry_id,e.character_id,e.world_id,e.definition_key,e.definition_revision,e.amount,e.quantity_before,e.previous_entry_id)
    IS DISTINCT FROM ((r.stash_input->>'entry')::uuid,r.character_id,r.world_id,r.stash_input->>'key',r.stash_input->>'revision',
      (r.stash_input->>'count')::bigint,(r.stash_input->>'quantity_before')::bigint,(r.stash_input->>'previous')::uuid)
   OR e.definition_key NOT IN ('oteryn:item.tibia.i46625','oteryn:item.tibia.i46626')
   OR e.quantity_after<>e.quantity_before-e.amount OR b.last_entry_id IS DISTINCT FROM e.entry_id OR b.quantity IS DISTINCT FROM e.quantity_after THEN
   RAISE EXCEPTION 'Atelier stash debit lacks exact ledger successor' USING ERRCODE='23514';
  END IF;
 END IF;
 IF (r.equipment_before IS NOT NULL) IS DISTINCT FROM EXISTS(SELECT 1 FROM jsonb_array_elements(r.inputs) l WHERE l->>'location'='equipment') THEN
  RAISE EXCEPTION 'Atelier equipment proof shape mismatch' USING ERRCODE='23514';
 END IF;
 IF r.equipment_before IS NOT NULL THEN
  SELECT * INTO s FROM game_character_equipment_state WHERE character_id=r.character_id;
  IF s.character_id IS NULL OR s.revision<>(r.equipment_before->>'revision')::numeric+1 OR s.last_transaction_id IS DISTINCT FROM r.transaction_id
    OR (to_jsonb(s)-ARRAY['revision','last_transaction_id']) IS DISTINCT FROM (r.equipment_before-ARRAY['revision','last_transaction_id']) THEN
   RAISE EXCEPTION 'Atelier equipment epoch lacks exact successor' USING ERRCODE='23514';
  END IF;
 END IF;
END $$;

-- Preserve all current guard-chain arms; only the Atelier source and debit proofs widen.
DO $migration$
DECLARE body TEXT; old_part TEXT; new_part TEXT; at_begin INT;
BEGIN
 body:=pg_get_functiondef('game_atelier_item_admit()'::regprocedure);
 IF position('game_atelier_input_present' IN body)=0 THEN
  old_part:='IF NOT EXISTS(SELECT 1 FROM game_item_container_slots b JOIN game_item_instances bag USING(item_instance_id,world_id)';
  new_part:='IF NEW.parent_item_instance_id IS NOT NULL AND NOT EXISTS(SELECT 1 FROM game_item_container_slots b JOIN game_item_instances bag USING(item_instance_id,world_id)';
  IF position(old_part IN body)=0 THEN RAISE EXCEPTION 'missing Atelier backpack admission anchor'; END IF;
  body:=replace(body,old_part,new_part);
  old_part:=$old$NOT EXISTS(SELECT 1 FROM game_item_container_entries e WHERE e.item_instance_id=i.item_instance_id
       AND e.character_id=NEW.character_id AND e.parent_item_instance_id=NEW.parent_item_instance_id
       AND e.world_id=NEW.world_id AND e.placement_ordinal=(line->>'ordinal')::numeric)$old$;
  IF position(old_part IN body)=0 THEN RAISE EXCEPTION 'missing Atelier input admission anchor'; END IF;
  body:=replace(body,old_part,'NOT game_atelier_input_present(NEW.character_id,NEW.world_id,NEW.parent_item_instance_id,line)');
  at_begin:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
  body:=overlay(body placing $arm$BEGIN
 IF (NEW.equipment_before IS NOT NULL) IS DISTINCT FROM EXISTS(SELECT 1 FROM jsonb_array_elements(NEW.inputs) l WHERE l->>'location'='equipment')
  OR (NEW.equipment_before IS NOT NULL AND NEW.equipment_before IS DISTINCT FROM (SELECT to_jsonb(s) FROM game_character_equipment_state s WHERE s.character_id=NEW.character_id)) THEN
  RAISE EXCEPTION 'Atelier requires current equipment snapshot' USING ERRCODE='23514';
 END IF;
$arm$ from at_begin for 5);
  EXECUTE body;
 END IF;

 body:=pg_get_functiondef('game_atelier_item_complete(uuid)'::regprocedure);
 IF position('game_atelier_supplement_complete' IN body)=0 THEN
  old_part:=$old$(i.quantity=0 AND EXISTS(SELECT 1 FROM game_item_container_entries WHERE item_instance_id=i.item_instance_id))
    OR (i.quantity>0 AND NOT EXISTS(SELECT 1 FROM game_item_container_entries WHERE item_instance_id=i.item_instance_id
       AND character_id=r.character_id AND world_id=r.world_id AND parent_item_instance_id=r.parent_item_instance_id AND placement_ordinal=(line->>'ordinal')::numeric))$old$;
  new_part:=$new$(i.quantity=0 AND (EXISTS(SELECT 1 FROM game_item_container_entries WHERE item_instance_id=i.item_instance_id)
      OR EXISTS(SELECT 1 FROM game_item_nested_entries WHERE item_instance_id=i.item_instance_id)
      OR EXISTS(SELECT 1 FROM game_character_equipment_slots WHERE item_instance_id=i.item_instance_id)))
    OR (i.quantity>0 AND NOT game_atelier_input_present(r.character_id,r.world_id,r.parent_item_instance_id,line))$new$;
  IF position(old_part IN body)=0 THEN RAISE EXCEPTION 'missing Atelier input completion anchor'; END IF;
  body:=replace(body,old_part,new_part);
  old_part:=E' END LOOP;\nEND';
  IF position(old_part IN body)=0 THEN RAISE EXCEPTION 'missing Atelier supplement completion anchor'; END IF;
  body:=replace(body,old_part,E' END LOOP;\n PERFORM game_atelier_supplement_complete(p_transaction);\nEND');
  EXECUTE body;
 END IF;

 body:=pg_get_functiondef('game_atelier_assets_guard()'::regprocedure);
 IF position('r.stash_input' IN body)=0 THEN
  old_part:='(SELECT sum((l->>''count'')::int) FROM jsonb_array_elements(r.inputs) l) IS DISTINCT FROM count_needed::bigint';
  new_part:='(SELECT coalesce(sum((l->>''count'')::int),0) FROM jsonb_array_elements(r.inputs) l)+coalesce((r.stash_input->>''count'')::bigint,0) IS DISTINCT FROM count_needed::bigint';
  IF position(old_part IN body)=0 THEN RAISE EXCEPTION 'missing Atelier material total anchor'; END IF;
  body:=replace(body,old_part,new_part);
  old_part:='OR EXISTS(SELECT 1 FROM jsonb_array_elements(r.inputs) l WHERE l->''before''->>''definition_production_key'' IS DISTINCT FROM key))';
  new_part:='OR EXISTS(SELECT 1 FROM jsonb_array_elements(r.inputs) l WHERE l->''before''->>''definition_production_key'' IS DISTINCT FROM key) OR (r.stash_input IS NOT NULL AND r.stash_input->>''key'' IS DISTINCT FROM key))';
  IF position(old_part IN body)=0 THEN RAISE EXCEPTION 'missing Atelier material key anchor'; END IF;
  EXECUTE replace(body,old_part,new_part);
 END IF;

 body:=pg_get_functiondef('game_stash_admit()'::regprocedure);
 IF position('atelier_transaction_id' IN body)=0 THEN
  at_begin:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
  EXECUTE overlay(body placing $arm$BEGIN
 IF TG_TABLE_NAME='game_character_stash_entries' THEN
  IF NEW.atelier_transaction_id IS NOT NULL THEN
   IF NEW.transaction_id IS NOT NULL OR NOT EXISTS(SELECT 1 FROM game_atelier_item_records a
      JOIN game_character_stash_balances b ON b.character_id=a.character_id AND b.world_id=a.world_id
       AND b.definition_key=a.stash_input->>'key' AND b.definition_revision=a.stash_input->>'revision'
     WHERE a.transaction_id=NEW.atelier_transaction_id AND a.cause='GRADE_UP' AND a.created_xact_id=pg_current_xact_id()
      AND (NEW.entry_id,NEW.character_id,NEW.world_id,NEW.definition_key,NEW.definition_revision,NEW.amount,NEW.quantity_before,NEW.previous_entry_id)
       IS NOT DISTINCT FROM ((a.stash_input->>'entry')::uuid,a.character_id,a.world_id,a.stash_input->>'key',a.stash_input->>'revision',
         (a.stash_input->>'count')::bigint,b.quantity,b.last_entry_id)
      AND b.quantity=(a.stash_input->>'quantity_before')::bigint AND b.last_entry_id=(a.stash_input->>'previous')::uuid
      AND NEW.quantity_after=NEW.quantity_before-NEW.amount) THEN
    RAISE EXCEPTION 'Atelier stash entry requires exact current balance and receipt' USING ERRCODE='23514';
   END IF;
   NEW.created_xact_id:=pg_current_xact_id(); RETURN NEW;
  END IF;
 END IF;
$arm$ from at_begin for 5);
 END IF;
 body:=pg_get_functiondef('game_stash_balance_guard()'::regprocedure);
 IF position('atelier_transaction_id' IN body)=0 THEN
  old_part:='SELECT * INTO r FROM game_character_stash_operations WHERE transaction_id=e.transaction_id;';
  IF position(old_part IN body)=0 THEN RAISE EXCEPTION 'missing stash balance owner anchor'; END IF;
  new_part:=$arm$IF e.atelier_transaction_id IS NOT NULL THEN
  IF TG_OP<>'UPDATE' OR e.created_xact_id<>pg_current_xact_id()
    OR (e.character_id,e.world_id,e.definition_key,e.definition_revision,e.quantity_after,e.quantity_before,e.previous_entry_id)
     IS DISTINCT FROM (NEW.character_id,NEW.world_id,NEW.definition_key,NEW.definition_revision,NEW.quantity,OLD.quantity,OLD.last_entry_id)
    OR (NEW.character_id,NEW.world_id,NEW.definition_key,NEW.definition_revision) IS DISTINCT FROM (OLD.character_id,OLD.world_id,OLD.definition_key,OLD.definition_revision)
    OR NOT EXISTS(SELECT 1 FROM game_atelier_item_records a WHERE a.transaction_id=e.atelier_transaction_id AND a.cause='GRADE_UP'
      AND a.created_xact_id=pg_current_xact_id() AND a.character_id=NEW.character_id AND a.world_id=NEW.world_id
      AND (a.stash_input->>'entry')::uuid=e.entry_id) THEN
   RAISE EXCEPTION 'Atelier stash balance requires exact ledger chain' USING ERRCODE='23514';
  END IF;
  RETURN NEW;
 END IF;
 SELECT * INTO r FROM game_character_stash_operations WHERE transaction_id=e.transaction_id;$arm$;
  EXECUTE replace(body,old_part,new_part);
 END IF;
 body:=pg_get_functiondef('game_stash_consistent()'::regprocedure);
 IF position('atelier_transaction_id' IN body)=0 THEN
  at_begin:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
  EXECUTE overlay(body placing $arm$BEGIN
 IF TG_TABLE_NAME='game_character_stash_entries' THEN
  IF NEW.atelier_transaction_id IS NOT NULL THEN PERFORM game_atelier_item_complete(NEW.atelier_transaction_id); RETURN NULL; END IF;
 END IF;
$arm$ from at_begin for 5);
 END IF;
END $migration$;

DO $migration$
DECLARE n TEXT; body TEXT; arm TEXT; at_begin INT;
BEGIN
 FOREACH n IN ARRAY ARRAY['game_item_nested_entry_proven','game_equipment_slot_proven','game_equipment_state_proven'] LOOP
  body:=pg_get_functiondef((n||'()')::regprocedure);
  IF position('game_atelier_item_records' IN body)>0 THEN CONTINUE; END IF;
  IF n='game_item_nested_entry_proven' THEN
   arm:=$arm$
 IF TG_OP='DELETE' AND EXISTS(SELECT 1 FROM game_atelier_item_records a, LATERAL jsonb_array_elements(a.inputs) l
   WHERE a.created_xact_id=pg_current_xact_id() AND a.world_id=OLD.world_id AND l->>'location'='nested'
    AND (l->>'item')::uuid=OLD.item_instance_id AND (l->>'parent')::uuid=OLD.parent_item_instance_id
    AND (l->>'ordinal')::numeric=OLD.placement_ordinal AND (l->'before'->>'quantity')::bigint=(l->>'count')::bigint) THEN
  PERFORM game_atelier_item_complete(a.transaction_id) FROM game_atelier_item_records a,LATERAL jsonb_array_elements(a.inputs) l
   WHERE a.created_xact_id=pg_current_xact_id() AND (l->>'item')::uuid=OLD.item_instance_id;
  RETURN NULL;
 END IF;
$arm$;
  ELSIF n='game_equipment_slot_proven' THEN
   arm:=$arm$
 IF TG_OP='DELETE' AND EXISTS(SELECT 1 FROM game_atelier_item_records a, LATERAL jsonb_array_elements(a.inputs) l
   WHERE a.created_xact_id=pg_current_xact_id() AND a.character_id=OLD.character_id AND a.world_id=OLD.world_id AND a.equipment_before IS NOT NULL
    AND l->>'location'='equipment' AND (l->>'item')::uuid=OLD.item_instance_id AND (l->>'slot')::smallint=OLD.slot
    AND (l->'before'->>'quantity')::bigint=(l->>'count')::bigint) THEN
  PERFORM game_atelier_item_complete(a.transaction_id) FROM game_atelier_item_records a
   WHERE a.created_xact_id=pg_current_xact_id() AND a.character_id=OLD.character_id AND a.equipment_before IS NOT NULL;
  RETURN NULL;
 END IF;
$arm$;
  ELSE
   arm:=$arm$
 IF EXISTS(SELECT 1 FROM game_atelier_item_records a WHERE a.transaction_id=NEW.last_transaction_id AND a.character_id=NEW.character_id
   AND a.created_xact_id=pg_current_xact_id() AND a.equipment_before=to_jsonb(OLD)) THEN
  PERFORM game_atelier_item_complete(NEW.last_transaction_id); RETURN NULL;
 END IF;
$arm$;
  END IF;
  at_begin:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
  IF at_begin=0 THEN RAISE EXCEPTION 'missing custody guard BEGIN: %',n; END IF;
  EXECUTE overlay(body placing E'BEGIN\n'||arm from at_begin for 5);
 END LOOP;
END $migration$;

REVOKE ALL ON FUNCTION game_atelier_carried_parent(UUID,UUID,UUID),game_atelier_input_present(UUID,UUID,UUID,JSONB),game_atelier_supplement_complete(UUID) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_atelier_carried_parent(UUID,UUID,UUID),game_atelier_input_present(UUID,UUID,UUID,JSONB),game_atelier_supplement_complete(UUID) TO oteryn_game_runtime;
DO $$ DECLARE n TEXT; BEGIN
 FOREACH n IN ARRAY ARRAY['game_atelier_carried_parent(uuid,uuid,uuid)','game_atelier_input_present(uuid,uuid,uuid,jsonb)','game_atelier_supplement_complete(uuid)',
  'game_atelier_item_admit()','game_atelier_item_complete(uuid)','game_atelier_assets_guard()','game_stash_admit()','game_stash_balance_guard()','game_stash_consistent()',
  'game_item_nested_entry_proven()','game_equipment_slot_proven()','game_equipment_state_proven()'] LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',n,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC',n);
 END LOOP;
END $$;
