-- Only these registered callbacks may consume a zero-gameplay greeting.
CREATE FUNCTION game_npc_source_readonly_greet_qualified(c game_npc_source_context)
RETURNS BOOLEAN LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE g JSONB;stage BIGINT;expected_npc TEXT;expected_effects JSONB;
BEGIN
 expected_npc:=CASE
 WHEN c.source_key IN('oteryn:npc-cosmetic/vescu/assassin-returner-greet','oteryn:npc-cosmetic/vescu/assassin-drunken-beginner-greet','oteryn:npc-cosmetic/vescu/assassin-sober-refusal-greet') THEN 'oteryn:npc.vescu'
 WHEN c.source_key IN('oteryn:npc-cosmetic/bron/barbarian-remorse-greet','oteryn:npc-cosmetic/bron/barbarian-default-greet') THEN 'oteryn:npc.bron'
 WHEN c.source_key IN('oteryn:npc-cosmetic/a_majestic_warwolf/druid-unqualified-refusal-greet','oteryn:npc-cosmetic/a_majestic_warwolf/druid-qualified-greet') THEN 'oteryn:npc.a_majestic_warwolf'
 WHEN c.source_key IN('oteryn:npc-cosmetic/bozo/jester-male-greet','oteryn:npc-cosmetic/bozo/jester-female-greet') THEN 'oteryn:npc.bozo' ELSE NULL END;
 IF expected_npc IS NULL OR c.backend_pid IS DISTINCT FROM pg_backend_pid() OR c.transaction_xid IS DISTINCT FROM pg_current_xact_id()
 OR c.npc_key IS DISTINCT FROM expected_npc OR c.captured OR c.cost_transaction_id IS NOT NULL OR c.fee_transaction_id IS NOT NULL
 OR game_node_lock_current_registration(c.node_id,c.registration) IS DISTINCT FROM TRUE THEN RETURN FALSE;END IF;
 g:=c.before_state->'greet';
 IF jsonb_typeof(g) IS DISTINCT FROM 'object'
 OR g->>'game_session_id' IS DISTINCT FROM c.game_session_id::text OR g->>'connection_generation' IS DISTINCT FROM c.connection_generation::text
 OR g->>'command_id' IS DISTINCT FROM c.command_id::text
 OR coalesce(g->>'conversation_generation','')!~'^[1-9][0-9]{0,19}$' OR coalesce(g->>'npc_generation','')!~'^[1-9][0-9]{0,19}$'
 OR coalesce(g->>'actor_identity','')!~'^[0-9a-f]{32}$' OR g->>'actor_identity'=repeat('0',32)
 OR coalesce(g->>'npc_identity','')!~'^[0-9a-f]{32}$' OR g->>'npc_identity'=repeat('0',32)
 OR coalesce(g->>'movement_revision','')!~'^[1-9][0-9]{0,19}$'
 OR coalesce(octet_length(g->>'catalogue_revision'),0) NOT BETWEEN 1 AND 256 OR (g->>'catalogue_revision')~'[[:cntrl:]]'
 THEN RETURN FALSE;END IF;
 expected_effects:=CASE WHEN c.source_key IN('oteryn:npc-cosmetic/vescu/assassin-sober-refusal-greet','oteryn:npc-cosmetic/a_majestic_warwolf/druid-unqualified-refusal-greet') THEN '[{"kind":"close_conversation"}]'::jsonb ELSE '[]'::jsonb END;
 IF c.expected_plan->'effects' IS DISTINCT FROM expected_effects OR c.expected_plan->'changes' IS DISTINCT FROM '[]'::jsonb
 OR c.expected_plan->'costs' IS DISTINCT FROM '[]'::jsonb OR c.expected_plan->'grant_keys' IS DISTINCT FROM '[]'::jsonb
 OR c.expected_plan->'outcome'->'committed' IS DISTINCT FROM 'false'::jsonb THEN RETURN FALSE;END IF;
 IF expected_npc='oteryn:npc.bron' THEN
  stage:=coalesce((c.before_state->'tracks'->>'oteryn:quest-progress/quest/u7_8/barbarian_outfits/barbarian_addon')::bigint,-1);
  RETURN (c.source_key='oteryn:npc-cosmetic/bron/barbarian-remorse-greet' AND stage=6)
   OR(c.source_key='oteryn:npc-cosmetic/bron/barbarian-default-greet' AND stage<>6);
 ELSIF expected_npc='oteryn:npc.a_majestic_warwolf' THEN
  stage:=coalesce((c.before_state->'tracks'->>'oteryn:quest-progress/quest/u7_8/druid_outfits/druid_hat_addon')::bigint,-1);
  RETURN (c.source_key='oteryn:npc-cosmetic/a_majestic_warwolf/druid-unqualified-refusal-greet' AND stage<9)
   OR(c.source_key='oteryn:npc-cosmetic/a_majestic_warwolf/druid-qualified-greet' AND stage>=9);
 ELSIF expected_npc='oteryn:npc.bozo' THEN
  RETURN(c.source_key='oteryn:npc-cosmetic/bozo/jester-male-greet' AND c.before_state->>'sex'='male')
   OR(c.source_key='oteryn:npc-cosmetic/bozo/jester-female-greet' AND c.before_state->>'sex'='female');
 END IF;
 -- Vescu raw-condition/branch validation already runs in the original private
 -- arm before this context exists; its exact source identity stays mandatory.
 RETURN jsonb_typeof(c.before_state->'actor_observation')='object';
END $$;
REVOKE ALL ON FUNCTION game_npc_source_readonly_greet_qualified(game_npc_source_context)
 FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
DO $path$ BEGIN EXECUTE format('ALTER FUNCTION game_npc_source_readonly_greet_qualified(game_npc_source_context) SET search_path=pg_catalog,%I,pg_temp',current_schema());END $path$;
