-- M34: a physical authored claim composes the existing Quest and MINT proofs.
-- Ordinary NPC rewards keep their command cause and identity derivation.
CREATE FUNCTION game_quest_native_claim_plan(p JSONB) RETURNS BOOLEAN
LANGUAGE SQL IMMUTABLE SET search_path=pg_catalog AS $function$
 SELECT coalesce(
   jsonb_typeof(p->'world_claim')='object'
   AND (p->'world_claim')-ARRAY['key','revision','authority_sha256']::TEXT[]='{}'::jsonb
   AND jsonb_typeof(p #> '{world_claim,key}')='string'
   AND (p #>> '{world_claim,key}') ~ '^oteryn:reward-claim\.[a-z0-9_./-]+$'
   AND octet_length(p #>> '{world_claim,key}')<=128
   AND jsonb_typeof(p #> '{world_claim,revision}')='string'
   AND (p #>> '{world_claim,revision}') ~ '^[a-zA-Z0-9._/:-]{1,128}$'
   AND jsonb_typeof(p #> '{world_claim,authority_sha256}')='string'
   AND (p #>> '{world_claim,authority_sha256}') ~ '^[0-9a-f]{64}$'
   AND p->>'npc'='' AND p->'keywords'='[]'::jsonb
   AND p->'topic'='0'::jsonb AND p->'next_topic'='0'::jsonb
   AND jsonb_array_length(p->'storage')>0
   AND NOT p ? 'carried_item' AND NOT p ? 'fee',false)
$function$;
REVOKE ALL ON FUNCTION game_quest_native_claim_plan(JSONB) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_quest_native_claim_plan(JSONB) TO oteryn_game_runtime;

DO $migration$
DECLARE body TEXT; anchor TEXT; replacement TEXT;
BEGIN
    body:=pg_get_functiondef('game_quest_reward_mint_claim_key(UUID,UUID,NUMERIC,TEXT,JSONB)'::regprocedure);
    anchor:='SELECT CASE WHEN';
    IF (length(body)-length(replace(body,anchor,'')))/length(anchor)<>1 THEN
        RAISE EXCEPTION 'native claim requires the exact existing quest MINT key owner'; END IF;
    EXECUTE replace(body,anchor,
        'SELECT CASE WHEN game_quest_native_claim_plan(v_plan) THEN v_plan #>> ''{world_claim,key}'' WHEN');

    body:=pg_get_functiondef('game_quest_reward_complete()'::regprocedure);
    -- The later M09 sibling proof uses the SQL alias actual. Keep the carried
    -- input PL/pgSQL variable distinct, both before and after that migration.
    FOREACH anchor IN ARRAY ARRAY['actual BIGINT','INTO actual FROM','IF actual IS DISTINCT'] LOOP
        IF (length(body)-length(replace(body,anchor,'')))/length(anchor)<>1 THEN
            RAISE EXCEPTION 'native claim requires the exact carried input proof anchor: %',anchor; END IF;
    END LOOP;
    body:=replace(replace(replace(body,'actual BIGINT','native_consumed_count BIGINT'),
        'INTO actual FROM','INTO native_consumed_count FROM'),
        'IF actual IS DISTINCT','IF native_consumed_count IS DISTINCT');
    anchor:='r.cause_kind=''command''';
    IF (length(body)-length(replace(body,anchor,'')))/length(anchor)<>1 THEN
        RAISE EXCEPTION 'native claim requires the exact quest receipt cause proof'; END IF;
    body:=replace(body,anchor,$proof$((r.cause_kind='command' AND NOT NEW.plan ? 'world_claim')
        OR (r.cause_kind='use' AND game_quest_native_claim_plan(NEW.plan)))$proof$);
    anchor:='m.claim_production_key=game_quest_reward_mint_claim_key(NEW.character_id,NEW.cause_id,NEW.cause_ordinal,NEW.transition_key,NEW.plan)';
    IF (length(body)-length(replace(body,anchor,'')))/length(anchor)<>1 THEN
        RAISE EXCEPTION 'native claim requires the exact existing native MINT proof'; END IF;
    replacement:=anchor || $proof$ AND (NOT NEW.plan ? 'world_claim' OR
        (game_quest_native_claim_plan(NEW.plan) AND m.claim_family='RewardClaim'
         AND m.claim_revision_ref=NEW.plan #>> '{world_claim,revision}'))$proof$;
    EXECUTE replace(body,anchor,replacement);
END $migration$;
