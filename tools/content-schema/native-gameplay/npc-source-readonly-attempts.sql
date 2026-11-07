-- Exact Character/current-XID completion scans must remain bounded as the
-- immutable source occurrence history grows. No authority predicate changes.
CREATE INDEX game_npc_source_readonly_greet_character_xact
 ON game_npc_source_readonly_greets(character_id,created_xact_id);
-- DRAFT: every fresh/replayed readonly occurrence has a current-XID no-gameplay proof.
-- The original immutable greeting row is never updated or assigned a new XID.
CREATE TABLE game_npc_source_readonly_greet_attempts(
 attempt_id UUID PRIMARY KEY,
 game_session_id UUID NOT NULL,connection_generation NUMERIC(20,0) NOT NULL,
 conversation_generation NUMERIC(20,0) NOT NULL,npc_identity BYTEA NOT NULL,npc_generation NUMERIC(20,0) NOT NULL,
 source_key TEXT NOT NULL,program_sha256 BYTEA NOT NULL CHECK(octet_length(program_sha256)=32),
 character_id UUID NOT NULL,original_revision NUMERIC(20,0) NOT NULL,command_id NUMERIC(20,0) NOT NULL,
 original_observation_id UUID NOT NULL,observation JSONB NOT NULL,
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
 FOREIGN KEY(game_session_id,connection_generation,conversation_generation,npc_identity,npc_generation,source_key)
 REFERENCES game_npc_source_readonly_greets(game_session_id,connection_generation,conversation_generation,npc_identity,npc_generation,source_key)
);
-- Leading columns match the complete occurrence foreign key.
CREATE INDEX game_npc_source_readonly_attempt_occurrence
 ON game_npc_source_readonly_greet_attempts(game_session_id,connection_generation,conversation_generation,npc_identity,npc_generation,source_key);
CREATE INDEX game_npc_source_readonly_attempt_character_xact
 ON game_npc_source_readonly_greet_attempts(character_id,created_xact_id);
REVOKE ALL ON game_npc_source_readonly_greet_attempts FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
CREATE TRIGGER game_npc_source_readonly_attempt_immutable BEFORE UPDATE OR DELETE ON game_npc_source_readonly_greet_attempts FOR EACH ROW EXECUTE FUNCTION game_character_immutable();
CREATE TRIGGER game_npc_source_readonly_attempt_no_truncate BEFORE TRUNCATE ON game_npc_source_readonly_greet_attempts FOR EACH STATEMENT EXECUTE FUNCTION game_character_reject_truncate();
CREATE FUNCTION game_npc_source_readonly_attempt_valid(p game_npc_source_readonly_greet_attempts)
 RETURNS BOOLEAN LANGUAGE SQL SECURITY DEFINER AS $$
 SELECT p.created_xact_id=pg_current_xact_id()
 AND EXISTS(SELECT 1 FROM game_npc_source_readonly_greets original
 JOIN game_npc_source_context c ON c.backend_pid=pg_backend_pid() AND c.transaction_xid=pg_current_xact_id()
 JOIN game_character_roots root ON root.character_id=p.character_id AND root.character_revision=p.original_revision
 WHERE original.game_session_id=p.game_session_id AND original.connection_generation=p.connection_generation
 AND original.conversation_generation=p.conversation_generation AND original.npc_identity=p.npc_identity
 AND original.npc_generation=p.npc_generation AND original.source_key=p.source_key
 AND original.observation_id=p.original_observation_id AND original.command_id=p.command_id
 AND original.character_id=p.character_id AND original.original_revision=p.original_revision
 AND original.program_sha256=p.program_sha256
 AND original.observation->'greet'=p.observation->'greet'
 AND original.observation->'actor_observation' IS NOT DISTINCT FROM p.observation->'actor_observation'
 AND game_npc_source_readonly_greet_qualified(c) AND c.npc_transaction_id=p.attempt_id AND NOT c.captured
 AND c.source_key=p.source_key AND c.program_sha256=p.program_sha256
 AND c.character_id=p.character_id AND c.original_revision=p.original_revision
 AND c.game_session_id=p.game_session_id AND c.connection_generation=p.connection_generation AND c.command_id=p.command_id
 AND c.before_state=p.observation
 AND NOT EXISTS(SELECT 1 FROM game_npc_cosmetic_receipts parent
 WHERE parent.transaction_id IN(p.attempt_id,p.original_observation_id)))
$$;
REVOKE ALL ON FUNCTION game_npc_source_readonly_attempt_valid(game_npc_source_readonly_greet_attempts) FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
CREATE FUNCTION game_npc_source_readonly_attempt_admit() RETURNS TRIGGER LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 NEW.created_xact_id:=pg_current_xact_id();
 IF game_npc_source_readonly_attempt_valid(NEW) IS DISTINCT FROM TRUE THEN
 RAISE EXCEPTION 'readonly attempt lacks exact sealed occurrence/current-XID authority' USING ERRCODE='23514';END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER game_npc_source_readonly_attempt_admit BEFORE INSERT ON game_npc_source_readonly_greet_attempts FOR EACH ROW EXECUTE FUNCTION game_npc_source_readonly_attempt_admit();
CREATE FUNCTION game_npc_source_readonly_attempt_complete() RETURNS TRIGGER LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE attempt game_npc_source_readonly_greet_attempts%ROWTYPE;
BEGIN
 FOR attempt IN SELECT * FROM game_npc_source_readonly_greet_attempts
 WHERE character_id=NEW.character_id AND created_xact_id=pg_current_xact_id() LOOP
 IF game_npc_source_readonly_attempt_valid(attempt) IS DISTINCT FROM TRUE THEN
 RAISE EXCEPTION 'readonly replay attempt cannot acquire a gameplay edge' USING ERRCODE='23514';END IF;
 END LOOP;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_npc_source_readonly_attempt_complete AFTER INSERT ON game_npc_source_readonly_greet_attempts DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_npc_source_readonly_attempt_complete();
CREATE CONSTRAINT TRIGGER game_npc_source_readonly_attempt_root_successor AFTER UPDATE ON game_character_roots DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_npc_source_readonly_attempt_complete();
REVOKE ALL ON FUNCTION game_npc_source_readonly_attempt_admit(),game_npc_source_readonly_attempt_complete() FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
DO $paths$ DECLARE signature TEXT;BEGIN
 FOREACH signature IN ARRAY ARRAY['game_npc_source_readonly_attempt_valid(game_npc_source_readonly_greet_attempts)','game_npc_source_readonly_attempt_admit()','game_npc_source_readonly_attempt_complete()'] LOOP
 EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',signature,current_schema());END LOOP;
END $paths$;

CREATE OR REPLACE FUNCTION game_npc_source_readonly_greet_capture() RETURNS VOID LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE c game_npc_source_context%ROWTYPE;g JSONB;prior game_npc_source_readonly_greets%ROWTYPE;
BEGIN
 SELECT * INTO c FROM game_npc_source_context WHERE backend_pid=pg_backend_pid() AND transaction_xid=pg_current_xact_id();
 IF NOT FOUND OR game_npc_source_readonly_greet_qualified(c) IS DISTINCT FROM TRUE THEN
 RAISE EXCEPTION 'read-only greeting lacks exact sealed registered callback' USING ERRCODE='23514';END IF;
 g:=c.before_state->'greet';
 INSERT INTO game_npc_source_readonly_greets VALUES(c.game_session_id,c.connection_generation,
 (g->>'conversation_generation')::numeric,decode(g->>'npc_identity','hex'),(g->>'npc_generation')::numeric,
 c.source_key,c.command_id,c.program_sha256,c.character_id,c.original_revision,c.npc_transaction_id,c.before_state,pg_current_xact_id()) ON CONFLICT DO NOTHING;
 IF NOT FOUND THEN
  SELECT * INTO STRICT prior FROM game_npc_source_readonly_greets
   WHERE game_session_id=c.game_session_id AND connection_generation=c.connection_generation
    AND conversation_generation=(g->>'conversation_generation')::numeric
    AND npc_identity=decode(g->>'npc_identity','hex') AND npc_generation=(g->>'npc_generation')::numeric AND source_key=c.source_key;
  IF prior.command_id IS DISTINCT FROM c.command_id OR prior.program_sha256 IS DISTINCT FROM c.program_sha256
   OR prior.character_id IS DISTINCT FROM c.character_id OR prior.original_revision IS DISTINCT FROM c.original_revision
   OR prior.observation->'greet' IS DISTINCT FROM g
   OR prior.observation->'actor_observation' IS DISTINCT FROM c.before_state->'actor_observation'
   OR EXISTS(SELECT 1 FROM game_npc_cosmetic_receipts WHERE transaction_id IN(prior.observation_id,c.npc_transaction_id))
   OR NOT EXISTS(SELECT 1 FROM game_character_roots WHERE character_id=c.character_id AND character_revision=prior.original_revision) THEN
   RAISE EXCEPTION 'read-only greeting replay differs from its original native occurrence' USING ERRCODE='23514';END IF;
 END IF;
 SELECT * INTO STRICT prior FROM game_npc_source_readonly_greets
 WHERE game_session_id=c.game_session_id AND connection_generation=c.connection_generation
 AND conversation_generation=(g->>'conversation_generation')::numeric
 AND npc_identity=decode(g->>'npc_identity','hex') AND npc_generation=(g->>'npc_generation')::numeric AND source_key=c.source_key;
 INSERT INTO game_npc_source_readonly_greet_attempts(attempt_id,game_session_id,connection_generation,conversation_generation,npc_identity,npc_generation,source_key,program_sha256,character_id,original_revision,command_id,original_observation_id,observation)
 VALUES(c.npc_transaction_id,c.game_session_id,c.connection_generation,(g->>'conversation_generation')::numeric,decode(g->>'npc_identity','hex'),(g->>'npc_generation')::numeric,c.source_key,c.program_sha256,c.character_id,c.original_revision,c.command_id,prior.observation_id,c.before_state)
 ON CONFLICT DO NOTHING;
 IF NOT FOUND AND NOT EXISTS(SELECT 1 FROM game_npc_source_readonly_greet_attempts a WHERE a.attempt_id=c.npc_transaction_id AND game_npc_source_readonly_attempt_valid(a)) THEN
 RAISE EXCEPTION 'readonly attempt identity conflict' USING ERRCODE='23514';END IF;
END $$;
