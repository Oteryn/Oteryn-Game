-- One genuine native conversation-owner greeting, never a text-derived claim.
CREATE TABLE game_npc_source_greet_occurrences(
 game_session_id UUID NOT NULL,connection_generation NUMERIC(20,0) NOT NULL,
 conversation_generation NUMERIC(20,0) NOT NULL CHECK(conversation_generation>=1),
 npc_identity BYTEA NOT NULL CHECK(octet_length(npc_identity)=16),
 npc_generation NUMERIC(20,0) NOT NULL CHECK(npc_generation>=1),source_key TEXT NOT NULL,
 parent_transaction_id UUID NOT NULL UNIQUE REFERENCES game_npc_cosmetic_receipts(transaction_id),
 actor_identity BYTEA NOT NULL CHECK(octet_length(actor_identity)=16),
 catalogue_revision TEXT NOT NULL CHECK(octet_length(catalogue_revision) BETWEEN 1 AND 256),
 movement_revision NUMERIC(20,0) NOT NULL CHECK(movement_revision>=1),
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
 PRIMARY KEY(game_session_id,connection_generation,conversation_generation,npc_identity,npc_generation,source_key)
);
REVOKE ALL ON game_npc_source_greet_occurrences FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
GRANT SELECT ON game_npc_source_greet_occurrences TO oteryn_game_runtime;
CREATE TRIGGER game_npc_source_greet_immutable BEFORE UPDATE OR DELETE ON game_npc_source_greet_occurrences FOR EACH ROW EXECUTE FUNCTION game_character_immutable();
CREATE TRIGGER game_npc_source_greet_no_truncate BEFORE TRUNCATE ON game_npc_source_greet_occurrences FOR EACH STATEMENT EXECUTE FUNCTION game_character_reject_truncate();
CREATE FUNCTION game_npc_source_greet_capture() RETURNS TRIGGER LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE c game_npc_source_context%ROWTYPE;g JSONB;
BEGIN
 IF NEW.program_sha256 IS NULL THEN RETURN NULL;END IF;
 SELECT * INTO c FROM game_npc_source_context WHERE backend_pid=pg_backend_pid() AND transaction_xid=pg_current_xact_id() AND npc_transaction_id=NEW.transaction_id;
 g:=c.before_state->'greet';
 IF NEW.source_key<>'oteryn:npc-cosmetic/ajax/barbarian-greet' THEN
  IF g IS NOT NULL AND g<>'null'::jsonb THEN RAISE EXCEPTION 'greeting observation cannot authorize a Say source' USING ERRCODE='23514';END IF;
  RETURN NULL;
 END IF;
 IF game_npc_source_parent_current(NEW.transaction_id,NEW.source_key) IS DISTINCT FROM TRUE
  OR c.npc_key IS DISTINCT FROM 'oteryn:npc.ajax' OR jsonb_typeof(g) IS DISTINCT FROM 'object'
  OR g->>'game_session_id' IS DISTINCT FROM c.game_session_id::text
  OR g->>'connection_generation' IS DISTINCT FROM c.connection_generation::text
  OR g->>'command_id' IS DISTINCT FROM c.command_id::text
  OR coalesce(g->>'actor_identity','')!~'^[0-9a-f]{32}$' OR coalesce(g->>'npc_identity','')!~'^[0-9a-f]{32}$'
  OR g->>'actor_identity'=repeat('0',32) OR g->>'npc_identity'=repeat('0',32)
  OR coalesce(g->>'npc_generation','')!~'^[1-9][0-9]{0,19}$'
  OR coalesce(g->>'conversation_generation','')!~'^[1-9][0-9]{0,19}$'
  OR coalesce(g->>'movement_revision','')!~'^[1-9][0-9]{0,19}$'
  OR coalesce(octet_length(g->>'catalogue_revision'),0) NOT BETWEEN 1 AND 256
  OR (g->>'catalogue_revision')~'[[:cntrl:]]' THEN
  RAISE EXCEPTION 'source Greet requires exact sealed current native occurrence' USING ERRCODE='23514';END IF;
 INSERT INTO game_npc_source_greet_occurrences VALUES(c.game_session_id,c.connection_generation,
  (g->>'conversation_generation')::numeric,decode(g->>'npc_identity','hex'),(g->>'npc_generation')::numeric,
  NEW.source_key,NEW.transaction_id,decode(g->>'actor_identity','hex'),g->>'catalogue_revision',
  (g->>'movement_revision')::numeric,pg_current_xact_id());
 RETURN NULL;
END $$;
REVOKE ALL ON FUNCTION game_npc_source_greet_capture() FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
DO $path$ BEGIN EXECUTE format('ALTER FUNCTION game_npc_source_greet_capture() SET search_path=pg_catalog,%I,pg_temp',current_schema());END $path$;
CREATE TRIGGER game_npc_source_greet_capture AFTER INSERT ON game_npc_cosmetic_receipts FOR EACH ROW EXECUTE FUNCTION game_npc_source_greet_capture();
