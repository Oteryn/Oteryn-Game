-- W19: extend the existing consume/transform receipt for donor-table event rewards.
-- Existing rust/lottery receipts retain their one-output and twenty-slot contract.
ALTER TABLE game_item_consume_transform_records
 ADD COLUMN parent_capacity INTEGER NOT NULL DEFAULT 20
 CHECK(parent_capacity BETWEEN 1 AND 500);

ALTER TABLE game_item_consume_transform_records
 DROP CONSTRAINT game_item_consume_transform_records_outputs_check;
ALTER TABLE game_item_consume_transform_records
 ADD CONSTRAINT game_item_consume_transform_records_outputs_check CHECK(
  jsonb_typeof(outputs)='array' AND jsonb_array_length(outputs) BETWEEN 0 AND
   CASE WHEN recipe LIKE 'oteryn:event-container.%' THEN 62 ELSE 1 END);
-- 62 is the maximum total quantity in seven distinct Christmas reward entries
-- (20+15+10+10+5+1+1); splitting nonstackable rewards cannot exceed that bound.
ALTER TABLE game_item_consume_transform_records
 ADD CONSTRAINT game_item_consume_transform_parent_capacity_check CHECK(
  recipe LIKE 'oteryn:event-container.%' OR parent_capacity<=20);

-- Keep the previous guard chain and every existing proof arm, changing only the
-- receipt-owned parent capacity. The writer resolves it from the active Item policy.
DO $migration$
DECLARE body TEXT; original TEXT;
BEGIN
 body:=pg_get_functiondef('game_item_consume_transform_complete(uuid)'::regprocedure);
 original:='(SELECT count(*) FROM game_item_container_entries WHERE parent_item_instance_id=r.parent_item_instance_id)>20';
 IF position(original IN body)=0 THEN
  RAISE EXCEPTION 'unexpected consume-transform capacity proof';
 END IF;
 EXECUTE replace(body,original,
  '(SELECT count(*) FROM game_item_container_entries WHERE parent_item_instance_id=r.parent_item_instance_id)>r.parent_capacity');
END $migration$;

-- W19: immutable semantic outcomes on the existing consume/transform receipt.
-- No second Item store, timer, achievement catalogue or presentation queue.
ALTER TABLE game_item_consume_transform_records
 ADD COLUMN consume_source BOOLEAN NOT NULL DEFAULT TRUE,
 ADD COLUMN effects JSONB NOT NULL DEFAULT '{}'::jsonb
   CHECK(jsonb_typeof(effects)='object' AND octet_length(effects::text)<=16384),
 ADD COLUMN extra_inputs JSONB NOT NULL DEFAULT '[]'::jsonb
   CHECK(jsonb_typeof(extra_inputs)='array' AND jsonb_array_length(extra_inputs)<=5),
 ADD COLUMN source_kind INTEGER NOT NULL DEFAULT 1 CHECK(source_kind IN(1,2)),
 ADD COLUMN content_digest BYTEA CHECK(content_digest IS NULL OR octet_length(content_digest)=32),
 ADD COLUMN receipt_sequence BIGINT GENERATED ALWAYS AS IDENTITY UNIQUE,
 ALTER COLUMN parent_item_instance_id DROP NOT NULL;
ALTER TABLE game_item_consume_transform_records ADD CONSTRAINT game_item_action_source_custody CHECK(
 (source_kind=1 AND parent_item_instance_id IS NOT NULL)
 OR (source_kind=2 AND parent_item_instance_id IS NULL AND NOT consume_source
     AND source_ordinal=1 AND effects->>'require_head'='true'));
ALTER TABLE game_item_consume_transform_records ADD CONSTRAINT game_item_action_no_consume_shape CHECK(
 consume_source OR (jsonb_array_length(outputs)=0 AND jsonb_array_length(extra_inputs)=0
     AND secondary_item_instance_id=source_item_instance_id));
ALTER TABLE game_item_consume_transform_records DROP CONSTRAINT game_item_consume_transform_parent_capacity_check;
ALTER TABLE game_item_consume_transform_records ADD CONSTRAINT game_item_consume_transform_parent_capacity_check CHECK(
 recipe LIKE 'oteryn:event-container.%' OR recipe LIKE 'oteryn:novelty-item.%'
 OR recipe LIKE 'oteryn:item-cosmetic.%' OR parent_capacity<=20);
ALTER TABLE game_item_consume_transform_records ADD CONSTRAINT game_item_action_head_effect CHECK(
 (effects->>'require_head') IS DISTINCT FROM 'true' OR source_kind=2);
CREATE INDEX game_item_action_dice_history ON game_item_consume_transform_records(character_id,receipt_sequence DESC)
 WHERE effects->>'kind'='dice';

-- Additional inputs use exactly the existing plain main-backpack custody contract.
CREATE FUNCTION game_item_action_extra_admit(p_record game_item_consume_transform_records) RETURNS void LANGUAGE plpgsql AS $$
DECLARE e JSONB; i game_item_instances%ROWTYPE;
BEGIN
 IF NOT p_record.consume_source AND jsonb_array_length(p_record.extra_inputs)>0 THEN
  RAISE EXCEPTION 'non-consuming action cannot spend additional inputs' USING ERRCODE='23514'; END IF;
 IF (SELECT count(DISTINCT value->>'item') FROM jsonb_array_elements(p_record.extra_inputs))<>jsonb_array_length(p_record.extra_inputs) THEN
  RAISE EXCEPTION 'duplicate additional action input' USING ERRCODE='23514'; END IF;
 FOR e IN SELECT value FROM jsonb_array_elements(p_record.extra_inputs) LOOP
  SELECT * INTO i FROM game_item_instances WHERE item_instance_id=(e->>'item')::uuid FOR UPDATE;
  IF i.item_instance_id IS NULL OR i.item_instance_id IN(p_record.source_item_instance_id,p_record.secondary_item_instance_id)
    OR to_jsonb(i) IS DISTINCT FROM e->'before' OR i.lifecycle<>1 OR i.quantity<1
    OR i.world_id<>p_record.world_id OR i.definition_family<>'Item'
    OR EXISTS(SELECT 1 FROM game_item_keys WHERE item_instance_id=i.item_instance_id)
    OR EXISTS(SELECT 1 FROM game_item_timed_states WHERE item_instance_id=i.item_instance_id)
    OR EXISTS(SELECT 1 FROM game_item_texts WHERE item_instance_id=i.item_instance_id)
    OR EXISTS(SELECT 1 FROM game_item_imbuements WHERE item_instance_id=i.item_instance_id)
    OR EXISTS(SELECT 1 FROM game_item_container_entries WHERE parent_item_instance_id=i.item_instance_id)
    OR EXISTS(SELECT 1 FROM game_item_nested_entries WHERE parent_item_instance_id=i.item_instance_id)
    OR NOT EXISTS(SELECT 1 FROM game_item_container_entries c JOIN game_item_container_slots b
       ON b.character_id=c.character_id AND b.item_instance_id=c.parent_item_instance_id
       WHERE c.item_instance_id=i.item_instance_id AND c.character_id=p_record.character_id AND c.world_id=p_record.world_id
        AND b.world_id=p_record.world_id AND c.parent_item_instance_id=(e->>'parent')::uuid
        AND c.placement_ordinal=(e->>'ordinal')::numeric)
   THEN RAISE EXCEPTION 'additional action input lacks exact plain carried predecessor' USING ERRCODE='23514'; END IF;
 END LOOP;
END $$;

CREATE FUNCTION game_item_action_extra_complete(p_record game_item_consume_transform_records) RETURNS void LANGUAGE plpgsql AS $$
DECLARE e JSONB; i game_item_instances%ROWTYPE;
BEGIN
 FOR e IN SELECT value FROM jsonb_array_elements(p_record.extra_inputs) LOOP
  SELECT * INTO i FROM game_item_instances WHERE item_instance_id=(e->>'item')::uuid;
  IF i.item_instance_id IS NULL OR i.quantity<>(e->'before'->>'quantity')::bigint-1
    OR i.lifecycle<>(CASE WHEN i.quantity=0 THEN 2 ELSE 1 END) OR i.last_transaction_id IS DISTINCT FROM p_record.transaction_id
    OR (to_jsonb(i)-ARRAY['quantity','lifecycle','last_transaction_id']) IS DISTINCT FROM
       ((e->'before')-ARRAY['quantity','lifecycle','last_transaction_id'])
    OR (i.quantity=0 AND EXISTS(SELECT 1 FROM game_item_container_entries WHERE item_instance_id=i.item_instance_id))
    OR (i.quantity>0 AND NOT EXISTS(SELECT 1 FROM game_item_container_entries WHERE item_instance_id=i.item_instance_id
        AND character_id=p_record.character_id AND world_id=p_record.world_id
        AND parent_item_instance_id=(e->>'parent')::uuid AND placement_ordinal=(e->>'ordinal')::numeric))
   THEN RAISE EXCEPTION 'additional action input lacks exact decrement and custody' USING ERRCODE='23514'; END IF;
 END LOOP;
END $$;

-- Append branches to the existing proof chain, preserving prior rules verbatim.
DO $migration$
DECLARE body TEXT; arm TEXT; original TEXT; begin_at INT;
BEGIN
 body:=pg_get_functiondef('game_item_consume_transform_admit()'::regprocedure);
 begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
 IF begin_at=0 THEN RAISE EXCEPTION 'missing action admission body'; END IF;
 arm:=$arm$
  PERFORM game_item_action_extra_admit(NEW);
  IF NEW.effects<>'{}'::jsonb AND NEW.content_digest IS NULL THEN
   RAISE EXCEPTION 'semantic action requires original content digest' USING ERRCODE='23514'; END IF;
  IF NOT NEW.consume_source THEN
   SELECT * INTO i FROM game_item_instances WHERE item_instance_id=NEW.source_item_instance_id FOR UPDATE;
   IF i.item_instance_id IS NULL OR to_jsonb(i) IS DISTINCT FROM NEW.before_item OR i.lifecycle<>1 OR i.quantity<1
     OR i.world_id<>NEW.world_id OR i.definition_family<>'Item' OR NEW.before_secondary IS DISTINCT FROM NEW.before_item
     OR EXISTS(SELECT 1 FROM game_item_keys WHERE item_instance_id=i.item_instance_id)
     OR EXISTS(SELECT 1 FROM game_item_timed_states WHERE item_instance_id=i.item_instance_id)
     OR EXISTS(SELECT 1 FROM game_item_texts WHERE item_instance_id=i.item_instance_id)
     OR EXISTS(SELECT 1 FROM game_item_imbuements WHERE item_instance_id=i.item_instance_id)
     OR EXISTS(SELECT 1 FROM game_item_container_entries WHERE parent_item_instance_id=i.item_instance_id)
     OR EXISTS(SELECT 1 FROM game_item_nested_entries WHERE parent_item_instance_id=i.item_instance_id)
     OR NOT ((NEW.source_kind=1 AND EXISTS(SELECT 1 FROM game_item_container_entries e JOIN game_item_container_slots b
          ON b.character_id=e.character_id AND b.item_instance_id=e.parent_item_instance_id
          WHERE e.item_instance_id=i.item_instance_id AND e.character_id=NEW.character_id AND e.world_id=NEW.world_id AND b.world_id=NEW.world_id
           AND e.parent_item_instance_id=NEW.parent_item_instance_id AND e.placement_ordinal=NEW.source_ordinal))
       OR (NEW.source_kind=2 AND i.definition_production_key='oteryn:item.tibia.i6578' AND i.quantity=1
          AND NEW.effects->>'require_head'='true' AND EXISTS(SELECT 1 FROM game_character_equipment_slots e
          WHERE e.item_instance_id=i.item_instance_id AND e.character_id=NEW.character_id AND e.world_id=NEW.world_id AND e.slot=1)))
    THEN RAISE EXCEPTION 'effect-only action requires unchanged exact source custody' USING ERRCODE='23514'; END IF;
   NEW.created_xact_id:=pg_current_xact_id();
   RETURN NEW;
  END IF;
$arm$;
 EXECUTE overlay(body placing E'BEGIN\n'||arm from begin_at for 5);
 body:=pg_get_functiondef('game_item_consume_transform_complete(uuid)'::regprocedure);
 original:='(CASE WHEN r.secondary_item_instance_id=r.source_item_instance_id THEN 1 ELSE 2 END)';
 IF position(original IN body)=0 THEN RAISE EXCEPTION 'missing action input count proof'; END IF;
 body:=replace(body,original,'((CASE WHEN r.secondary_item_instance_id=r.source_item_instance_id THEN 1 ELSE 2 END)+jsonb_array_length(r.extra_inputs))');
 begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
 arm:=$arm$
  SELECT * INTO r FROM game_item_consume_transform_records WHERE transaction_id=p_transaction;
  IF r.transaction_id IS NULL OR r.created_xact_id<>pg_current_xact_id() THEN
   RAISE EXCEPTION 'action proof requires its original transaction' USING ERRCODE='23514'; END IF;
  PERFORM game_item_action_extra_complete(r);
  IF NOT r.consume_source THEN
   SELECT * INTO i FROM game_item_instances WHERE item_instance_id=r.source_item_instance_id;
   IF to_jsonb(i) IS DISTINCT FROM r.before_item
     OR EXISTS(SELECT 1 FROM game_item_instances WHERE minted_transaction_id=r.transaction_id OR last_transaction_id=r.transaction_id)
     OR NOT ((r.source_kind=1 AND EXISTS(SELECT 1 FROM game_item_container_entries e JOIN game_item_container_slots b
          ON b.character_id=e.character_id AND b.item_instance_id=e.parent_item_instance_id
          WHERE e.item_instance_id=i.item_instance_id AND e.character_id=r.character_id AND e.world_id=r.world_id AND b.world_id=r.world_id
           AND e.parent_item_instance_id=r.parent_item_instance_id AND e.placement_ordinal=r.source_ordinal))
       OR (r.source_kind=2 AND EXISTS(SELECT 1 FROM game_character_equipment_slots e
          WHERE e.item_instance_id=i.item_instance_id AND e.character_id=r.character_id AND e.world_id=r.world_id AND e.slot=1)))
     OR (SELECT count(*) FROM game_item_audit_outbox WHERE transaction_id=r.transaction_id)<>1
     OR NOT EXISTS(SELECT 1 FROM game_item_audit_outbox WHERE event_id=r.event_id AND transaction_id=r.transaction_id
          AND item_instance_id=r.source_item_instance_id AND occurred_at=r.occurred_at AND envelope_sha256=r.envelope_sha256
          AND created_xact_id=pg_current_xact_id() AND transaction_ordinal=1 AND transaction_count=1
          AND publication_state=1 AND published_at IS NULL)
    THEN RAISE EXCEPTION 'effect-only action mutated Item state or lacks exact audit' USING ERRCODE='23514'; END IF;
   RETURN;
  END IF;
  FOR o IN SELECT value FROM jsonb_array_elements(r.outputs) LOOP
   IF o->'decay' IS NOT NULL AND o->'decay'<>'null'::jsonb THEN
    IF NOT EXISTS(SELECT 1 FROM game_spell_item_temporal_schedules p WHERE p.item_instance_id=(o->>'item')::uuid
       AND p.source_transaction_id=r.transaction_id AND p.created_xact_id=r.created_xact_id) THEN
     RAISE EXCEPTION 'timed action output lacks original deadline' USING ERRCODE='23514'; END IF;
   ELSIF EXISTS(SELECT 1 FROM game_spell_item_temporal_schedules p WHERE p.item_instance_id=(o->>'item')::uuid) THEN
    RAISE EXCEPTION 'untimed action output acquired fabricated deadline' USING ERRCODE='23514'; END IF;
  END LOOP;
$arm$;
 EXECUTE overlay(body placing E'BEGIN\n'||arm from begin_at for 5);
END $migration$;

-- Additional consumed pieces reuse the normal quantity/custody proof functions.
DO $migration$
DECLARE n TEXT; body TEXT; arm TEXT; begin_at INT;
BEGIN
 FOREACH n IN ARRAY ARRAY['game_item_instance_guard','game_item_instance_change_proven','game_item_container_entry_removal_proven'] LOOP
  body:=pg_get_functiondef((n||'()')::regprocedure);
  begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
  IF begin_at=0 THEN RAISE EXCEPTION 'missing additional action proof: %',n; END IF;
  IF n='game_item_instance_guard' THEN
   arm:=$arm$
    IF TG_OP='UPDATE' AND EXISTS(SELECT 1 FROM game_item_consume_transform_records r,LATERAL jsonb_array_elements(r.extra_inputs) e
      WHERE r.consume_source AND (e->>'item')::uuid=OLD.item_instance_id AND e->'before'=to_jsonb(OLD)
       AND r.transaction_id=NEW.last_transaction_id AND r.created_xact_id=pg_current_xact_id()) THEN
     IF NEW.quantity<>OLD.quantity-1 OR NEW.lifecycle<>(CASE WHEN NEW.quantity=0 THEN 2 ELSE 1 END)
       OR (to_jsonb(NEW)-ARRAY['quantity','lifecycle','last_transaction_id']) IS DISTINCT FROM
          (to_jsonb(OLD)-ARRAY['quantity','lifecycle','last_transaction_id']) THEN
      RAISE EXCEPTION 'additional action decrements exactly one input unit' USING ERRCODE='23514'; END IF;
     RETURN NEW;
    END IF;
$arm$;
  ELSIF n='game_item_instance_change_proven' THEN
   arm:=$arm$
    IF EXISTS(SELECT 1 FROM game_item_consume_transform_records r,LATERAL jsonb_array_elements(r.extra_inputs) e
      WHERE (e->>'item')::uuid=NEW.item_instance_id AND r.transaction_id=NEW.last_transaction_id
       AND r.created_xact_id=pg_current_xact_id()) THEN
     PERFORM game_item_consume_transform_complete(NEW.last_transaction_id); RETURN NULL;
    END IF;
$arm$;
  ELSE
   arm:=$arm$
    IF TG_OP='DELETE' AND EXISTS(SELECT 1 FROM game_item_consume_transform_records r,LATERAL jsonb_array_elements(r.extra_inputs) e,
      game_item_instances i WHERE (e->>'item')::uuid=OLD.item_instance_id AND i.item_instance_id=OLD.item_instance_id
       AND r.character_id=OLD.character_id AND r.world_id=OLD.world_id AND (e->>'parent')::uuid=OLD.parent_item_instance_id
       AND (e->>'ordinal')::numeric=OLD.placement_ordinal AND i.quantity=0 AND i.lifecycle=2
       AND i.last_transaction_id=r.transaction_id AND r.created_xact_id=pg_current_xact_id()) THEN RETURN NULL; END IF;
$arm$;
  END IF;
  EXECUTE overlay(body placing E'BEGIN\n'||arm from begin_at for 5);
 END LOOP;
END $migration$;

DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_item_action_extra_admit(game_item_consume_transform_records) SET search_path=pg_catalog,%I,pg_temp',current_schema());
 EXECUTE format('ALTER FUNCTION game_item_action_extra_complete(game_item_consume_transform_records) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_item_action_extra_admit(game_item_consume_transform_records),game_item_action_extra_complete(game_item_consume_transform_records) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_item_action_extra_admit(game_item_consume_transform_records),game_item_action_extra_complete(game_item_consume_transform_records) TO oteryn_game_runtime;
-- W19 ground toys and nested carried leaves use the same immutable action owner.
ALTER TABLE game_item_consume_transform_records
 ADD COLUMN source_location JSONB NOT NULL DEFAULT '{}'::jsonb CHECK(jsonb_typeof(source_location)='object'),
 DROP CONSTRAINT game_item_consume_transform_records_source_kind_check,
 ADD CONSTRAINT game_item_consume_transform_records_source_kind_check CHECK(source_kind BETWEEN 1 AND 4),
 DROP CONSTRAINT game_item_action_source_custody,
 ADD CONSTRAINT game_item_action_source_custody CHECK(
  (source_kind IN(1,4) AND parent_item_instance_id IS NOT NULL)
  OR (source_kind=2 AND parent_item_instance_id IS NULL AND NOT consume_source AND source_ordinal=1 AND effects->>'require_head'='true')
  OR (source_kind=3 AND parent_item_instance_id IS NULL AND consume_source
     AND jsonb_array_length(extra_inputs)=0 AND secondary_item_instance_id=source_item_instance_id
     AND effects->>'kind' IN('dice','firework','transform','house_transform')
     AND source_location ? 'stack_ordinal'));

CREATE FUNCTION game_item_action_custody_current(r game_item_consume_transform_records) RETURNS BOOLEAN LANGUAGE sql STABLE AS $$
 SELECT CASE r.source_kind
  WHEN 3 THEN EXISTS(SELECT 1 FROM game_item_ground_locations g WHERE g.item_instance_id=r.source_item_instance_id
     AND to_jsonb(g)=r.source_location-'qualified_house')
  WHEN 4 THEN EXISTS(SELECT 1 FROM game_item_nested_entries n,LATERAL game_item_loop_owner(n.parent_item_instance_id) owner
     WHERE n.item_instance_id=r.source_item_instance_id AND n.world_id=r.world_id AND n.parent_item_instance_id=r.parent_item_instance_id
      AND n.placement_ordinal=r.source_ordinal AND to_jsonb(n)=r.source_location
      AND owner.owner_character=r.character_id AND owner.depth BETWEEN 0 AND 7)
  ELSE FALSE END
$$;
CREATE FUNCTION game_item_action_extended_admit(r game_item_consume_transform_records) RETURNS void LANGUAGE plpgsql AS $$
DECLARE i game_item_instances%ROWTYPE; s game_item_instances%ROWTYPE; o JSONB; highest NUMERIC;
BEGIN
 SELECT * INTO i FROM game_item_instances WHERE item_instance_id=r.source_item_instance_id FOR UPDATE;
 IF i.item_instance_id IS NULL OR to_jsonb(i) IS DISTINCT FROM r.before_item OR i.lifecycle<>1 OR i.quantity<1
    OR i.world_id<>r.world_id OR i.definition_family<>'Item' OR r.content_digest IS NULL
    OR NOT game_item_action_custody_current(r)
    OR EXISTS(SELECT 1 FROM game_item_keys WHERE item_instance_id=i.item_instance_id)
    OR EXISTS(SELECT 1 FROM game_item_timed_states WHERE item_instance_id=i.item_instance_id)
    OR EXISTS(SELECT 1 FROM game_item_texts WHERE item_instance_id=i.item_instance_id)
    OR EXISTS(SELECT 1 FROM game_item_imbuements WHERE item_instance_id=i.item_instance_id)
    OR EXISTS(SELECT 1 FROM game_item_container_entries WHERE parent_item_instance_id=i.item_instance_id)
    OR EXISTS(SELECT 1 FROM game_item_nested_entries WHERE parent_item_instance_id=i.item_instance_id)
 THEN RAISE EXCEPTION 'extended action requires exact plain source and current custody' USING ERRCODE='23514'; END IF;
 IF r.source_kind=3 THEN
  IF (r.source_location->>'world_id')::uuid<>r.world_id
    OR (r.source_location->>'stack_ordinal')::numeric<>r.source_ordinal
    OR NOT EXISTS(SELECT 1 FROM game_durability_reconnect_sessions sess
       WHERE sess.game_session_id=r.game_session_id AND sess.character_id=r.character_id AND sess.session_state=2
        AND sess.runtime_scope_kind=1 AND sess.runtime_scope_world_id=r.world_id
        AND sess.runtime_scope_channel_id=(r.source_location->>'channel_id')::uuid
        AND sess.scope_ownership_generation=(r.source_location->>'runtime_scope_ownership_generation')::numeric)
    OR (r.effects->>'kind'='house_transform' AND (r.source_location->>'qualified_house') IS NULL)
  THEN RAISE EXCEPTION 'ground action requires current source Channel and qualified House membership' USING ERRCODE='23514'; END IF;
 END IF;
 PERFORM game_item_action_extra_admit(r);
 IF r.secondary_item_instance_id<>r.source_item_instance_id THEN
  SELECT * INTO s FROM game_item_instances WHERE item_instance_id=r.secondary_item_instance_id FOR UPDATE;
  IF r.source_kind<>4 OR to_jsonb(s) IS DISTINCT FROM r.before_secondary OR s.lifecycle<>1 OR s.quantity<1
    OR s.world_id<>r.world_id OR s.definition_family<>'Item'
    OR EXISTS(SELECT 1 FROM game_item_keys WHERE item_instance_id=s.item_instance_id)
    OR EXISTS(SELECT 1 FROM game_item_timed_states WHERE item_instance_id=s.item_instance_id)
    OR EXISTS(SELECT 1 FROM game_item_texts WHERE item_instance_id=s.item_instance_id)
    OR EXISTS(SELECT 1 FROM game_item_imbuements WHERE item_instance_id=s.item_instance_id)
    OR EXISTS(SELECT 1 FROM game_item_container_entries WHERE parent_item_instance_id=s.item_instance_id)
    OR EXISTS(SELECT 1 FROM game_item_nested_entries WHERE parent_item_instance_id=s.item_instance_id)
    OR NOT EXISTS(SELECT 1 FROM game_item_container_entries e JOIN game_item_container_slots b
        ON b.character_id=e.character_id AND b.item_instance_id=e.parent_item_instance_id
        WHERE e.item_instance_id=s.item_instance_id AND e.character_id=r.character_id AND e.world_id=r.world_id AND b.world_id=r.world_id)
  THEN RAISE EXCEPTION 'nested action secondary requires plain main-backpack source' USING ERRCODE='23514'; END IF;
 ELSIF r.before_secondary IS DISTINCT FROM r.before_item THEN
  RAISE EXCEPTION 'action source predecessor mismatch' USING ERRCODE='23514';
 END IF;
 IF (SELECT count(DISTINCT value->>'item') FROM jsonb_array_elements(r.outputs))<>jsonb_array_length(r.outputs)
  OR (SELECT count(DISTINCT value->>'ordinal') FROM jsonb_array_elements(r.outputs))<>jsonb_array_length(r.outputs) THEN
  RAISE EXCEPTION 'duplicate extended action output' USING ERRCODE='23514'; END IF;
 SELECT coalesce(max(placement_ordinal),0) INTO highest FROM game_item_nested_entries WHERE parent_item_instance_id=r.parent_item_instance_id;
 FOR o IN SELECT value FROM jsonb_array_elements(r.outputs) LOOP
  IF NOT game_character_is_uuid_v7((o->>'item')::uuid)
    OR EXISTS(SELECT 1 FROM game_item_instances WHERE item_instance_id=(o->>'item')::uuid)
    OR (o->>'count')::bigint NOT BETWEEN 1 AND 100
    OR octet_length(o->>'key') NOT BETWEEN 1 AND 512 OR octet_length(o->>'revision') NOT BETWEEN 1 AND 512
    OR (r.source_kind=3 AND (jsonb_array_length(r.outputs)<>1 OR i.quantity<>1 OR (o->>'count')::bigint<>1 OR (o->>'ordinal')::numeric<>r.source_ordinal))
    OR (r.source_kind=4 AND ((o->>'ordinal')::numeric<=highest
        AND NOT (r.consume_source AND i.quantity=r.source_count AND o=r.outputs->0 AND (o->>'ordinal')::numeric=r.source_ordinal)))
   THEN RAISE EXCEPTION 'extended action output is not exact fresh source replacement' USING ERRCODE='23514'; END IF;
 END LOOP;
END $$;

CREATE FUNCTION game_item_action_extended_complete(r game_item_consume_transform_records) RETURNS void LANGUAGE plpgsql AS $$
DECLARE i game_item_instances%ROWTYPE; s game_item_instances%ROWTYPE; o JSONB; expected BIGINT;
BEGIN
 SELECT * INTO i FROM game_item_instances WHERE item_instance_id=r.source_item_instance_id;
 expected:=(r.before_item->>'quantity')::bigint-r.source_count;
 IF r.created_xact_id<>pg_current_xact_id() OR i.item_instance_id IS NULL OR i.quantity<>expected
  OR i.lifecycle<>(CASE WHEN expected=0 THEN 2 ELSE 1 END)
  OR (r.consume_source AND (i.last_transaction_id IS DISTINCT FROM r.transaction_id
      OR (to_jsonb(i)-ARRAY['quantity','lifecycle','last_transaction_id']) IS DISTINCT FROM
         (r.before_item-ARRAY['quantity','lifecycle','last_transaction_id'])))
  OR (NOT r.consume_source AND to_jsonb(i) IS DISTINCT FROM r.before_item)
  OR (expected>0 AND NOT game_item_action_custody_current(r))
  OR (expected=0 AND (EXISTS(SELECT 1 FROM game_item_ground_locations WHERE item_instance_id=i.item_instance_id)
                  OR EXISTS(SELECT 1 FROM game_item_nested_entries WHERE item_instance_id=i.item_instance_id)))
  OR (SELECT count(*) FROM game_item_instances WHERE minted_transaction_id=r.transaction_id)<>jsonb_array_length(r.outputs)
  OR (SELECT count(*) FROM game_item_instances WHERE last_transaction_id=r.transaction_id)<>
      (CASE WHEN NOT r.consume_source THEN 0 WHEN r.secondary_item_instance_id=r.source_item_instance_id THEN 1 ELSE 2 END)+jsonb_array_length(r.extra_inputs)
  OR (r.source_kind=4 AND r.consume_source AND (SELECT count(*) FROM game_item_nested_entries WHERE parent_item_instance_id=r.parent_item_instance_id)>r.parent_capacity)
  OR (SELECT count(*) FROM game_item_audit_outbox WHERE transaction_id=r.transaction_id)<>1
  OR NOT EXISTS(SELECT 1 FROM game_item_audit_outbox WHERE event_id=r.event_id AND transaction_id=r.transaction_id
       AND item_instance_id=r.source_item_instance_id AND occurred_at=r.occurred_at AND envelope_sha256=r.envelope_sha256
       AND created_xact_id=pg_current_xact_id() AND transaction_ordinal=1 AND transaction_count=1
       AND publication_state=1 AND published_at IS NULL)
 THEN RAISE EXCEPTION 'extended action lacks atomic input,custody,output or audit' USING ERRCODE='23514'; END IF;
 IF r.secondary_item_instance_id<>r.source_item_instance_id THEN
  SELECT * INTO s FROM game_item_instances WHERE item_instance_id=r.secondary_item_instance_id;
  IF s.quantity<>(r.before_secondary->>'quantity')::bigint-1 OR s.lifecycle<>(CASE WHEN s.quantity=0 THEN 2 ELSE 1 END)
    OR s.last_transaction_id IS DISTINCT FROM r.transaction_id
    OR (to_jsonb(s)-ARRAY['quantity','lifecycle','last_transaction_id']) IS DISTINCT FROM
       (r.before_secondary-ARRAY['quantity','lifecycle','last_transaction_id'])
    OR (s.quantity=0 AND EXISTS(SELECT 1 FROM game_item_container_entries WHERE item_instance_id=s.item_instance_id))
   THEN RAISE EXCEPTION 'nested action secondary decrement mismatch' USING ERRCODE='23514'; END IF;
 END IF;
 PERFORM game_item_action_extra_complete(r);
 FOR o IN SELECT value FROM jsonb_array_elements(r.outputs) LOOP
  IF NOT EXISTS(SELECT 1 FROM game_item_instances n WHERE n.item_instance_id=(o->>'item')::uuid AND n.world_id=r.world_id
      AND n.quantity=(o->>'count')::bigint AND n.lifecycle=1 AND n.definition_family='Item'
      AND n.definition_production_key=o->>'key' AND n.definition_revision_ref=o->>'revision'
      AND n.minted_transaction_id=r.transaction_id AND n.last_transaction_id IS NULL)
    OR (r.source_kind=3 AND NOT EXISTS(SELECT 1 FROM game_item_ground_locations g WHERE g.item_instance_id=(o->>'item')::uuid
        AND (to_jsonb(g)-'item_instance_id')=(r.source_location-ARRAY['item_instance_id','qualified_house'])
        AND g.stack_ordinal=(o->>'ordinal')::bigint))
    OR (r.source_kind=4 AND NOT EXISTS(SELECT 1 FROM game_item_nested_entries n,LATERAL game_item_loop_owner(n.parent_item_instance_id) owner
        WHERE n.item_instance_id=(o->>'item')::uuid AND n.world_id=r.world_id AND n.parent_item_instance_id=r.parent_item_instance_id
          AND n.placement_ordinal=(o->>'ordinal')::numeric AND n.placed_transaction_id=r.transaction_id
          AND owner.owner_character=r.character_id AND owner.depth BETWEEN 0 AND 7))
   THEN RAISE EXCEPTION 'extended action output or exact custody mismatch' USING ERRCODE='23514'; END IF;
  IF o->'decay' IS NOT NULL AND o->'decay'<>'null'::jsonb THEN
   IF NOT EXISTS(SELECT 1 FROM game_spell_item_temporal_schedules p WHERE p.item_instance_id=(o->>'item')::uuid
      AND p.source_transaction_id=r.transaction_id AND p.created_xact_id=r.created_xact_id) THEN
    RAISE EXCEPTION 'timed extended action output lacks original deadline' USING ERRCODE='23514'; END IF;
  ELSIF EXISTS(SELECT 1 FROM game_spell_item_temporal_schedules WHERE item_instance_id=(o->>'item')::uuid) THEN
   RAISE EXCEPTION 'untimed extended action output acquired fabricated deadline' USING ERRCODE='23514'; END IF;
 END LOOP;
END $$;

DO $migration$
DECLARE n TEXT; body TEXT; arm TEXT; begin_at INT;
BEGIN
 FOREACH n IN ARRAY ARRAY['game_item_consume_transform_admit','game_item_consume_transform_complete',
   'game_item_nested_entry_proven','game_item_ground_removal_proven','game_item_ground_insertion_guard','game_item_ground_stack_stamp'] LOOP
  body:=pg_get_functiondef((n||CASE WHEN n='game_item_consume_transform_complete' THEN '(uuid)' ELSE '()' END)::regprocedure);
  begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
  IF begin_at=0 THEN RAISE EXCEPTION 'missing extended action proof: %',n; END IF;
  IF n='game_item_consume_transform_admit' THEN
   arm:=$arm$
    IF NEW.source_kind IN(3,4) THEN
     PERFORM game_item_action_extended_admit(NEW); NEW.created_xact_id:=pg_current_xact_id(); RETURN NEW;
    END IF;
$arm$;
  ELSIF n='game_item_consume_transform_complete' THEN
   arm:=$arm$
    SELECT * INTO r FROM game_item_consume_transform_records WHERE transaction_id=p_transaction;
    IF r.source_kind IN(3,4) THEN PERFORM game_item_action_extended_complete(r); RETURN; END IF;
$arm$;
  ELSIF n='game_item_nested_entry_proven' THEN
   arm:=$arm$
    IF TG_OP='DELETE' AND EXISTS(SELECT 1 FROM game_item_consume_transform_records r JOIN game_item_instances i
       ON i.item_instance_id=r.source_item_instance_id WHERE r.source_kind=4 AND r.source_item_instance_id=OLD.item_instance_id
        AND r.source_location=to_jsonb(OLD) AND r.created_xact_id=pg_current_xact_id()
        AND i.last_transaction_id=r.transaction_id AND i.quantity=0 AND i.lifecycle=2) THEN RETURN NULL; END IF;
    IF TG_OP='INSERT' AND EXISTS(SELECT 1 FROM game_item_consume_transform_records r,LATERAL jsonb_array_elements(r.outputs) o
       WHERE r.source_kind=4 AND (o->>'item')::uuid=NEW.item_instance_id AND (o->>'ordinal')::numeric=NEW.placement_ordinal
        AND r.parent_item_instance_id=NEW.parent_item_instance_id AND r.world_id=NEW.world_id
        AND r.transaction_id=NEW.placed_transaction_id AND r.created_xact_id=pg_current_xact_id()) THEN RETURN NULL; END IF;
$arm$;
  ELSIF n='game_item_ground_removal_proven' THEN
   arm:=$arm$
    IF EXISTS(SELECT 1 FROM game_item_consume_transform_records r JOIN game_item_instances i ON i.item_instance_id=r.source_item_instance_id
       WHERE r.source_kind=3 AND r.source_item_instance_id=OLD.item_instance_id AND r.source_location-'qualified_house'=to_jsonb(OLD)
        AND i.last_transaction_id=r.transaction_id AND i.lifecycle=2 AND i.quantity=0 AND r.created_xact_id=pg_current_xact_id()) THEN RETURN NULL; END IF;
$arm$;
  ELSIF n='game_item_ground_insertion_guard' THEN
   arm:=$arm$
    IF EXISTS(SELECT 1 FROM game_item_consume_transform_records r,LATERAL jsonb_array_elements(r.outputs) o
       WHERE r.source_kind=3 AND (o->>'item')::uuid=NEW.item_instance_id
         AND (to_jsonb(NEW)-'item_instance_id')=(r.source_location-ARRAY['item_instance_id','qualified_house'])
         AND (o->>'ordinal')::bigint=NEW.stack_ordinal AND r.created_xact_id=pg_current_xact_id()) THEN RETURN NULL; END IF;
$arm$;
  ELSE
   arm:=$arm$
    -- Only exact source replacement keeps its old owner-assigned order. Ordinary
    -- new Ground mints still acquire the existing sequence's append order.
    IF NEW.stack_ordinal IS NOT NULL AND EXISTS(SELECT 1 FROM game_item_consume_transform_records r,LATERAL jsonb_array_elements(r.outputs) o
       WHERE r.source_kind=3 AND (o->>'item')::uuid=NEW.item_instance_id AND (o->>'ordinal')::bigint=NEW.stack_ordinal
        AND r.source_ordinal=NEW.stack_ordinal AND r.created_xact_id=pg_current_xact_id()
        AND NOT EXISTS(SELECT 1 FROM game_item_ground_locations WHERE stack_ordinal=NEW.stack_ordinal)) THEN RETURN NEW; END IF;
$arm$;
  END IF;
  EXECUTE overlay(body placing E'BEGIN\n'||arm from begin_at for 5);
 END LOOP;
END $migration$;
DO $$ DECLARE name TEXT; BEGIN
 FOREACH name IN ARRAY ARRAY['game_item_action_custody_current','game_item_action_extended_admit','game_item_action_extended_complete'] LOOP
  EXECUTE format('ALTER FUNCTION %I(game_item_consume_transform_records) SET search_path=pg_catalog,%I,pg_temp',name,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %I(game_item_consume_transform_records) FROM PUBLIC',name);
  EXECUTE format('GRANT EXECUTE ON FUNCTION %I(game_item_consume_transform_records) TO oteryn_game_runtime',name);
 END LOOP;
END $$;

-- A transformed Ground item remains in the existing map projection and pickup
-- owner even when its predecessor was a source-map mint rather than a loop drop.
CREATE FUNCTION game_item_action_ground_source(p_item UUID) RETURNS BOOLEAN LANGUAGE sql STABLE AS $$
 SELECT EXISTS(SELECT 1 FROM game_item_instances i JOIN game_item_consume_transform_records r
    ON r.transaction_id=i.minted_transaction_id,LATERAL jsonb_array_elements(r.outputs) o
   WHERE i.item_instance_id=p_item AND r.source_kind=3 AND (o->>'item')::uuid=p_item)
$$;
DO $migration$ DECLARE body TEXT; original TEXT;
BEGIN
 body:=pg_get_functiondef('game_item_loop_line_admit()'::regprocedure);
 original:='game_item_loop_map_mint_source(NEW.item_instance_id)';
 IF position(original IN body)=0 THEN RAISE EXCEPTION 'missing source-map loop pickup admission'; END IF;
 EXECUTE replace(body,original,'('||original||' OR game_item_action_ground_source(NEW.item_instance_id))');
 EXECUTE format('ALTER FUNCTION game_item_action_ground_source(uuid) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $migration$;
REVOKE ALL ON FUNCTION game_item_action_ground_source(UUID) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_item_action_ground_source(UUID) TO oteryn_game_runtime;

-- W19 Afflicted pieces use the same bounded carried tree as the primary input.
-- Each additional cosmetic input remains exactly one unit.
CREATE FUNCTION game_item_action_extra_custody(r game_item_consume_transform_records,e JSONB)
RETURNS BOOLEAN LANGUAGE sql STABLE AS $$
 SELECT CASE (e->>'source_kind')::integer
 WHEN 1 THEN EXISTS(SELECT 1 FROM game_item_container_entries c JOIN game_item_container_slots b
   ON b.character_id=c.character_id AND b.item_instance_id=c.parent_item_instance_id
   WHERE c.item_instance_id=(e->>'item')::uuid AND c.character_id=r.character_id AND c.world_id=r.world_id
    AND b.world_id=r.world_id AND c.parent_item_instance_id=(e->>'parent')::uuid
    AND c.placement_ordinal=(e->>'ordinal')::numeric)
 WHEN 4 THEN EXISTS(SELECT 1 FROM game_item_nested_entries n,LATERAL game_item_loop_owner(n.parent_item_instance_id) owner
   WHERE n.item_instance_id=(e->>'item')::uuid AND n.world_id=r.world_id
    AND n.parent_item_instance_id=(e->>'parent')::uuid AND n.placement_ordinal=(e->>'ordinal')::numeric
    AND owner.owner_character=r.character_id AND owner.depth BETWEEN 0 AND 7)
 ELSE FALSE END
$$;

CREATE OR REPLACE FUNCTION game_item_action_extra_admit(p_record game_item_consume_transform_records)
RETURNS void LANGUAGE plpgsql AS $$
DECLARE e JSONB; i game_item_instances%ROWTYPE;
BEGIN
 IF NOT p_record.consume_source AND jsonb_array_length(p_record.extra_inputs)>0 THEN
  RAISE EXCEPTION 'non-consuming action cannot spend additional inputs' USING ERRCODE='23514'; END IF;
 IF (SELECT count(DISTINCT value->>'item') FROM jsonb_array_elements(p_record.extra_inputs))<>jsonb_array_length(p_record.extra_inputs) THEN
  RAISE EXCEPTION 'duplicate additional action input' USING ERRCODE='23514'; END IF;
 FOR e IN SELECT value FROM jsonb_array_elements(p_record.extra_inputs) LOOP
  SELECT * INTO i FROM game_item_instances WHERE item_instance_id=(e->>'item')::uuid FOR UPDATE;
  IF i.item_instance_id IS NULL OR i.item_instance_id IN(p_record.source_item_instance_id,p_record.secondary_item_instance_id)
    OR to_jsonb(i) IS DISTINCT FROM e->'before' OR i.lifecycle<>1 OR i.quantity<1
    OR (e->>'count')::integer IS DISTINCT FROM 1
    OR i.world_id<>p_record.world_id OR i.definition_family<>'Item'
    OR EXISTS(SELECT 1 FROM game_item_keys WHERE item_instance_id=i.item_instance_id)
    OR EXISTS(SELECT 1 FROM game_item_timed_states WHERE item_instance_id=i.item_instance_id)
    OR EXISTS(SELECT 1 FROM game_item_texts WHERE item_instance_id=i.item_instance_id)
    OR EXISTS(SELECT 1 FROM game_item_imbuements WHERE item_instance_id=i.item_instance_id)
    OR EXISTS(SELECT 1 FROM game_item_container_entries WHERE parent_item_instance_id=i.item_instance_id)
    OR EXISTS(SELECT 1 FROM game_item_nested_entries WHERE parent_item_instance_id=i.item_instance_id)
    OR NOT game_item_action_extra_custody(p_record,e)
   THEN RAISE EXCEPTION 'additional action input lacks exact plain carried predecessor' USING ERRCODE='23514'; END IF;
 END LOOP;
END $$;

CREATE OR REPLACE FUNCTION game_item_action_extra_complete(p_record game_item_consume_transform_records)
RETURNS void LANGUAGE plpgsql AS $$
DECLARE e JSONB; i game_item_instances%ROWTYPE;
BEGIN
 FOR e IN SELECT value FROM jsonb_array_elements(p_record.extra_inputs) LOOP
  SELECT * INTO i FROM game_item_instances WHERE item_instance_id=(e->>'item')::uuid;
  IF i.item_instance_id IS NULL OR i.quantity<>(e->'before'->>'quantity')::bigint-1
    OR (e->>'count')::integer IS DISTINCT FROM 1
    OR i.lifecycle<>(CASE WHEN i.quantity=0 THEN 2 ELSE 1 END) OR i.last_transaction_id IS DISTINCT FROM p_record.transaction_id
    OR (to_jsonb(i)-ARRAY['quantity','lifecycle','last_transaction_id']) IS DISTINCT FROM
       ((e->'before')-ARRAY['quantity','lifecycle','last_transaction_id'])
    OR (i.quantity=0 AND (EXISTS(SELECT 1 FROM game_item_container_entries WHERE item_instance_id=i.item_instance_id)
                      OR EXISTS(SELECT 1 FROM game_item_nested_entries WHERE item_instance_id=i.item_instance_id)))
    OR (i.quantity>0 AND NOT game_item_action_extra_custody(p_record,e))
   THEN RAISE EXCEPTION 'additional action input lacks exact decrement and custody' USING ERRCODE='23514'; END IF;
 END LOOP;
END $$;

DO $migration$
DECLARE body TEXT; arm TEXT; begin_at INTEGER;
BEGIN
 body:=pg_get_functiondef('game_item_nested_entry_proven()'::regprocedure);
 begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
 IF begin_at=0 THEN RAISE EXCEPTION 'missing additional nested action custody proof'; END IF;
 arm:=$arm$
   IF TG_OP='DELETE' AND EXISTS(SELECT 1 FROM game_item_consume_transform_records r,
     LATERAL jsonb_array_elements(r.extra_inputs) e,game_item_instances i
     WHERE (e->>'source_kind')::integer=4 AND (e->>'count')::integer=1
      AND (e->>'item')::uuid=OLD.item_instance_id AND (e->>'parent')::uuid=OLD.parent_item_instance_id
      AND (e->>'ordinal')::numeric=OLD.placement_ordinal AND r.world_id=OLD.world_id
      AND i.item_instance_id=OLD.item_instance_id AND i.quantity=0 AND i.lifecycle=2
      AND i.last_transaction_id=r.transaction_id AND r.created_xact_id=pg_current_xact_id()) THEN RETURN NULL; END IF;
$arm$;
 EXECUTE overlay(body placing E'BEGIN\n'||arm from begin_at for 5);
 EXECUTE format('ALTER FUNCTION game_item_action_extra_custody(game_item_consume_transform_records,jsonb) SET search_path=pg_catalog,%I,pg_temp',current_schema());
 EXECUTE format('ALTER FUNCTION game_item_action_extra_admit(game_item_consume_transform_records) SET search_path=pg_catalog,%I,pg_temp',current_schema());
 EXECUTE format('ALTER FUNCTION game_item_action_extra_complete(game_item_consume_transform_records) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $migration$;
REVOKE ALL ON FUNCTION game_item_action_extra_custody(game_item_consume_transform_records,jsonb) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_item_action_extra_custody(game_item_consume_transform_records,jsonb) TO oteryn_game_runtime;

-- Anniversary reward boxes use item:remove() without a count (Crystal line70).
-- Every other W19 use spends one source unit, or preserves its source entirely.
ALTER TABLE game_item_consume_transform_records
 ADD COLUMN source_count BIGINT NOT NULL DEFAULT 1;
ALTER TABLE game_item_consume_transform_records ADD CONSTRAINT game_item_action_count_shape CHECK(
 (recipe IN('oteryn:event-container.39704','oteryn:event-container.39705',
            'oteryn:event-container.39706','oteryn:event-container.39710')
  AND consume_source AND source_count BETWEEN 1 AND 100
  AND (before_item->>'definition_production_key') IS NOT DISTINCT FROM
      ('oteryn:item.tibia.i'||split_part(recipe,'.',2))
  AND source_count=(before_item->>'quantity')::bigint)
 OR (recipe NOT IN('oteryn:event-container.39704','oteryn:event-container.39705',
                  'oteryn:event-container.39706','oteryn:event-container.39710')
  AND source_count=CASE WHEN consume_source THEN 1 ELSE 0 END));
DO $$ DECLARE name TEXT; BEGIN
 SELECT conname INTO STRICT name FROM pg_constraint
 WHERE conrelid='game_item_consume_transform_records'::regclass AND contype='c'
  AND pg_get_constraintdef(oid) LIKE '%extra_inputs%' AND pg_get_constraintdef(oid) LIKE '%jsonb_typeof%';
 EXECUTE format('ALTER TABLE game_item_consume_transform_records DROP CONSTRAINT %I',name);
END $$;
ALTER TABLE game_item_consume_transform_records ADD CONSTRAINT game_item_action_extra_input_bound
 CHECK(jsonb_typeof(extra_inputs)='array' AND jsonb_array_length(extra_inputs)<=5);

ALTER TABLE game_item_consume_transform_records
 ADD COLUMN audit_context JSONB NOT NULL DEFAULT '{}'::jsonb
  CHECK(jsonb_typeof(audit_context)='object' AND octet_length(audit_context::text)<=1024),
 ADD COLUMN audit_commitment_sha256 BYTEA
  CHECK(audit_commitment_sha256 IS NULL OR octet_length(audit_commitment_sha256)=32);

CREATE FUNCTION game_item_action_commitment_basis(r game_item_consume_transform_records)
RETURNS JSONB LANGUAGE sql IMMUTABLE AS $$
 -- The envelope hash is circular; XID and sequence are database assignment metadata.
 -- Every remaining stored field, including all full input snapshots, is committed.
 SELECT to_jsonb(r)-ARRAY['envelope_sha256','created_xact_id','receipt_sequence','audit_commitment_sha256']
$$;

-- A small bounded protobuf reader is used only to verify the already registered
-- EventEnvelope.payload(26) -> OneItemTransaction.Craft(15) -> proof(1) path.
-- It rejects noncanonical varints, duplicate requested fields and unsupported wire
-- groups. It never admits a new event or bypasses the canonical Rust envelope owner.
CREATE FUNCTION game_item_action_wire_varint(w BYTEA,p INTEGER,OUT value NUMERIC,OUT next_offset INTEGER)
RETURNS RECORD LANGUAGE plpgsql IMMUTABLE STRICT AS $$
DECLARE b INTEGER; n INTEGER; multiplier NUMERIC:=1;
BEGIN
 value:=0; next_offset:=p;
 FOR n IN 1..10 LOOP
  IF next_offset<0 OR next_offset>=octet_length(w) THEN
   RAISE EXCEPTION 'truncated action audit varint' USING ERRCODE='23514'; END IF;
  b:=get_byte(w,next_offset); next_offset:=next_offset+1;
  IF n=10 AND b>1 THEN RAISE EXCEPTION 'oversized action audit varint' USING ERRCODE='23514'; END IF;
  value:=value+(b & 127)*multiplier;
  IF b<128 THEN
   IF n>1 AND b=0 THEN RAISE EXCEPTION 'noncanonical action audit varint' USING ERRCODE='23514'; END IF;
   RETURN;
  END IF;
  multiplier:=multiplier*128;
 END LOOP;
 RAISE EXCEPTION 'unterminated action audit varint' USING ERRCODE='23514';
END $$;

CREATE FUNCTION game_item_action_wire_bytes(w BYTEA,wanted INTEGER)
RETURNS BYTEA LANGUAGE plpgsql IMMUTABLE STRICT AS $$
DECLARE p INTEGER:=0; v NUMERIC; field_no INTEGER; wire_type INTEGER; width INTEGER;
        found BOOLEAN:=FALSE; result BYTEA;
BEGIN
 IF octet_length(w)>9216 OR wanted<1 OR wanted>536870911 THEN
  RAISE EXCEPTION 'action audit wire bound' USING ERRCODE='23514'; END IF;
 WHILE p<octet_length(w) LOOP
  SELECT value,next_offset INTO v,p FROM game_item_action_wire_varint(w,p);
  IF v<8 OR v>4294967295 THEN RAISE EXCEPTION 'invalid action audit field tag' USING ERRCODE='23514'; END IF;
  field_no:=trunc(v/8)::integer; wire_type:=mod(v,8)::integer;
  IF field_no=wanted AND (found OR wire_type<>2) THEN
   RAISE EXCEPTION 'duplicate or mistyped action audit field' USING ERRCODE='23514'; END IF;
  IF wire_type=0 THEN
   SELECT next_offset INTO p FROM game_item_action_wire_varint(w,p);
   CONTINUE;
  ELSIF wire_type=1 THEN width:=8;
  ELSIF wire_type=5 THEN width:=4;
  ELSIF wire_type=2 THEN
   SELECT value,next_offset INTO v,p FROM game_item_action_wire_varint(w,p);
   IF v>9216 THEN RAISE EXCEPTION 'oversized action audit field' USING ERRCODE='23514'; END IF;
   width:=v::integer;
  ELSE RAISE EXCEPTION 'unsupported action audit wire type' USING ERRCODE='23514';
  END IF;
  IF p+width>octet_length(w) THEN RAISE EXCEPTION 'truncated action audit field' USING ERRCODE='23514'; END IF;
  IF field_no=wanted THEN result:=substring(w FROM p+1 FOR width); found:=TRUE; END IF;
  p:=p+width;
 END LOOP;
 RETURN result;
END $$;

CREATE FUNCTION game_item_action_commitment_proven() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE envelope BYTEA; payload BYTEA; proof JSONB; expected JSONB;
BEGIN
 IF NEW.audit_commitment_sha256 IS NULL THEN RETURN NULL; END IF;
 IF NEW.created_xact_id<>pg_current_xact_id()
   OR NEW.audit_commitment_sha256 IS DISTINCT FROM
     sha256(convert_to(game_item_action_commitment_basis(NEW)::text,'UTF8'))
   OR (SELECT count(*) FROM jsonb_object_keys(NEW.audit_context))<>3
   OR (NEW.audit_context->>'scope_generation')::numeric IS NULL
   OR (NEW.audit_context->>'lease_generation')::numeric IS NULL
   OR (NEW.audit_context->>'connection_generation')::numeric IS NULL
 THEN RAISE EXCEPTION 'action audit commitment does not bind its full immutable receipt' USING ERRCODE='23514'; END IF;
 SELECT a.envelope INTO envelope FROM game_item_audit_outbox a
 WHERE a.transaction_id=NEW.transaction_id AND a.event_id=NEW.event_id
  AND a.item_instance_id=NEW.source_item_instance_id AND a.occurred_at=NEW.occurred_at
  AND a.envelope_sha256=NEW.envelope_sha256 AND sha256(a.envelope)=NEW.envelope_sha256
  AND a.created_xact_id=pg_current_xact_id() AND a.transaction_ordinal=1 AND a.transaction_count=1
  AND a.publication_state=1 AND a.published_at IS NULL;
 IF envelope IS NULL THEN RAISE EXCEPTION 'action audit commitment lacks its atomic envelope' USING ERRCODE='23514'; END IF;
 payload:=game_item_action_wire_bytes(envelope,26);
 IF payload IS NULL OR octet_length(payload)>7936
  OR game_item_action_wire_bytes(envelope,27) IS DISTINCT FROM sha256(payload)
 THEN RAISE EXCEPTION 'action audit commitment payload mismatch' USING ERRCODE='23514'; END IF;
 proof:=convert_from(game_item_action_wire_bytes(game_item_action_wire_bytes(payload,15),1),'UTF8')::jsonb;
 expected:=jsonb_build_object('schema','OTERYN_ITEM_ACTION_RECEIPT_COMMITMENT/v1',
  'encoding','postgresql-jsonb-text-utf8/v1','transaction_id',NEW.transaction_id::text,
  'receipt_sha256',encode(NEW.audit_commitment_sha256,'hex'));
 IF proof IS DISTINCT FROM expected THEN
  RAISE EXCEPTION 'actual Craft proof differs from committed receipt digest' USING ERRCODE='23514'; END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_item_action_commitment_proven
 AFTER INSERT ON game_item_consume_transform_records DEFERRABLE INITIALLY DEFERRED
 FOR EACH ROW EXECUTE FUNCTION game_item_action_commitment_proven();
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_item_action_commitment_basis(game_item_consume_transform_records) SET search_path=pg_catalog,%I,pg_temp',current_schema());
 EXECUTE format('ALTER FUNCTION game_item_action_wire_varint(bytea,integer) SET search_path=pg_catalog,%I,pg_temp',current_schema());
 EXECUTE format('ALTER FUNCTION game_item_action_wire_bytes(bytea,integer) SET search_path=pg_catalog,%I,pg_temp',current_schema());
 EXECUTE format('ALTER FUNCTION game_item_action_commitment_proven() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_item_action_commitment_basis(game_item_consume_transform_records),
 game_item_action_wire_varint(bytea,integer),game_item_action_wire_bytes(bytea,integer),game_item_action_commitment_proven() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_item_action_commitment_basis(game_item_consume_transform_records),
 game_item_action_wire_varint(bytea,integer),game_item_action_wire_bytes(bytea,integer) TO oteryn_game_runtime;

-- W19 reward and remaining novelty actions may use an actual Ground instance.
-- The same consume receipt distinguishes source-tile gifts from player rewards.
ALTER TABLE game_item_consume_transform_records DROP CONSTRAINT game_item_action_source_custody;
ALTER TABLE game_item_consume_transform_records ADD CONSTRAINT game_item_action_source_custody CHECK(
 (source_kind IN(1,4) AND parent_item_instance_id IS NOT NULL)
 OR (source_kind=2 AND parent_item_instance_id IS NULL AND NOT consume_source AND source_ordinal=1 AND effects->>'require_head'='true')
 OR (source_kind=3 AND jsonb_array_length(extra_inputs)=0 AND secondary_item_instance_id=source_item_instance_id
  AND source_location ? 'stack_ordinal'
  AND effects->>'kind' IN('dice','firework','transform','house_transform','costume','consume_effect','player_target_effect','reward')
  AND (parent_item_instance_id IS NULL OR (consume_source AND effects->>'kind'='reward' AND recipe LIKE 'oteryn:event-container.%'))));

CREATE OR REPLACE FUNCTION game_item_action_extended_admit(r game_item_consume_transform_records) RETURNS void LANGUAGE plpgsql AS $$
DECLARE i game_item_instances%ROWTYPE; s game_item_instances%ROWTYPE; o JSONB; highest NUMERIC;
BEGIN
 SELECT * INTO i FROM game_item_instances WHERE item_instance_id=r.source_item_instance_id FOR UPDATE;
 IF i.item_instance_id IS NULL OR to_jsonb(i) IS DISTINCT FROM r.before_item OR i.lifecycle<>1 OR i.quantity<1
    OR i.world_id<>r.world_id OR i.definition_family<>'Item' OR r.content_digest IS NULL
    OR NOT game_item_action_custody_current(r)
    OR EXISTS(SELECT 1 FROM game_item_keys WHERE item_instance_id=i.item_instance_id)
    OR EXISTS(SELECT 1 FROM game_item_timed_states WHERE item_instance_id=i.item_instance_id)
    OR EXISTS(SELECT 1 FROM game_item_texts WHERE item_instance_id=i.item_instance_id)
    OR EXISTS(SELECT 1 FROM game_item_imbuements WHERE item_instance_id=i.item_instance_id)
    OR EXISTS(SELECT 1 FROM game_item_container_entries WHERE parent_item_instance_id=i.item_instance_id)
    OR EXISTS(SELECT 1 FROM game_item_nested_entries WHERE parent_item_instance_id=i.item_instance_id)
 THEN RAISE EXCEPTION 'extended action requires exact plain source and current custody' USING ERRCODE='23514'; END IF;
 IF r.source_kind=3 THEN
  IF (r.source_location->>'world_id')::uuid<>r.world_id
    OR (r.source_location->>'stack_ordinal')::numeric<>r.source_ordinal
    OR NOT EXISTS(SELECT 1 FROM game_durability_reconnect_sessions sess
       WHERE sess.game_session_id=r.game_session_id AND sess.character_id=r.character_id AND sess.session_state=2
        AND sess.runtime_scope_kind=1 AND sess.runtime_scope_world_id=r.world_id
        AND sess.runtime_scope_channel_id=(r.source_location->>'channel_id')::uuid
        AND sess.scope_ownership_generation=(r.source_location->>'runtime_scope_ownership_generation')::numeric)
    OR (r.effects->>'kind'='house_transform' AND (r.source_location->>'qualified_house') IS NULL)
    OR (r.parent_item_instance_id IS NOT NULL AND NOT EXISTS(SELECT 1 FROM game_item_container_slots b
       JOIN game_item_instances parent ON parent.item_instance_id=b.item_instance_id
       WHERE b.character_id=r.character_id AND b.world_id=r.world_id AND b.item_instance_id=r.parent_item_instance_id
        AND parent.world_id=r.world_id AND parent.lifecycle=1))
  THEN RAISE EXCEPTION 'ground action requires current source Channel and qualified House membership' USING ERRCODE='23514'; END IF;
 END IF;
 PERFORM game_item_action_extra_admit(r);
 IF r.secondary_item_instance_id<>r.source_item_instance_id THEN
  SELECT * INTO s FROM game_item_instances WHERE item_instance_id=r.secondary_item_instance_id FOR UPDATE;
  IF r.source_kind<>4 OR to_jsonb(s) IS DISTINCT FROM r.before_secondary OR s.lifecycle<>1 OR s.quantity<1
    OR s.world_id<>r.world_id OR s.definition_family<>'Item'
    OR EXISTS(SELECT 1 FROM game_item_keys WHERE item_instance_id=s.item_instance_id)
    OR EXISTS(SELECT 1 FROM game_item_timed_states WHERE item_instance_id=s.item_instance_id)
    OR EXISTS(SELECT 1 FROM game_item_texts WHERE item_instance_id=s.item_instance_id)
    OR EXISTS(SELECT 1 FROM game_item_imbuements WHERE item_instance_id=s.item_instance_id)
    OR EXISTS(SELECT 1 FROM game_item_container_entries WHERE parent_item_instance_id=s.item_instance_id)
    OR EXISTS(SELECT 1 FROM game_item_nested_entries WHERE parent_item_instance_id=s.item_instance_id)
    OR NOT EXISTS(SELECT 1 FROM game_item_container_entries e JOIN game_item_container_slots b
        ON b.character_id=e.character_id AND b.item_instance_id=e.parent_item_instance_id
        WHERE e.item_instance_id=s.item_instance_id AND e.character_id=r.character_id AND e.world_id=r.world_id AND b.world_id=r.world_id)
  THEN RAISE EXCEPTION 'nested action secondary requires plain main-backpack source' USING ERRCODE='23514'; END IF;
 ELSIF r.before_secondary IS DISTINCT FROM r.before_item THEN
  RAISE EXCEPTION 'action source predecessor mismatch' USING ERRCODE='23514';
 END IF;
 IF (SELECT count(DISTINCT value->>'item') FROM jsonb_array_elements(r.outputs))<>jsonb_array_length(r.outputs)
  OR (SELECT count(DISTINCT value->>'ordinal') FROM jsonb_array_elements(r.outputs))<>jsonb_array_length(r.outputs) THEN
  RAISE EXCEPTION 'duplicate extended action output' USING ERRCODE='23514'; END IF;
 SELECT coalesce(max(placement_ordinal),0) INTO highest FROM (
  SELECT placement_ordinal FROM game_item_nested_entries WHERE parent_item_instance_id=r.parent_item_instance_id
  UNION ALL SELECT placement_ordinal FROM game_item_container_entries WHERE parent_item_instance_id=r.parent_item_instance_id) entries;
 FOR o IN SELECT value FROM jsonb_array_elements(r.outputs) LOOP
  IF NOT game_character_is_uuid_v7((o->>'item')::uuid)
    OR EXISTS(SELECT 1 FROM game_item_instances WHERE item_instance_id=(o->>'item')::uuid)
    OR (o->>'count')::bigint NOT BETWEEN 1 AND 100
    OR octet_length(o->>'key') NOT BETWEEN 1 AND 512 OR octet_length(o->>'revision') NOT BETWEEN 1 AND 512

    OR (r.source_kind=3 AND r.parent_item_instance_id IS NOT NULL AND (o->>'ordinal')::numeric<=highest)
    OR (r.source_kind=3 AND r.parent_item_instance_id IS NULL AND (
      jsonb_array_length(r.outputs)<>1 OR (o->>'count')::bigint<>1
      OR CASE WHEN r.effects->>'kind'='reward' THEN
        (o->>'ordinal')::bigint<>currval('game_item_ground_stack_ordinal_seq') OR (o->>'ordinal')::numeric<=r.source_ordinal
       ELSE i.quantity<>1 OR (o->>'ordinal')::numeric<>r.source_ordinal END))
    OR (r.source_kind=4 AND ((o->>'ordinal')::numeric<=highest
        AND NOT (r.consume_source AND i.quantity=r.source_count AND o=r.outputs->0 AND (o->>'ordinal')::numeric=r.source_ordinal)))
   THEN RAISE EXCEPTION 'extended action output is not exact fresh source replacement' USING ERRCODE='23514'; END IF;
 END LOOP;
END $$;

-- The ordinary Ground sequence still assigns new gift order. Only a same-XID
-- source-authored reward receipt may pre-reserve its sequence value for the audit.
DO $migration$
DECLARE name TEXT; body TEXT; arm TEXT; begin_at INTEGER;
BEGIN
 FOREACH name IN ARRAY ARRAY['game_item_ground_insertion_guard','game_item_ground_stack_stamp'] LOOP
  body:=pg_get_functiondef((name||'()')::regprocedure);
  begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
  IF begin_at=0 THEN RAISE EXCEPTION 'missing source Ground reward proof: %',name; END IF;
  IF name='game_item_ground_insertion_guard' THEN
   arm:=$arm$
    IF EXISTS(SELECT 1 FROM game_item_consume_transform_records r,LATERAL jsonb_array_elements(r.outputs) o
       WHERE r.source_kind=3 AND r.parent_item_instance_id IS NULL AND r.effects->>'kind'='reward'
        AND (o->>'item')::uuid=NEW.item_instance_id AND (o->>'ordinal')::bigint=NEW.stack_ordinal
        AND (to_jsonb(NEW)-ARRAY['item_instance_id','stack_ordinal'])=
            (r.source_location-ARRAY['item_instance_id','stack_ordinal','qualified_house'])
        AND r.created_xact_id=pg_current_xact_id()) THEN RETURN NULL; END IF;
$arm$;
  ELSE
   arm:=$arm$
    IF NEW.stack_ordinal IS NOT NULL AND EXISTS(SELECT 1 FROM game_item_consume_transform_records r,
       LATERAL jsonb_array_elements(r.outputs) o WHERE r.source_kind=3 AND r.parent_item_instance_id IS NULL
        AND r.effects->>'kind'='reward' AND (o->>'item')::uuid=NEW.item_instance_id
        AND (o->>'ordinal')::bigint=NEW.stack_ordinal AND NEW.stack_ordinal>r.source_ordinal
        AND r.created_xact_id=pg_current_xact_id()
        AND NEW.stack_ordinal=currval('game_item_ground_stack_ordinal_seq')
        AND NOT EXISTS(SELECT 1 FROM game_item_ground_locations WHERE stack_ordinal=NEW.stack_ordinal)) THEN RETURN NEW; END IF;
$arm$;
  END IF;
  EXECUTE overlay(body placing E'BEGIN\n'||arm from begin_at for 5);
 END LOOP;
 EXECUTE format('ALTER FUNCTION game_item_action_extended_admit(game_item_consume_transform_records) SET search_path=pg_catalog,%I,pg_temp',current_schema());
 EXECUTE format('ALTER FUNCTION game_item_action_extended_complete(game_item_consume_transform_records) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $migration$;

CREATE OR REPLACE FUNCTION game_item_action_extended_complete(r game_item_consume_transform_records) RETURNS void LANGUAGE plpgsql AS $$
DECLARE i game_item_instances%ROWTYPE; s game_item_instances%ROWTYPE; o JSONB; expected BIGINT;
BEGIN
 SELECT * INTO i FROM game_item_instances WHERE item_instance_id=r.source_item_instance_id;
 expected:=(r.before_item->>'quantity')::bigint-r.source_count;
 IF r.created_xact_id<>pg_current_xact_id() OR i.item_instance_id IS NULL OR i.quantity<>expected
  OR i.lifecycle<>(CASE WHEN expected=0 THEN 2 ELSE 1 END)
  OR (r.consume_source AND (i.last_transaction_id IS DISTINCT FROM r.transaction_id
      OR (to_jsonb(i)-ARRAY['quantity','lifecycle','last_transaction_id']) IS DISTINCT FROM
         (r.before_item-ARRAY['quantity','lifecycle','last_transaction_id'])))
  OR (NOT r.consume_source AND to_jsonb(i) IS DISTINCT FROM r.before_item)
  OR (expected>0 AND NOT game_item_action_custody_current(r))
  OR (expected=0 AND (EXISTS(SELECT 1 FROM game_item_ground_locations WHERE item_instance_id=i.item_instance_id)
                  OR EXISTS(SELECT 1 FROM game_item_nested_entries WHERE item_instance_id=i.item_instance_id)))
  OR (SELECT count(*) FROM game_item_instances WHERE minted_transaction_id=r.transaction_id)<>jsonb_array_length(r.outputs)
  OR (SELECT count(*) FROM game_item_instances WHERE last_transaction_id=r.transaction_id)<>
      (CASE WHEN NOT r.consume_source THEN 0 WHEN r.secondary_item_instance_id=r.source_item_instance_id THEN 1 ELSE 2 END)+jsonb_array_length(r.extra_inputs)
  OR (r.source_kind=4 AND r.consume_source AND (SELECT count(*) FROM game_item_nested_entries WHERE parent_item_instance_id=r.parent_item_instance_id)>r.parent_capacity)
  OR (r.source_kind=3 AND r.parent_item_instance_id IS NOT NULL AND
      (SELECT count(*) FROM game_item_container_entries WHERE parent_item_instance_id=r.parent_item_instance_id)>r.parent_capacity)
  OR (SELECT count(*) FROM game_item_audit_outbox WHERE transaction_id=r.transaction_id)<>1
  OR NOT EXISTS(SELECT 1 FROM game_item_audit_outbox WHERE event_id=r.event_id AND transaction_id=r.transaction_id
       AND item_instance_id=r.source_item_instance_id AND occurred_at=r.occurred_at AND envelope_sha256=r.envelope_sha256
       AND created_xact_id=pg_current_xact_id() AND transaction_ordinal=1 AND transaction_count=1
       AND publication_state=1 AND published_at IS NULL)
 THEN RAISE EXCEPTION 'extended action lacks atomic input,custody,output or audit' USING ERRCODE='23514'; END IF;
 IF r.secondary_item_instance_id<>r.source_item_instance_id THEN
  SELECT * INTO s FROM game_item_instances WHERE item_instance_id=r.secondary_item_instance_id;
  IF s.quantity<>(r.before_secondary->>'quantity')::bigint-1 OR s.lifecycle<>(CASE WHEN s.quantity=0 THEN 2 ELSE 1 END)
    OR s.last_transaction_id IS DISTINCT FROM r.transaction_id
    OR (to_jsonb(s)-ARRAY['quantity','lifecycle','last_transaction_id']) IS DISTINCT FROM
       (r.before_secondary-ARRAY['quantity','lifecycle','last_transaction_id'])
    OR (s.quantity=0 AND EXISTS(SELECT 1 FROM game_item_container_entries WHERE item_instance_id=s.item_instance_id))
   THEN RAISE EXCEPTION 'nested action secondary decrement mismatch' USING ERRCODE='23514'; END IF;
 END IF;
 PERFORM game_item_action_extra_complete(r);
 FOR o IN SELECT value FROM jsonb_array_elements(r.outputs) LOOP
  IF NOT EXISTS(SELECT 1 FROM game_item_instances n WHERE n.item_instance_id=(o->>'item')::uuid AND n.world_id=r.world_id
      AND n.quantity=(o->>'count')::bigint AND n.lifecycle=1 AND n.definition_family='Item'
      AND n.definition_production_key=o->>'key' AND n.definition_revision_ref=o->>'revision'
      AND n.minted_transaction_id=r.transaction_id AND n.last_transaction_id IS NULL)
    OR (r.source_kind=3 AND r.parent_item_instance_id IS NULL AND NOT EXISTS(SELECT 1 FROM game_item_ground_locations g
        WHERE g.item_instance_id=(o->>'item')::uuid
         AND (to_jsonb(g)-ARRAY['item_instance_id','stack_ordinal'])=(r.source_location-ARRAY['item_instance_id','stack_ordinal','qualified_house'])
         AND g.stack_ordinal=(o->>'ordinal')::bigint))
    OR (r.source_kind=3 AND r.parent_item_instance_id IS NOT NULL AND NOT EXISTS(SELECT 1 FROM game_item_container_entries e
        JOIN game_item_container_slots b ON b.character_id=e.character_id AND b.item_instance_id=e.parent_item_instance_id
        WHERE e.item_instance_id=(o->>'item')::uuid AND e.world_id=r.world_id AND e.character_id=r.character_id
         AND e.parent_item_instance_id=r.parent_item_instance_id AND e.placement_ordinal=(o->>'ordinal')::numeric
         AND e.placed_transaction_id=r.transaction_id AND b.world_id=r.world_id))
    OR (r.source_kind=4 AND NOT EXISTS(SELECT 1 FROM game_item_nested_entries n,LATERAL game_item_loop_owner(n.parent_item_instance_id) owner
        WHERE n.item_instance_id=(o->>'item')::uuid AND n.world_id=r.world_id AND n.parent_item_instance_id=r.parent_item_instance_id
          AND n.placement_ordinal=(o->>'ordinal')::numeric AND n.placed_transaction_id=r.transaction_id
          AND owner.owner_character=r.character_id AND owner.depth BETWEEN 0 AND 7))
   THEN RAISE EXCEPTION 'extended action output or exact custody mismatch' USING ERRCODE='23514'; END IF;
  IF o->'decay' IS NOT NULL AND o->'decay'<>'null'::jsonb THEN
   IF NOT EXISTS(SELECT 1 FROM game_spell_item_temporal_schedules p WHERE p.item_instance_id=(o->>'item')::uuid
      AND p.source_transaction_id=r.transaction_id AND p.created_xact_id=r.created_xact_id) THEN
    RAISE EXCEPTION 'timed extended action output lacks original deadline' USING ERRCODE='23514'; END IF;
  ELSIF EXISTS(SELECT 1 FROM game_spell_item_temporal_schedules WHERE item_instance_id=(o->>'item')::uuid) THEN
   RAISE EXCEPTION 'untimed extended action output acquired fabricated deadline' USING ERRCODE='23514'; END IF;
 END LOOP;
END $$;

DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_item_action_extended_admit(game_item_consume_transform_records) SET search_path=pg_catalog,%I,pg_temp',current_schema());
 EXECUTE format('ALTER FUNCTION game_item_action_extended_complete(game_item_consume_transform_records) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;


-- Extend only the existing source debit arm for those exact whole-stack records.
DO $migration$
DECLARE body TEXT; begin_at INTEGER; arm TEXT;
BEGIN
 body:=pg_get_functiondef('game_item_instance_guard()'::regprocedure);
 begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
 IF begin_at=0 THEN RAISE EXCEPTION 'missing event action instance guard'; END IF;
 arm:=$arm$
  IF TG_OP='UPDATE' AND EXISTS(SELECT 1 FROM game_item_consume_transform_records r
    WHERE r.source_item_instance_id=OLD.item_instance_id AND r.before_item=to_jsonb(OLD)
     AND r.recipe IN('oteryn:event-container.39704','oteryn:event-container.39705',
                    'oteryn:event-container.39706','oteryn:event-container.39710')
     AND r.consume_source AND r.source_count=OLD.quantity
     AND r.transaction_id=NEW.last_transaction_id AND r.created_xact_id=pg_current_xact_id()) THEN
   IF NEW.quantity<>0 OR NEW.lifecycle<>2 OR
      (to_jsonb(NEW)-ARRAY['quantity','lifecycle','last_transaction_id']) IS DISTINCT FROM
      (to_jsonb(OLD)-ARRAY['quantity','lifecycle','last_transaction_id']) THEN
    RAISE EXCEPTION 'anniversary source must retire the exact original stack' USING ERRCODE='23514'; END IF;
   RETURN NEW;
  END IF;
$arm$;
 EXECUTE overlay(body placing E'BEGIN\n'||arm from begin_at for 5);
END $migration$;
