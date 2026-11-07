-- Native actor-owned food is a live semantic clock. Its saved logout snapshot
-- is not a substitute for that live before-state. A second current-Node secret
-- proof arms one immutable actor observation inside the existing World context.
CREATE TABLE game_cake_food_observations (
 backend_pid INTEGER PRIMARY KEY CHECK(backend_pid>0), transaction_xid XID8 NOT NULL,
 world_id UUID NOT NULL, channel_id UUID NOT NULL, event_start BIGINT NOT NULL,
 node_id UUID NOT NULL, registration NUMERIC(20,0) NOT NULL,
 character_id UUID NOT NULL, game_session_id UUID NOT NULL,
 command_id NUMERIC(20,0) NOT NULL, lease_generation NUMERIC(20,0) NOT NULL,
 actor_identity BYTEA NOT NULL CHECK(octet_length(actor_identity)=16),
 movement_revision NUMERIC(20,0) NOT NULL,state_revision NUMERIC(20,0) NOT NULL,
 content_digest BYTEA NOT NULL CHECK(octet_length(content_digest)=32),
 conditions_before JSONB NOT NULL,before_sha256 BYTEA NOT NULL CHECK(octet_length(before_sha256)=32),
 observed_after_sha256 BYTEA NOT NULL CHECK(octet_length(observed_after_sha256)=32),
 observed_at BIGINT NOT NULL
);
REVOKE ALL ON game_cake_food_observations FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
CREATE FUNCTION game_cake_arm_food_observation(p_node UUID,p_registration NUMERIC,p_secret BYTEA,
 p_character UUID,p_session UUID,p_command NUMERIC,p_lease NUMERIC,p_actor BYTEA,
 p_movement NUMERIC,p_state NUMERIC,p_content BYTEA,p_before JSONB,p_after JSONB) RETURNS VOID
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE c game_cake_world_node_context%ROWTYPE; prior game_cake_food_observations%ROWTYPE;
BEGIN
 SELECT * INTO c FROM game_cake_world_node_context WHERE backend_pid=pg_backend_pid()
  AND transaction_xid=pg_current_xact_id() FOR SHARE;
 IF NOT FOUND OR game_cake_world_context_is_current(c.world_id,c.event_start) IS DISTINCT FROM TRUE
  OR (c.node_id,c.registration) IS DISTINCT FROM(p_node,p_registration)
  OR game_node_prove_current_incarnation(p_node,p_registration,p_secret) IS DISTINCT FROM TRUE
  OR c.now_seconds>=c.event_start+518400
  OR p_command IS NULL OR p_lease IS NULL OR p_movement IS NULL OR p_state IS NULL
  OR p_command NOT BETWEEN 1 AND 18446744073709551615 OR p_command<>trunc(p_command)
  OR p_lease NOT BETWEEN 1 AND 18446744073709551615 OR p_lease<>trunc(p_lease)
  OR p_movement NOT BETWEEN 1 AND 18446744073709551615 OR p_movement<>trunc(p_movement)
  OR p_state NOT BETWEEN 1 AND 18446744073709551615 OR p_state<>trunc(p_state)
  OR octet_length(p_actor) IS DISTINCT FROM 16 OR p_actor=decode(repeat('00',16),'hex')
  OR octet_length(p_content) IS DISTINCT FROM 32
  OR (jsonb_typeof(p_before)='object' AND p_before->'schema'='1'::jsonb
    AND jsonb_typeof(p_before->'conditions')='array' AND jsonb_array_length(p_before->'conditions')<=32
    AND octet_length(p_before::text)<=32768
    AND jsonb_typeof(p_after)='object' AND p_after->'schema'='1'::jsonb
    AND jsonb_typeof(p_after->'conditions')='array' AND jsonb_array_length(p_after->'conditions')<=32 AND octet_length(p_after::text)<=32768) IS DISTINCT FROM TRUE
 THEN RAISE EXCEPTION 'Cake food observation lacks exact current Node and bounded actor state' USING ERRCODE='23514'; END IF;
 PERFORM 1 FROM game_durability_reconnect_sessions s JOIN game_character_roots r ON r.character_id=s.character_id
  WHERE s.game_session_id=p_session AND s.character_id=p_character AND s.character_lease_generation=p_lease
   AND s.session_state IN(1,2) AND s.world_id=c.world_id AND s.runtime_scope_kind=1
   AND s.runtime_scope_world_id=c.world_id AND s.runtime_scope_channel_id=c.channel_id
   AND s.runtime_scope_instance_id IS NULL AND s.runtime_scope_house_key IS NULL
   AND s.scope_ownership_generation=c.generation AND r.account_id=s.account_id
   AND r.world_id=c.world_id AND r.lifecycle=1 FOR SHARE OF s,r;
 IF NOT FOUND THEN RAISE EXCEPTION 'Cake food observation has stale actual actor session' USING ERRCODE='23514'; END IF;
 SELECT * INTO prior FROM game_cake_food_observations WHERE backend_pid=pg_backend_pid() FOR UPDATE;
 IF FOUND AND prior.transaction_xid=pg_current_xact_id() THEN
  IF (prior.world_id,prior.channel_id,prior.event_start,prior.node_id,prior.registration,
   prior.character_id,prior.game_session_id,prior.command_id,prior.lease_generation,
   prior.actor_identity,prior.movement_revision,prior.state_revision,prior.content_digest,prior.conditions_before,prior.observed_after_sha256)
   IS DISTINCT FROM(c.world_id,c.channel_id,c.event_start,p_node,p_registration,
   p_character,p_session,p_command,p_lease,p_actor,p_movement,p_state,p_content,p_before,sha256(convert_to(p_after::text,'UTF8')))
  THEN RAISE EXCEPTION 'Cake food observation cannot replace its actor predecessor' USING ERRCODE='23514'; END IF;
  RETURN;
 END IF;
 DELETE FROM game_cake_food_observations f WHERE f.backend_pid=pg_backend_pid()
  OR NOT EXISTS(SELECT 1 FROM pg_stat_activity a WHERE a.pid=f.backend_pid);
 INSERT INTO game_cake_food_observations(backend_pid,transaction_xid,world_id,channel_id,event_start,node_id,registration,
  character_id,game_session_id,command_id,lease_generation,actor_identity,movement_revision,state_revision,
  content_digest,conditions_before,before_sha256,observed_after_sha256,observed_at)
 VALUES(pg_backend_pid(),pg_current_xact_id(),c.world_id,c.channel_id,c.event_start,p_node,p_registration,
  p_character,p_session,p_command,p_lease,p_actor,p_movement,p_state,p_content,p_before,
  sha256(convert_to(p_before::text,'UTF8')),sha256(convert_to(p_after::text,'UTF8')),c.now_seconds);
END $$;
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_cake_arm_food_observation(uuid,numeric,bytea,uuid,uuid,numeric,numeric,bytea,numeric,numeric,bytea,jsonb,jsonb) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_cake_arm_food_observation(uuid,numeric,bytea,uuid,uuid,numeric,numeric,bytea,numeric,numeric,bytea,jsonb,jsonb) FROM PUBLIC,oteryn_game_control;
GRANT EXECUTE ON FUNCTION game_cake_arm_food_observation(uuid,numeric,bytea,uuid,uuid,numeric,numeric,bytea,numeric,numeric,bytea,jsonb,jsonb) TO oteryn_game_runtime;
