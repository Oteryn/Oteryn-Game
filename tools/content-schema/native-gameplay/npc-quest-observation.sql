-- Source-qualified: preserve raw tracks for old source arm; derive only two chest-backed
-- source values independently, with no stored acknowledgement fabricated.
CREATE FUNCTION game_npc_quest_observed_tracks(p_character UUID,p_source TEXT,p_before JSONB)
 RETURNS JSONB LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE values_actual JSONB:='{}';claims_actual JSONB:='{}';key TEXT;projected RECORD;
BEGIN
 IF p_source IN('oteryn:npc-cosmetic/avar_tar/demon-base','oteryn:npc-cosmetic/avar_tar/demon-shield','oteryn:npc-cosmetic/avar_tar/demon-helmet') THEN
  FOREACH key IN ARRAY ARRAY[
   'oteryn:quest-progress/quest/u7_24/the_annihilator/reward',
   'oteryn:quest-progress/quest/u6_4/demon_helmet/rewards/demon_helmet'] LOOP
   SELECT * INTO projected FROM game_npc_final_claim_projection(p_character,key);
   IF NOT FOUND THEN RAISE EXCEPTION 'Missing closed chest projection' USING ERRCODE='23514';END IF;
   values_actual:=values_actual||jsonb_build_object(key,projected.source_value);
   IF projected.claim_transaction_id IS NOT NULL THEN
    claims_actual:=claims_actual||jsonb_build_object(key,projected.claim_transaction_id::text);
   END IF;
  END LOOP;
 END IF;
 IF p_before->'quest_source_values' IS DISTINCT FROM values_actual
  OR p_before->'quest_claims' IS DISTINCT FROM claims_actual THEN
 RAISE EXCEPTION 'Final source observation lacks actual claim ancestry' USING ERRCODE='23514';END IF;
 RETURN (p_before->'tracks')||values_actual;
END $$;
REVOKE ALL ON FUNCTION game_npc_quest_observed_tracks(UUID,TEXT,JSONB) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_npc_quest_observed_tracks(UUID,TEXT,JSONB) TO oteryn_game_runtime;
DO $path$ BEGIN
 EXECUTE format('ALTER FUNCTION game_npc_quest_observed_tracks(uuid,text,jsonb) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $path$;
