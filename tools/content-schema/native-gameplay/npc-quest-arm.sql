-- DRAFT, composed after the qualified source parent and Quest seal helpers.
-- The original arm remains byte-for-byte and private; its locked raw-before,
-- Node/session/scope, clock, costs, owned writes and grant checks all run first.
ALTER FUNCTION game_npc_arm_source(UUID,NUMERIC,BYTEA,JSONB,BYTEA,JSONB,JSONB)
 RENAME TO game_npc_arm_source_before_quest_children;
REVOKE ALL ON FUNCTION game_npc_arm_source_before_quest_children(UUID,NUMERIC,BYTEA,JSONB,BYTEA,JSONB,JSONB)
 FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
CREATE FUNCTION game_npc_arm_source(p_node UUID,p_registration NUMERIC,p_secret BYTEA,
 p_context JSONB,p_program BYTEA,p_before JSONB,p_plan JSONB) RETURNS BYTEA
 LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE digest BYTEA;actual_context game_npc_source_context%ROWTYPE;
BEGIN
 digest:=game_npc_arm_source_before_quest_children(p_node,p_registration,p_secret,p_context,p_program,p_before,p_plan);
 SELECT * INTO actual_context FROM game_npc_source_context
  WHERE backend_pid=pg_backend_pid() AND transaction_xid=pg_current_xact_id();
 IF NOT FOUND OR actual_context.program_sha256 IS DISTINCT FROM p_program
  OR actual_context.expected_plan IS DISTINCT FROM p_plan
  OR actual_context.before_state IS DISTINCT FROM p_before THEN
 RAISE EXCEPTION 'Quest source arm lost its original sealed context' USING ERRCODE='23514';END IF;
 IF EXISTS(SELECT 1 FROM game_npc_source_quest_policies policy
   WHERE policy.program_sha256=actual_context.program_sha256 AND policy.source_key=actual_context.source_key) THEN
  IF game_npc_quest_plan_valid(actual_context.program_sha256,actual_context.source_key,
    actual_context.character_id,p_before,p_plan) IS DISTINCT FROM TRUE THEN
  RAISE EXCEPTION 'Quest source arm lacks exact finite child coverage' USING ERRCODE='23514';END IF;
 ELSIF p_plan ? 'quest_changes' OR p_plan ? 'quest_children' OR p_plan ? 'quest_noops' THEN
 RAISE EXCEPTION 'Ordinary source cannot invent Quest authority' USING ERRCODE='23514';
 END IF;
 RETURN digest;
END $$;
REVOKE ALL ON FUNCTION game_npc_arm_source(UUID,NUMERIC,BYTEA,JSONB,BYTEA,JSONB,JSONB)
 FROM PUBLIC,oteryn_game_control;
GRANT EXECUTE ON FUNCTION game_npc_arm_source(UUID,NUMERIC,BYTEA,JSONB,BYTEA,JSONB,JSONB)
 TO oteryn_game_runtime;
DO $path$ BEGIN
 EXECUTE format('ALTER FUNCTION game_npc_arm_source(uuid,numeric,bytea,jsonb,bytea,jsonb,jsonb) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $path$;
