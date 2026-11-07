-- Read-only registered Vescu callbacks consume a sealed genuine greeting without
-- manufacturing a gameplay Root edge or a cosmetic acquisition receipt.
CREATE TABLE game_npc_source_readonly_greets(
 game_session_id UUID NOT NULL,connection_generation NUMERIC(20,0) NOT NULL,
 conversation_generation NUMERIC(20,0) NOT NULL CHECK(conversation_generation>=1),
 npc_identity BYTEA NOT NULL CHECK(octet_length(npc_identity)=16),
 npc_generation NUMERIC(20,0) NOT NULL CHECK(npc_generation>=1),source_key TEXT NOT NULL,
 command_id NUMERIC(20,0) NOT NULL,program_sha256 BYTEA NOT NULL CHECK(octet_length(program_sha256)=32),
 character_id UUID NOT NULL,original_revision NUMERIC(20,0) NOT NULL,observation_id UUID NOT NULL,
 observation JSONB NOT NULL,created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
 PRIMARY KEY(game_session_id,connection_generation,conversation_generation,npc_identity,npc_generation,source_key)
);
REVOKE ALL ON game_npc_source_readonly_greets FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
CREATE TRIGGER game_npc_source_readonly_greet_immutable BEFORE UPDATE OR DELETE ON game_npc_source_readonly_greets FOR EACH ROW EXECUTE FUNCTION game_character_immutable();
CREATE TRIGGER game_npc_source_readonly_greet_no_truncate BEFORE TRUNCATE ON game_npc_source_readonly_greets FOR EACH STATEMENT EXECUTE FUNCTION game_character_reject_truncate();
CREATE FUNCTION game_npc_source_readonly_greet_capture() RETURNS VOID LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE c game_npc_source_context%ROWTYPE;g JSONB;expected JSONB;
BEGIN
 SELECT * INTO c FROM game_npc_source_context WHERE backend_pid=pg_backend_pid() AND transaction_xid=pg_current_xact_id();
 IF NOT FOUND OR c.captured OR c.npc_key IS DISTINCT FROM 'oteryn:npc.vescu' OR c.source_key NOT IN(
 'oteryn:npc-cosmetic/vescu/assassin-returner-greet',
 'oteryn:npc-cosmetic/vescu/assassin-drunken-beginner-greet',
 'oteryn:npc-cosmetic/vescu/assassin-sober-refusal-greet') THEN
 RAISE EXCEPTION 'read-only greeting lacks sealed source observation' USING ERRCODE='23514';END IF;
 expected:=CASE WHEN c.source_key='oteryn:npc-cosmetic/vescu/assassin-sober-refusal-greet' THEN '[{"kind":"close_conversation"}]'::jsonb ELSE '[]'::jsonb END;
 IF c.expected_plan->'effects' IS DISTINCT FROM expected
 OR c.expected_plan->'changes' IS DISTINCT FROM '[]'::jsonb
 OR c.expected_plan->'costs' IS DISTINCT FROM '[]'::jsonb
 OR c.expected_plan->'grant_keys' IS DISTINCT FROM '[]'::jsonb THEN
 RAISE EXCEPTION 'read-only greeting cannot mutate gameplay' USING ERRCODE='23514';END IF;
 g:=c.before_state->'greet';
 INSERT INTO game_npc_source_readonly_greets VALUES(c.game_session_id,c.connection_generation,
 (g->>'conversation_generation')::numeric,decode(g->>'npc_identity','hex'),(g->>'npc_generation')::numeric,
 c.source_key,c.command_id,c.program_sha256,c.character_id,c.original_revision,c.npc_transaction_id,c.before_state,pg_current_xact_id());
END $$;
REVOKE ALL ON FUNCTION game_npc_source_readonly_greet_capture() FROM PUBLIC,oteryn_game_control;
GRANT EXECUTE ON FUNCTION game_npc_source_readonly_greet_capture() TO oteryn_game_runtime;
DO $path$ BEGIN EXECUTE format('ALTER FUNCTION game_npc_source_readonly_greet_capture() SET search_path=pg_catalog,%I,pg_temp',current_schema());END $path$;

CREATE FUNCTION game_npc_source_readonly_greet_complete() RETURNS TRIGGER LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE r game_npc_source_readonly_greets%ROWTYPE;target UUID;
BEGIN
 target:=NEW.character_id;
 FOR r IN SELECT * FROM game_npc_source_readonly_greets WHERE character_id=target AND created_xact_id=pg_current_xact_id() LOOP
 IF NOT EXISTS(SELECT 1 FROM game_character_roots root WHERE root.character_id=r.character_id AND root.character_revision=r.original_revision)
 OR EXISTS(SELECT 1 FROM game_npc_cosmetic_receipts parent WHERE parent.transaction_id=r.observation_id)
 OR NOT EXISTS(SELECT 1 FROM game_npc_source_context c WHERE c.backend_pid=pg_backend_pid() AND c.transaction_xid=pg_current_xact_id()
 AND c.npc_transaction_id=r.observation_id AND c.character_id=r.character_id AND c.original_revision=r.original_revision
 AND NOT c.captured AND c.before_state=r.observation AND c.program_sha256=r.program_sha256 AND c.source_key=r.source_key) THEN
 RAISE EXCEPTION 'read-only greeting cannot acquire a gameplay edge' USING ERRCODE='23514';END IF;
 END LOOP;
 RETURN NULL;
END $$;
REVOKE ALL ON FUNCTION game_npc_source_readonly_greet_complete() FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
DO $path$ BEGIN EXECUTE format('ALTER FUNCTION game_npc_source_readonly_greet_complete() SET search_path=pg_catalog,%I,pg_temp',current_schema());END $path$;
CREATE CONSTRAINT TRIGGER game_npc_source_readonly_greet_complete AFTER INSERT ON game_npc_source_readonly_greets
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_npc_source_readonly_greet_complete();
CREATE CONSTRAINT TRIGGER game_npc_source_readonly_greet_root_successor AFTER UPDATE ON game_character_roots
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_npc_source_readonly_greet_complete();
