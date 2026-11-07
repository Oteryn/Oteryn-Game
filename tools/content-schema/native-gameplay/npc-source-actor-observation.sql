-- Three registered Vescu Greet branches over a genuine native witness and
-- the complete Node-captured raw conditions. This is eligibility, not mutation.
ALTER FUNCTION game_npc_arm_source(UUID,NUMERIC,BYTEA,JSONB,BYTEA,JSONB,JSONB)
 RENAME TO game_npc_arm_source_before_actor_observation;
REVOKE ALL ON FUNCTION game_npc_arm_source_before_actor_observation(UUID,NUMERIC,BYTEA,JSONB,BYTEA,JSONB,JSONB)
 FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
CREATE FUNCTION game_npc_arm_source(p_node UUID,p_registration NUMERIC,p_secret BYTEA,
 p_context JSONB,p_program BYTEA,p_before JSONB,p_plan JSONB) RETURNS BYTEA
 LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE digest BYTEA;c game_npc_source_context%ROWTYPE;g JSONB;a JSONB;entry JSONB;
 drunk BOOLEAN;stage BIGINT;
BEGIN
 digest:=game_npc_arm_source_before_actor_observation(p_node,p_registration,p_secret,p_context,p_program,p_before,p_plan);
 SELECT * INTO c FROM game_npc_source_context WHERE backend_pid=pg_backend_pid() AND transaction_xid=pg_current_xact_id();
 g:=c.before_state->'greet';a:=c.before_state->'actor_observation';
 IF c.source_key NOT IN('oteryn:npc-cosmetic/vescu/assassin-returner-greet','oteryn:npc-cosmetic/vescu/assassin-drunken-beginner-greet','oteryn:npc-cosmetic/vescu/assassin-sober-refusal-greet') THEN
  IF a IS NOT NULL AND a<>'null'::jsonb THEN RAISE EXCEPTION 'Actor condition observation cannot authorize another source' USING ERRCODE='23514';END IF;
  RETURN digest;
 END IF;
 IF c.npc_key IS DISTINCT FROM 'oteryn:npc.vescu' OR jsonb_typeof(a) IS DISTINCT FROM 'object' OR jsonb_typeof(g) IS DISTINCT FROM 'object'
 OR g->>'game_session_id' IS DISTINCT FROM c.game_session_id::text OR g->>'connection_generation' IS DISTINCT FROM c.connection_generation::text
 OR g->>'command_id' IS DISTINCT FROM c.command_id::text
 OR coalesce(g->>'conversation_generation','')!~'^[1-9][0-9]{0,19}$' OR coalesce(g->>'npc_generation','')!~'^[1-9][0-9]{0,19}$'
 OR coalesce(g->>'npc_identity','')!~'^[0-9a-f]{32}$' OR g->>'npc_identity'=repeat('0',32)
 OR coalesce(g->>'catalogue_revision','')='' OR octet_length(g->>'catalogue_revision')>256
 OR coalesce(a->>'actor_identity','')!~'^[0-9a-f]{32}$' OR a->>'actor_identity'=repeat('0',32)
 OR a->>'actor_identity' IS DISTINCT FROM g->>'actor_identity' OR a->>'movement_revision' IS DISTINCT FROM g->>'movement_revision'
 OR coalesce(a->>'movement_revision','')!~'^[1-9][0-9]{0,19}$' OR coalesce(a->>'state_revision','')!~'^[1-9][0-9]{0,19}$'
 OR coalesce(a->>'observed_owner_us','')!~'^(0|[1-9][0-9]{0,19})$'
 OR jsonb_typeof(a->'player_conditions') IS DISTINCT FROM 'object' OR a->'player_conditions'->>'schema' IS DISTINCT FROM '1'
 OR jsonb_typeof(a->'player_statuses') IS DISTINCT FROM 'array' OR jsonb_array_length(a->'player_statuses')>64
 OR jsonb_typeof(a->'native_statuses') IS DISTINCT FROM 'array' OR jsonb_array_length(a->'native_statuses')>64
 OR octet_length(a::text)>65536 THEN RAISE EXCEPTION 'Vescu lacks exact current raw condition capture' USING ERRCODE='23514';END IF;
 FOR entry IN SELECT value FROM jsonb_array_elements((a->'player_statuses')||(a->'native_statuses')) LOOP
  IF jsonb_typeof(entry) IS DISTINCT FROM 'array' OR jsonb_array_length(entry)<>2
   OR coalesce(entry->>1,'')!~'^-?(0|[1-9][0-9]{0,18})$' OR (entry->>1)::numeric NOT BETWEEN -1 AND 9223372036854775807 THEN
  RAISE EXCEPTION 'Vescu raw condition duration malformed' USING ERRCODE='23514';END IF;
 END LOOP;
 SELECT EXISTS(SELECT 1 FROM jsonb_array_elements((a->'player_statuses')||(a->'native_statuses')) status
  WHERE status->0='{"Status":"Drunk"}'::jsonb AND (status->>1)::bigint<>0) INTO drunk;
 IF c.before_state->'actor_condition_drunk' IS DISTINCT FROM to_jsonb(drunk) THEN RAISE EXCEPTION 'Vescu condition result differs from raw capture' USING ERRCODE='23514';END IF;
 stage:=coalesce((c.before_state->'tracks'->>'oteryn:quest-progress/quest/u7_8/assassin_outfits/assassin_base_outfit')::bigint,-1);
 IF (c.source_key='oteryn:npc-cosmetic/vescu/assassin-returner-greet' AND stage<1)
 OR (c.source_key='oteryn:npc-cosmetic/vescu/assassin-drunken-beginner-greet' AND (stage>=1 OR NOT drunk))
 OR (c.source_key='oteryn:npc-cosmetic/vescu/assassin-sober-refusal-greet' AND (stage>=1 OR drunk)) THEN
 RAISE EXCEPTION 'Vescu greeting differs from actual stage and raw condition' USING ERRCODE='23514';END IF;
 RETURN digest;
END $$;
REVOKE ALL ON FUNCTION game_npc_arm_source(UUID,NUMERIC,BYTEA,JSONB,BYTEA,JSONB,JSONB) FROM PUBLIC,oteryn_game_control;
GRANT EXECUTE ON FUNCTION game_npc_arm_source(UUID,NUMERIC,BYTEA,JSONB,BYTEA,JSONB,JSONB) TO oteryn_game_runtime;
DO $path$ BEGIN
 EXECUTE format('ALTER FUNCTION game_npc_arm_source(uuid,numeric,bytea,jsonb,bytea,jsonb,jsonb) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $path$;
