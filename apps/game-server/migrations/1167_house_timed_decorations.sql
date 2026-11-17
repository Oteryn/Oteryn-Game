-- W18 timed aqueducts. Crystal 201189f data/items/items.xml:67147-67173,67218-67243;
-- Tile::updateThing src/items/tile.cpp:1155-1174; Item::setID src/items/item.cpp:430-451;
-- Game::startDecay src/game/game.cpp:12994-13011; src/items/decay/decay.cpp:29-54.
-- Existing Item action receipts and timed-state writes retain their authorities.
CREATE FUNCTION game_house_timed_decoration_duration(p_key TEXT) RETURNS BIGINT
LANGUAGE sql IMMUTABLE STRICT AS $$ SELECT CASE p_key
 WHEN 'oteryn:item.tibia.i35949' THEN 200000 WHEN 'oteryn:item.tibia.i35950' THEN 1000
 WHEN 'oteryn:item.tibia.i35951' THEN 200000 WHEN 'oteryn:item.tibia.i35952' THEN 1000
 WHEN 'oteryn:item.tibia.i35969' THEN 200000 WHEN 'oteryn:item.tibia.i35970' THEN 1000
 WHEN 'oteryn:item.tibia.i35971' THEN 200000 WHEN 'oteryn:item.tibia.i35972' THEN 1000 END $$;
CREATE FUNCTION game_house_timed_decoration_next(p_key TEXT) RETURNS TEXT
LANGUAGE sql IMMUTABLE STRICT AS $$ SELECT CASE p_key
 WHEN 'oteryn:item.tibia.i35949' THEN 'oteryn:item.tibia.i35950'
 WHEN 'oteryn:item.tibia.i35950' THEN 'oteryn:item.tibia.i35951'
 WHEN 'oteryn:item.tibia.i35951' THEN 'oteryn:item.tibia.i35952'
 WHEN 'oteryn:item.tibia.i35952' THEN 'oteryn:item.tibia.i35949'
 WHEN 'oteryn:item.tibia.i35969' THEN 'oteryn:item.tibia.i35970'
 WHEN 'oteryn:item.tibia.i35970' THEN 'oteryn:item.tibia.i35971'
 WHEN 'oteryn:item.tibia.i35971' THEN 'oteryn:item.tibia.i35972'
 WHEN 'oteryn:item.tibia.i35972' THEN 'oteryn:item.tibia.i35969' END $$;
CREATE FUNCTION game_house_timed_decoration_rotation(p_key TEXT) RETURNS TEXT
LANGUAGE sql IMMUTABLE STRICT AS $$ SELECT CASE p_key
 WHEN 'oteryn:item.tibia.i35949' THEN 'oteryn:item.tibia.i35969'
 WHEN 'oteryn:item.tibia.i35969' THEN 'oteryn:item.tibia.i35949'
 WHEN 'oteryn:item.tibia.i35951' THEN 'oteryn:item.tibia.i35971'
 WHEN 'oteryn:item.tibia.i35971' THEN 'oteryn:item.tibia.i35951' END $$;

CREATE FUNCTION game_house_timed_decoration_register(p_revision TEXT) RETURNS VOID
LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 INSERT INTO game_item_timed_definitions
 SELECT 'Item',s.key,p_revision,NULL,game_house_timed_decoration_duration(s.key),true
 FROM unnest(ARRAY['oteryn:item.tibia.i35949','oteryn:item.tibia.i35950',
  'oteryn:item.tibia.i35951','oteryn:item.tibia.i35952','oteryn:item.tibia.i35969',
  'oteryn:item.tibia.i35970','oteryn:item.tibia.i35971','oteryn:item.tibia.i35972']) s(key)
 ON CONFLICT DO NOTHING;
 IF (SELECT count(*) FROM game_item_timed_definitions d WHERE d.definition_family='Item'
   AND d.definition_revision_ref=p_revision AND game_house_timed_decoration_duration(d.definition_production_key) IS NOT NULL
   AND d.full_charges IS NULL AND d.lit_continuous
   AND d.full_remaining_ms=game_house_timed_decoration_duration(d.definition_production_key))<>8
 THEN RAISE EXCEPTION 'aqueduct definition disagrees with pinned lifetime' USING ERRCODE='23514'; END IF;
END $$;

-- A delayed observer catches up the whole closed animation, rather than restarting
-- a House timer when a connection arrives. At most four residual stages are visited.
CREATE FUNCTION game_house_timed_decoration_phase(p_key TEXT,p_deadline BIGINT,p_now BIGINT)
RETURNS JSONB LANGUAGE plpgsql IMMUTABLE STRICT AS $$
DECLARE k TEXT:=p_key; d BIGINT:=p_deadline; cycles BIGINT; n INTEGER;
BEGIN
 IF game_house_timed_decoration_duration(k) IS NULL OR p_deadline<=0 OR p_now<p_deadline
 THEN RAISE EXCEPTION 'invalid aqueduct expiry predecessor' USING ERRCODE='23514'; END IF;
 cycles:=(p_now-p_deadline)/402000; d:=d+cycles*402000;
 FOR n IN 1..4 LOOP
  k:=game_house_timed_decoration_next(k); d:=d+game_house_timed_decoration_duration(k);
  IF d>p_now THEN RETURN jsonb_build_object('key',k,'deadline',d,'cycles',cycles,'steps',n); END IF;
 END LOOP;
 RAISE EXCEPTION 'aqueduct cycle did not advance' USING ERRCODE='23514';
END $$;

-- Crystal src/game/game.cpp:3118-3128 takes the requested form's decayTo at <=1ms.
ALTER FUNCTION game_house_decoration_pair(TEXT,TEXT) RENAME TO game_house_decoration_pair_before_timers;
CREATE FUNCTION game_house_decoration_pair(p_source TEXT,p_target TEXT) RETURNS BOOLEAN
LANGUAGE sql IMMUTABLE STRICT SECURITY DEFINER AS $$ SELECT game_house_decoration_pair_before_timers(p_source,p_target)
 OR (p_source,p_target) IN(
 ('oteryn:item.tibia.i35949','oteryn:item.tibia.i35970'),
 ('oteryn:item.tibia.i35969','oteryn:item.tibia.i35950'),
 ('oteryn:item.tibia.i35951','oteryn:item.tibia.i35972'),
 ('oteryn:item.tibia.i35971','oteryn:item.tibia.i35952')) $$;

CREATE FUNCTION game_house_decoration_timed_action_complete(p_transaction UUID) RETURNS VOID
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE r game_item_consume_transform_records%ROWTYPE; w game_item_timed_state_writes%ROWTYPE;
 t game_item_timed_states%ROWTYPE; m JSONB; b JSONB; selected TEXT; duration BIGINT; expired BOOLEAN;
BEGIN
 SELECT * INTO r FROM game_item_consume_transform_records WHERE transaction_id=p_transaction;
 SELECT * INTO w FROM game_item_timed_state_writes WHERE transaction_id=p_transaction;
 SELECT * INTO t FROM game_item_timed_states WHERE item_instance_id=r.source_item_instance_id;
 m:=r.effects->'timed_decoration'; b:=m->'before'; expired:=(m->>'expired_on_rotation')::boolean;
 selected:=game_house_timed_decoration_rotation(r.before_item->>'definition_production_key');
 IF expired THEN selected:=game_house_timed_decoration_next(selected); END IF;
 duration:=game_house_timed_decoration_duration(selected);
 IF r.transaction_id IS NULL OR r.created_xact_id<>pg_current_xact_id() OR r.source_kind<>5
  OR r.effects->>'kind' IS DISTINCT FROM 'house_decoration'
  OR r.effects->>'decoration_family' IS DISTINCT FROM 'native_transform'
  OR m->>'schema' IS DISTINCT FROM 'oteryn-house-timed-decoration-v1'
  OR selected IS NULL OR m->>'selected_key' IS DISTINCT FROM selected
  OR r.effects->'house_after'->>'key' IS DISTINCT FROM selected
  OR expired IS NULL OR m->>'observed_ms' IS NULL OR (m->>'observed_ms')::bigint<=0
  OR expired IS DISTINCT FROM coalesce((b->>'deadline_at')::bigint<=(m->>'observed_ms')::bigint+1,false)
  OR (b IS DISTINCT FROM 'null'::jsonb AND (jsonb_typeof(b)<>'object'
   OR b->>'item_instance_id' IS DISTINCT FROM r.source_item_instance_id::text
   OR b->'charges' IS DISTINCT FROM 'null'::jsonb
   OR (b->>'remaining_ms')::bigint NOT BETWEEN 1 AND 200000))
  OR w.item_instance_id IS DISTINCT FROM r.source_item_instance_id
  OR w.created_xact_id<>pg_current_xact_id() OR w.holder_character_id IS NOT NULL
  OR w.expected_revision IS DISTINCT FROM coalesce((b->>'state_revision')::numeric,0)
  OR b IS DISTINCT FROM (CASE WHEN w.expected_revision=0 THEN 'null'::jsonb ELSE
   jsonb_build_object('item_instance_id',w.item_instance_id,'charges',w.charges_before,
    'remaining_ms',w.remaining_ms_before,'deadline_at',w.deadline_before,'state_revision',w.expected_revision) END)
  OR w.cause IS DISTINCT FROM (CASE WHEN expired THEN 4 ELSE 5 END)
  OR w.definition_before_family IS DISTINCT FROM 'Item' OR w.definition_after_family IS DISTINCT FROM 'Item'
  OR w.definition_before_production_key IS DISTINCT FROM r.before_item->>'definition_production_key'
  OR w.definition_before_revision_ref IS DISTINCT FROM r.before_item->>'definition_revision_ref'
  OR w.definition_after_production_key IS DISTINCT FROM selected
  OR w.definition_after_revision_ref IS DISTINCT FROM r.before_item->>'definition_revision_ref'
  OR w.charges_before IS NOT NULL OR w.charges_after IS NOT NULL OR w.full_charges IS NOT NULL
  OR w.full_remaining_ms IS DISTINCT FROM duration OR NOT w.lit_continuous
  OR w.remaining_ms_before IS DISTINCT FROM coalesce((b->>'remaining_ms')::bigint,200000)
  OR w.deadline_before IS DISTINCT FROM (b->>'deadline_at')::bigint
  OR w.remaining_ms_after IS DISTINCT FROM duration
  OR w.deadline_after IS DISTINCT FROM w.committed_at+duration
  OR w.committed_at<(m->>'observed_ms')::bigint
  OR t.item_instance_id IS NULL OR t.state_revision<>w.expected_revision+1
  OR t.charges IS NOT NULL OR t.remaining_ms IS DISTINCT FROM duration OR t.deadline_at IS DISTINCT FROM w.deadline_after
  OR EXISTS(SELECT 1 FROM game_item_timed_state_writes other WHERE other.item_instance_id=w.item_instance_id
   AND other.created_xact_id=pg_current_xact_id() AND other.expected_revision<>w.expected_revision)
 THEN RAISE EXCEPTION 'aqueduct rotation requires exact timed receipt, predecessor and reset' USING ERRCODE='23514'; END IF;
 PERFORM game_house_decoration_complete(p_transaction);
END $$;

CREATE FUNCTION game_house_decoration_timed_action_guard() RETURNS TRIGGER
LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 IF NEW.source_kind=5 AND game_house_timed_decoration_rotation(NEW.before_item->>'definition_production_key') IS NOT NULL
 THEN PERFORM game_house_decoration_timed_action_complete(NEW.transaction_id); END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_house_decoration_timed_action_guard AFTER INSERT ON game_item_consume_transform_records
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_house_decoration_timed_action_guard();

CREATE FUNCTION game_house_decoration_timed_deadline_stamp() RETURNS TRIGGER
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE r game_item_consume_transform_records%ROWTYPE;
BEGIN
 SELECT * INTO r FROM game_item_consume_transform_records WHERE transaction_id=NEW.transaction_id;
 IF r.created_xact_id=pg_current_xact_id() AND r.source_kind=5
  AND r.source_item_instance_id=NEW.item_instance_id AND r.effects->>'kind'='house_decoration'
  AND r.effects->'timed_decoration'->>'schema'='oteryn-house-timed-decoration-v1'
  AND game_house_timed_decoration_rotation(r.before_item->>'definition_production_key') IS NOT NULL
  AND NEW.cause IN(4,5)
 THEN NEW.deadline_after:=NEW.committed_at+game_house_timed_decoration_duration(r.effects->'timed_decoration'->>'selected_key'); END IF;
 RETURN NEW;
END $$;
-- PostgreSQL orders BEFORE triggers by name: the ordinary write stamp runs first.
CREATE TRIGGER zz_game_house_decoration_timed_deadline_stamp BEFORE INSERT ON game_item_timed_state_writes
 FOR EACH ROW EXECUTE FUNCTION game_house_decoration_timed_deadline_stamp();

CREATE FUNCTION game_house_decoration_timed_apply(p_transaction UUID) RETURNS VOID
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE r game_item_consume_transform_records%ROWTYPE; t game_item_timed_states%ROWTYPE;
 w game_item_timed_state_writes%ROWTYPE; m JSONB; selected TEXT; duration BIGINT;
BEGIN
 SELECT * INTO r FROM game_item_consume_transform_records WHERE transaction_id=p_transaction;
 m:=r.effects->'timed_decoration'; selected:=m->>'selected_key'; duration:=game_house_timed_decoration_duration(selected);
 IF r.transaction_id IS NULL OR r.created_xact_id<>pg_current_xact_id() OR r.source_kind<>5
  OR r.effects->>'kind' IS DISTINCT FROM 'house_decoration'
  OR m->>'schema' IS DISTINCT FROM 'oteryn-house-timed-decoration-v1' OR duration IS NULL
  OR game_house_timed_decoration_rotation(r.before_item->>'definition_production_key') IS NULL
 THEN RAISE EXCEPTION 'aqueduct timer requires its current action receipt' USING ERRCODE='23514'; END IF;
 SELECT * INTO t FROM game_item_timed_states WHERE item_instance_id=r.source_item_instance_id FOR UPDATE;
 IF (CASE WHEN t.item_instance_id IS NULL THEN 'null'::jsonb ELSE to_jsonb(t) END) IS DISTINCT FROM m->'before'
 THEN RAISE EXCEPTION 'aqueduct timer predecessor changed' USING ERRCODE='23514'; END IF;
 PERFORM game_house_timed_decoration_register(r.before_item->>'definition_revision_ref');
 w.item_instance_id:=r.source_item_instance_id; w.expected_revision:=coalesce(t.state_revision,0);
 w.cause:=CASE WHEN (m->>'expired_on_rotation')::boolean THEN 4 ELSE 5 END; w.transaction_id:=r.transaction_id;
 w.definition_before_family:='Item'; w.definition_before_production_key:=r.before_item->>'definition_production_key';
 w.definition_before_revision_ref:=r.before_item->>'definition_revision_ref'; w.definition_after_family:='Item';
 w.definition_after_production_key:=selected; w.definition_after_revision_ref:=w.definition_before_revision_ref;
 w.full_remaining_ms:=duration; w.lit_continuous:=true;
 w.remaining_ms_before:=coalesce(t.remaining_ms,200000); w.deadline_before:=t.deadline_at; w.remaining_ms_after:=duration;
 INSERT INTO game_item_timed_state_writes SELECT(w).* RETURNING * INTO w;
 IF w.expected_revision=0 THEN
  INSERT INTO game_item_timed_states VALUES(w.item_instance_id,NULL,duration,w.deadline_after,1);
 ELSE
  UPDATE game_item_timed_states SET remaining_ms=duration,deadline_at=w.deadline_after,state_revision=state_revision+1
   WHERE item_instance_id=w.item_instance_id;
 END IF;
END $$;

CREATE TABLE game_house_timed_decoration_expiries (
 transaction_id UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(transaction_id)),
 item_instance_id UUID NOT NULL REFERENCES game_item_instances,
 game_session_id UUID NOT NULL, character_id UUID NOT NULL REFERENCES game_character_roots,
 connection_generation NUMERIC(20,0) NOT NULL, lease_generation NUMERIC(20,0) NOT NULL,
 scope_generation NUMERIC(20,0) NOT NULL, node_id UUID NOT NULL, node_revision NUMERIC(20,0) NOT NULL,
 world_id UUID NOT NULL, house_key TEXT NOT NULL,
 before_item JSONB NOT NULL, before_timed JSONB NOT NULL, after_item JSONB NOT NULL, location JSONB NOT NULL,
 observed_ms BIGINT NOT NULL, after_deadline BIGINT NOT NULL,
 skipped_cycles BIGINT NOT NULL, steps INTEGER NOT NULL,
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
 FOREIGN KEY(world_id,house_key) REFERENCES game_house_properties(world_id,house_key)
);
CREATE INDEX game_house_timed_expiry_item ON game_house_timed_decoration_expiries(item_instance_id,observed_ms DESC);
CREATE INDEX game_house_timed_expiry_character ON game_house_timed_decoration_expiries(character_id);
CREATE INDEX game_house_timed_expiry_house ON game_house_timed_decoration_expiries(world_id,house_key);
REVOKE ALL ON game_house_timed_decoration_expiries FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
GRANT SELECT ON game_house_timed_decoration_expiries TO oteryn_game_runtime,oteryn_game_control;
CREATE TRIGGER game_house_timed_expiry_immutable BEFORE UPDATE OR DELETE ON game_house_timed_decoration_expiries
 FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_house_timed_expiry_no_truncate BEFORE TRUNCATE ON game_house_timed_decoration_expiries
 FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();

CREATE FUNCTION game_house_timed_decoration_expiry_stamp() RETURNS TRIGGER
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE i game_item_instances%ROWTYPE; t game_item_timed_states%ROWTYPE;
 h game_item_house_interior_locations%ROWTYPE; phase JSONB;
BEGIN
 PERFORM game_house_podium_fence(NEW.game_session_id,NEW.character_id,NEW.connection_generation,
  NEW.lease_generation,NEW.scope_generation,NEW.node_id,NEW.node_revision,NEW.world_id,NEW.house_key);
 SELECT * INTO i FROM game_item_instances WHERE item_instance_id=NEW.item_instance_id FOR UPDATE;
 SELECT * INTO t FROM game_item_timed_states WHERE item_instance_id=NEW.item_instance_id FOR UPDATE;
 SELECT * INTO h FROM game_item_house_interior_locations WHERE item_instance_id=NEW.item_instance_id;
 NEW.observed_ms:=floor(extract(epoch FROM clock_timestamp())*1000)::bigint;
 IF i.item_instance_id IS NULL OR i.lifecycle<>1 OR i.quantity<>1 OR i.definition_family<>'Item'
  OR i.world_id<>NEW.world_id OR h.world_id IS DISTINCT FROM NEW.world_id OR h.house_key IS DISTINCT FROM NEW.house_key
  OR game_house_timed_decoration_duration(i.definition_production_key) IS NULL
  OR t.item_instance_id IS NULL OR t.charges IS NOT NULL OR t.deadline_at IS NULL OR t.deadline_at>NEW.observed_ms
  OR t.remaining_ms NOT BETWEEN 1 AND game_house_timed_decoration_duration(i.definition_production_key)
  OR i.state_revision>=18446744073709551615 OR t.state_revision>=18446744073709551615
 THEN RAISE EXCEPTION 'aqueduct expiry requires current due House item' USING ERRCODE='23514'; END IF;
 phase:=game_house_timed_decoration_phase(i.definition_production_key,t.deadline_at,NEW.observed_ms);
 NEW.before_item:=to_jsonb(i); NEW.before_timed:=to_jsonb(t); NEW.location:=to_jsonb(h);
 NEW.after_item:=to_jsonb(i)||jsonb_build_object('definition_production_key',phase->>'key',
  'state_revision',i.state_revision+1,'last_transaction_id',NEW.transaction_id);
 NEW.after_deadline:=(phase->>'deadline')::bigint; NEW.skipped_cycles:=(phase->>'cycles')::bigint;
 NEW.steps:=(phase->>'steps')::integer; NEW.created_xact_id:=pg_current_xact_id();
 RETURN NEW;
END $$;
CREATE TRIGGER game_house_timed_decoration_expiry_stamp BEFORE INSERT ON game_house_timed_decoration_expiries
 FOR EACH ROW EXECUTE FUNCTION game_house_timed_decoration_expiry_stamp();

CREATE FUNCTION game_house_timed_decoration_expiry_complete(p_transaction UUID) RETURNS VOID
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE r game_house_timed_decoration_expiries%ROWTYPE; w game_item_timed_state_writes%ROWTYPE;
 t game_item_timed_states%ROWTYPE; phase JSONB;
BEGIN
 SELECT * INTO r FROM game_house_timed_decoration_expiries WHERE transaction_id=p_transaction;
 SELECT * INTO w FROM game_item_timed_state_writes WHERE transaction_id=p_transaction;
 SELECT * INTO t FROM game_item_timed_states WHERE item_instance_id=r.item_instance_id;
 phase:=game_house_timed_decoration_phase(r.before_item->>'definition_production_key',
  (r.before_timed->>'deadline_at')::bigint,r.observed_ms);
 IF r.transaction_id IS NULL OR r.created_xact_id<>pg_current_xact_id()
  OR w.created_xact_id<>pg_current_xact_id() OR w.item_instance_id IS DISTINCT FROM r.item_instance_id
  OR w.cause<>4 OR w.holder_character_id IS NOT NULL
  OR w.expected_revision IS DISTINCT FROM (r.before_timed->>'state_revision')::numeric
  OR w.definition_before_family IS DISTINCT FROM 'Item' OR w.definition_after_family IS DISTINCT FROM 'Item'
  OR w.definition_before_production_key IS DISTINCT FROM r.before_item->>'definition_production_key'
  OR w.definition_before_revision_ref IS DISTINCT FROM r.before_item->>'definition_revision_ref'
  OR w.definition_after_production_key IS DISTINCT FROM phase->>'key'
  OR w.definition_after_revision_ref IS DISTINCT FROM r.before_item->>'definition_revision_ref'
  OR w.charges_before IS NOT NULL OR w.charges_after IS NOT NULL OR w.full_charges IS NOT NULL
  OR w.remaining_ms_before IS DISTINCT FROM (r.before_timed->>'remaining_ms')::bigint
  OR w.deadline_before IS DISTINCT FROM (r.before_timed->>'deadline_at')::bigint
  OR w.full_remaining_ms IS DISTINCT FROM game_house_timed_decoration_duration(phase->>'key') OR NOT w.lit_continuous
  OR w.remaining_ms_after IS DISTINCT FROM w.full_remaining_ms
  OR w.deadline_after IS DISTINCT FROM (phase->>'deadline')::bigint
  OR r.after_deadline IS DISTINCT FROM w.deadline_after OR r.skipped_cycles<>(phase->>'cycles')::bigint
  OR r.steps<>(phase->>'steps')::integer OR r.after_deadline<=r.observed_ms
  OR NOT EXISTS(SELECT 1 FROM game_item_instances i WHERE i.item_instance_id=r.item_instance_id AND to_jsonb(i)=r.after_item)
  OR NOT EXISTS(SELECT 1 FROM game_item_house_interior_locations h WHERE h.item_instance_id=r.item_instance_id AND to_jsonb(h)=r.location)
  OR t.item_instance_id IS NULL OR t.state_revision<>w.expected_revision+1 OR t.charges IS NOT NULL
  OR t.remaining_ms IS DISTINCT FROM w.remaining_ms_after OR t.deadline_at IS DISTINCT FROM w.deadline_after
  OR EXISTS(SELECT 1 FROM game_item_timed_state_writes other WHERE other.item_instance_id=w.item_instance_id
   AND other.created_xact_id=pg_current_xact_id() AND other.expected_revision<>w.expected_revision)
 THEN RAISE EXCEPTION 'aqueduct expiry lacks exact current item and timed write' USING ERRCODE='23514'; END IF;
 PERFORM game_house_podium_fence(r.game_session_id,r.character_id,r.connection_generation,
  r.lease_generation,r.scope_generation,r.node_id,r.node_revision,r.world_id,r.house_key);
END $$;
CREATE FUNCTION game_house_timed_decoration_expiry_guard() RETURNS TRIGGER
LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN PERFORM game_house_timed_decoration_expiry_complete(NEW.transaction_id); RETURN NULL; END $$;
CREATE CONSTRAINT TRIGGER game_house_timed_decoration_expiry_guard AFTER INSERT ON game_house_timed_decoration_expiries
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_house_timed_decoration_expiry_guard();

CREATE FUNCTION game_house_decoration_timed_expire(p_session UUID,p_character UUID,p_connection NUMERIC,
 p_lease NUMERIC,p_scope NUMERIC,p_node UUID,p_node_revision NUMERIC,p_world UUID,p_house TEXT)
RETURNS INTEGER LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE c RECORD; r game_house_timed_decoration_expiries%ROWTYPE; w game_item_timed_state_writes%ROWTYPE; n INTEGER:=0;
BEGIN
 PERFORM game_house_podium_fence(p_session,p_character,p_connection,p_lease,p_scope,p_node,p_node_revision,p_world,p_house);
 PERFORM 1 FROM game_character_roots WHERE character_id=p_character FOR UPDATE;
 PERFORM 1 FROM game_house_properties WHERE world_id=p_world AND house_key=p_house FOR UPDATE;
 PERFORM pg_advisory_xact_lock(hashtextextended('oteryn:house-items:'||p_world::text||':'||p_house,0));
 FOR c IN SELECT i.item_instance_id FROM game_item_instances i JOIN game_item_timed_states t USING(item_instance_id)
  JOIN game_item_house_interior_locations h USING(item_instance_id)
  WHERE h.world_id=p_world AND h.house_key=p_house AND i.lifecycle=1 AND i.quantity=1 AND i.definition_family='Item'
   AND game_house_timed_decoration_duration(i.definition_production_key) IS NOT NULL
   AND t.deadline_at<=floor(extract(epoch FROM clock_timestamp())*1000)::bigint
  ORDER BY i.item_instance_id LIMIT 16 FOR UPDATE OF i,t LOOP
  INSERT INTO game_house_timed_decoration_expiries(transaction_id,item_instance_id,game_session_id,character_id,
   connection_generation,lease_generation,scope_generation,node_id,node_revision,world_id,house_key)
  VALUES(game_character_uuid_v7(),c.item_instance_id,p_session,p_character,p_connection,p_lease,p_scope,p_node,p_node_revision,p_world,p_house)
  RETURNING * INTO r;
  PERFORM game_house_timed_decoration_register(r.before_item->>'definition_revision_ref');
  w:=NULL; w.item_instance_id:=r.item_instance_id; w.expected_revision:=(r.before_timed->>'state_revision')::numeric;
  w.cause:=4; w.transaction_id:=r.transaction_id; w.definition_before_family:='Item';
  w.definition_before_production_key:=r.before_item->>'definition_production_key';
  w.definition_before_revision_ref:=r.before_item->>'definition_revision_ref'; w.definition_after_family:='Item';
  w.definition_after_production_key:=r.after_item->>'definition_production_key';
  w.definition_after_revision_ref:=w.definition_before_revision_ref; w.lit_continuous:=true;
  w.full_remaining_ms:=game_house_timed_decoration_duration(w.definition_after_production_key);
  w.remaining_ms_before:=(r.before_timed->>'remaining_ms')::bigint; w.deadline_before:=(r.before_timed->>'deadline_at')::bigint;
  w.remaining_ms_after:=w.full_remaining_ms; w.deadline_after:=r.after_deadline;
  INSERT INTO game_item_timed_state_writes SELECT(w).* RETURNING * INTO w;
  UPDATE game_item_instances SET definition_production_key=w.definition_after_production_key,
   last_transaction_id=w.transaction_id,state_revision=state_revision+1 WHERE item_instance_id=r.item_instance_id;
  UPDATE game_item_timed_states SET remaining_ms=w.remaining_ms_after,deadline_at=w.deadline_after,state_revision=state_revision+1
   WHERE item_instance_id=r.item_instance_id;
  n:=n+1;
 END LOOP;
 RETURN n;
END $$;

DO $migration$
DECLARE body TEXT; name TEXT; at INTEGER; arm TEXT;
BEGIN
 FOREACH name IN ARRAY ARRAY['game_item_instance_guard','game_item_instance_change_proven','game_item_timed_state_write_consistency_guard'] LOOP
  body:=pg_get_functiondef((name||'()')::regprocedure); at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
  IF at=0 THEN RAISE EXCEPTION 'missing timed decoration extension boundary: %',name; END IF;
  IF name='game_item_instance_guard' THEN arm:=$arm$
   IF TG_OP='UPDATE' AND EXISTS(SELECT 1 FROM game_house_timed_decoration_expiries e WHERE e.item_instance_id=OLD.item_instance_id
    AND e.created_xact_id=pg_current_xact_id() AND e.before_item=to_jsonb(OLD) AND e.after_item=to_jsonb(NEW)
    AND e.transaction_id=NEW.last_transaction_id) THEN RETURN NEW; END IF;
$arm$;
  ELSIF name='game_item_instance_change_proven' THEN arm:=$arm$
   IF EXISTS(SELECT 1 FROM game_house_timed_decoration_expiries e WHERE e.item_instance_id=NEW.item_instance_id
    AND e.created_xact_id=pg_current_xact_id() AND e.transaction_id=NEW.last_transaction_id AND e.after_item=to_jsonb(NEW))
   THEN PERFORM game_house_timed_decoration_expiry_complete(NEW.last_transaction_id); RETURN NULL; END IF;
$arm$;
  ELSE arm:=$arm$
   IF EXISTS(SELECT 1 FROM game_item_consume_transform_records r WHERE r.transaction_id=NEW.transaction_id
    AND r.created_xact_id=pg_current_xact_id() AND r.source_kind=5
    AND game_house_timed_decoration_rotation(r.before_item->>'definition_production_key') IS NOT NULL)
   THEN PERFORM game_house_decoration_timed_action_complete(NEW.transaction_id); RETURN NULL; END IF;
   IF EXISTS(SELECT 1 FROM game_house_timed_decoration_expiries e WHERE e.transaction_id=NEW.transaction_id
    AND e.created_xact_id=pg_current_xact_id())
   THEN PERFORM game_house_timed_decoration_expiry_complete(NEW.transaction_id); RETURN NULL; END IF;
$arm$;
  END IF;
  EXECUTE overlay(body placing E'BEGIN\n'||arm from at for 5);
 END LOOP;
END $migration$;

ALTER FUNCTION game_house_furniture_destroy_expire(UUID,UUID,NUMERIC,NUMERIC,NUMERIC,UUID,NUMERIC,UUID,TEXT)
 RENAME TO game_house_furniture_destroy_expire_before_timers;
CREATE FUNCTION game_house_furniture_destroy_expire(p_session UUID,p_character UUID,p_connection NUMERIC,
 p_lease NUMERIC,p_scope NUMERIC,p_node UUID,p_node_revision NUMERIC,p_world UUID,p_house TEXT)
RETURNS INTEGER LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN RETURN game_house_furniture_destroy_expire_before_timers(p_session,p_character,p_connection,p_lease,p_scope,p_node,p_node_revision,p_world,p_house)
 +game_house_decoration_timed_expire(p_session,p_character,p_connection,p_lease,p_scope,p_node,p_node_revision,p_world,p_house); END $$;

DO $migration$
DECLARE signature TEXT;
BEGIN
 FOREACH signature IN ARRAY ARRAY[
  'game_house_timed_decoration_duration(TEXT)','game_house_timed_decoration_next(TEXT)',
  'game_house_timed_decoration_rotation(TEXT)','game_house_timed_decoration_register(TEXT)',
  'game_house_timed_decoration_phase(TEXT,BIGINT,BIGINT)','game_house_decoration_pair(TEXT,TEXT)',
  'game_house_decoration_timed_action_complete(UUID)','game_house_decoration_timed_action_guard()',
  'game_house_decoration_timed_deadline_stamp()','game_house_decoration_timed_apply(UUID)',
  'game_house_timed_decoration_expiry_stamp()','game_house_timed_decoration_expiry_complete(UUID)',
  'game_house_timed_decoration_expiry_guard()',
  'game_house_decoration_timed_expire(UUID,UUID,NUMERIC,NUMERIC,NUMERIC,UUID,NUMERIC,UUID,TEXT)',
  'game_house_furniture_destroy_expire(UUID,UUID,NUMERIC,NUMERIC,NUMERIC,UUID,NUMERIC,UUID,TEXT)'] LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',signature,current_schema());
  EXECUTE 'REVOKE ALL ON FUNCTION '||signature||' FROM PUBLIC,oteryn_game_runtime,oteryn_game_control';
 END LOOP;
END $migration$;
REVOKE ALL ON FUNCTION game_house_decoration_pair_before_timers(TEXT,TEXT),
 game_house_furniture_destroy_expire_before_timers(UUID,UUID,NUMERIC,NUMERIC,NUMERIC,UUID,NUMERIC,UUID,TEXT)
 FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
GRANT EXECUTE ON FUNCTION game_house_decoration_pair(TEXT,TEXT),game_house_decoration_timed_apply(UUID),
 game_house_furniture_destroy_expire(UUID,UUID,NUMERIC,NUMERIC,NUMERIC,UUID,NUMERIC,UUID,TEXT) TO oteryn_game_runtime;
-- Crystal src/game/game.cpp:4897-4913,4933-4949 transforms through the
-- decoration kit, clears its duration, and starts a fresh duration on unwrap.
CREATE FUNCTION game_house_timed_decoration_wrap_pair(p_before TEXT,p_after TEXT,p_action SMALLINT)
RETURNS BOOLEAN LANGUAGE sql IMMUTABLE STRICT AS $$ SELECT
 (p_action=1 AND game_house_timed_decoration_duration(p_before) IS NOT NULL AND p_after='oteryn:item.tibia.i23398') OR
 (p_action=2 AND p_before='oteryn:item.tibia.i23398' AND game_house_timed_decoration_duration(p_after) IS NOT NULL) $$;

-- The generated 1161 forward predicate retains qualified Crystal XML
-- wrapableto file:line evidence, including items.xml:67147-67154,67218-67225.
-- Unwrap keeps exact item lineage;
-- a fabricated kit without a matching prior wrap gains no placement authority.
ALTER FUNCTION game_house_furniture_wrap_pair(UUID,TEXT,TEXT,SMALLINT)
 RENAME TO game_house_furniture_wrap_pair_before_timers;
CREATE FUNCTION game_house_furniture_wrap_pair(p_item UUID,p_before TEXT,p_after TEXT,p_action SMALLINT)
RETURNS BOOLEAN LANGUAGE sql STABLE STRICT SECURITY DEFINER AS $$
 WITH latest AS (
  SELECT before_item,after_key FROM game_house_furniture_wrap_receipts
  WHERE item_instance_id=p_item AND action=1
  ORDER BY (before_item->>'state_revision')::numeric DESC LIMIT 1
 )
 SELECT (game_house_furniture_wrap_pair_before_timers(p_item,p_before,p_after,p_action)
  AND (p_action<>2 OR NOT EXISTS(SELECT 1 FROM latest) OR EXISTS(SELECT 1 FROM latest
   WHERE after_key=p_before AND before_item->>'definition_production_key'=p_after)))
 OR (p_action=1 AND p_before<>'oteryn:item.tibia.i23398'
  AND p_after='oteryn:item.tibia.i23398' AND game_house_native_wrap_eligible(p_before))
 OR (p_action=2 AND p_before='oteryn:item.tibia.i23398' AND game_house_native_wrap_eligible(p_after)
  AND EXISTS(SELECT 1 FROM latest WHERE after_key=p_before
   AND before_item->>'definition_production_key'=p_after))
$$;
DO $migration$ BEGIN
 EXECUTE format('ALTER FUNCTION game_house_furniture_wrap_pair(UUID,TEXT,TEXT,SMALLINT) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $migration$;
REVOKE ALL ON FUNCTION game_house_furniture_wrap_pair(UUID,TEXT,TEXT,SMALLINT),
 game_house_furniture_wrap_pair_before_timers(UUID,TEXT,TEXT,SMALLINT)
 FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;

-- Crystal src/game/game.cpp:4897-4913: each wrap replaces unWrapId with
-- the current form, overriding its original purchase orientation.
DO $migration$
DECLARE body TEXT; needle TEXT;
BEGIN
 body:=pg_get_functiondef('game_house_furniture_catalogue(UUID,UUID,NUMERIC,NUMERIC,NUMERIC,UUID,NUMERIC,UUID,TEXT)'::regprocedure);
 needle:='origin.target_key,origin.target_revision';
 IF position(needle IN body)=0 THEN RAISE EXCEPTION 'missing wrap target projection boundary'; END IF;
 body:=replace(body,needle,$projection$
  CASE WHEN i.definition_production_key=latest.after_key THEN latest.before_item->>'definition_production_key' ELSE origin.target_key END,
  CASE WHEN i.definition_production_key=latest.after_key THEN latest.before_item->>'definition_revision_ref' ELSE origin.target_revision END
 $projection$);
 needle:='LEFT JOIN game_house_furniture_kit_origins origin ON origin.item_instance_id=i.item_instance_id';
 IF position(needle IN body)=0 THEN RAISE EXCEPTION 'missing wrap provenance join boundary'; END IF;
 body:=replace(body,needle,$join$
 LEFT JOIN LATERAL(SELECT before_item,after_key FROM game_house_furniture_wrap_receipts prior
  WHERE prior.item_instance_id=i.item_instance_id AND prior.action=1
  ORDER BY (before_item->>'state_revision')::numeric DESC LIMIT 1) latest ON true
 LEFT JOIN game_house_furniture_kit_origins origin ON origin.item_instance_id=i.item_instance_id
 $join$);
 EXECUTE body;
END $migration$;

CREATE FUNCTION game_house_timed_decoration_wrap_complete(p_transaction UUID) RETURNS VOID
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE r game_house_furniture_wrap_receipts%ROWTYPE; w game_item_timed_state_writes%ROWTYPE;
 t game_item_timed_states%ROWTYPE; duration BIGINT;
BEGIN
 SELECT * INTO r FROM game_house_furniture_wrap_receipts WHERE transaction_id=p_transaction;
 duration:=game_house_timed_decoration_duration(CASE WHEN r.action=1 THEN r.before_item->>'definition_production_key' ELSE r.after_key END);
 SELECT * INTO w FROM game_item_timed_state_writes WHERE transaction_id=p_transaction;
 SELECT * INTO t FROM game_item_timed_states WHERE item_instance_id=r.item_instance_id;
 IF r.transaction_id IS NULL OR r.created_xact_id<>pg_current_xact_id()
  OR game_house_timed_decoration_wrap_pair(r.before_item->>'definition_production_key',r.after_key,r.action) IS DISTINCT FROM true
  OR w.transaction_id IS NULL OR w.created_xact_id<>pg_current_xact_id() OR w.item_instance_id IS DISTINCT FROM r.item_instance_id
  OR w.holder_character_id IS NOT NULL OR w.charges_before IS NOT NULL OR w.charges_after IS NOT NULL
  OR w.definition_before_family IS DISTINCT FROM 'Item' OR w.definition_after_family IS DISTINCT FROM 'Item'
  OR w.definition_before_production_key IS DISTINCT FROM r.before_item->>'definition_production_key'
  OR w.definition_after_production_key IS DISTINCT FROM r.after_key
  OR w.definition_before_revision_ref IS DISTINCT FROM r.before_item->>'definition_revision_ref'
  OR w.definition_after_revision_ref IS DISTINCT FROM r.before_item->>'definition_revision_ref'
  OR w.full_charges IS NOT NULL
  OR (w.expected_revision=0 AND (w.deadline_before IS NOT NULL
   OR w.remaining_ms_before IS DISTINCT FROM (CASE WHEN r.action=1 THEN duration ELSE NULL::bigint END)))
  OR (r.action=1 AND (w.cause<>6 OR w.lit_continuous OR w.full_remaining_ms IS NOT NULL
   OR w.remaining_ms_after IS NOT NULL OR w.deadline_after IS NOT NULL
   OR w.remaining_ms_before IS NULL OR w.remaining_ms_before NOT BETWEEN 1 AND duration))
  OR (r.action=2 AND (w.cause<>5 OR NOT w.lit_continuous OR w.full_remaining_ms IS DISTINCT FROM duration
   OR w.remaining_ms_before IS NOT NULL OR w.deadline_before IS NOT NULL
   OR w.remaining_ms_after IS DISTINCT FROM duration OR w.deadline_after IS DISTINCT FROM w.committed_at+duration))
  OR t.item_instance_id IS NULL OR t.state_revision<>w.expected_revision+1
  OR t.charges IS NOT NULL OR t.remaining_ms IS DISTINCT FROM w.remaining_ms_after OR t.deadline_at IS DISTINCT FROM w.deadline_after
  OR EXISTS(SELECT 1 FROM game_item_timed_state_writes other WHERE other.item_instance_id=w.item_instance_id
    AND other.created_xact_id=pg_current_xact_id() AND other.expected_revision<>w.expected_revision)
 THEN RAISE EXCEPTION 'aqueduct wrap requires exact current wrap and timed writes' USING ERRCODE='23514'; END IF;
 PERFORM game_house_furniture_wrap_complete(p_transaction);
END $$;
CREATE FUNCTION game_house_timed_decoration_wrap_guard() RETURNS TRIGGER
LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 IF game_house_timed_decoration_wrap_pair(NEW.before_item->>'definition_production_key',NEW.after_key,NEW.action)
 THEN PERFORM game_house_timed_decoration_wrap_complete(NEW.transaction_id); END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_house_timed_decoration_wrap_guard AFTER INSERT ON game_house_furniture_wrap_receipts
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_house_timed_decoration_wrap_guard();
CREATE FUNCTION game_house_timed_decoration_wrap_apply(p_transaction UUID) RETURNS VOID
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE r game_house_furniture_wrap_receipts%ROWTYPE; t game_item_timed_states%ROWTYPE;
 w game_item_timed_state_writes%ROWTYPE; duration BIGINT;
BEGIN
 SELECT * INTO r FROM game_house_furniture_wrap_receipts WHERE transaction_id=p_transaction;
 IF r.transaction_id IS NULL OR r.created_xact_id<>pg_current_xact_id()
  OR game_house_timed_decoration_wrap_pair(r.before_item->>'definition_production_key',r.after_key,r.action) IS DISTINCT FROM true
 THEN RAISE EXCEPTION 'aqueduct wrapping requires its current wrap receipt' USING ERRCODE='23514'; END IF;
 duration:=game_house_timed_decoration_duration(CASE WHEN r.action=1 THEN r.before_item->>'definition_production_key' ELSE r.after_key END);
 SELECT * INTO t FROM game_item_timed_states WHERE item_instance_id=r.item_instance_id FOR UPDATE;
 IF t.item_instance_id IS NOT NULL AND (t.charges IS NOT NULL
   OR (r.action=1 AND (t.remaining_ms IS NULL OR t.remaining_ms NOT BETWEEN 1 AND duration))
   OR (r.action=2 AND (t.remaining_ms IS NOT NULL OR t.deadline_at IS NOT NULL)))
 THEN RAISE EXCEPTION 'invalid aqueduct wrapping timer predecessor' USING ERRCODE='23514'; END IF;
 PERFORM game_house_timed_decoration_register(r.before_item->>'definition_revision_ref');
 INSERT INTO game_item_timed_definitions VALUES('Item','oteryn:item.tibia.i23398',r.before_item->>'definition_revision_ref',NULL,NULL,false)
 ON CONFLICT DO NOTHING;
 IF NOT EXISTS(SELECT 1 FROM game_item_timed_definitions WHERE definition_family='Item'
   AND definition_production_key='oteryn:item.tibia.i23398' AND definition_revision_ref=r.before_item->>'definition_revision_ref'
   AND full_charges IS NULL AND full_remaining_ms IS NULL AND NOT lit_continuous)
 THEN RAISE EXCEPTION 'decoration kit cannot have a duration' USING ERRCODE='23514'; END IF;
 w.item_instance_id:=r.item_instance_id; w.expected_revision:=coalesce(t.state_revision,0);
 w.cause:=CASE WHEN r.action=1 THEN 6 ELSE 5 END; w.transaction_id:=r.transaction_id;
 w.definition_before_family:='Item';w.definition_before_production_key:=r.before_item->>'definition_production_key';
 w.definition_before_revision_ref:=r.before_item->>'definition_revision_ref';w.definition_after_family:='Item';
 w.definition_after_production_key:=r.after_key;w.definition_after_revision_ref:=w.definition_before_revision_ref;
 w.remaining_ms_before:=CASE WHEN t.item_instance_id IS NULL AND r.action=1 THEN duration ELSE t.remaining_ms END;
 w.deadline_before:=t.deadline_at; w.lit_continuous:=r.action=2;
 IF r.action=2 THEN w.full_remaining_ms:=duration;w.remaining_ms_after:=duration; END IF;
 INSERT INTO game_item_timed_state_writes SELECT(w).* RETURNING * INTO w;
 IF w.expected_revision=0 THEN
  INSERT INTO game_item_timed_states VALUES(w.item_instance_id,NULL,w.remaining_ms_after,w.deadline_after,1);
 ELSE
  UPDATE game_item_timed_states SET remaining_ms=w.remaining_ms_after,deadline_at=w.deadline_after,state_revision=state_revision+1
   WHERE item_instance_id=w.item_instance_id;
 END IF;
END $$;
DO $migration$
DECLARE body TEXT; needle TEXT; hook TEXT;
BEGIN
 body:=pg_get_functiondef('game_house_furniture_wrap(uuid,uuid,numeric,numeric,numeric,uuid,numeric,uuid,text,numeric,uuid,numeric,numeric,smallint,bytea,bytea,text,integer,text)'::regprocedure);
 needle:='OR EXISTS(SELECT 1 FROM game_item_timed_states WHERE item_instance_id=p_item)';
 IF position(needle IN body)=0 THEN RAISE EXCEPTION 'missing wrap timed admission boundary'; END IF;
 body:=replace(body,needle,'OR (EXISTS(SELECT 1 FROM game_item_timed_states WHERE item_instance_id=p_item) AND NOT game_house_timed_decoration_wrap_pair(i.definition_production_key,p_after,p_action))');
 needle:=E' IF p_action=1 THEN\n  DELETE FROM game_item_house_reclaim_provenance';
 IF position(needle IN body)=0 THEN RAISE EXCEPTION 'missing wrap timer cancellation boundary'; END IF;
 hook:=E' IF p_action=1 AND game_house_timed_decoration_wrap_pair(i.definition_production_key,p_after,p_action) THEN PERFORM game_house_timed_decoration_wrap_apply(tx); END IF;\n';
 body:=replace(body,needle,hook||needle);
 needle:=' RETURN p_revision+1;';
 IF position(needle IN body)=0 THEN RAISE EXCEPTION 'missing unwrap timer activation boundary'; END IF;
 hook:=E' IF p_action=2 AND game_house_timed_decoration_wrap_pair(i.definition_production_key,p_after,p_action) THEN PERFORM game_house_timed_decoration_wrap_apply(tx); END IF;\n';
 EXECUTE replace(body,needle,hook||needle);
 body:=pg_get_functiondef('game_house_decoration_timed_deadline_stamp()'::regprocedure);
 needle:=' RETURN NEW;';
 hook:=E' IF EXISTS(SELECT 1 FROM game_house_furniture_wrap_receipts wr WHERE wr.transaction_id=NEW.transaction_id AND wr.created_xact_id=pg_current_xact_id() AND wr.action=2 AND wr.item_instance_id=NEW.item_instance_id AND game_house_timed_decoration_wrap_pair(wr.before_item->>''definition_production_key'',wr.after_key,wr.action)) THEN NEW.deadline_after:=NEW.committed_at+game_house_timed_decoration_duration(NEW.definition_after_production_key); END IF;\n';
 EXECUTE replace(body,needle,hook||needle);
 body:=pg_get_functiondef('game_item_timed_state_write_consistency_guard()'::regprocedure);
 needle:=substring(body from E'(?i)\\mBEGIN\\M');
 hook:=E'BEGIN\n IF EXISTS(SELECT 1 FROM game_house_furniture_wrap_receipts r WHERE r.transaction_id=NEW.transaction_id AND r.created_xact_id=pg_current_xact_id() AND game_house_timed_decoration_wrap_pair(r.before_item->>''definition_production_key'',r.after_key,r.action)) THEN PERFORM game_house_timed_decoration_wrap_complete(NEW.transaction_id); RETURN NULL; END IF;\n';
 EXECUTE overlay(body placing hook from position(needle IN body) for 5);
END $migration$;
DO $migration$
DECLARE signature TEXT;
BEGIN
 FOREACH signature IN ARRAY ARRAY['game_house_timed_decoration_wrap_pair(TEXT,TEXT,SMALLINT)',
  'game_house_timed_decoration_wrap_complete(UUID)','game_house_timed_decoration_wrap_guard()',
  'game_house_timed_decoration_wrap_apply(UUID)'] LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',signature,current_schema());
  EXECUTE 'REVOKE ALL ON FUNCTION '||signature||' FROM PUBLIC,oteryn_game_runtime,oteryn_game_control';
 END LOOP;
END $migration$;
