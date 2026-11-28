-- Live enchanted gem uses reuse the atomic craft receipt and its completion proof.
-- Admission applies only to new receipts. Existing receipts retain their replay semantics.
CREATE OR REPLACE FUNCTION game_item_enchanting_result(s INTEGER,t INTEGER) RETURNS INTEGER
LANGUAGE sql IMMUTABLE SECURITY DEFINER AS $$
 SELECT CASE
 WHEN t=8077 THEN (ARRAY[8078,8079,8081,8080])[array_position(ARRAY[676,675,677,678],s)]
 WHEN s=676 AND t=9020 THEN 9019
 END
$$;

CREATE OR REPLACE FUNCTION game_item_enchanting_admit(r game_item_craft_records) RETURNS void
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE d JSONB; i game_item_instances%ROWTYPE; o JSONB; s INTEGER; t INTEGER;
BEGIN
 IF r.recipe<>'oteryn:enchanting.weapon'
  OR NOT r.consume_source OR jsonb_typeof(r.enchanting->'debits') IS DISTINCT FROM 'array'
  OR jsonb_array_length(r.enchanting->'debits')<>2
  OR jsonb_array_length(r.outputs)<>1
  OR (r.enchanting->'debits'->0->'before') IS DISTINCT FROM r.before_item
  OR (r.before_item->>'item_instance_id')::uuid IS DISTINCT FROM r.source_item_instance_id
  OR (r.enchanting->'debits'->0->>'parent')::uuid IS DISTINCT FROM r.parent_item_instance_id
  OR (r.enchanting->'debits'->0->>'ordinal')::numeric IS DISTINCT FROM r.source_ordinal
 THEN RAISE EXCEPTION 'invalid enchanted gem proof' USING ERRCODE='23514'; END IF;
 FOR d IN SELECT value FROM jsonb_array_elements(r.enchanting->'debits') LOOP
  SELECT * INTO i FROM game_item_instances WHERE item_instance_id=(d->'before'->>'item_instance_id')::uuid FOR UPDATE;
  IF i.item_instance_id IS NULL OR to_jsonb(i) IS DISTINCT FROM d->'before'
   OR i.world_id<>r.world_id OR i.definition_family<>'Item' OR i.lifecycle<>1
   OR (d->>'count')::bigint NOT BETWEEN 1 AND i.quantity
   OR EXISTS(SELECT 1 FROM game_item_keys WHERE item_instance_id=i.item_instance_id)
   -- Expiry keeps a spent timed row on the re-enchantable base item.
   OR EXISTS(SELECT 1 FROM game_item_timed_states ts WHERE ts.item_instance_id=i.item_instance_id
     AND (i.definition_production_key NOT IN ('oteryn:item.tibia.i8077','oteryn:item.tibia.i9020')
      OR ts.charges IS NOT NULL OR ts.remaining_ms IS NOT NULL OR ts.deadline_at IS NOT NULL))
   OR EXISTS(SELECT 1 FROM game_item_texts WHERE item_instance_id=i.item_instance_id)
   OR EXISTS(SELECT 1 FROM game_item_imbuements WHERE item_instance_id=i.item_instance_id)
   OR EXISTS(SELECT 1 FROM game_item_container_entries WHERE parent_item_instance_id=i.item_instance_id)
   OR EXISTS(SELECT 1 FROM game_item_nested_entries WHERE parent_item_instance_id=i.item_instance_id)
   OR NOT EXISTS(SELECT 1 FROM game_item_container_entries e JOIN game_item_container_slots b
      ON b.character_id=e.character_id AND b.item_instance_id=e.parent_item_instance_id
      WHERE e.item_instance_id=i.item_instance_id AND e.world_id=r.world_id AND b.world_id=r.world_id
       AND e.character_id=r.character_id AND e.parent_item_instance_id=(d->>'parent')::uuid
       AND e.placement_ordinal=(d->>'ordinal')::numeric)
  THEN RAISE EXCEPTION 'enchanting requires plain carried inputs' USING ERRCODE='23514'; END IF;
 END LOOP;
 s:=substring(r.before_item->>'definition_production_key' FROM '^oteryn:item.tibia.i([0-9]+)$')::integer;
 t:=substring(r.enchanting->'debits'->1->'before'->>'definition_production_key' FROM '^oteryn:item.tibia.i([0-9]+)$')::integer;
 o:=r.outputs->0;
 IF (r.enchanting->'debits'->0->>'count')::bigint IS DISTINCT FROM 1
  OR (r.enchanting->'debits'->1->>'count')::bigint IS DISTINCT FROM 1
  OR (r.enchanting->'debits'->1->'before'->>'quantity')::bigint IS DISTINCT FROM 1
  OR (r.enchanting->'debits'->1->'before'->>'item_instance_id')::uuid=r.source_item_instance_id
  OR (o->>'count')::bigint IS DISTINCT FROM 1 OR o->>'charges' IS NOT NULL
  OR r.enchanting->'cost' IS DISTINCT FROM 'null'::jsonb
 THEN RAISE EXCEPTION 'invalid enchanted gem debits' USING ERRCODE='23514'; END IF;
 IF s IS NULL OR t IS NULL OR game_item_enchanting_result(s,t) IS NULL
  OR o->>'key' IS DISTINCT FROM 'oteryn:item.tibia.i'||game_item_enchanting_result(s,t)
  OR NOT game_character_is_uuid_v7((o->>'item')::uuid)
  OR EXISTS(SELECT 1 FROM game_item_instances WHERE item_instance_id=(o->>'item')::uuid)
  OR coalesce(o->>'revision','')='' OR (o->>'ordinal')::numeric<=0
  OR EXISTS(SELECT 1 FROM game_item_container_entries WHERE parent_item_instance_id=r.parent_item_instance_id AND placement_ordinal=(o->>'ordinal')::numeric)
 THEN RAISE EXCEPTION 'invalid declared enchanting output' USING ERRCODE='23514'; END IF;
 IF t=8077 THEN
  IF (r.enchanting->>'content_digest' ~ '^[0-9a-f]{64}$') IS NOT TRUE
   OR ((o->'decay'->>'duration_millis')::bigint BETWEEN 1 AND 4294967295) IS NOT TRUE
   OR o->'decay'->'target' IS DISTINCT FROM jsonb_build_object('family','Item',
      'key','oteryn:item.tibia.i8077','revision',r.enchanting->'debits'->1->'before'->>'definition_revision_ref')
  THEN RAISE EXCEPTION 'invalid rainbow shield deadline proof' USING ERRCODE='23514'; END IF;
 ELSIF o->'decay' IS NOT NULL AND o->'decay'<>'null'::jsonb THEN
  RAISE EXCEPTION 'unexpected enchanted gem deadline' USING ERRCODE='23514';
 END IF;
END $$;

DO $migration$
DECLARE name TEXT;
BEGIN
 FOREACH name IN ARRAY ARRAY['game_item_enchanting_result(integer,integer)','game_item_enchanting_admit(game_item_craft_records)'] LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',name,current_schema());
 END LOOP;
END $migration$;

-- Reuse the absolute deadline owner for all four shield outputs, with the craft receipt
-- as same-transaction source. The completion proof also requires the schedule to exist.
DO $migration$
DECLARE body TEXT; arm TEXT; begin_at INT;
BEGIN
 body:=pg_get_functiondef('game_spell_item_temporal_schedule_proven()'::regprocedure);
 begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
 IF begin_at=0 THEN RAISE EXCEPTION 'missing original temporal mint proof'; END IF;
 arm:=$arm$
  IF EXISTS(SELECT 1 FROM game_item_craft_records c JOIN game_item_instances i
      ON i.item_instance_id=(c.outputs->0->>'item')::uuid
    WHERE c.transaction_id=NEW.source_transaction_id AND c.recipe='oteryn:enchanting.weapon'
      AND c.enchanting->'debits'->1->'before'->>'definition_production_key'='oteryn:item.tibia.i8077'
      AND c.created_xact_id=pg_current_xact_id() AND NEW.created_xact_id=c.created_xact_id
      AND i.item_instance_id=NEW.item_instance_id AND i.minted_transaction_id=c.transaction_id
      AND i.world_id=c.world_id AND i.lifecycle=1 AND i.quantity=1 AND NEW.source_ordinal=1
      AND (i.definition_family,i.definition_production_key,i.definition_revision_ref)
        =(NEW.definition_family,NEW.definition_production_key,NEW.definition_revision)
      AND (c.outputs->0->>'key',c.outputs->0->>'revision')=(NEW.definition_production_key,NEW.definition_revision)
      AND c.enchanting->>'content_digest'=encode(NEW.content_digest,'hex')
      AND NEW.expires_at_unix_ms=c.occurred_at+NEW.duration_millis
      AND c.outputs->0->'decay'=jsonb_build_object('duration_millis',NEW.duration_millis,
        'target',jsonb_build_object('family',NEW.target_family,'key',NEW.target_production_key,'revision',NEW.target_revision)))
  THEN RETURN NULL; END IF;
$arm$;
 EXECUTE overlay(body placing E'BEGIN\n'||arm from begin_at for 5);

 body:=pg_get_functiondef('game_item_enchanting_complete(game_item_craft_records)'::regprocedure);
 begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
 IF begin_at=0 THEN RAISE EXCEPTION 'missing original enchanting completion proof'; END IF;
 arm:=$arm$
  IF r.recipe='oteryn:enchanting.weapon'
   AND r.enchanting->'debits'->1->'before'->>'definition_production_key'='oteryn:item.tibia.i8077'
   AND NOT EXISTS(SELECT 1 FROM game_spell_item_temporal_schedules s
    WHERE s.item_instance_id=(r.outputs->0->>'item')::uuid AND s.source_transaction_id=r.transaction_id
      AND s.created_xact_id=r.created_xact_id)
  THEN RAISE EXCEPTION 'enchanted rainbow shield requires its original deadline' USING ERRCODE='23514'; END IF;
$arm$;
 EXECUTE overlay(body placing E'BEGIN\n'||arm from begin_at for 5);
END $migration$;
