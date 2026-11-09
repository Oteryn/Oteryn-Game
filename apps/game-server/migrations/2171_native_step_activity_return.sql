-- A committed native translation is an autonomous source STEP, never a client
-- command. Keep 1140's complete non-Exit command requirements unchanged.
-- Compare parser-produced predicates, rather than silently replacing a newer
-- server-reason contract or relying on SQL whitespace.
DO $compatibility$
DECLARE actual TEXT; expected TEXT;
BEGIN
 CREATE TEMP TABLE native_step_2171_expected(command_id NUMERIC(20,0),reason SMALLINT,
   CHECK(command_id IS NOT NULL OR reason IN(2,3,4,5,6,7,8,9,10,11,52))) ON COMMIT DROP;
 SELECT pg_get_expr(conbin,conrelid) INTO STRICT expected FROM pg_constraint
   WHERE conrelid='pg_temp.native_step_2171_expected'::regclass AND contype='c';
 SELECT pg_get_expr(conbin,conrelid) INTO STRICT actual FROM pg_constraint
   WHERE conrelid='game_activity_runtime_returns'::regclass AND conname='nx1_return_server_command';
 IF actual IS DISTINCT FROM expected THEN
  RAISE EXCEPTION 'native STEP Exit requires unchanged 1140 server reason guards';
 END IF;
 DROP TABLE native_step_2171_expected;
END $compatibility$;
ALTER TABLE game_activity_runtime_returns ADD COLUMN native_step_exit JSONB;
CREATE FUNCTION game_activity_native_step_exit_shape(p JSONB) RETURNS BOOLEAN
LANGUAGE plpgsql IMMUTABLE STRICT AS $$
DECLARE k TEXT; a JSONB; n NUMERIC;
BEGIN
 IF jsonb_typeof(p)<>'object' OR (SELECT count(*) FROM jsonb_object_keys(p))<>17
    OR p->>'schema' IS DISTINCT FROM 'native-step-exit.v1' THEN RETURN false; END IF;
 FOREACH k IN ARRAY ARRAY['movement_binding','frame','map','server'] LOOP
  IF NOT COALESCE(jsonb_typeof(p->k)='string' AND p->>k ~ '^[0-9a-f]{64}$'
      AND p->>k<>repeat('0',64),false) THEN RETURN false; END IF;
 END LOOP;
 FOREACH k IN ARRAY ARRAY['session','character','actor'] LOOP
  IF NOT COALESCE(jsonb_typeof(p->k)='string' AND p->>k ~ '^[0-9a-f]{32}$',false)
      THEN RETURN false; END IF;
 END LOOP;
 IF NOT COALESCE(jsonb_typeof(p->'scope')='string' AND p->>'scope' ~ '^03[0-9a-f]{64}$',false)
    THEN RETURN false; END IF;
 FOREACH k IN ARRAY ARRAY['lease','generation','incarnation','before_revision','after_revision','observed_us'] LOOP
  IF NOT COALESCE(jsonb_typeof(p->k)='string' AND p->>k ~ '^(0|[1-9][0-9]{0,19})$',false)
      THEN RETURN false; END IF;
  n:=(p->>k)::numeric;
  IF n>18446744073709551615 OR (k IN('lease','generation','incarnation','after_revision') AND n=0)
      THEN RETURN false; END IF;
 END LOOP;
 FOREACH k IN ARRAY ARRAY['before','after'] LOOP
  a:=p->k;
  IF jsonb_typeof(a) IS DISTINCT FROM 'array' THEN RETURN false; END IF;
  IF jsonb_array_length(a)<>3 THEN RETURN false; END IF;
  IF NOT COALESCE(jsonb_typeof(a->0)='number' AND jsonb_typeof(a->1)='number' AND jsonb_typeof(a->2)='number'
      AND a->>0 ~ '^-?[0-9]+$' AND a->>1 ~ '^-?[0-9]+$' AND a->>2 ~ '^-?[0-9]+$',false)
      THEN RETURN false; END IF;
  IF (a->>0)::numeric NOT BETWEEN -2147483648 AND 2147483647
     OR (a->>1)::numeric NOT BETWEEN -2147483648 AND 2147483647
     OR (a->>2)::numeric NOT BETWEEN -32768 AND 32767 THEN RETURN false; END IF;
 END LOOP;
 RETURN (p->>'after_revision')::numeric>(p->>'before_revision')::numeric
    AND p->'before'<>p->'after';
END $$;
REVOKE ALL ON FUNCTION game_activity_native_step_exit_shape(JSONB) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_activity_native_step_exit_shape(JSONB) TO oteryn_game_runtime;
ALTER TABLE game_activity_runtime_returns
 DROP CONSTRAINT nx1_return_server_command,
 ADD CONSTRAINT nx1_return_server_command CHECK
   (command_id IS NOT NULL OR reason IN(2,3,4,5,6,7,8,9,10,11,52)
      OR (reason=1 AND native_step_exit IS NOT NULL)),
 ADD CONSTRAINT native_step_exit_only CHECK(native_step_exit IS NULL OR
   (reason=1 AND command_id IS NULL AND game_activity_native_step_exit_shape(native_step_exit)) IS TRUE);
-- Match the native u64 big-endian and hash_field framing exactly.
CREATE FUNCTION game_activity_native_step_u64(p NUMERIC) RETURNS BYTEA
LANGUAGE sql IMMUTABLE STRICT AS $$
 SELECT int8send((CASE WHEN p>9223372036854775807 THEN p-18446744073709551616 ELSE p END)::bigint)
$$;
CREATE FUNCTION game_activity_native_step_field(p BYTEA) RETURNS BYTEA
LANGUAGE sql IMMUTABLE STRICT AS $$ SELECT int8send(octet_length(p)::bigint)||p $$;
CREATE FUNCTION game_activity_native_step_movement_binding(p JSONB) RETURNS BYTEA
LANGUAGE sql IMMUTABLE STRICT AS $$
 SELECT sha256(convert_to('oteryn:committed-movement-occurrence:v1','UTF8')
   ||substring(decode(p->>'scope','hex') from 2 for 16)||decode('02','hex')
   ||substring(decode(p->>'scope','hex') from 18 for 16)
   ||game_activity_native_step_u64((p->>'generation')::numeric)
   ||decode(p->>'actor','hex')||game_activity_native_step_u64((p->>'incarnation')::numeric)
   ||game_activity_native_step_u64((p->>'before_revision')::numeric)
   ||game_activity_native_step_u64((p->>'after_revision')::numeric)
   ||int4send((p->'before'->>0)::integer)||int4send((p->'before'->>1)::integer)||int2send((p->'before'->>2)::smallint)
   ||int4send((p->'after'->>0)::integer)||int4send((p->'after'->>1)::integer)||int2send((p->'after'->>2)::smallint))
$$;
CREATE FUNCTION game_activity_native_step_request_binding(r game_activity_runtime_returns) RETURNS BYTEA
LANGUAGE sql IMMUTABLE STRICT AS $$
 SELECT sha256(game_activity_native_step_field(convert_to('native-activity-return-v1','UTF8'))
   ||game_activity_native_step_field(uuid_send((r).handoff_id))
   ||game_activity_native_step_field(uuid_send((r).character_id))
   ||game_activity_native_step_field(uuid_send((r).source_game_session_id))
   ||game_activity_native_step_field(game_activity_native_step_u64((r).source_connection_generation))
   ||game_activity_native_step_field(game_activity_native_step_u64((r).source_lease_generation))
   ||game_activity_native_step_field(decode((r).native_step_exit->>'scope','hex'))
   ||game_activity_native_step_field(game_activity_native_step_u64(((r).native_step_exit->>'generation')::numeric))
   ||game_activity_native_step_field(int2send((r).reason))
   ||game_activity_native_step_field(decode('00','hex'))
   ||game_activity_native_step_field(uuid_send((r).return_id))
   ||game_activity_native_step_field(convert_to('native-step-exit.v1','UTF8'))
   ||game_activity_native_step_field(decode((r).native_step_exit->>'movement_binding','hex'))
   ||game_activity_native_step_field(game_activity_native_step_u64(((r).native_step_exit->>'observed_us')::numeric))
   ||game_activity_native_step_field(decode((r).native_step_exit->>'frame','hex'))
   ||game_activity_native_step_field(decode((r).native_step_exit->>'map','hex'))
   ||game_activity_native_step_field(decode((r).native_step_exit->>'server','hex')))
$$;
REVOKE ALL ON FUNCTION game_activity_native_step_u64(NUMERIC),game_activity_native_step_field(BYTEA),
 game_activity_native_step_movement_binding(JSONB),game_activity_native_step_request_binding(game_activity_runtime_returns) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_activity_native_step_u64(NUMERIC),game_activity_native_step_field(BYTEA),
 game_activity_native_step_movement_binding(JSONB),game_activity_native_step_request_binding(game_activity_runtime_returns) TO oteryn_game_runtime;
CREATE FUNCTION game_activity_native_step_exit_proven() RETURNS trigger
LANGUAGE plpgsql SET search_path=pg_catalog,public,pg_temp AS $$
BEGIN
 IF NEW.native_step_exit IS NOT NULL AND (
  decode(NEW.native_step_exit->>'movement_binding','hex') IS DISTINCT FROM game_activity_native_step_movement_binding(NEW.native_step_exit)
  OR NEW.request_binding IS DISTINCT FROM game_activity_native_step_request_binding(NEW)
  OR NOT EXISTS(
  SELECT 1 FROM game_activity_handoff_groups g
  JOIN game_activity_runtime_entries e USING(handoff_id)
  JOIN game_activity_handoff_participants p USING(handoff_id)
  JOIN game_durability_reconnect_sessions s ON s.game_session_id=NEW.source_game_session_id
  JOIN game_runtime_scope_assignments owner ON owner.scope_key=game_activity_scope_key(g.world_id,g.instance_id)
  WHERE g.handoff_id=NEW.handoff_id AND p.character_id=NEW.character_id
   AND p.destination_game_session_id=NEW.source_game_session_id
   AND s.character_id=NEW.character_id AND s.world_id=g.world_id
   AND s.runtime_scope_kind=2 AND s.runtime_scope_world_id=g.world_id AND s.runtime_scope_instance_id=g.instance_id
   AND s.runtime_scope_channel_id IS NULL AND s.runtime_scope_house_key IS NULL
   AND owner.scope_kind=3 AND owner.state=1 AND owner.world_id=g.world_id
   AND owner.activity_instance_id=g.instance_id AND owner.channel_id IS NULL
   AND owner.ownership_generation=s.scope_ownership_generation
   AND s.scope_ownership_generation=g.ownership_generation
   AND s.character_lease_generation=NEW.source_lease_generation
   AND s.current_generation=NEW.source_connection_generation
   AND NEW.native_step_exit->>'session'=encode(uuid_send(NEW.source_game_session_id),'hex')
   AND NEW.native_step_exit->>'character'=encode(uuid_send(NEW.character_id),'hex')
   AND NEW.native_step_exit->>'lease'=NEW.source_lease_generation::text
   AND NEW.native_step_exit->>'generation'=g.ownership_generation::text
   AND NEW.native_step_exit->>'scope'=encode(game_activity_scope_key(g.world_id,g.instance_id),'hex')
   AND NEW.native_step_exit->>'frame'=encode(e.destination_coordinate_frame_digest,'hex')
   AND NEW.native_step_exit->>'map'=encode(e.destination_map_revision_digest,'hex')
   AND NEW.native_step_exit->>'server'=encode(e.destination_server_artifact_digest,'hex')
 )) THEN RAISE EXCEPTION 'native STEP Exit requires exact retained movement and source session scope'
   USING ERRCODE='23514'; END IF;
 RETURN NULL;
END $$;
REVOKE ALL ON FUNCTION game_activity_native_step_exit_proven() FROM PUBLIC;
CREATE CONSTRAINT TRIGGER game_activity_native_step_exit_proven AFTER INSERT
 ON game_activity_runtime_returns DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
 EXECUTE FUNCTION game_activity_native_step_exit_proven();

DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_activity_native_step_u64(NUMERIC) SET search_path=pg_catalog,%I,pg_temp',current_schema());
 EXECUTE format('ALTER FUNCTION game_activity_native_step_field(BYTEA) SET search_path=pg_catalog,%I,pg_temp',current_schema());
 EXECUTE format('ALTER FUNCTION game_activity_native_step_movement_binding(JSONB) SET search_path=pg_catalog,%I,pg_temp',current_schema());
 EXECUTE format('ALTER FUNCTION game_activity_native_step_request_binding(game_activity_runtime_returns) SET search_path=pg_catalog,%I,pg_temp',current_schema());
 EXECUTE format('ALTER FUNCTION game_activity_native_step_exit_shape(JSONB) SET search_path=pg_catalog,%I,pg_temp',current_schema());
 EXECUTE format('ALTER FUNCTION game_activity_native_step_exit_proven() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
