-- Generic registered quest consequences of immutable original source leases.
ALTER TABLE game_character_quest_receipts DROP CONSTRAINT game_character_quest_receipts_cause_kind_check;
ALTER TABLE game_character_quest_receipts ADD CONSTRAINT game_character_quest_receipts_cause_kind_check
 CHECK(cause_kind IN ('command','use','step','claim_obligation','creature_death','server_timer'));
DO $$ DECLARE c RECORD; BEGIN
 FOR c IN SELECT conname FROM pg_constraint WHERE conrelid='game_character_quest_receipts'::regclass
  AND contype='c' AND pg_get_constraintdef(oid) LIKE '%cause_kind%' AND pg_get_constraintdef(oid) LIKE '%cause_ordinal%' LOOP
  EXECUTE format('ALTER TABLE game_character_quest_receipts DROP CONSTRAINT %I',c.conname);
 END LOOP;
END $$;
ALTER TABLE game_character_quest_receipts ADD CONSTRAINT game_quest_timer_cause_ordinal
 CHECK((cause_kind IN ('creature_death','server_timer'))=(cause_ordinal=0));
CREATE TABLE game_quest_timer_rules (
 recipe TEXT PRIMARY KEY CHECK(length(recipe) BETWEEN 1 AND 128),
 quest_key TEXT NOT NULL CHECK(length(quest_key) BETWEEN 1 AND 128),
 source_recipe TEXT NOT NULL REFERENCES game_shared_switch_recipes(recipe),
 track_key TEXT NOT NULL CHECK(length(track_key) BETWEEN 1 AND 128),
 maximum_before BIGINT NOT NULL CHECK(maximum_before>=-1),
 value_after BIGINT NOT NULL CHECK(value_after>maximum_before),
 binding BYTEA NOT NULL CHECK(binding=sha256(convert_to('oteryn:quest-timer/v1|'||recipe||'|'||quest_key||'|'||source_recipe||'|'||track_key||'|-1|'||maximum_before::text||'|'||value_after::text,'UTF8')))
);
INSERT INTO game_quest_timer_rules
 SELECT p.*,sha256(convert_to('oteryn:quest-timer/v1|'||recipe||'|'||quest_key||'|'||source_recipe||'|'||track_key||'|-1|'||maximum_before::text||'|'||value_after::text,'UTF8'))
 FROM (VALUES
 ('oteryn:quest-timer/ancient-tombs-thalas-treasure','oteryn:quest.the_ancient_tombs_quest','oteryn:quest-callback/ancient-tombs-thalas-switch','oteryn:quest-track/ancient-tombs/thalas-treasure',0::bigint,1::bigint),
 ('oteryn:quest-timer/ancient-tombs-diprath-treasure','oteryn:quest.the_ancient_tombs_quest','oteryn:quest-callback/ancient-tombs-diprath-switch','oteryn:quest-track/ancient-tombs/diprath-treasure',1::bigint,2::bigint)
 ) p(recipe,quest_key,source_recipe,track_key,maximum_before,value_after);
CREATE INDEX game_quest_timer_actor_due ON game_shared_switch_leases(character_id,game_session_id,expires_at_unix_ms,transaction_id);
CREATE FUNCTION game_quest_timer_source_current(p_lease UUID,p_recipe TEXT,p_character UUID,p_world UUID,p_channel UUID,p_session UUID,p_generation NUMERIC)
RETURNS BOOLEAN LANGUAGE sql STABLE AS $$
 SELECT EXISTS(SELECT 1 FROM game_shared_switch_leases l JOIN game_quest_timer_rules p ON p.source_recipe=l.recipe
 JOIN game_durability_reconnect_sessions s ON s.game_session_id=l.game_session_id AND s.character_id=l.character_id
 JOIN game_durability_admission_character_guards g ON g.character_id=l.character_id
 JOIN game_character_roots c ON c.character_id=l.character_id
 WHERE l.transaction_id=p_lease AND p.recipe=p_recipe AND l.character_id=p_character AND l.world_id=p_world
  AND l.game_session_id=p_session AND l.character_lease_generation=p_generation
  AND l.expires_at_unix_ms<=floor(extract(epoch FROM statement_timestamp())*1000)::bigint
  AND s.session_state=2 AND s.character_lease_generation=p_generation AND s.runtime_scope_kind=1
  AND s.runtime_scope_world_id=p_world AND s.runtime_scope_channel_id=p_channel
  AND g.eligible AND g.holder_game_session_id=p_session AND g.lease_generation=p_generation
  AND c.world_id=p_world AND c.lifecycle=1
  AND game_world_reset_map_epoch_current(p_world,(l.context->>'reset_epoch')::numeric)
  AND EXISTS(SELECT 1 FROM game_content_activations a WHERE a.world_id=p_world AND a.channel_id=p_channel
   AND 'sha256:'||encode(a.server_artifact_digest,'hex')=l.context->>'content_revision'
   AND (SELECT jsonb_agg(get_byte(a.frame_binding_digest,n) ORDER BY n) FROM generate_series(0,octet_length(a.frame_binding_digest)-1) n)=l.context->'placement_context'
   AND a.activation_sequence=(SELECT max(activation_sequence) FROM game_content_activations WHERE world_id=p_world AND channel_id=p_channel)))
$$;
CREATE TABLE game_quest_timer_observations (
 lease_id UUID NOT NULL REFERENCES game_shared_switch_leases(transaction_id),
 recipe TEXT NOT NULL REFERENCES game_quest_timer_rules(recipe),
 event_id UUID NOT NULL UNIQUE DEFAULT game_character_uuid_v7() CHECK(game_character_is_uuid_v7(event_id)),
 character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
 world_id UUID NOT NULL, channel_id UUID NOT NULL, game_session_id UUID NOT NULL,
 character_lease_generation NUMERIC(20,0) NOT NULL CHECK(character_lease_generation>0),
 connection_generation NUMERIC(20,0) NOT NULL CHECK(connection_generation>0),
 ownership_generation NUMERIC(20,0) NOT NULL CHECK(ownership_generation>0),
 root_revision_before NUMERIC(20,0) NOT NULL,
 value_before BIGINT NOT NULL, value_after BIGINT NOT NULL,
 committed_revision NUMERIC(20,0),
 observed_at_unix_ms BIGINT NOT NULL,
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
 PRIMARY KEY(lease_id,recipe),
 CHECK(committed_revision IS NULL OR committed_revision=root_revision_before+1)
);
CREATE TABLE game_quest_timer_audit (
 lease_id UUID NOT NULL, recipe TEXT NOT NULL,
 envelope BYTEA NOT NULL CHECK(octet_length(envelope) BETWEEN 1 AND 65536),
 envelope_sha256 BYTEA NOT NULL CHECK(envelope_sha256=sha256(envelope)),
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
 PRIMARY KEY(lease_id,recipe),
 FOREIGN KEY(lease_id,recipe) REFERENCES game_quest_timer_observations
);
CREATE FUNCTION game_quest_timer_admit() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE p game_quest_timer_rules%ROWTYPE; actual BIGINT;
BEGIN
 NEW.created_xact_id:=pg_current_xact_id();
 NEW.observed_at_unix_ms:=floor(extract(epoch FROM statement_timestamp())*1000)::bigint;
 SELECT * INTO STRICT p FROM game_quest_timer_rules WHERE recipe=NEW.recipe;
 SELECT character_revision INTO STRICT NEW.root_revision_before FROM game_character_roots WHERE character_id=NEW.character_id;
 SELECT coalesce((SELECT value FROM game_character_quest_tracks WHERE character_id=NEW.character_id AND track_key=p.track_key),-1) INTO actual;
 IF NOT game_quest_timer_source_current(NEW.lease_id,NEW.recipe,NEW.character_id,NEW.world_id,NEW.channel_id,NEW.game_session_id,NEW.character_lease_generation)
  OR NOT EXISTS(SELECT 1 FROM game_durability_reconnect_sessions s WHERE s.game_session_id=NEW.game_session_id AND s.character_id=NEW.character_id
   AND s.current_generation=NEW.connection_generation AND s.scope_ownership_generation=NEW.ownership_generation)
  OR NEW.value_before<>actual
  OR (NEW.committed_revision IS NOT NULL AND (actual>p.maximum_before OR NEW.value_after<>p.value_after OR NEW.committed_revision<>NEW.root_revision_before+1))
  OR (NEW.committed_revision IS NULL AND (actual<=p.maximum_before OR NEW.value_after<>actual))
 THEN RAISE EXCEPTION 'quest timer requires original due lease, online actor and exact registered progress' USING ERRCODE='23514'; END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER quest_timer_admit BEFORE INSERT ON game_quest_timer_observations FOR EACH ROW EXECUTE FUNCTION game_quest_timer_admit();
CREATE FUNCTION game_quest_timer_envelope(p_lease UUID,p_recipe TEXT) RETURNS BYTEA LANGUAGE sql STABLE AS $$
 SELECT convert_to(jsonb_build_object('lease',to_jsonb(l),'source_recipe',to_jsonb(s),'rule',to_jsonb(p),'observation',to_jsonb(o),'quest_receipt',to_jsonb(q))::text,'UTF8')
 FROM game_quest_timer_observations o JOIN game_shared_switch_leases l ON l.transaction_id=o.lease_id
 JOIN game_shared_switch_recipes s ON s.recipe=l.recipe JOIN game_quest_timer_rules p ON p.recipe=o.recipe
 LEFT JOIN game_character_quest_receipts q ON q.character_id=o.character_id AND q.cause_kind='server_timer'
  AND q.cause_id=o.lease_id AND q.cause_ordinal=0 AND q.transition_key=o.recipe
 WHERE o.lease_id=p_lease AND o.recipe=p_recipe
$$;
CREATE FUNCTION game_quest_timer_complete(p_lease UUID,p_recipe TEXT) RETURNS VOID LANGUAGE plpgsql AS $$
DECLARE o game_quest_timer_observations%ROWTYPE; p game_quest_timer_rules%ROWTYPE; a game_quest_timer_audit%ROWTYPE; q game_character_quest_receipts%ROWTYPE;
BEGIN
 SELECT * INTO STRICT o FROM game_quest_timer_observations WHERE lease_id=p_lease AND recipe=p_recipe;
 SELECT * INTO STRICT p FROM game_quest_timer_rules WHERE recipe=p_recipe;
 SELECT * INTO a FROM game_quest_timer_audit WHERE lease_id=p_lease AND recipe=p_recipe;
 SELECT * INTO q FROM game_character_quest_receipts WHERE character_id=o.character_id AND cause_kind='server_timer' AND cause_id=o.lease_id AND cause_ordinal=0 AND transition_key=o.recipe;
 IF a.created_xact_id IS DISTINCT FROM o.created_xact_id OR a.envelope IS DISTINCT FROM game_quest_timer_envelope(p_lease,p_recipe)
  OR a.envelope_sha256 IS DISTINCT FROM sha256(a.envelope)
  OR (o.committed_revision IS NULL AND (q.character_id IS NOT NULL OR (SELECT character_revision FROM game_character_roots WHERE character_id=o.character_id)<>o.root_revision_before))
  OR (o.committed_revision IS NOT NULL AND (q.created_xact_id IS DISTINCT FROM o.created_xact_id OR q.committed_character_revision IS DISTINCT FROM o.committed_revision
   OR q.original_character_revision IS DISTINCT FROM o.root_revision_before OR q.quest_key IS DISTINCT FROM p.quest_key OR q.completes
   OR q.track_keys IS DISTINCT FROM ARRAY[p.track_key] OR q.values_before IS DISTINCT FROM ARRAY[o.value_before] OR q.values_after IS DISTINCT FROM ARRAY[o.value_after]))
 THEN RAISE EXCEPTION 'quest timer requires complete immutable same-XID outcome and source audit' USING ERRCODE='23514'; END IF;
END $$;
CREATE FUNCTION game_quest_timer_deferred() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN PERFORM game_quest_timer_complete(NEW.lease_id,NEW.recipe); RETURN NULL; END $$;
CREATE CONSTRAINT TRIGGER quest_timer_observation_complete AFTER INSERT ON game_quest_timer_observations DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_quest_timer_deferred();
CREATE CONSTRAINT TRIGGER quest_timer_audit_complete AFTER INSERT ON game_quest_timer_audit DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_quest_timer_deferred();
CREATE FUNCTION game_quest_timer_receipt_proven() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NEW.cause_kind='server_timer' THEN
  IF NOT EXISTS(SELECT 1 FROM game_quest_timer_observations o WHERE o.character_id=NEW.character_id AND o.lease_id=NEW.cause_id
   AND o.recipe=NEW.transition_key AND NEW.cause_ordinal=0 AND o.committed_revision=NEW.committed_character_revision AND o.created_xact_id=NEW.created_xact_id)
  THEN RAISE EXCEPTION 'server timer receipt lacks its original lease proof' USING ERRCODE='23514'; END IF;
  PERFORM game_quest_timer_complete(NEW.cause_id,NEW.transition_key);
 END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER quest_timer_receipt_proven AFTER INSERT ON game_character_quest_receipts DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_quest_timer_receipt_proven();
CREATE FUNCTION game_quest_timer_immutable() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN RAISE EXCEPTION 'quest timer source history is immutable' USING ERRCODE='23514'; END $$;
DO $$ DECLARE t TEXT; BEGIN
 FOREACH t IN ARRAY ARRAY['game_quest_timer_rules','game_quest_timer_observations','game_quest_timer_audit'] LOOP
  EXECUTE format('CREATE TRIGGER quest_timer_immutable BEFORE UPDATE OR DELETE ON %I FOR EACH ROW EXECUTE FUNCTION game_quest_timer_immutable()',t);
  EXECUTE format('CREATE TRIGGER quest_timer_no_truncate BEFORE TRUNCATE ON %I FOR EACH STATEMENT EXECUTE FUNCTION game_quest_timer_immutable()',t);
 END LOOP;
END $$;
