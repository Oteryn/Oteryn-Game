-- I19: extend the existing craft owner with atomic whole-stack and two-input proofs.
ALTER TABLE game_item_craft_records ADD COLUMN enchanting JSONB;

CREATE FUNCTION game_item_enchanting_result(s INTEGER,t INTEGER) RETURNS INTEGER
LANGUAGE sql IMMUTABLE SECURITY DEFINER AS $$
 SELECT CASE
 WHEN s=3030 AND t BETWEEN 146 AND 149 THEN 676
 WHEN s=3029 AND t BETWEEN 150 AND 153 THEN 675
 WHEN s=3032 AND t BETWEEN 158 AND 161 THEN 677
 WHEN s=3033 AND t BETWEEN 154 AND 157 THEN 678
 ELSE (SELECT a[array_position(ARRAY[676,675,677,678],s)] FROM (VALUES
 (3271,ARRAY[660,679,779,794]),(7383,ARRAY[661,680,780,795]),
 (7384,ARRAY[662,681,781,796]),(7406,ARRAY[663,682,782,797]),
 (7402,ARRAY[664,683,783,798]),(3317,ARRAY[665,684,784,801]),
 (3318,ARRAY[666,685,785,802]),(7389,ARRAY[667,686,786,803]),
 (7380,ARRAY[668,687,787,804]),(3342,ARRAY[669,688,788,805]),
 (3311,ARRAY[670,689,789,806]),(3333,ARRAY[671,690,790,807]),
 (7415,ARRAY[672,691,791,808]),(7392,ARRAY[673,692,792,809]),
 (3279,ARRAY[674,693,793,810]),(3447,ARRAY[763,762,774,761]),
 (8077,ARRAY[8078,8079,8081,8080])) AS rows(id,a) WHERE id=t) END
$$;

CREATE FUNCTION game_item_enchanting_admit(r game_item_craft_records) RETURNS void
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE d JSONB; i game_item_instances%ROWTYPE; o JSONB; s INTEGER; t INTEGER;
 n BIGINT; weapon BOOLEAN; c JSONB;
BEGIN
 weapon:=r.recipe='oteryn:enchanting.weapon';
 IF r.recipe NOT IN ('oteryn:enchanting.shrine','oteryn:enchanting.weapon')
  OR NOT r.consume_source OR jsonb_typeof(r.enchanting->'debits') IS DISTINCT FROM 'array'
  OR jsonb_array_length(r.enchanting->'debits')<>(CASE WHEN weapon THEN 2 ELSE 1 END)
  OR jsonb_array_length(r.outputs)<>1
  OR (r.enchanting->'debits'->0->'before') IS DISTINCT FROM r.before_item
  OR (r.before_item->>'item_instance_id')::uuid IS DISTINCT FROM r.source_item_instance_id
  OR (r.enchanting->'debits'->0->>'parent')::uuid IS DISTINCT FROM r.parent_item_instance_id
  OR (r.enchanting->'debits'->0->>'ordinal')::numeric IS DISTINCT FROM r.source_ordinal
 THEN RAISE EXCEPTION 'invalid enchanting proof' USING ERRCODE='23514'; END IF;
 FOR d IN SELECT value FROM jsonb_array_elements(r.enchanting->'debits') LOOP
  SELECT * INTO i FROM game_item_instances WHERE item_instance_id=(d->'before'->>'item_instance_id')::uuid FOR UPDATE;
  IF i.item_instance_id IS NULL OR to_jsonb(i) IS DISTINCT FROM d->'before'
   OR i.world_id<>r.world_id OR i.definition_family<>'Item' OR i.lifecycle<>1
   OR (d->>'count')::bigint NOT BETWEEN 1 AND i.quantity
   OR EXISTS(SELECT 1 FROM game_item_keys WHERE item_instance_id=i.item_instance_id)
   OR EXISTS(SELECT 1 FROM game_item_timed_states WHERE item_instance_id=i.item_instance_id)
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
 n:=(r.before_item->>'quantity')::bigint;
 o:=r.outputs->0;
 IF weapon THEN
  t:=substring(r.enchanting->'debits'->1->'before'->>'definition_production_key' FROM '^oteryn:item.tibia.i([0-9]+)$')::integer;
  IF (r.enchanting->'debits'->0->>'count')::bigint<>1
   OR (r.enchanting->'debits'->1->>'count')::bigint IS DISTINCT FROM (o->>'count')::bigint
   OR (r.enchanting->'debits'->1->'before'->>'quantity')::bigint IS DISTINCT FROM (o->>'count')::bigint
   OR (r.enchanting->'debits'->1->'before'->>'item_instance_id')::uuid=r.source_item_instance_id
   OR ((t IN (3447,8077)) AND o->>'charges' IS NOT NULL)
   OR ((t NOT IN (3447,8077)) AND ((o->>'count')::bigint<>1 OR (o->>'charges')::integer IS DISTINCT FROM 1000))
   OR r.enchanting->'cost' IS DISTINCT FROM 'null'::jsonb
  THEN RAISE EXCEPTION 'invalid weapon enchanting debits or charges' USING ERRCODE='23514'; END IF;
 ELSE
  t:=(r.enchanting->>'shrine')::integer;
  c:=r.enchanting->'cost';
  IF (r.enchanting->'debits'->0->>'count')::bigint<>n OR (o->>'count')::bigint<>n
   OR o->>'charges' IS NOT NULL OR c->>'version' IS DISTINCT FROM '1'
   OR (c->>'mana_before')::bigint-(c->>'mana_after')::bigint IS DISTINCT FROM 300*n
   OR (c->>'soul_before')::bigint-(c->>'soul_after')::bigint IS DISTINCT FROM 2*n
   OR (c->>'mana_after')::bigint<0 OR (c->>'soul_after')::bigint<0
   OR (c->>'before_revision')::numeric+1 IS DISTINCT FROM (c->>'after_revision')::numeric
  THEN RAISE EXCEPTION 'invalid shrine resource debit' USING ERRCODE='23514'; END IF;
 END IF;
 IF s IS NULL OR t IS NULL OR game_item_enchanting_result(s,t) IS NULL
  OR o->>'key' IS DISTINCT FROM 'oteryn:item.tibia.i'||game_item_enchanting_result(s,t)
  OR NOT game_character_is_uuid_v7((o->>'item')::uuid)
  OR EXISTS(SELECT 1 FROM game_item_instances WHERE item_instance_id=(o->>'item')::uuid)
  OR (o->>'count')::bigint NOT BETWEEN 1 AND 100
  OR coalesce(o->>'revision','')='' OR (o->>'ordinal')::numeric<=0
  OR EXISTS(SELECT 1 FROM game_item_container_entries WHERE parent_item_instance_id=r.parent_item_instance_id AND placement_ordinal=(o->>'ordinal')::numeric)
 THEN RAISE EXCEPTION 'invalid declared enchanting output' USING ERRCODE='23514'; END IF;
END $$;

CREATE FUNCTION game_item_enchanting_complete(r game_item_craft_records) RETURNS void
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE d JSONB; i game_item_instances%ROWTYPE; o JSONB;
BEGIN
 IF r.created_xact_id<>pg_current_xact_id()
  OR (SELECT count(*) FROM game_item_instances WHERE minted_transaction_id=r.transaction_id)<>1
  OR (SELECT count(*) FROM game_item_instances WHERE last_transaction_id=r.transaction_id)<>jsonb_array_length(r.enchanting->'debits')
  OR (SELECT count(*) FROM game_item_audit_outbox WHERE transaction_id=r.transaction_id)<>1
  OR NOT EXISTS(SELECT 1 FROM game_item_audit_outbox WHERE event_id=r.event_id AND transaction_id=r.transaction_id
   AND item_instance_id=r.source_item_instance_id AND envelope_sha256=r.envelope_sha256 AND envelope_sha256=sha256(envelope)
   AND occurred_at=r.occurred_at AND created_xact_id=pg_current_xact_id() AND transaction_ordinal=1 AND transaction_count=1 AND publication_state=1 AND published_at IS NULL)
 THEN RAISE EXCEPTION 'incomplete enchanting transaction' USING ERRCODE='23514'; END IF;
 FOR d IN SELECT value FROM jsonb_array_elements(r.enchanting->'debits') LOOP
  SELECT * INTO i FROM game_item_instances WHERE item_instance_id=(d->'before'->>'item_instance_id')::uuid;
  IF i.item_instance_id IS NULL OR i.quantity IS DISTINCT FROM (d->'before'->>'quantity')::bigint-(d->>'count')::bigint
   OR i.lifecycle IS DISTINCT FROM (CASE WHEN i.quantity=0 THEN 2 ELSE 1 END)
   OR i.last_transaction_id IS DISTINCT FROM r.transaction_id
   OR (to_jsonb(i)-ARRAY['quantity','lifecycle','last_transaction_id']) IS DISTINCT FROM ((d->'before')-ARRAY['quantity','lifecycle','last_transaction_id'])
   OR (i.quantity=0 AND EXISTS(SELECT 1 FROM game_item_container_entries WHERE item_instance_id=i.item_instance_id))
   OR (i.quantity>0 AND NOT EXISTS(SELECT 1 FROM game_item_container_entries WHERE item_instance_id=i.item_instance_id AND character_id=r.character_id AND world_id=r.world_id AND parent_item_instance_id=(d->>'parent')::uuid AND placement_ordinal=(d->>'ordinal')::numeric))
   OR (SELECT count(*) FROM game_item_container_entries WHERE parent_item_instance_id=(d->>'parent')::uuid)>20
  THEN RAISE EXCEPTION 'enchanting debit mismatch' USING ERRCODE='23514'; END IF;
 END LOOP;
 o:=r.outputs->0;
 IF NOT EXISTS(SELECT 1 FROM game_item_instances n JOIN game_item_container_entries e USING(item_instance_id)
  WHERE n.item_instance_id=(o->>'item')::uuid AND n.world_id=r.world_id AND n.lifecycle=1 AND n.quantity=(o->>'count')::bigint
   AND n.definition_family='Item' AND n.definition_production_key=o->>'key' AND n.definition_revision_ref=o->>'revision'
   AND n.minted_transaction_id=r.transaction_id AND n.last_transaction_id IS NULL
   AND e.world_id=r.world_id AND e.character_id=r.character_id AND e.parent_item_instance_id=r.parent_item_instance_id
   AND e.placement_ordinal=(o->>'ordinal')::numeric AND e.placed_transaction_id=r.transaction_id)
 THEN RAISE EXCEPTION 'enchanting output mismatch' USING ERRCODE='23514'; END IF;
 IF o->>'charges' IS NOT NULL THEN
  IF NOT EXISTS(SELECT 1 FROM game_item_timed_states WHERE item_instance_id=(o->>'item')::uuid AND charges=1000 AND remaining_ms IS NULL AND deadline_at IS NULL AND state_revision=1)
  THEN RAISE EXCEPTION 'enchanting charge row mismatch' USING ERRCODE='23514'; END IF;
  INSERT INTO game_item_timed_definitions(definition_family,definition_production_key,definition_revision_ref,full_charges,full_remaining_ms,lit_continuous)
   VALUES('Item',o->>'key',o->>'revision',1000,NULL,false) ON CONFLICT DO NOTHING;
  IF NOT EXISTS(SELECT 1 FROM game_item_timed_definitions WHERE definition_family='Item' AND definition_production_key=o->>'key' AND definition_revision_ref=o->>'revision' AND full_charges=1000 AND full_remaining_ms IS NULL AND NOT lit_continuous)
  THEN RAISE EXCEPTION 'enchanting disagrees with timed definition' USING ERRCODE='23514'; END IF;
 ELSIF EXISTS(SELECT 1 FROM game_item_timed_states WHERE item_instance_id=(o->>'item')::uuid) THEN
  RAISE EXCEPTION 'unexpected enchanting timed row' USING ERRCODE='23514';
 END IF;
 IF r.recipe='oteryn:enchanting.shrine' AND NOT EXISTS(SELECT 1 FROM game_character_last_positions
   WHERE character_id=r.character_id AND world_id=r.world_id
    AND mana=(r.enchanting->'cost'->>'mana_after')::integer
    AND rest[1]=(r.enchanting->'cost'->>'soul_after')::integer
    AND write_sequence=(r.enchanting->>'save_sequence')::numeric
    AND character_lease_generation=(r.enchanting->>'lease_generation')::numeric)
 THEN RAISE EXCEPTION 'enchanting resource save missing' USING ERRCODE='23514'; END IF;
END $$;

-- Add closed enchanting arms ahead of the existing craft/gathering proofs.
DO $migration$
DECLARE body TEXT; name TEXT; arm TEXT; at INTEGER;
BEGIN
 FOREACH name IN ARRAY ARRAY['game_item_craft_admit()','game_item_craft_complete(uuid)',
  'game_item_instance_guard()','game_item_instance_change_proven()','game_item_container_entry_removal_proven()'] LOOP
  body:=pg_get_functiondef(name::regprocedure);
  at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
  IF at=0 THEN RAISE EXCEPTION 'missing proof body %',name; END IF;
  IF name='game_item_craft_admit()' THEN
   arm:=$arm$IF NEW.enchanting IS NOT NULL THEN PERFORM game_item_enchanting_admit(NEW); NEW.created_xact_id:=pg_current_xact_id(); RETURN NEW; END IF;
    IF NEW.recipe IN ('oteryn:enchanting.shrine','oteryn:enchanting.weapon') THEN RAISE EXCEPTION 'missing enchanting proof' USING ERRCODE='23514'; END IF;$arm$;
  ELSIF name='game_item_craft_complete(uuid)' THEN
   arm:=$arm$SELECT * INTO r FROM game_item_craft_records WHERE transaction_id=p_transaction;
    IF r.enchanting IS NOT NULL THEN PERFORM game_item_enchanting_complete(r); RETURN; END IF;$arm$;
  ELSIF name='game_item_instance_guard()' THEN
   arm:=$arm$IF TG_OP='UPDATE' AND EXISTS(SELECT 1 FROM game_item_craft_records r,LATERAL jsonb_array_elements(r.enchanting->'debits') d
    WHERE r.enchanting IS NOT NULL AND r.transaction_id=NEW.last_transaction_id AND r.created_xact_id=pg_current_xact_id()
     AND d->'before'=to_jsonb(OLD) AND NEW.quantity=OLD.quantity-(d->>'count')::bigint
     AND NEW.lifecycle=CASE WHEN NEW.quantity=0 THEN 2 ELSE 1 END
     AND (to_jsonb(NEW)-ARRAY['quantity','lifecycle','last_transaction_id'])=(to_jsonb(OLD)-ARRAY['quantity','lifecycle','last_transaction_id'])) THEN RETURN NEW; END IF;$arm$;
  ELSIF name='game_item_instance_change_proven()' THEN
   arm:=$arm$IF EXISTS(SELECT 1 FROM game_item_craft_records r,LATERAL jsonb_array_elements(r.enchanting->'debits') d
    WHERE r.enchanting IS NOT NULL AND r.transaction_id=NEW.last_transaction_id AND r.created_xact_id=pg_current_xact_id()
     AND (d->'before'->>'item_instance_id')::uuid=NEW.item_instance_id) THEN PERFORM game_item_craft_complete(NEW.last_transaction_id); RETURN NULL; END IF;$arm$;
  ELSE
   arm:=$arm$IF TG_OP='DELETE' AND EXISTS(SELECT 1 FROM game_item_craft_records r,LATERAL jsonb_array_elements(r.enchanting->'debits') d
    JOIN game_item_instances i ON i.item_instance_id=(d->'before'->>'item_instance_id')::uuid
    WHERE r.enchanting IS NOT NULL AND r.created_xact_id=pg_current_xact_id() AND i.item_instance_id=OLD.item_instance_id
     AND r.world_id=OLD.world_id AND r.character_id=OLD.character_id AND (d->>'parent')::uuid=OLD.parent_item_instance_id
     AND (d->>'ordinal')::numeric=OLD.placement_ordinal AND i.quantity=0 AND i.lifecycle=2 AND i.last_transaction_id=r.transaction_id) THEN RETURN NULL; END IF;$arm$;
  END IF;
  EXECUTE overlay(body placing E'BEGIN\n'||arm from at for 5);
 END LOOP;
 FOREACH name IN ARRAY ARRAY['game_item_enchanting_result(integer,integer)','game_item_enchanting_admit(game_item_craft_records)','game_item_enchanting_complete(game_item_craft_records)'] LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',name,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC',name);
  EXECUTE format('GRANT EXECUTE ON FUNCTION %s TO oteryn_game_runtime',name);
 END LOOP;
END $migration$;
