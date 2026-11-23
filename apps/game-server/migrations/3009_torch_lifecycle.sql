-- Crystal 201189f8: data/items/items.xml:6312-6350 and
-- data/scripts/actions/items/decay_to.lua (the three reversible use pairs).
-- One clock per physical Item. Custody writers leave this clock untouched.
CREATE TABLE game_torch_clocks (
 item_instance_id UUID PRIMARY KEY REFERENCES game_item_instances,
 world_id UUID NOT NULL,
 deadline_ms BIGINT,
 remaining_ms BIGINT,
 CHECK ((deadline_ms IS NOT NULL AND deadline_ms>0 AND remaining_ms IS NULL)
     OR (deadline_ms IS NULL AND remaining_ms IS NOT NULL AND remaining_ms BETWEEN 1 AND 600000))
);
CREATE INDEX game_torch_due ON game_torch_clocks(world_id,deadline_ms,item_instance_id)
 WHERE deadline_ms IS NOT NULL;

CREATE TABLE game_torch_transitions (
 transaction_id UUID PRIMARY KEY DEFAULT game_character_uuid_v7(),
 item_instance_id UUID NOT NULL REFERENCES game_item_instances,
 game_session_id UUID,
 command_id NUMERIC(20,0),
 intent BYTEA,
 before_item JSONB NOT NULL,
 before_location JSONB NOT NULL,
 custody_kind SMALLINT NOT NULL CHECK(custody_kind BETWEEN 1 AND 4),
 equipment_before JSONB,
 target_key TEXT NOT NULL,
 deadline_before_ms BIGINT,
 deadline_after_ms BIGINT,
 remaining_after_ms BIGINT,
 occurred_at_ms BIGINT NOT NULL,
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
 UNIQUE(game_session_id,command_id),
 CHECK(num_nonnulls(game_session_id,command_id,intent) IN (0,3))
);
CREATE INDEX game_torch_transition_item ON game_torch_transitions(item_instance_id);
CREATE TRIGGER game_torch_transition_immutable BEFORE UPDATE OR DELETE ON game_torch_transitions
 FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_torch_transition_no_truncate BEFORE TRUNCATE ON game_torch_transitions
 EXECUTE FUNCTION game_item_reject_truncate();

-- Durations belong to each lit stage. The spent article has its own removal clock.
CREATE FUNCTION game_torch_stage(p_key TEXT)
RETURNS TABLE(id INTEGER,duration_ms BIGINT) LANGUAGE sql IMMUTABLE STRICT AS $$
 SELECT v.id,v.duration_ms FROM (VALUES
  (2920,600000::bigint),(2921,600000),(2922,300000),(2923,300000),
  (2924,300000),(2925,300000),(2926,300000)
 ) v(id,duration_ms) WHERE p_key='oteryn:item.tibia.i'||v.id
$$;

-- Resolve a moved container through its actual chain to the current carrier or
-- Ground cell. An inaccessible or unsupported location produces no custody.
CREATE FUNCTION game_torch_custody(p_item UUID)
RETURNS TABLE(kind SMALLINT,location JSONB,character_id UUID,ground JSONB,equipment JSONB)
LANGUAGE sql STABLE AS $$
 WITH RECURSIVE ancestry(item,depth) AS (
  VALUES(p_item,0)
  UNION ALL
  SELECT n.parent_item_instance_id,a.depth+1
   FROM ancestry a JOIN game_item_nested_entries n ON n.item_instance_id=a.item
   WHERE a.depth<16
 ), root AS (SELECT item FROM ancestry ORDER BY depth DESC LIMIT 1),
 holder AS (
  SELECT e.character_id FROM game_item_container_entries e JOIN root ON root.item=e.item_instance_id
  UNION ALL SELECT b.character_id FROM game_item_container_slots b JOIN root ON root.item=b.item_instance_id
  UNION ALL SELECT q.character_id FROM game_character_equipment_slots q JOIN root ON root.item=q.item_instance_id
 ), tile AS (
  SELECT to_jsonb(g) AS location FROM game_item_ground_locations g JOIN root ON root.item=g.item_instance_id
 )
 SELECT 1::smallint,t.location,NULL::uuid,t.location,NULL::jsonb FROM tile t WHERE (t.location->>'item_instance_id')::uuid=p_item
 UNION ALL SELECT 2::smallint,to_jsonb(e),e.character_id,NULL,NULL
  FROM game_item_container_entries e WHERE e.item_instance_id=p_item
 UNION ALL SELECT 3::smallint,to_jsonb(q),q.character_id,NULL,to_jsonb(es)
  FROM game_character_equipment_slots q JOIN game_character_equipment_state es USING(character_id)
  WHERE q.item_instance_id=p_item
 UNION ALL SELECT 4::smallint,to_jsonb(n),h.character_id,t.location,NULL
  FROM game_item_nested_entries n LEFT JOIN holder h ON TRUE LEFT JOIN tile t ON TRUE
  WHERE n.item_instance_id=p_item AND (h.character_id IS NOT NULL OR t.location IS NOT NULL)
$$;

CREATE FUNCTION game_torch_apply(r game_torch_transitions) RETURNS VOID LANGUAGE plpgsql AS $$
DECLARE retired BOOLEAN:=num_nonnulls(r.deadline_after_ms,r.remaining_after_ms)=0;
BEGIN
 INSERT INTO game_torch_transitions SELECT (r).*;
 UPDATE game_item_instances SET
  definition_production_key=r.target_key,
  quantity=CASE WHEN retired THEN 0 ELSE quantity END,
  lifecycle=CASE WHEN retired THEN 2 ELSE lifecycle END,
  last_transaction_id=r.transaction_id WHERE item_instance_id=r.item_instance_id;
 IF retired THEN
  CASE r.custody_kind
   WHEN 1 THEN DELETE FROM game_item_ground_locations WHERE item_instance_id=r.item_instance_id;
   WHEN 2 THEN DELETE FROM game_item_container_entries WHERE item_instance_id=r.item_instance_id;
   WHEN 3 THEN DELETE FROM game_character_equipment_slots WHERE item_instance_id=r.item_instance_id;
   WHEN 4 THEN DELETE FROM game_item_nested_entries WHERE item_instance_id=r.item_instance_id;
  END CASE;
  IF NOT FOUND THEN RAISE EXCEPTION 'torch expiry requires current custody' USING ERRCODE='23514'; END IF;
 ELSE
  INSERT INTO game_torch_clocks VALUES(r.item_instance_id,(r.before_item->>'world_id')::uuid,
      r.deadline_after_ms,r.remaining_after_ms)
  ON CONFLICT (item_instance_id) DO UPDATE
   SET deadline_ms=excluded.deadline_ms,remaining_ms=excluded.remaining_ms;
 END IF;
 IF r.custody_kind=3 THEN
  UPDATE game_character_equipment_state SET revision=revision+1,last_transaction_id=r.transaction_id
   WHERE character_id=(r.equipment_before->>'character_id')::uuid
     AND to_jsonb(game_character_equipment_state)=r.equipment_before;
  IF NOT FOUND THEN RAISE EXCEPTION 'torch requires current equipment epoch' USING ERRCODE='23514'; END IF;
 END IF;
END $$;

CREATE FUNCTION game_torch_use(p_item UUID,p_session UUID,p_command NUMERIC,p_intent BYTEA,
 p_generation NUMERIC,p_connection NUMERIC,p_position INTEGER[],p_map TEXT,p_frame BYTEA,p_digest TEXT)
RETURNS SMALLINT LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE
 r game_torch_transitions%ROWTYPE; i game_item_instances%ROWTYPE; c game_torch_clocks%ROWTYPE;
 s game_durability_reconnect_sessions%ROWTYPE; k RECORD; stage RECORD; pos BYTEA;
 now_ms BIGINT;
BEGIN
 SELECT * INTO s FROM game_durability_reconnect_sessions s0 WHERE s0.game_session_id=p_session;
 IF NOT FOUND OR p_intent IS NULL OR octet_length(p_intent) NOT BETWEEN 1 AND 529
  OR p_command IS NULL OR p_command NOT BETWEEN 1 AND 18446744073709551615 THEN RETURN 2; END IF;
 SELECT * INTO r FROM game_torch_transitions h WHERE h.game_session_id=p_session AND h.command_id=p_command;
 IF FOUND THEN RETURN CASE WHEN r.intent=p_intent THEN 1 ELSE 2 END; END IF;
 SELECT * INTO i FROM game_item_instances i0 WHERE i0.item_instance_id=p_item
  AND i0.definition_family='Item' AND i0.definition_revision_ref='definition-r1'
  AND EXISTS(SELECT 1 FROM game_torch_stage(i0.definition_production_key))
  FOR UPDATE;
 IF NOT FOUND THEN RETURN 0; END IF;
 IF i.lifecycle<>1 OR i.quantity<>1
  OR s.current_transport_ref IS NULL
  OR s.session_state <> CASE WHEN COALESCE(s.fresh_replay_key,s.initial_fresh_replay_key) IS NOT NULL
                            THEN 2 ELSE 1 END
  OR s.runtime_scope_kind<>1
  OR i.world_id IS DISTINCT FROM s.runtime_scope_world_id
  OR s.scope_ownership_generation IS DISTINCT FROM p_generation OR s.current_generation IS DISTINCT FROM p_connection
  OR NOT EXISTS(SELECT 1 FROM game_durability_admission_character_guards g WHERE g.character_id=s.character_id
   AND g.eligible AND g.holder_game_session_id=p_session AND g.lease_generation=s.character_lease_generation)
  OR NOT EXISTS(SELECT 1 FROM game_runtime_scope_assignments a WHERE a.world_id=i.world_id
   AND a.channel_id=s.runtime_scope_channel_id AND a.state=1 AND a.ownership_generation=p_generation)
 THEN RETURN 2; END IF;
 SELECT * INTO k FROM game_torch_custody(p_item);
 IF NOT FOUND THEN RETURN 2; END IF;
 IF k.ground IS NOT NULL THEN
  IF (k.ground->>'channel_id')::uuid IS DISTINCT FROM s.runtime_scope_channel_id
   OR (k.ground->>'runtime_scope_ownership_generation')::numeric IS DISTINCT FROM p_generation
   OR k.ground->>'map_revision' IS DISTINCT FROM p_map OR k.ground->>'content_revision' IS DISTINCT FROM p_digest
   OR k.ground->>'native_room_placement_context' IS DISTINCT FROM ('\x'||encode(p_frame,'hex'))
   OR cardinality(p_position) IS DISTINCT FROM 3 OR array_position(p_position,NULL) IS NOT NULL
   OR p_position[1] NOT BETWEEN 0 AND 65535 OR p_position[2] NOT BETWEEN 0 AND 65535
   OR p_position[3] NOT BETWEEN 0 AND 32767
  THEN RETURN 2; END IF;
  pos:=decode(substring(k.ground->>'spatial_position' FROM 3),'hex');
  IF octet_length(pos)<>10 THEN RETURN 2; END IF;
  -- Runtime positions are positive z. Ground custody stores x/y and native -z.
  IF NOT EXISTS(SELECT 1 FROM generate_series(-1,1) dx CROSS JOIN generate_series(-1,1) dy
   WHERE pos=int4send(p_position[1]+dx)||int4send(p_position[2]+dy)||int2send((-p_position[3])::smallint))
  THEN RETURN 3; END IF;
 ELSIF k.character_id IS DISTINCT FROM s.character_id THEN RETURN 2;
 END IF;

 now_ms:=floor(extract(epoch FROM clock_timestamp())*1000)::bigint;
 SELECT * INTO STRICT stage FROM game_torch_stage(i.definition_production_key);
 SELECT * INTO c FROM game_torch_clocks t WHERE t.item_instance_id=p_item;
 IF stage.id=2926 OR c.deadline_ms<=now_ms THEN RETURN 4; END IF;
 r.transaction_id:=game_character_uuid_v7();
 r.item_instance_id:=p_item;
 r.game_session_id:=p_session;
 r.command_id:=p_command;
 r.intent:=p_intent;
 r.before_item:=to_jsonb(i);
 r.before_location:=k.location;
 r.custody_kind:=k.kind;
 r.equipment_before:=k.equipment;
 r.deadline_before_ms:=c.deadline_ms;
 r.occurred_at_ms:=now_ms;
 r.created_xact_id:=pg_current_xact_id();
 IF mod(stage.id,2)=0 THEN
  r.target_key:='oteryn:item.tibia.i'||(stage.id+1);
  r.deadline_after_ms:=now_ms+coalesce(c.remaining_ms,stage.duration_ms);
 ELSE
  r.target_key:='oteryn:item.tibia.i'||(stage.id-1);
  r.remaining_after_ms:=coalesce(c.deadline_ms-now_ms,stage.duration_ms);
 END IF;
 PERFORM game_torch_apply(r);
 RETURN 1;
END $$;

-- The predecessor deadline, rather than the worker's wake-up, starts each
-- successor. A single late pass can settle several elapsed stages atomically.
CREATE FUNCTION game_torch_drain(p_world UUID,p_channel UUID,p_generation NUMERIC,p_limit INTEGER)
RETURNS INTEGER LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE
 r game_torch_transitions%ROWTYPE; ready RECORD; k RECORD; stage RECORD; successor RECORD;
 now_ms BIGINT:=floor(extract(epoch FROM clock_timestamp())*1000)::bigint;
 next_id INTEGER; next_deadline BIGINT; changed INTEGER:=0;
BEGIN
 IF p_limit IS NULL OR p_limit NOT BETWEEN 1 AND 500 OR NOT EXISTS(
  SELECT 1 FROM game_runtime_scope_assignments a WHERE a.world_id=p_world AND a.channel_id=p_channel
   AND a.state=1 AND a.ownership_generation=p_generation)
 THEN RETURN 0; END IF;

 FOR ready IN
  SELECT i.item_instance_id,to_jsonb(i) AS before_item,i.definition_production_key,c.deadline_ms
  FROM game_torch_clocks c JOIN game_item_instances i USING(item_instance_id)
  CROSS JOIN LATERAL game_torch_custody(i.item_instance_id) custody
  WHERE c.world_id=p_world AND i.world_id=p_world AND i.lifecycle=1 AND i.quantity=1
    AND i.definition_family='Item' AND i.definition_revision_ref='definition-r1'
    AND c.deadline_ms<=now_ms
    AND ((custody.ground->>'channel_id')::uuid=p_channel
        AND (custody.ground->>'runtime_scope_ownership_generation')::numeric=p_generation
      OR (custody.character_id IS NOT NULL AND NOT EXISTS(
       SELECT 1 FROM game_durability_reconnect_sessions s WHERE s.character_id=custody.character_id
        AND s.session_state IN(1,2) AND (s.runtime_scope_kind<>1
          OR s.runtime_scope_world_id IS DISTINCT FROM p_world OR s.runtime_scope_channel_id IS DISTINCT FROM p_channel
          OR s.scope_ownership_generation IS DISTINCT FROM p_generation))))
  ORDER BY c.deadline_ms,i.item_instance_id LIMIT p_limit FOR UPDATE OF i SKIP LOCKED
 LOOP
  BEGIN
   SELECT * INTO k FROM game_torch_custody(ready.item_instance_id);
   IF NOT FOUND THEN CONTINUE; END IF;
   IF k.character_id IS NOT NULL THEN
    -- Skip a competing move or handoff. Recheck the root and its current session
    -- under the normal Character lock before changing an item in its tree.
    PERFORM 1 FROM game_character_roots c WHERE c.character_id=k.character_id
     AND c.world_id=p_world AND c.lifecycle=1 FOR UPDATE NOWAIT;
    IF NOT FOUND OR NOT EXISTS(SELECT 1 FROM game_torch_custody(ready.item_instance_id) current
      WHERE current.character_id=k.character_id AND current.location=k.location)
     OR EXISTS(SELECT 1 FROM game_bazaar_locks b WHERE b.character_id=k.character_id AND b.state='locked')
     OR EXISTS(SELECT 1 FROM game_durability_reconnect_sessions s WHERE s.character_id=k.character_id
       AND s.session_state IN(1,2) AND (s.runtime_scope_kind<>1 OR s.runtime_scope_world_id IS DISTINCT FROM p_world
         OR s.runtime_scope_channel_id IS DISTINCT FROM p_channel OR s.scope_ownership_generation IS DISTINCT FROM p_generation))
    THEN CONTINUE; END IF;
   ELSIF (k.ground->>'channel_id')::uuid IS DISTINCT FROM p_channel
    OR (k.ground->>'runtime_scope_ownership_generation')::numeric IS DISTINCT FROM p_generation
   THEN CONTINUE;
   END IF;

   SELECT * INTO stage FROM game_torch_stage(ready.definition_production_key);
   IF NOT FOUND OR stage.id NOT IN(2921,2923,2925,2926) THEN CONTINUE; END IF;
   next_id:=stage.id;
   next_deadline:=ready.deadline_ms;
   WHILE next_deadline<=now_ms LOOP
    IF next_id=2926 THEN
     next_deadline:=NULL;
     EXIT;
    END IF;
    next_id:=least(next_id+2,2926);
    SELECT * INTO STRICT successor FROM game_torch_stage('oteryn:item.tibia.i'||next_id);
    next_deadline:=next_deadline+successor.duration_ms;
   END LOOP;

   r.transaction_id:=game_character_uuid_v7();
   r.item_instance_id:=ready.item_instance_id;
   r.before_item:=ready.before_item;
   r.before_location:=k.location;
   r.custody_kind:=k.kind;
   r.equipment_before:=k.equipment;
   r.target_key:='oteryn:item.tibia.i'||next_id;
   r.deadline_before_ms:=ready.deadline_ms;
   r.deadline_after_ms:=next_deadline;
   r.occurred_at_ms:=now_ms;
   r.created_xact_id:=pg_current_xact_id();
   PERFORM game_torch_apply(r);
   changed:=changed+1;
  EXCEPTION WHEN lock_not_available THEN
   NULL;
  END;
 END LOOP;
 RETURN changed;
END $$;

-- Give pre-existing visible lit/spent instances their stage budget at migration.
-- Minted instances receive the same budget in their original mint transaction.
INSERT INTO game_torch_clocks
 SELECT i.item_instance_id,i.world_id,
  floor(extract(epoch FROM statement_timestamp())*1000)::bigint+s.duration_ms,NULL
 FROM game_item_instances i CROSS JOIN LATERAL game_torch_stage(i.definition_production_key) s
 WHERE i.lifecycle=1 AND i.quantity=1 AND i.definition_family='Item'
  AND i.definition_revision_ref='definition-r1' AND s.id IN(2921,2923,2925,2926);

CREATE FUNCTION game_torch_clock_instance() RETURNS TRIGGER
LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 IF TG_OP='UPDATE' THEN
  DELETE FROM game_torch_clocks WHERE item_instance_id=NEW.item_instance_id;
 ELSIF NEW.lifecycle=1 AND NEW.quantity=1 AND NEW.definition_family='Item'
   AND NEW.definition_revision_ref='definition-r1' THEN
  INSERT INTO game_torch_clocks
   SELECT NEW.item_instance_id,NEW.world_id,
    floor(extract(epoch FROM statement_timestamp())*1000)::bigint+s.duration_ms,NULL
   FROM game_torch_stage(NEW.definition_production_key) s WHERE s.id IN(2921,2923,2925,2926);
 END IF;
 RETURN NULL;
END $$;
CREATE TRIGGER game_torch_clock_created AFTER INSERT ON game_item_instances FOR EACH ROW
 WHEN (NEW.definition_production_key IN ('oteryn:item.tibia.i2921','oteryn:item.tibia.i2923',
     'oteryn:item.tibia.i2925','oteryn:item.tibia.i2926'))
 EXECUTE FUNCTION game_torch_clock_instance();
CREATE TRIGGER game_torch_clock_retired AFTER UPDATE ON game_item_instances FOR EACH ROW
 WHEN (OLD.lifecycle=1 AND NEW.lifecycle<>1 AND OLD.definition_production_key IN (
    'oteryn:item.tibia.i2920','oteryn:item.tibia.i2921','oteryn:item.tibia.i2922','oteryn:item.tibia.i2923',
    'oteryn:item.tibia.i2924','oteryn:item.tibia.i2925','oteryn:item.tibia.i2926'))
 EXECUTE FUNCTION game_torch_clock_instance();

-- Only the owner functions can insert these same-XID receipts. Match exact
-- predecessors and successors, including location in removal proofs, so another
-- Item writer cannot borrow the receipt to change an unrelated row.
CREATE FUNCTION game_torch_item_change(p_old JSONB,p_new JSONB) RETURNS BOOLEAN
LANGUAGE sql STABLE AS $$
 SELECT EXISTS(SELECT 1 FROM game_torch_transitions r
  WHERE r.transaction_id=(p_new->>'last_transaction_id')::uuid
   AND r.created_xact_id=pg_current_xact_id() AND r.before_item=p_old
   AND p_new->>'definition_production_key'=r.target_key
   AND (p_new->>'quantity')::bigint=CASE WHEN num_nonnulls(r.deadline_after_ms,r.remaining_after_ms)=0 THEN 0 ELSE 1 END
   AND (p_new->>'lifecycle')::integer=CASE WHEN num_nonnulls(r.deadline_after_ms,r.remaining_after_ms)=0 THEN 2 ELSE 1 END
   AND (p_new-ARRAY['definition_production_key','quantity','lifecycle','last_transaction_id','state_revision'])
     =(p_old-ARRAY['definition_production_key','quantity','lifecycle','last_transaction_id','state_revision']))
$$;
CREATE FUNCTION game_torch_location_removed(p_location JSONB,p_kind INTEGER) RETURNS BOOLEAN
LANGUAGE sql STABLE AS $$
 SELECT EXISTS(SELECT 1 FROM game_item_instances i JOIN game_torch_transitions r
    ON r.transaction_id=i.last_transaction_id AND r.item_instance_id=i.item_instance_id
  WHERE i.item_instance_id=(p_location->>'item_instance_id')::uuid AND i.lifecycle=2
    AND r.created_xact_id=pg_current_xact_id() AND r.custody_kind=p_kind
    AND r.before_location=p_location AND num_nonnulls(r.deadline_after_ms,r.remaining_after_ms)=0)
$$;

DO $migration$
DECLARE proc TEXT; body TEXT; arm TEXT; start_at INTEGER; spec RECORD;
BEGIN
 FOREACH proc IN ARRAY ARRAY['game_item_instance_guard','game_item_instance_change_proven','game_equipment_state_proven'] LOOP
  body:=pg_get_functiondef((proc||'()')::regprocedure);
  start_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
  IF start_at=0 THEN RAISE EXCEPTION 'missing torch guard predecessor %',proc; END IF;
  CASE proc
   WHEN 'game_item_instance_guard' THEN arm:=$arm$
    IF TG_OP='UPDATE' AND NEW.state_revision=OLD.state_revision
       AND game_torch_item_change(to_jsonb(OLD),to_jsonb(NEW)) THEN
      NEW.state_revision:=OLD.state_revision+1;
      RETURN NEW;
    END IF;
   $arm$;
   WHEN 'game_item_instance_change_proven' THEN arm:=$arm$
    IF TG_OP='UPDATE' AND NEW.state_revision=OLD.state_revision+1
       AND game_torch_item_change(to_jsonb(OLD),to_jsonb(NEW)) THEN RETURN NULL; END IF;
   $arm$;
   ELSE arm:=$arm$
    IF TG_OP='UPDATE' AND NEW.revision=OLD.revision+1
      AND (to_jsonb(NEW)-ARRAY['revision','last_transaction_id'])=(to_jsonb(OLD)-ARRAY['revision','last_transaction_id'])
      AND EXISTS(SELECT 1 FROM game_torch_transitions r JOIN game_item_instances i ON i.item_instance_id=r.item_instance_id
        WHERE r.transaction_id=NEW.last_transaction_id AND r.created_xact_id=pg_current_xact_id()
          AND r.custody_kind=3 AND r.equipment_before=to_jsonb(OLD) AND i.last_transaction_id=r.transaction_id)
    THEN RETURN NULL; END IF;
   $arm$;
  END CASE;
  EXECUTE overlay(body placing E'BEGIN\n'||arm FROM start_at FOR 5);
 END LOOP;

 FOR spec IN SELECT * FROM (VALUES ('game_item_ground_removal_proven',1),
   ('game_item_container_entry_removal_proven',2),('game_equipment_slot_proven',3),
   ('game_item_nested_entry_proven',4)) v(name,kind)
 LOOP
  body:=pg_get_functiondef((spec.name||'()')::regprocedure);
  start_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
  IF start_at=0 THEN RAISE EXCEPTION 'missing torch custody predecessor %',spec.name; END IF;
  arm:=format(E'\n IF TG_OP=''DELETE'' AND game_torch_location_removed(to_jsonb(OLD),%s) THEN RETURN NULL; END IF;',spec.kind);
  EXECUTE overlay(body placing 'BEGIN'||arm FROM start_at FOR 5);
 END LOOP;

 FOREACH proc IN ARRAY ARRAY['game_torch_stage(text)','game_torch_custody(uuid)','game_torch_apply(game_torch_transitions)',
  'game_torch_use(uuid,uuid,numeric,bytea,numeric,numeric,integer[],text,bytea,text)',
  'game_torch_drain(uuid,uuid,numeric,integer)','game_torch_clock_instance()',
  'game_torch_item_change(jsonb,jsonb)','game_torch_location_removed(jsonb,integer)'] LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',proc,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC',proc);
 END LOOP;
END $migration$;

REVOKE ALL ON game_torch_clocks,game_torch_transitions FROM PUBLIC;
GRANT SELECT ON game_torch_clocks,game_torch_transitions TO oteryn_game_runtime;
GRANT SELECT ON game_torch_clocks,game_torch_transitions TO oteryn_game_control;
GRANT EXECUTE ON FUNCTION game_torch_use(uuid,uuid,numeric,bytea,numeric,numeric,integer[],text,bytea,text),
 game_torch_drain(uuid,uuid,numeric,integer),game_torch_stage(text),game_torch_item_change(jsonb,jsonb),
 game_torch_location_removed(jsonb,integer) TO oteryn_game_runtime;
