-- Bounded source-keyed World switch leases. Counter reads derive from original
-- database deadlines; replay never extends a contribution or invents Quest progress.
CREATE TABLE game_shared_switch_recipes (
 recipe TEXT PRIMARY KEY CHECK(length(recipe) BETWEEN 1 AND 160),
 group_key TEXT NOT NULL CHECK(length(group_key) BETWEEN 1 AND 160),
 action_id INTEGER NOT NULL UNIQUE CHECK(action_id BETWEEN 1 AND 65535),
 lifetime_ms BIGINT NOT NULL CHECK(lifetime_ms BETWEEN 1 AND 604800000)
);
INSERT INTO game_shared_switch_recipes VALUES
 ('oteryn:quest-callback/ancient-tombs-thalas-switch','oteryn:world-counter/ancient-tombs/thalas',12121,1200000),
 ('oteryn:quest-callback/ancient-tombs-diprath-switch','oteryn:world-counter/ancient-tombs/diprath',12122,1200000),
 ('oteryn:quest-callback/ancient-tombs-ashmunrah-switch','oteryn:world-counter/ancient-tombs/ashmunrah',12123,1200000);
CREATE TABLE game_shared_switch_leases (
 transaction_id UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(transaction_id)),
 event_id UUID NOT NULL UNIQUE CHECK(game_character_is_uuid_v7(event_id)),
 recipe TEXT NOT NULL REFERENCES game_shared_switch_recipes,
 character_id UUID NOT NULL REFERENCES game_character_roots,
 world_id UUID NOT NULL, channel_id UUID NOT NULL, game_session_id UUID NOT NULL,
 cause_ordinal NUMERIC(20,0) NOT NULL CHECK(cause_ordinal BETWEEN 1 AND 18446744073709551615),
 character_lease_generation NUMERIC(20,0) NOT NULL CHECK(character_lease_generation>0),
 connection_generation NUMERIC(20,0) NOT NULL CHECK(connection_generation>0),
 ownership_generation NUMERIC(20,0) NOT NULL CHECK(ownership_generation>0),
 placement BYTEA NOT NULL CHECK(octet_length(placement)=40),
 object_revision NUMERIC(20,0) NOT NULL CHECK(object_revision>=0),
 context JSONB NOT NULL CHECK(jsonb_typeof(context)='object' AND octet_length(context::text)<=4096),
 intent BYTEA NOT NULL CHECK(octet_length(intent) BETWEEN 1 AND 8192),
 intent_sha256 BYTEA NOT NULL CHECK(intent_sha256=sha256(intent)),
 occurred_at_unix_ms BIGINT NOT NULL CHECK(occurred_at_unix_ms>0),
 expires_at_unix_ms BIGINT NOT NULL CHECK(expires_at_unix_ms>occurred_at_unix_ms),
 active_before INTEGER NOT NULL CHECK(active_before BETWEEN 0 AND 499),
 active_after INTEGER NOT NULL CHECK(active_after=active_before+1),
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
 UNIQUE(game_session_id,cause_ordinal)
);
CREATE INDEX game_shared_switch_scope_due ON game_shared_switch_leases
 (world_id,(context->>'map_revision'),((context->>'reset_epoch')::numeric),expires_at_unix_ms);
CREATE TABLE game_shared_switch_audit (
 event_id UUID PRIMARY KEY REFERENCES game_shared_switch_leases(event_id),
 transaction_id UUID NOT NULL UNIQUE REFERENCES game_shared_switch_leases,
 envelope BYTEA NOT NULL CHECK(octet_length(envelope) BETWEEN 1 AND 32768),
 envelope_sha256 BYTEA NOT NULL CHECK(envelope_sha256=sha256(envelope)),
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id()
);
CREATE FUNCTION game_shared_switch_lock(p_world UUID,p_map TEXT,p_epoch NUMERIC)
RETURNS VOID LANGUAGE sql AS $$
 SELECT pg_advisory_xact_lock(hashtextextended('oteryn:world-switch:'||p_world::text||':'||p_map||':'||p_epoch::text,0))
$$;
CREATE FUNCTION game_shared_switch_count(p_world UUID,p_map TEXT,p_epoch NUMERIC,p_group TEXT,p_now BIGINT)
RETURNS BIGINT LANGUAGE sql STABLE AS $$
 SELECT count(*) FROM game_shared_switch_leases r JOIN game_shared_switch_recipes p USING(recipe)
 WHERE r.world_id=p_world AND r.context->>'map_revision'=p_map
  AND (r.context->>'reset_epoch')::numeric=p_epoch AND p.group_key=p_group AND r.expires_at_unix_ms>p_now
$$;
CREATE FUNCTION game_shared_switch_admit() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE p game_shared_switch_recipes%ROWTYPE; actual BIGINT;
BEGIN
 NEW.created_xact_id:=pg_current_xact_id();
 SELECT * INTO STRICT p FROM game_shared_switch_recipes WHERE recipe=NEW.recipe;
 IF convert_from(NEW.intent,'UTF8')::jsonb IS DISTINCT FROM jsonb_build_object(
   'recipe',NEW.recipe,'placement',encode(NEW.placement,'hex'),'object_revision',NEW.object_revision,
   'context',NEW.context,'character',NEW.character_id::text)
  OR NOT (NEW.context ?& ARRAY['map_revision','content_revision','placement_context','reset_epoch','actor_placement','movement_revision'])
  OR jsonb_typeof(NEW.context->'reset_epoch') IS DISTINCT FROM 'number'
  OR jsonb_typeof(NEW.context->'movement_revision') IS DISTINCT FROM 'number'
  OR (NEW.context->>'movement_revision')::numeric<1
  OR NEW.expires_at_unix_ms-NEW.occurred_at_unix_ms<>p.lifetime_ms
  OR NEW.occurred_at_unix_ms>floor(extract(epoch FROM statement_timestamp())*1000)
  OR NOT EXISTS(SELECT 1 FROM game_durability_reconnect_sessions s
   JOIN game_durability_admission_character_guards g USING(character_id)
   JOIN game_character_roots c USING(character_id)
   WHERE s.game_session_id=NEW.game_session_id AND s.character_id=NEW.character_id AND s.session_state=2
    AND s.current_generation=NEW.connection_generation AND s.character_lease_generation=NEW.character_lease_generation
    AND s.runtime_scope_kind=1 AND s.runtime_scope_world_id=NEW.world_id AND s.runtime_scope_channel_id=NEW.channel_id
    AND s.scope_ownership_generation=NEW.ownership_generation AND g.eligible AND g.holder_game_session_id=s.game_session_id
    AND g.lease_generation=s.character_lease_generation AND c.world_id=NEW.world_id AND c.lifecycle=1)
  OR NOT game_world_reset_map_epoch_current(NEW.world_id,(NEW.context->>'reset_epoch')::numeric)
  OR NEW.context->>'map_revision' IS NULL OR NEW.context->>'content_revision' IS NULL
  OR NOT EXISTS(SELECT 1 FROM game_content_activations a WHERE a.world_id=NEW.world_id AND a.channel_id=NEW.channel_id
    AND 'sha256:'||encode(a.server_artifact_digest,'hex')=NEW.context->>'content_revision'
    AND (SELECT jsonb_agg(get_byte(a.frame_binding_digest,n) ORDER BY n) FROM generate_series(0,octet_length(a.frame_binding_digest)-1) n)=NEW.context->'placement_context'
    AND a.activation_sequence=(SELECT max(activation_sequence) FROM game_content_activations WHERE world_id=NEW.world_id AND channel_id=NEW.channel_id))
 THEN RAISE EXCEPTION 'shared switch requires exact source intent, original timer and current authority' USING ERRCODE='23514'; END IF;
 PERFORM game_shared_switch_lock(NEW.world_id,NEW.context->>'map_revision',(NEW.context->>'reset_epoch')::numeric);
 IF EXISTS(SELECT 1 FROM game_shared_switch_leases r WHERE r.world_id=NEW.world_id
   AND r.context->>'map_revision'=NEW.context->>'map_revision'
   AND r.context->'reset_epoch'=NEW.context->'reset_epoch' AND r.placement=NEW.placement
   AND r.expires_at_unix_ms>NEW.occurred_at_unix_ms)
 THEN RAISE EXCEPTION 'one physical switch contributes at most once while active' USING ERRCODE='23514'; END IF;
 actual:=game_shared_switch_count(NEW.world_id,NEW.context->>'map_revision',(NEW.context->>'reset_epoch')::numeric,p.group_key,NEW.occurred_at_unix_ms);
 IF actual<>NEW.active_before OR NEW.active_after<>actual+1
 THEN RAISE EXCEPTION 'shared switch count must join the same serialized World fact' USING ERRCODE='23514'; END IF;
 RETURN NEW;
END $$;
CREATE FUNCTION game_shared_switch_complete(p_transaction UUID) RETURNS VOID LANGUAGE plpgsql AS $$
DECLARE r game_shared_switch_leases%ROWTYPE; p game_shared_switch_recipes%ROWTYPE; a game_shared_switch_audit%ROWTYPE; expected BYTEA;
BEGIN
 SELECT * INTO STRICT r FROM game_shared_switch_leases WHERE transaction_id=p_transaction;
 SELECT * INTO STRICT p FROM game_shared_switch_recipes WHERE recipe=r.recipe;
 SELECT * INTO a FROM game_shared_switch_audit WHERE transaction_id=p_transaction;
 expected:=convert_to(jsonb_build_object('lease',to_jsonb(r),'recipe',to_jsonb(p))::text,'UTF8');
 IF a.event_id IS DISTINCT FROM r.event_id OR a.created_xact_id IS DISTINCT FROM r.created_xact_id
  OR a.envelope IS DISTINCT FROM expected OR a.envelope_sha256 IS DISTINCT FROM sha256(expected)
  OR EXISTS(SELECT 1 FROM game_item_instances WHERE last_transaction_id=p_transaction)
 THEN RAISE EXCEPTION 'shared switch requires its complete same-XID lease audit and no Item debit' USING ERRCODE='23514'; END IF;
END $$;
CREATE FUNCTION game_shared_switch_deferred() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN PERFORM game_shared_switch_complete(NEW.transaction_id); RETURN NULL; END $$;
CREATE TRIGGER shared_switch_admit BEFORE INSERT ON game_shared_switch_leases FOR EACH ROW EXECUTE FUNCTION game_shared_switch_admit();
CREATE CONSTRAINT TRIGGER shared_switch_lease_complete AFTER INSERT ON game_shared_switch_leases DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_shared_switch_deferred();
CREATE CONSTRAINT TRIGGER shared_switch_audit_complete AFTER INSERT ON game_shared_switch_audit DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_shared_switch_deferred();
DO $$ DECLARE t TEXT; BEGIN
 FOREACH t IN ARRAY ARRAY['game_shared_switch_recipes','game_shared_switch_leases','game_shared_switch_audit'] LOOP
  EXECUTE format('CREATE TRIGGER shared_switch_immutable BEFORE UPDATE OR DELETE ON %I FOR EACH ROW EXECUTE FUNCTION game_item_immutable()',t);
 END LOOP;
END $$;
