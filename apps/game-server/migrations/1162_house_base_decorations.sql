-- W18 static House decorations. Crystal 201189f windows.lua:13-47 and
-- game.cpp:3019-3057 transform source placements, not Character Item custody.
CREATE TABLE game_house_base_decoration_receipts (
 transaction_id UUID PRIMARY KEY DEFAULT game_character_uuid_v7() CHECK(game_character_is_uuid_v7(transaction_id)),
 game_session_id UUID NOT NULL,
 command_id NUMERIC(20,0) CHECK(command_id BETWEEN 1 AND 18446744073709551615),
 ordinal SMALLINT NOT NULL DEFAULT 0 CHECK(ordinal BETWEEN 0 AND 2000),
 event_kind SMALLINT NOT NULL DEFAULT 1 CHECK(event_kind IN(1,2,3)),
 character_id UUID NOT NULL REFERENCES game_character_roots,
 world_id UUID NOT NULL,
 house_key TEXT NOT NULL,
 bundle_digest BYTEA NOT NULL CHECK(octet_length(bundle_digest)=32),
 placement_key NUMERIC(20,0) NOT NULL CHECK(placement_key BETWEEN 1 AND 281474976710655),
 before_key TEXT NOT NULL,
 after_key TEXT NOT NULL,
 before_revision NUMERIC(20,0) NOT NULL CHECK(before_revision BETWEEN 0 AND 18446744073709551614),
 before_deadline_ms BIGINT CHECK(before_deadline_ms>0),
 after_deadline_ms BIGINT CHECK(after_deadline_ms>0),
 sampled_at_ms BIGINT CHECK(sampled_at_ms>0),
 intent BYTEA NOT NULL CHECK(octet_length(intent) BETWEEN 1 AND 529),
 parent_item_transaction UUID REFERENCES game_item_consume_transform_records(transaction_id),
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
 UNIQUE(game_session_id,command_id,ordinal),
 CHECK((event_kind=1)=(command_id IS NOT NULL))
);
CREATE INDEX game_house_base_decoration_receipt_character ON game_house_base_decoration_receipts(character_id);
CREATE INDEX game_house_base_decoration_receipt_parent ON game_house_base_decoration_receipts(parent_item_transaction);

CREATE TABLE game_house_base_decorations (
 world_id UUID NOT NULL,
 house_key TEXT NOT NULL,
 bundle_digest BYTEA NOT NULL CHECK(octet_length(bundle_digest)=32),
 placement_key NUMERIC(20,0) NOT NULL CHECK(placement_key BETWEEN 1 AND 281474976710655),
 definition_key TEXT NOT NULL CHECK(definition_key~'^oteryn:item\.tibia\.i[0-9]{1,10}$'),
 revision NUMERIC(20,0) NOT NULL CHECK(revision BETWEEN 0 AND 18446744073709551615),
 decay_deadline_ms BIGINT CHECK(decay_deadline_ms>0),
 last_transaction_id UUID NOT NULL REFERENCES game_house_base_decoration_receipts,
 PRIMARY KEY(world_id,house_key,bundle_digest,placement_key),
 FOREIGN KEY(world_id,house_key) REFERENCES game_house_properties(world_id,house_key) ON DELETE CASCADE
);
CREATE INDEX game_house_base_decoration_last_transaction ON game_house_base_decorations(last_transaction_id);
CREATE TABLE game_house_base_materializations (
 world_id UUID NOT NULL,
 house_key TEXT NOT NULL,
 bundle_digest BYTEA NOT NULL CHECK(octet_length(bundle_digest)=32),
 materialized_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
 PRIMARY KEY(world_id,house_key,bundle_digest),
 FOREIGN KEY(world_id,house_key) REFERENCES game_house_properties(world_id,house_key) ON DELETE CASCADE
);
REVOKE ALL ON game_house_base_decoration_receipts,game_house_base_decorations
 FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
REVOKE ALL ON game_house_base_materializations FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;

CREATE FUNCTION game_house_base_decoration_receipt_immutable() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
 RAISE EXCEPTION 'House base decoration receipts are immutable' USING ERRCODE='23514';
END $$;
CREATE TRIGGER game_house_base_decoration_receipt_immutable BEFORE UPDATE OR DELETE ON game_house_base_decoration_receipts
 FOR EACH ROW EXECUTE FUNCTION game_house_base_decoration_receipt_immutable();

CREATE FUNCTION game_house_base_decoration_state_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE r game_house_base_decoration_receipts%ROWTYPE; advanced JSONB; spec JSONB;
BEGIN
 SELECT * INTO r FROM game_house_base_decoration_receipts WHERE transaction_id=NEW.last_transaction_id;
 IF r.transaction_id IS NULL OR r.created_xact_id<>pg_current_xact_id()
  OR r.world_id<>NEW.world_id OR r.house_key<>NEW.house_key OR r.bundle_digest<>NEW.bundle_digest
  OR r.placement_key<>NEW.placement_key OR r.after_key<>NEW.definition_key
  OR r.before_revision+(CASE WHEN r.event_kind=2 THEN 0 ELSE 1 END)<>NEW.revision
  OR NEW.decay_deadline_ms IS DISTINCT FROM r.after_deadline_ms
  OR (TG_OP='INSERT' AND r.before_revision<>0)
  OR (TG_OP='UPDATE' AND (OLD.world_id<>NEW.world_id OR OLD.house_key<>NEW.house_key
   OR OLD.bundle_digest<>NEW.bundle_digest OR OLD.placement_key<>NEW.placement_key
   OR OLD.definition_key<>r.before_key OR OLD.revision<>r.before_revision
   OR OLD.decay_deadline_ms IS DISTINCT FROM r.before_deadline_ms))
 THEN RAISE EXCEPTION 'House base decoration lacks its exact receipt' USING ERRCODE='23514'; END IF;
 IF r.event_kind=2 THEN
  spec:=game_house_base_decay_spec(r.before_key);
  IF TG_OP<>'INSERT' OR r.before_key<>r.after_key OR r.before_revision<>0 OR r.before_deadline_ms IS NOT NULL
   OR spec->>'static_rotation' IS DISTINCT FROM 'true' OR r.sampled_at_ms IS NULL
   OR r.after_deadline_ms IS DISTINCT FROM r.sampled_at_ms+(spec->>'duration_ms')::bigint
  THEN RAISE EXCEPTION 'invalid House base timer materialization' USING ERRCODE='23514'; END IF;
 ELSIF r.event_kind=3 THEN
  advanced:=game_house_base_decay_advance(r.before_key,r.before_deadline_ms,r.sampled_at_ms);
  IF TG_OP<>'UPDATE' OR (advanced->>'steps')::numeric<=0
   OR advanced->>'key' IS DISTINCT FROM r.after_key
   OR (advanced->>'deadline_ms')::bigint IS DISTINCT FROM r.after_deadline_ms
  THEN RAISE EXCEPTION 'invalid House base timer expiry' USING ERRCODE='23514'; END IF;
 ELSE
  spec:=game_house_base_decay_spec(r.after_key);
  IF (spec IS NULL AND r.after_deadline_ms IS NOT NULL)
   OR (spec IS NOT NULL AND (r.sampled_at_ms IS NULL
    OR spec->>'static_rotation' IS DISTINCT FROM 'true'
    OR r.after_deadline_ms IS DISTINCT FROM r.sampled_at_ms+(spec->>'duration_ms')::bigint))
  THEN RAISE EXCEPTION 'invalid House base timer transition' USING ERRCODE='23514'; END IF;
 END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER game_house_base_decoration_state_guard BEFORE INSERT OR UPDATE ON game_house_base_decorations
 FOR EACH ROW EXECUTE FUNCTION game_house_base_decoration_state_guard();

CREATE FUNCTION game_house_base_decoration_authorize(
 p_session UUID,p_character UUID,p_connection NUMERIC,p_lease NUMERIC,p_scope NUMERIC,
 p_node UUID,p_node_revision NUMERIC,p_world UUID,p_house TEXT)
RETURNS BOOLEAN LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 IF NOT game_node_lock_current_registration(p_node,p_node_revision)
  OR NOT game_item_action_house_current(p_session,p_character,p_connection,p_lease,p_scope,
   p_node,p_node_revision,p_world,p_house) THEN RETURN FALSE; END IF;
 PERFORM 1 FROM game_house_properties WHERE world_id=p_world AND house_key=p_house FOR SHARE;
 RETURN FOUND AND EXISTS(SELECT 1 FROM game_house_access(p_world,p_house,p_character)
  WHERE content_fenced IS FALSE AND role IN('OWNER','SUBOWNER','GUEST'));
END $$;

-- Aqueduct source cycles are four phases per orientation. Skip complete cycles
-- arithmetically, then evaluate only their bounded suffix after an offline gap.
CREATE FUNCTION game_house_base_decay_advance(p_key TEXT,p_deadline BIGINT,p_now BIGINT)
RETURNS JSONB LANGUAGE plpgsql IMMUTABLE AS $$
DECLARE key TEXT:=p_key; deadline BIGINT:=p_deadline; spec JSONB; next_spec JSONB;
 seen JSONB:='{}'; period BIGINT; cycles BIGINT; steps NUMERIC:=0; cycle_steps NUMERIC; turns INTEGER:=0;
BEGIN
 IF p_now IS NULL OR p_now<=0 OR p_deadline IS NULL OR p_deadline<=0
 THEN RAISE EXCEPTION 'invalid House base timer clock' USING ERRCODE='23514'; END IF;
 WHILE deadline IS NOT NULL AND deadline<=p_now LOOP
  turns:=turns+1;
  IF turns>128 THEN RAISE EXCEPTION 'House base timer graph exceeds bounded source closure' USING ERRCODE='23514'; END IF;
  spec:=game_house_base_decay_spec(key);
  IF spec IS NULL OR spec->>'static_rotation' IS DISTINCT FROM 'true'
   THEN RAISE EXCEPTION 'unqualified House base timer phase' USING ERRCODE='23514'; END IF;
  IF seen ? key THEN
   period:=deadline-(seen->key->>'deadline_ms')::bigint;
   cycle_steps:=steps-(seen->key->>'steps')::numeric;
   IF period>0 AND cycle_steps>0 THEN
    cycles:=(p_now-deadline)/period;
    IF cycles>0 THEN deadline:=deadline+cycles*period; steps:=steps+cycles*cycle_steps; END IF;
   END IF;
  ELSE
   seen:=seen||jsonb_build_object(key,jsonb_build_object('deadline_ms',deadline,'steps',steps));
  END IF;
  key:=spec->>'target';
  IF key IS NULL THEN RAISE EXCEPTION 'House base timer cannot retire immutable geometry' USING ERRCODE='23514'; END IF;
  steps:=steps+1;
  next_spec:=game_house_base_decay_spec(key);
  IF next_spec IS NULL THEN deadline:=NULL;
  ELSE deadline:=deadline+(next_spec->>'duration_ms')::bigint; END IF;
 END LOOP;
 RETURN jsonb_build_object('key',key,'deadline_ms',deadline,'steps',steps);
END $$;

CREATE FUNCTION game_house_base_decoration_materialize_fenced(
 p_session UUID,p_character UUID,p_connection NUMERIC,p_lease NUMERIC,p_scope NUMERIC,
 p_node UUID,p_node_revision NUMERIC,p_world UUID,p_house TEXT,p_bundle BYTEA,p_sources JSONB)
RETURNS BOOLEAN LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE item JSONB; spec JSONB; anchor BIGINT; deadline BIGINT; placement NUMERIC; key TEXT; receipt UUID;
BEGIN
 IF NOT game_house_base_decoration_authorize(p_session,p_character,p_connection,p_lease,p_scope,
  p_node,p_node_revision,p_world,p_house) THEN RETURN FALSE; END IF;
 IF p_bundle IS NULL OR octet_length(p_bundle)<>32 OR jsonb_typeof(p_sources) IS DISTINCT FROM 'array'
  OR jsonb_array_length(p_sources)>2000 THEN RETURN FALSE; END IF;
 PERFORM pg_advisory_xact_lock(hashtextextended('oteryn:house-base:'||p_world::text||':'||p_house,0));
 IF EXISTS(SELECT 1 FROM game_house_base_materializations WHERE world_id=p_world AND house_key=p_house AND bundle_digest=p_bundle)
 THEN RETURN TRUE; END IF;
 IF jsonb_array_length(p_sources)>0 THEN
  -- Immutable map materialization is dated by the active source bundle's reset,
  -- never by the first observer. Existing deadlines survive logout and reload.
  SELECT floor(extract(epoch FROM e.activated_at)*1000)::bigint INTO anchor
   FROM game_world_reset_active a JOIN game_world_reset_epochs e USING(world_id,reset_epoch)
   WHERE a.world_id=p_world AND e.target_digest=p_bundle FOR SHARE OF a;
  IF NOT FOUND OR anchor<=0 THEN RETURN FALSE; END IF;
 END IF;
 IF (SELECT count(DISTINCT value->>'placement_key') FROM jsonb_array_elements(p_sources))<>jsonb_array_length(p_sources)
 THEN RETURN FALSE; END IF;
 FOR item IN SELECT value FROM jsonb_array_elements(p_sources) ORDER BY (value->>'placement_key')::numeric LOOP
  placement:=(item->>'placement_key')::numeric;
  key:=item->>'before_key';
  spec:=game_house_base_decay_spec(key);
  IF placement IS NULL OR placement NOT BETWEEN 1 AND 281474976710655 OR placement<>trunc(placement)
   OR spec->>'static_rotation' IS DISTINCT FROM 'true'
   OR jsonb_typeof(item->'position') IS DISTINCT FROM 'array' OR jsonb_array_length(item->'position')<>3
   OR game_house_access_tile(p_world,p_house,int4send((item->'position'->>0)::integer)
    ||int4send((item->'position'->>1)::integer)||int4send((item->'position'->>2)::integer)) IS DISTINCT FROM TRUE
  THEN RAISE EXCEPTION 'unqualified House base timer source' USING ERRCODE='23514'; END IF;
  IF EXISTS(SELECT 1 FROM game_house_base_decorations WHERE world_id=p_world AND house_key=p_house
   AND bundle_digest=p_bundle AND placement_key=placement) THEN CONTINUE; END IF;
  IF (SELECT count(*) FROM game_house_base_decorations WHERE world_id=p_world AND house_key=p_house AND bundle_digest=p_bundle)>=2000
  THEN RAISE EXCEPTION 'House base timer projection budget' USING ERRCODE='23514'; END IF;
  deadline:=anchor+(spec->>'duration_ms')::bigint;
  INSERT INTO game_house_base_decoration_receipts(game_session_id,command_id,event_kind,character_id,world_id,house_key,
   bundle_digest,placement_key,before_key,after_key,before_revision,after_deadline_ms,sampled_at_ms,intent)
  VALUES(p_session,NULL,2,p_character,p_world,p_house,p_bundle,placement,key,key,0,deadline,anchor,
   convert_to('House base timer materialization','UTF8')) RETURNING transaction_id INTO receipt;
  INSERT INTO game_house_base_decorations(world_id,house_key,bundle_digest,placement_key,definition_key,revision,decay_deadline_ms,last_transaction_id)
  VALUES(p_world,p_house,p_bundle,placement,key,0,deadline,receipt);
 END LOOP;
 INSERT INTO game_house_base_materializations(world_id,house_key,bundle_digest) VALUES(p_world,p_house,p_bundle);
 RETURN TRUE;
END $$;

CREATE FUNCTION game_house_base_decoration_expire_fenced(
 p_session UUID,p_character UUID,p_connection NUMERIC,p_lease NUMERIC,p_scope NUMERIC,
 p_node UUID,p_node_revision NUMERIC,p_world UUID,p_house TEXT,p_bundle BYTEA)
RETURNS VOID LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE d game_house_base_decorations%ROWTYPE; advanced JSONB; instant BIGINT; receipt UUID;
BEGIN
 IF NOT game_house_base_decoration_authorize(p_session,p_character,p_connection,p_lease,p_scope,
  p_node,p_node_revision,p_world,p_house) THEN RETURN; END IF;
 PERFORM pg_advisory_xact_lock(hashtextextended('oteryn:house-base:'||p_world::text||':'||p_house,0));
 instant:=floor(extract(epoch FROM clock_timestamp())*1000)::bigint;
 FOR d IN SELECT * FROM game_house_base_decorations WHERE world_id=p_world AND house_key=p_house
  AND bundle_digest=p_bundle AND decay_deadline_ms<=instant ORDER BY placement_key LIMIT 2000 FOR UPDATE LOOP
  advanced:=game_house_base_decay_advance(d.definition_key,d.decay_deadline_ms,instant);
  IF (advanced->>'steps')::numeric<=0 THEN CONTINUE; END IF;
  INSERT INTO game_house_base_decoration_receipts(game_session_id,command_id,event_kind,character_id,world_id,house_key,
   bundle_digest,placement_key,before_key,after_key,before_revision,before_deadline_ms,after_deadline_ms,sampled_at_ms,intent)
  VALUES(p_session,NULL,3,p_character,p_world,p_house,p_bundle,d.placement_key,d.definition_key,advanced->>'key',d.revision,
   d.decay_deadline_ms,(advanced->>'deadline_ms')::bigint,instant,convert_to('House base timer expiry','UTF8'))
   RETURNING transaction_id INTO receipt;
  UPDATE game_house_base_decorations SET definition_key=advanced->>'key',revision=d.revision+1,
   decay_deadline_ms=(advanced->>'deadline_ms')::bigint,last_transaction_id=receipt
   WHERE world_id=p_world AND house_key=p_house AND bundle_digest=p_bundle AND placement_key=d.placement_key;
 END LOOP;
END $$;

CREATE FUNCTION game_house_base_decoration_load_fenced(
 p_session UUID,p_character UUID,p_connection NUMERIC,p_lease NUMERIC,p_scope NUMERIC,
 p_node UUID,p_node_revision NUMERIC,p_world UUID,p_house TEXT,p_bundle BYTEA)
RETURNS TABLE(placement_key NUMERIC,definition_key TEXT,revision NUMERIC,materialized BOOLEAN)
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE initialized BOOLEAN;
BEGIN
 IF NOT game_house_base_decoration_authorize(p_session,p_character,p_connection,p_lease,p_scope,
  p_node,p_node_revision,p_world,p_house) THEN RETURN; END IF;
 PERFORM game_house_base_decoration_expire_fenced(p_session,p_character,p_connection,p_lease,p_scope,
  p_node,p_node_revision,p_world,p_house,p_bundle);
 initialized:=EXISTS(SELECT 1 FROM game_house_base_materializations WHERE world_id=p_world AND house_key=p_house AND bundle_digest=p_bundle);
 RETURN QUERY SELECT d.placement_key,d.definition_key,d.revision,initialized FROM game_house_base_decorations d
  WHERE d.world_id=p_world AND d.house_key=p_house AND d.bundle_digest=p_bundle
  ORDER BY d.placement_key LIMIT 2001;
 IF NOT FOUND THEN RETURN QUERY SELECT NULL::numeric,NULL::text,NULL::numeric,initialized; END IF;
END $$;

CREATE FUNCTION game_house_base_decoration_replay_fenced(
 p_session UUID,p_character UUID,p_connection NUMERIC,p_lease NUMERIC,p_scope NUMERIC,
 p_node UUID,p_node_revision NUMERIC,p_world UUID,p_house TEXT,p_bundle BYTEA,p_command NUMERIC,p_intent BYTEA)
RETURNS TEXT LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE r game_house_base_decoration_receipts%ROWTYPE;
BEGIN
 IF NOT game_house_base_decoration_authorize(p_session,p_character,p_connection,p_lease,p_scope,
  p_node,p_node_revision,p_world,p_house) THEN RETURN 'stale'; END IF;
 SELECT * INTO r FROM game_house_base_decoration_receipts WHERE game_session_id=p_session AND command_id=p_command AND ordinal=0;
 IF NOT FOUND THEN RETURN NULL; END IF;
 IF r.character_id<>p_character OR r.world_id<>p_world OR r.house_key<>p_house
  OR r.bundle_digest<>p_bundle OR r.intent<>p_intent OR r.parent_item_transaction IS NOT NULL THEN RETURN 'stale'; END IF;
 RETURN 'committed';
END $$;

CREATE FUNCTION game_house_base_decoration_set_fenced(
 p_session UUID,p_character UUID,p_connection NUMERIC,p_lease NUMERIC,p_scope NUMERIC,
 p_node UUID,p_node_revision NUMERIC,p_world UUID,p_house TEXT,p_bundle BYTEA,p_command NUMERIC,
 p_placement NUMERIC,p_expected NUMERIC,p_before TEXT,p_after TEXT,p_intent BYTEA,p_position BYTEA)
RETURNS TEXT LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE d game_house_base_decorations%ROWTYPE; replay TEXT; receipt UUID; had_state BOOLEAN;
 spec JSONB; instant BIGINT; deadline BIGINT;
BEGIN
 IF NOT game_house_base_decoration_authorize(p_session,p_character,p_connection,p_lease,p_scope,
  p_node,p_node_revision,p_world,p_house) THEN RETURN 'stale'; END IF;
 IF p_bundle IS NULL OR octet_length(p_bundle)<>32 OR p_command IS NULL
  OR p_command NOT BETWEEN 1 AND 18446744073709551615 OR p_command<>trunc(p_command)
  OR p_placement IS NULL OR p_placement NOT BETWEEN 1 AND 281474976710655 OR p_placement<>trunc(p_placement)
  OR p_expected IS NULL OR p_expected NOT BETWEEN 0 AND 18446744073709551614 OR p_expected<>trunc(p_expected)
  OR p_intent IS NULL OR octet_length(p_intent) NOT BETWEEN 1 AND 529
  OR p_position IS NULL OR octet_length(p_position)<>12 THEN RETURN 'invalid'; END IF;
 -- Admission's relation locks precede this House-specific serialization in the Rust owner.
 PERFORM pg_advisory_xact_lock(hashtextextended('oteryn:house-base:'||p_world::text||':'||p_house,0));
 replay:=game_house_base_decoration_replay_fenced(p_session,p_character,p_connection,p_lease,p_scope,
  p_node,p_node_revision,p_world,p_house,p_bundle,p_command,p_intent);
 IF replay IS NOT NULL THEN RETURN replay; END IF;
 IF game_house_access_tile(p_world,p_house,p_position) IS DISTINCT FROM TRUE
  OR game_house_base_decoration_pair(p_before,p_after) IS DISTINCT FROM TRUE THEN RETURN 'invalid'; END IF;
 PERFORM game_house_base_decoration_expire_fenced(p_session,p_character,p_connection,p_lease,p_scope,
  p_node,p_node_revision,p_world,p_house,p_bundle);
 SELECT * INTO d FROM game_house_base_decorations WHERE world_id=p_world AND house_key=p_house
  AND bundle_digest=p_bundle AND placement_key=p_placement FOR UPDATE;
 had_state:=FOUND;
 IF (had_state AND (d.revision<>p_expected OR d.definition_key<>p_before))
  OR (NOT had_state AND p_expected<>0) THEN RETURN 'stale'; END IF;
 -- Crystal game.cpp:3054-3057 returns the original Item for a missing requested type.
 -- Such a source-qualified no-op must not create a revision or an overlay record.
 IF p_before=p_after THEN RETURN 'committed'; END IF;
 IF NOT had_state AND (SELECT count(*) FROM game_house_base_decorations WHERE world_id=p_world
  AND house_key=p_house AND bundle_digest=p_bundle)>=2000 THEN RETURN 'exhausted'; END IF;
 instant:=floor(extract(epoch FROM clock_timestamp())*1000)::bigint;
 spec:=game_house_base_decay_spec(p_after);
 IF spec IS NOT NULL THEN
  IF spec->>'static_rotation' IS DISTINCT FROM 'true' THEN RETURN 'invalid'; END IF;
  -- Crystal Item::setID item.cpp:430-451 and Decay::startDecay decay.cpp:29-54:
  -- these source variants have stopTime=false, so rotation resets the target duration.
  deadline:=instant+(spec->>'duration_ms')::bigint;
 ELSE deadline:=NULL; END IF;
 INSERT INTO game_house_base_decoration_receipts(game_session_id,command_id,character_id,world_id,house_key,
  bundle_digest,placement_key,before_key,after_key,before_revision,before_deadline_ms,after_deadline_ms,sampled_at_ms,intent)
 VALUES(p_session,p_command,p_character,p_world,p_house,p_bundle,p_placement,p_before,p_after,p_expected,
  CASE WHEN had_state THEN d.decay_deadline_ms ELSE NULL END,deadline,instant,p_intent)
 RETURNING transaction_id INTO receipt;
 IF had_state THEN
  UPDATE game_house_base_decorations SET definition_key=p_after,revision=p_expected+1,decay_deadline_ms=deadline,last_transaction_id=receipt
   WHERE world_id=p_world AND house_key=p_house AND bundle_digest=p_bundle AND placement_key=p_placement;
 ELSE
  INSERT INTO game_house_base_decorations(world_id,house_key,bundle_digest,placement_key,definition_key,revision,decay_deadline_ms,last_transaction_id)
  VALUES(p_world,p_house,p_bundle,p_placement,p_after,p_expected+1,deadline,receipt);
 END IF;
 RETURN 'committed';
END $$;

DO $migration$
DECLARE signature TEXT;
BEGIN
 FOREACH signature IN ARRAY ARRAY[
  'game_house_base_decoration_state_guard()',
  'game_house_base_decoration_receipt_immutable()',
  'game_house_base_decoration_authorize(UUID,UUID,NUMERIC,NUMERIC,NUMERIC,UUID,NUMERIC,UUID,TEXT)',
  'game_house_base_decay_advance(TEXT,BIGINT,BIGINT)',
  'game_house_base_decoration_materialize_fenced(UUID,UUID,NUMERIC,NUMERIC,NUMERIC,UUID,NUMERIC,UUID,TEXT,BYTEA,JSONB)',
  'game_house_base_decoration_expire_fenced(UUID,UUID,NUMERIC,NUMERIC,NUMERIC,UUID,NUMERIC,UUID,TEXT,BYTEA)',
  'game_house_base_decoration_load_fenced(UUID,UUID,NUMERIC,NUMERIC,NUMERIC,UUID,NUMERIC,UUID,TEXT,BYTEA)',
  'game_house_base_decoration_replay_fenced(UUID,UUID,NUMERIC,NUMERIC,NUMERIC,UUID,NUMERIC,UUID,TEXT,BYTEA,NUMERIC,BYTEA)',
  'game_house_base_decoration_set_fenced(UUID,UUID,NUMERIC,NUMERIC,NUMERIC,UUID,NUMERIC,UUID,TEXT,BYTEA,NUMERIC,NUMERIC,NUMERIC,TEXT,TEXT,BYTEA,BYTEA)'
 ] LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',signature,current_schema());
  EXECUTE 'REVOKE ALL ON FUNCTION '||signature||' FROM PUBLIC,oteryn_game_runtime,oteryn_game_control';
 END LOOP;
END $migration$;
GRANT EXECUTE ON FUNCTION
 game_house_base_decoration_load_fenced(UUID,UUID,NUMERIC,NUMERIC,NUMERIC,UUID,NUMERIC,UUID,TEXT,BYTEA),
 game_house_base_decoration_materialize_fenced(UUID,UUID,NUMERIC,NUMERIC,NUMERIC,UUID,NUMERIC,UUID,TEXT,BYTEA,JSONB),
 game_house_base_decoration_replay_fenced(UUID,UUID,NUMERIC,NUMERIC,NUMERIC,UUID,NUMERIC,UUID,TEXT,BYTEA,NUMERIC,BYTEA),
 game_house_base_decoration_set_fenced(UUID,UUID,NUMERIC,NUMERIC,NUMERIC,UUID,NUMERIC,UUID,TEXT,BYTEA,NUMERIC,NUMERIC,NUMERIC,TEXT,TEXT,BYTEA,BYTEA)
 TO oteryn_game_runtime;

-- Crystal bed_modification_kits.lua:10-16,37-46 changes both source BED parts,
-- then consumes the kit. The parent Item action and these two static changes
-- share one physical transaction; neither can publish a partial modification.
CREATE FUNCTION game_house_base_apply_changes(p_transaction UUID,p_changes JSONB)
RETURNS VOID LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE r game_item_consume_transform_records%ROWTYPE; item JSONB; layout JSONB; pair JSONB;
 bundle BYTEA; house TEXT; kit TEXT; head JSONB; partner JSONB; direction TEXT;
 placement NUMERIC; expected NUMERIC; before_key TEXT; after_key TEXT; receipt UUID;
 d game_house_base_decorations%ROWTYPE; had_state BOOLEAN; n INTEGER;
BEGIN
 SELECT * INTO r FROM game_item_consume_transform_records WHERE transaction_id=p_transaction;
 IF r.transaction_id IS NULL OR r.created_xact_id<>pg_current_xact_id() OR r.source_kind<>5
  OR r.effects->>'kind' IS DISTINCT FROM 'house_decoration'
  OR r.effects->>'decoration_family' IS DISTINCT FROM 'bed_kit'
  OR r.effects->'base_changes' IS DISTINCT FROM p_changes OR NOT r.consume_source OR r.source_count<>1
  OR jsonb_typeof(p_changes) IS DISTINCT FROM 'object'
  OR jsonb_typeof(p_changes->'bundle_digest') IS DISTINCT FROM 'array'
  OR jsonb_array_length(p_changes->'bundle_digest')<>32
  OR jsonb_typeof(p_changes->'items') IS DISTINCT FROM 'array'
  OR jsonb_array_length(p_changes->'items')<>2
 THEN RAISE EXCEPTION 'invalid House bed modification parent' USING ERRCODE='23514'; END IF;
 IF EXISTS(SELECT 1 FROM jsonb_array_elements(p_changes->'bundle_digest') value
  WHERE jsonb_typeof(value)<>'number' OR value::text!~'^[0-9]{1,3}$' OR (value::text)::integer>255)
 THEN RAISE EXCEPTION 'invalid House bed bundle' USING ERRCODE='23514'; END IF;
 SELECT decode(string_agg(lpad(to_hex(value::integer),2,'0'),'' ORDER BY ordinal),'hex') INTO bundle
  FROM jsonb_array_elements_text(p_changes->'bundle_digest') WITH ORDINALITY bytes(value,ordinal);
 house:=r.source_location->'house'->>'house_key';
 kit:=r.before_item->>'definition_production_key';
 IF NOT game_house_base_decoration_authorize(r.game_session_id,r.character_id,
  (r.audit_context->>'connection_generation')::numeric,(r.audit_context->>'lease_generation')::numeric,
  (r.audit_context->>'scope_generation')::numeric,(r.source_location->>'node')::uuid,
  (r.source_location->>'node_revision')::numeric,r.world_id,house)
 THEN RAISE EXCEPTION 'stale House bed modification' USING ERRCODE='23514'; END IF;
 layout:=game_house_bed_kit_layout(kit);
 head:=p_changes->'items'->0;
 partner:=p_changes->'items'->1;
 IF head->>'after_key'='oteryn:item.tibia.i'||(layout->'south'->>0) THEN direction:='south';
 ELSIF head->>'after_key'='oteryn:item.tibia.i'||(layout->'east'->>0) THEN direction:='east';
 ELSE RAISE EXCEPTION 'invalid House bed head' USING ERRCODE='23514'; END IF;
 pair:=layout->direction;
 IF partner->>'after_key' IS DISTINCT FROM 'oteryn:item.tibia.i'||(pair->>1)
  OR head->>'placement_key'=partner->>'placement_key'
  OR jsonb_typeof(head->'position') IS DISTINCT FROM 'array' OR jsonb_array_length(head->'position')<>3
  OR jsonb_typeof(partner->'position') IS DISTINCT FROM 'array' OR jsonb_array_length(partner->'position')<>3
  OR (head->'position'->>2)::integer<>(partner->'position'->>2)::integer
  OR (partner->'position'->>0)::integer<>(head->'position'->>0)::integer+(CASE WHEN direction='east' THEN 1 ELSE 0 END)
  OR (partner->'position'->>1)::integer<>(head->'position'->>1)::integer+(CASE WHEN direction='south' THEN 1 ELSE 0 END)
 THEN RAISE EXCEPTION 'invalid House bed partner' USING ERRCODE='23514'; END IF;
 PERFORM pg_advisory_xact_lock(hashtextextended('oteryn:house-base:'||r.world_id::text||':'||house,0));
 -- Lock changing base rows by placement, independent of head/partner orientation.
 FOR item,n IN SELECT value,ordinal::integer-1 FROM jsonb_array_elements(p_changes->'items') WITH ORDINALITY entries(value,ordinal)
  ORDER BY (value->>'placement_key')::numeric LOOP
  placement:=(item->>'placement_key')::numeric;
  expected:=(item->>'expected_revision')::numeric;
  before_key:=item->>'before_key';
  after_key:=item->>'after_key';
  IF placement IS NULL OR placement NOT BETWEEN 1 AND 281474976710655 OR placement<>trunc(placement)
   OR expected IS NULL OR expected NOT BETWEEN 0 AND 18446744073709551614 OR expected<>trunc(expected)
   OR game_house_bed_kit_pair(kit,before_key,after_key) IS DISTINCT FROM TRUE
   OR game_house_access_tile(r.world_id,house,int4send((item->'position'->>0)::integer)
    ||int4send((item->'position'->>1)::integer)||int4send((item->'position'->>2)::integer)) IS DISTINCT FROM TRUE
  THEN RAISE EXCEPTION 'unqualified House bed change' USING ERRCODE='23514'; END IF;
  SELECT * INTO d FROM game_house_base_decorations WHERE world_id=r.world_id AND house_key=house
   AND bundle_digest=bundle AND placement_key=placement FOR UPDATE;
  had_state:=FOUND;
  IF (had_state AND (d.definition_key<>before_key OR d.revision<>expected)) OR (NOT had_state AND expected<>0)
  THEN RAISE EXCEPTION 'stale House bed cell' USING ERRCODE='23514'; END IF;
  IF NOT had_state AND (SELECT count(*) FROM game_house_base_decorations WHERE world_id=r.world_id
   AND house_key=house AND bundle_digest=bundle)>=2000
  THEN RAISE EXCEPTION 'House bed projection budget' USING ERRCODE='23514'; END IF;
  INSERT INTO game_house_base_decoration_receipts(game_session_id,command_id,ordinal,character_id,world_id,house_key,
   bundle_digest,placement_key,before_key,after_key,before_revision,intent,parent_item_transaction)
  VALUES(r.game_session_id,r.command_id,n,r.character_id,r.world_id,house,bundle,placement,before_key,after_key,expected,r.intent,p_transaction)
  RETURNING transaction_id INTO receipt;
  IF had_state THEN
   UPDATE game_house_base_decorations SET definition_key=after_key,revision=expected+1,last_transaction_id=receipt
    WHERE world_id=r.world_id AND house_key=house AND bundle_digest=bundle AND placement_key=placement;
  ELSE
   INSERT INTO game_house_base_decorations(world_id,house_key,bundle_digest,placement_key,definition_key,revision,last_transaction_id)
   VALUES(r.world_id,house,bundle,placement,after_key,expected+1,receipt);
  END IF;
 END LOOP;
END $$;
REVOKE ALL ON FUNCTION game_house_base_apply_changes(UUID,JSONB) FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_house_base_apply_changes(UUID,JSONB) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
