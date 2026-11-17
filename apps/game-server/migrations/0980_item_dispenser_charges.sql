-- SRX-1: typed charges travel with the dispenser ItemInstance. The existing
-- consume/transform receipt binds flask consumption, potion outputs and depletion.
DO $$ DECLARE prior TEXT; BEGIN
 SELECT pg_get_expr(conbin,conrelid) INTO STRICT prior FROM pg_constraint
   WHERE conrelid='game_item_consume_transform_records'::regclass AND conname='game_item_action_count_shape';
 ALTER TABLE game_item_consume_transform_records DROP CONSTRAINT game_item_action_count_shape;
 EXECUTE format($check$ALTER TABLE game_item_consume_transform_records
   ADD CONSTRAINT game_item_action_count_shape CHECK(
    ((%s) AND recipe<>'oteryn:dispenser.fill') OR
    (recipe='oteryn:dispenser.fill' AND consume_source AND source_count BETWEEN 1 AND 100
      AND before_item->>'definition_production_key' IN('oteryn:item.tibia.i283','oteryn:item.tibia.i284','oteryn:item.tibia.i285')
      AND source_count=least((before_item->>'quantity')::bigint,(effects->'dispenser'->>'before')::bigint)
      AND source_count=(effects->'dispenser'->>'filled')::bigint
      AND jsonb_array_length(outputs)=1 AND source_count=(outputs->0->>'count')::bigint) IS TRUE)$check$,prior);
END $$;

-- Extend the guard chain only for the exact receipt-backed dispenser debit.
DO $migration$
DECLARE body TEXT; begin_at INTEGER;
BEGIN
 body:=pg_get_functiondef('game_item_instance_guard()'::regprocedure);
 begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
 IF begin_at=0 THEN RAISE EXCEPTION 'missing dispenser instance guard'; END IF;
 EXECUTE overlay(body placing E'BEGIN\n'||$arm$
  IF TG_OP='UPDATE' AND EXISTS(SELECT 1 FROM game_item_consume_transform_records r
    WHERE r.recipe='oteryn:dispenser.fill' AND r.source_item_instance_id=OLD.item_instance_id
      AND r.before_item=to_jsonb(OLD) AND r.consume_source
      AND r.source_count BETWEEN 1 AND OLD.quantity AND r.transaction_id=NEW.last_transaction_id
      AND r.created_xact_id=pg_current_xact_id()) THEN
   IF NEW.quantity<>OLD.quantity-(SELECT r.source_count FROM game_item_consume_transform_records r WHERE r.transaction_id=NEW.last_transaction_id)
      OR NEW.lifecycle<>(CASE WHEN NEW.quantity=0 THEN 2 ELSE 1 END)
      OR (to_jsonb(NEW)-ARRAY['quantity','lifecycle','last_transaction_id']) IS DISTINCT FROM
         (to_jsonb(OLD)-ARRAY['quantity','lifecycle','last_transaction_id']) THEN
    RAISE EXCEPTION 'dispenser flask debit differs' USING ERRCODE='23514';
   END IF;
   RETURN NEW;
  END IF;
$arm$ from begin_at for 5);
END $migration$;

-- Depletion uses the existing additional-input burn, retaining the preceding
-- timed row exactly as TIMED-RT expiry does. No second charge counter is created.
CREATE FUNCTION game_item_dispenser_exhaustion(r game_item_consume_transform_records,e JSONB)
RETURNS BOOLEAN LANGUAGE sql STABLE AS $$
 SELECT coalesce(r.recipe='oteryn:dispenser.fill'
   AND (r.effects->'dispenser'->>'item')::uuid=(e->>'item')::uuid
   AND r.effects->'dispenser'->>'after'='0'
   AND (r.effects->'dispenser'->>'before')::integer=r.source_count
   AND NOT EXISTS(SELECT 1 FROM game_item_timed_states t WHERE t.item_instance_id=(e->>'item')::uuid
     AND (t.charges IS DISTINCT FROM (r.effects->'dispenser'->>'before')::integer
       OR t.state_revision IS DISTINCT FROM (r.effects->'dispenser'->>'expected_revision')::numeric
       OR t.remaining_ms IS NOT NULL OR t.deadline_at IS NOT NULL)),false)
$$;
DO $migration$
DECLARE body TEXT; original TEXT;
BEGIN
 body:=pg_get_functiondef('game_item_action_extra_admit(game_item_consume_transform_records)'::regprocedure);
 original:='OR EXISTS(SELECT 1 FROM game_item_timed_states WHERE item_instance_id=i.item_instance_id)';
 IF position(original IN body)=0 THEN RAISE EXCEPTION 'missing plain additional-input timed guard'; END IF;
 EXECUTE replace(body,original,
   'OR (EXISTS(SELECT 1 FROM game_item_timed_states WHERE item_instance_id=i.item_instance_id) AND NOT game_item_dispenser_exhaustion(p_record,e))');
END $migration$;

CREATE FUNCTION game_item_dispenser_consistent() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE d JSONB; i game_item_instances%ROWTYPE; t game_item_timed_states%ROWTYPE;
BEGIN
 d:=NEW.effects->'dispenser';
 SELECT * INTO i FROM game_item_instances WHERE item_instance_id=(d->>'item')::uuid;
 SELECT * INTO t FROM game_item_timed_states WHERE item_instance_id=i.item_instance_id;
 IF NEW.recipe IS DISTINCT FROM 'oteryn:dispenser.fill'
    OR NEW.created_xact_id IS DISTINCT FROM pg_current_xact_id()
    OR i.world_id IS DISTINCT FROM NEW.world_id
    OR (d->'before_item'->>'item_instance_id')::uuid IS DISTINCT FROM i.item_instance_id
    OR i.definition_production_key IS DISTINCT FROM d->'before_item'->>'definition_production_key'
    OR i.definition_revision_ref IS DISTINCT FROM d->'before_item'->>'definition_revision_ref'
    OR NOT coalesce((d->>'initial')::integer BETWEEN 1 AND 65535
      AND (d->>'before')::integer BETWEEN 1 AND (d->>'initial')::integer
      AND (d->>'after')::integer=(d->>'before')::integer-NEW.source_count,false)
 THEN RAISE EXCEPTION 'invalid dispenser charge proof' USING ERRCODE='23514'; END IF;
 IF (d->>'after')::integer>0 THEN
   IF i.lifecycle<>1 OR NOT EXISTS(SELECT 1 FROM game_item_timed_state_writes w
      WHERE w.item_instance_id=i.item_instance_id AND w.transaction_id=NEW.transaction_id
        AND w.created_xact_id=NEW.created_xact_id AND w.cause=1
        AND w.holder_character_id=NEW.character_id
        AND w.expected_revision=(d->>'expected_revision')::numeric
        AND w.charges_before=(d->>'before')::integer
        AND w.charges_after=(d->>'after')::integer AND w.full_charges=(d->>'initial')::integer
        AND w.remaining_ms_before IS NULL AND w.remaining_ms_after IS NULL
        AND w.deadline_before IS NULL AND w.deadline_after IS NULL)
   THEN RAISE EXCEPTION 'dispenser fill lacks timed checkpoint' USING ERRCODE='23514'; END IF;
 ELSE
   IF i.lifecycle<>2 OR i.quantity<>0
      OR coalesce(t.charges,(d->>'initial')::integer) IS DISTINCT FROM (d->>'before')::integer
      OR coalesce(t.state_revision,0) IS DISTINCT FROM (d->>'expected_revision')::numeric
      OR t.remaining_ms IS NOT NULL OR t.deadline_at IS NOT NULL
      OR NOT EXISTS(SELECT 1 FROM jsonb_array_elements(NEW.extra_inputs) e
        WHERE (e->>'item')::uuid=i.item_instance_id AND game_item_dispenser_exhaustion(NEW,e))
   THEN RAISE EXCEPTION 'depleted dispenser lacks exact retirement' USING ERRCODE='23514'; END IF;
 END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_item_dispenser_receipt_consistent AFTER INSERT
 ON game_item_consume_transform_records DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
 WHEN(NEW.effects ? 'dispenser') EXECUTE FUNCTION game_item_dispenser_consistent();
REVOKE ALL ON FUNCTION game_item_dispenser_exhaustion(game_item_consume_transform_records,JSONB),game_item_dispenser_consistent() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_item_dispenser_exhaustion(game_item_consume_transform_records,JSONB),game_item_dispenser_consistent() TO oteryn_game_runtime;
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_item_dispenser_exhaustion(game_item_consume_transform_records,JSONB) SET search_path=pg_catalog,%I,pg_temp',current_schema());
 EXECUTE format('ALTER FUNCTION game_item_dispenser_consistent() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;

-- A completely consumed flask stack frees its exact source ordinal, just as a
-- single consumed item does. The count CHECK above binds this to the fill only.
DO $migration$
DECLARE body TEXT; original TEXT;
BEGIN
 body:=pg_get_functiondef('game_item_consume_transform_admit()'::regprocedure);
 original:='i.quantity=1 AND o=NEW.outputs->0';
 IF position(original IN body)=0 THEN RAISE EXCEPTION 'missing source ordinal reuse proof'; END IF;
 EXECUTE replace(body,original,
  '(i.quantity=1 OR (NEW.recipe=''oteryn:dispenser.fill'' AND NEW.source_count=i.quantity)) AND o=NEW.outputs->0');
END $migration$;

-- Main-backpack completion predates counted inputs. Preserve its one-item
-- contract for every older recipe; dispenser fills prove the exact stack debit.
DO $migration$
DECLARE body TEXT; original TEXT;
BEGIN
 body:=pg_get_functiondef('game_item_consume_transform_complete(uuid)'::regprocedure);
 original:='i.quantity<>(r.before_item->>''quantity'')::bigint-1';
 IF position(original IN body)=0 THEN RAISE EXCEPTION 'missing main source count completion proof'; END IF;
 EXECUTE replace(body,original,
  'i.quantity<>(r.before_item->>''quantity'')::bigint-(CASE WHEN r.recipe=''oteryn:dispenser.fill'' THEN r.source_count ELSE 1 END)');
END $migration$;

-- A placed dispenser is a target, never a portable primary input. Bind its
-- complete, independently qualified Ground snapshot to this exact fill receipt.
CREATE FUNCTION game_item_dispenser_target_custody(r game_item_consume_transform_records)
RETURNS BOOLEAN LANGUAGE sql STABLE AS $$
 SELECT CASE (r.effects->'dispenser'->>'source_kind')::integer
 WHEN 3 THEN EXISTS(SELECT 1 FROM game_item_ground_locations g
   WHERE g.item_instance_id=(r.effects->'dispenser'->>'item')::uuid
     AND g.world_id=r.world_id AND to_jsonb(g)=r.effects->'dispenser'->'location'
     AND g.runtime_scope_ownership_generation=(r.audit_context->>'scope_generation')::numeric
     AND EXISTS(SELECT 1 FROM game_durability_reconnect_sessions sess
       WHERE sess.game_session_id=r.game_session_id AND sess.character_id=r.character_id
         AND sess.session_state IN(1,2) AND sess.runtime_scope_kind=1
         AND sess.runtime_scope_world_id=g.world_id AND sess.runtime_scope_channel_id=g.channel_id
         AND sess.scope_ownership_generation=g.runtime_scope_ownership_generation
         AND sess.current_generation=(r.audit_context->>'connection_generation')::numeric
         AND sess.character_lease_generation=(r.audit_context->>'lease_generation')::numeric))
 WHEN 4 THEN EXISTS(SELECT 1 FROM game_item_nested_entries n,
   LATERAL game_item_loop_owner(n.parent_item_instance_id) owner
   WHERE n.item_instance_id=(r.effects->'dispenser'->>'item')::uuid
     AND n.world_id=r.world_id AND owner.owner_character=r.character_id AND owner.depth BETWEEN 0 AND 7)
 ELSE FALSE END
$$;
CREATE FUNCTION game_item_dispenser_checkpoint_held(p_item UUID,p_holder UUID,p_transaction UUID)
RETURNS BOOLEAN LANGUAGE sql STABLE AS $$
 SELECT EXISTS(SELECT 1 FROM game_item_consume_transform_records r
   WHERE r.transaction_id=p_transaction AND r.created_xact_id=pg_current_xact_id()
    AND r.recipe='oteryn:dispenser.fill' AND r.character_id=p_holder
    AND (r.effects->'dispenser'->>'item')::uuid=p_item
    AND (r.effects->'dispenser'->>'after')::integer>0
    AND game_item_dispenser_target_custody(r))
$$;
DO $migration$
DECLARE body TEXT; original TEXT; begin_at INTEGER;
BEGIN
 body:=pg_get_functiondef('game_item_timed_state_write_consistency_guard()'::regprocedure);
 original:='OR EXISTS (SELECT 1 FROM game_item_container_entries e';
 IF position(original IN body)=0 THEN RAISE EXCEPTION 'missing timed custody arm'; END IF;
 EXECUTE replace(body,original,'OR game_item_dispenser_checkpoint_held(NEW.item_instance_id,NEW.holder_character_id,NEW.transaction_id) '||original);
 body:=pg_get_functiondef('game_item_action_extra_custody(game_item_consume_transform_records,jsonb)'::regprocedure);
 original:='ELSE FALSE END';
 IF position(original IN body)=0 THEN RAISE EXCEPTION 'missing additional-input custody arm'; END IF;
 EXECUTE replace(body,original,$arm$WHEN 3 THEN r.recipe='oteryn:dispenser.fill'
   AND (r.effects->'dispenser'->>'item')::uuid=(e->>'item')::uuid
   AND r.effects->'dispenser'->>'after'='0' AND game_item_dispenser_target_custody(r)
 ELSE FALSE END$arm$);
 body:=pg_get_functiondef('game_item_ground_removal_proven()'::regprocedure);
 begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
 IF begin_at=0 THEN RAISE EXCEPTION 'missing dispenser Ground removal proof'; END IF;
 EXECUTE overlay(body placing E'BEGIN\n'||$arm$
 IF TG_OP='DELETE' AND EXISTS(SELECT 1 FROM game_item_consume_transform_records r
   JOIN game_item_instances i ON i.item_instance_id=(r.effects->'dispenser'->>'item')::uuid
   WHERE r.recipe='oteryn:dispenser.fill' AND r.created_xact_id=pg_current_xact_id()
    AND r.effects->'dispenser'->>'after'='0' AND r.effects->'dispenser'->'location'=to_jsonb(OLD)
    AND i.item_instance_id=OLD.item_instance_id AND i.quantity=0 AND i.lifecycle=2
    AND i.last_transaction_id=r.transaction_id
    AND EXISTS(SELECT 1 FROM jsonb_array_elements(r.extra_inputs) e
      WHERE (e->>'item')::uuid=i.item_instance_id AND (e->>'source_kind')::integer=3)) THEN RETURN NULL; END IF;
$arm$ from begin_at for 5);
END $migration$;
DO $$ DECLARE n TEXT; BEGIN
 FOREACH n IN ARRAY ARRAY['game_item_dispenser_target_custody(game_item_consume_transform_records)',
 'game_item_dispenser_checkpoint_held(uuid,uuid,uuid)'] LOOP
 EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',n,current_schema());
 EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC',n);
 EXECUTE format('GRANT EXECUTE ON FUNCTION %s TO oteryn_game_runtime',n);
 END LOOP;
END $$;

-- Final Ground retirement must remove custody in the same transaction.
DO $migration$ DECLARE body TEXT; original TEXT; BEGIN
 body:=pg_get_functiondef('game_item_dispenser_consistent()'::regprocedure);
 original:='IF i.lifecycle<>2 OR i.quantity<>0';
 IF position(original IN body)=0 THEN RAISE EXCEPTION 'missing exhausted dispenser consistency'; END IF;
 EXECUTE replace(body,original,original||' OR EXISTS(SELECT 1 FROM game_item_ground_locations g WHERE g.item_instance_id=i.item_instance_id)');
END $migration$;

-- Named-House USE retains the same carried flask receipt; only its placed
-- target needs the independent House owner. No Channel fence stands in for it.
DO $migration$ DECLARE body TEXT; original TEXT; BEGIN
 body:=pg_get_functiondef('game_item_dispenser_target_custody(game_item_consume_transform_records)'::regprocedure);
 original:='ELSE FALSE END';
 IF position(original IN body)=0 THEN RAISE EXCEPTION 'missing dispenser target custody'; END IF;
 EXECUTE replace(body,original,$arm$
 WHEN 5 THEN EXISTS(SELECT 1 FROM game_item_house_interior_locations h
   JOIN game_item_house_reclaim_provenance p USING(item_instance_id,world_id,house_key)
   WHERE h.item_instance_id=(r.effects->'dispenser'->>'item')::uuid AND h.world_id=r.world_id
    AND h.house_key=r.effects->'dispenser'->'house_context'->>'house'
    AND to_jsonb(h)=r.effects->'dispenser'->'location'->'house'
    AND to_jsonb(p)=r.effects->'dispenser'->'location'->'provenance'
    AND p.placement_transaction_id=h.placed_transaction_id)
 ELSE FALSE END$arm$);
END $migration$;
CREATE FUNCTION game_item_dispenser_house_admit(r game_item_consume_transform_records)
RETURNS VOID LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE c JSONB;
BEGIN
 c:=r.effects->'dispenser'->'house_context';
 IF r.recipe IS DISTINCT FROM 'oteryn:dispenser.fill' OR jsonb_typeof(c) IS DISTINCT FROM 'object'
  OR (r.effects->'dispenser'->>'source_kind')::integer NOT IN(1,4,5)
 THEN RAISE EXCEPTION 'dispenser requires exact named-House context' USING ERRCODE='23514'; END IF;
 PERFORM game_house_podium_fence(r.game_session_id,r.character_id,
  (r.audit_context->>'connection_generation')::numeric,(r.audit_context->>'lease_generation')::numeric,
  (r.audit_context->>'scope_generation')::numeric,(c->>'node')::uuid,
  (c->>'node_revision')::numeric,r.world_id,c->>'house');
 IF r.effects->'dispenser'->>'source_kind'='5' AND NOT game_item_dispenser_target_custody(r) THEN
  RAISE EXCEPTION 'dispenser House target differs from exact placement/provenance' USING ERRCODE='23514'; END IF;
END $$;
DO $migration$ DECLARE body TEXT; begin_at INTEGER; BEGIN
 body:=pg_get_functiondef('game_item_consume_transform_admit()'::regprocedure);
 begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
 IF begin_at=0 THEN RAISE EXCEPTION 'missing House dispenser receipt admission'; END IF;
 EXECUTE overlay(body placing E'BEGIN\n'||$arm$
 IF NEW.recipe='oteryn:dispenser.fill' AND
   (coalesce(NEW.effects->'dispenser'->'house_context','null'::jsonb) IS DISTINCT FROM 'null'::jsonb
    OR NEW.effects->'dispenser'->>'source_kind'='5') THEN
  PERFORM game_item_dispenser_house_admit(NEW);
 END IF;
$arm$ from begin_at for 5);
 -- An exhausted House target is an additional input; all ordinary inputs retain
 -- their existing carried-only checks.
 body:=pg_get_functiondef('game_item_action_extra_custody(game_item_consume_transform_records,jsonb)'::regprocedure);
 IF position('ELSE FALSE END' IN body)=0 THEN RAISE EXCEPTION 'missing House dispenser extra custody'; END IF;
 EXECUTE replace(body,'ELSE FALSE END',$arm$WHEN 5 THEN r.recipe='oteryn:dispenser.fill'
   AND r.effects->'dispenser'->>'source_kind'='5'
   AND (r.effects->'dispenser'->>'item')::uuid=(e->>'item')::uuid
   AND r.effects->'dispenser'->>'after'='0' AND game_item_dispenser_target_custody(r)
 ELSE FALSE END$arm$);
 body:=pg_get_functiondef('game_house_location_proven()'::regprocedure);
 begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
 IF begin_at=0 THEN RAISE EXCEPTION 'missing House dispenser retirement proof'; END IF;
 EXECUTE overlay(body placing E'BEGIN\n'||$arm$
 IF TG_OP='DELETE' AND EXISTS(SELECT 1 FROM game_item_consume_transform_records r
   JOIN game_item_instances i ON i.item_instance_id=(r.effects->'dispenser'->>'item')::uuid
   WHERE r.recipe='oteryn:dispenser.fill' AND r.created_xact_id=pg_current_xact_id()
    AND r.effects->'dispenser'->>'source_kind'='5' AND r.effects->'dispenser'->>'after'='0'
    AND r.effects->'dispenser'->'location'->'house'=to_jsonb(OLD)
    AND i.item_instance_id=OLD.item_instance_id AND i.quantity=0 AND i.lifecycle=2
    AND i.last_transaction_id=r.transaction_id
    AND EXISTS(SELECT 1 FROM jsonb_array_elements(r.extra_inputs) e
      WHERE (e->>'item')::uuid=i.item_instance_id AND (e->>'source_kind')::integer=5)) THEN RETURN NULL; END IF;
$arm$ from begin_at for 5);
 body:=pg_get_functiondef('game_item_dispenser_consistent()'::regprocedure);
 IF position('IF i.lifecycle<>2 OR i.quantity<>0' IN body)=0 THEN RAISE EXCEPTION 'missing final House dispenser consistency'; END IF;
 EXECUTE replace(body,'IF i.lifecycle<>2 OR i.quantity<>0','IF i.lifecycle<>2 OR i.quantity<>0 OR EXISTS(SELECT 1 FROM game_item_house_interior_locations h WHERE h.item_instance_id=i.item_instance_id) OR EXISTS(SELECT 1 FROM game_item_house_reclaim_provenance p WHERE p.item_instance_id=i.item_instance_id)');
END $migration$;
CREATE FUNCTION game_item_dispenser_house_retire(p_transaction UUID) RETURNS VOID
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE r game_item_consume_transform_records%ROWTYPE; d JSONB; i game_item_instances%ROWTYPE;
BEGIN
 SELECT * INTO r FROM game_item_consume_transform_records WHERE transaction_id=p_transaction;
 d:=r.effects->'dispenser';
 SELECT * INTO i FROM game_item_instances WHERE item_instance_id=(d->>'item')::uuid FOR UPDATE;
 IF r.transaction_id IS NULL OR r.recipe<>'oteryn:dispenser.fill' OR r.created_xact_id<>pg_current_xact_id()
  OR d->>'source_kind' IS DISTINCT FROM '5' OR d->>'after' IS DISTINCT FROM '0'
  OR i.item_instance_id IS NULL OR i.quantity<>0 OR i.lifecycle<>2 OR i.last_transaction_id<>r.transaction_id
  OR NOT game_item_dispenser_target_custody(r)
 THEN RAISE EXCEPTION 'House dispenser retirement lacks exact current receipt' USING ERRCODE='23514'; END IF;
 PERFORM game_item_dispenser_house_admit(r);
 DELETE FROM game_item_house_reclaim_provenance WHERE item_instance_id=i.item_instance_id;
 IF NOT FOUND THEN RAISE EXCEPTION 'House dispenser provenance disappeared' USING ERRCODE='23514'; END IF;
 DELETE FROM game_item_house_interior_locations WHERE item_instance_id=i.item_instance_id;
 IF NOT FOUND THEN RAISE EXCEPTION 'House dispenser placement disappeared' USING ERRCODE='23514'; END IF;
END $$;
DO $$ DECLARE n TEXT; BEGIN
 FOREACH n IN ARRAY ARRAY['game_item_dispenser_house_admit(game_item_consume_transform_records)',
 'game_item_dispenser_house_retire(uuid)'] LOOP
 EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',n,current_schema());
 EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC',n);
 EXECUTE format('GRANT EXECUTE ON FUNCTION %s TO oteryn_game_runtime',n);
 END LOOP;
END $$;

-- Frozen charge/duration budgets follow the existing ItemInstance custody.
-- A movement cannot create, spend, rewrite or delete a timed row. Continuous
-- lit/deadline items still require their separate stop/put-out owner first.
ALTER TABLE game_item_loop_move_lines ADD COLUMN before_frozen_timed JSONB;
CREATE FUNCTION game_item_loop_frozen_timed_shape(l game_item_loop_move_lines)
RETURNS BOOLEAN LANGUAGE sql STABLE SECURITY DEFINER AS $$
 SELECT coalesce(l.quantity_before=1 AND l.quantity_after=1
  AND l.from_kind IN(1,2,3,4,5,6,7) AND l.to_kind IN(1,2,3,4)
  AND l.before_frozen_timed IS NOT NULL
  AND EXISTS(SELECT 1 FROM game_item_timed_states t
   JOIN game_item_loop_move_receipts r ON r.transaction_id=l.transaction_id
   LEFT JOIN game_item_timed_definitions d ON
    (d.definition_family,d.definition_production_key,d.definition_revision_ref)=
    (l.definition_family,l.definition_production_key,l.definition_revision_ref)
   WHERE t.item_instance_id=l.item_instance_id AND to_jsonb(t)=l.before_frozen_timed
    AND t.state_revision>0 AND t.deadline_at IS NULL
    AND r.created_xact_id=pg_current_xact_id() AND l.created_xact_id=r.created_xact_id
    AND ((t.charges IS NULL AND t.remaining_ms IS NULL)
      OR (d.definition_family IS NOT NULL AND NOT d.lit_continuous
       AND (t.charges IS NULL)=(d.full_charges IS NULL)
       AND (t.remaining_ms IS NULL)=(d.full_remaining_ms IS NULL)
       AND (t.charges IS NULL OR t.charges BETWEEN 1 AND d.full_charges)
       AND (t.remaining_ms IS NULL OR t.remaining_ms BETWEEN 1 AND d.full_remaining_ms)))),false)
$$;
CREATE FUNCTION game_item_loop_charge_capture() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 IF NEW.before_frozen_timed IS NOT NULL THEN
  RAISE EXCEPTION 'movement timed snapshot is owner-captured' USING ERRCODE='23514'; END IF;
 IF NEW.from_kind<>0 THEN
  PERFORM 1 FROM game_item_instances WHERE item_instance_id=NEW.item_instance_id FOR UPDATE;
  SELECT to_jsonb(t) INTO NEW.before_frozen_timed FROM game_item_timed_states t
   WHERE t.item_instance_id=NEW.item_instance_id;
 END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER game_item_loop_charge_capture BEFORE INSERT ON game_item_loop_move_lines
 FOR EACH ROW EXECUTE FUNCTION game_item_loop_charge_capture();
DO $migration$ DECLARE body TEXT; original TEXT; BEGIN
 body:=pg_get_functiondef('game_item_loop_line_admit()'::regprocedure);
 original:='EXISTS (SELECT 1 FROM game_item_timed_states WHERE item_instance_id = NEW.item_instance_id)';
 IF (length(body)-length(replace(body,original,'')))/length(original)<>1 THEN
  RAISE EXCEPTION 'unexpected hosted timed item guard count'; END IF;
 EXECUTE replace(body,original,'('||original||' AND NOT game_item_loop_frozen_timed_shape(NEW))');
END $migration$;
CREATE FUNCTION game_item_loop_frozen_timed_proven() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 IF (SELECT to_jsonb(t) FROM game_item_timed_states t WHERE t.item_instance_id=NEW.item_instance_id)
      IS DISTINCT FROM NEW.before_frozen_timed
  OR NOT EXISTS(SELECT 1 FROM game_item_loop_move_receipts r
   WHERE r.transaction_id=NEW.transaction_id AND r.created_xact_id=NEW.created_xact_id
    AND r.created_xact_id=pg_current_xact_id()) THEN
  RAISE EXCEPTION 'movement changed its frozen timed state' USING ERRCODE='23514'; END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_item_loop_frozen_timed_proven AFTER INSERT ON game_item_loop_move_lines
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_item_loop_frozen_timed_proven();
DO $$ DECLARE n TEXT; BEGIN
 FOREACH n IN ARRAY ARRAY['game_item_loop_frozen_timed_shape(game_item_loop_move_lines)',
  'game_item_loop_charge_capture()','game_item_loop_frozen_timed_proven()'] LOOP
 EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',n,current_schema());
 EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC',n);
 END LOOP;
END $$;
GRANT EXECUTE ON FUNCTION game_item_loop_frozen_timed_shape(game_item_loop_move_lines) TO oteryn_game_runtime;
