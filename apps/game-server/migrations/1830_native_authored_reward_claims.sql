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
    carried_proof TEXT := $carried$    FOR expected IN SELECT value FROM jsonb_array_elements(NEW.plan->'take') LOOP
        SELECT coalesce(sum(c.quantity),0) INTO actual FROM game_quest_callback_consumptions c
            WHERE c.character_id=NEW.character_id AND c.cause_id=NEW.cause_id
                AND c.cause_ordinal=NEW.cause_ordinal AND c.transition_key=NEW.transition_key
                AND c.before_item->>'definition_production_key'=expected->>'item'
                AND c.location_kind IN('container','nested','backpack','equipment')
                AND c.replacement IS NULL AND c.created_xact_id=pg_current_xact_id();
        IF actual IS DISTINCT FROM (expected->>'count')::bigint THEN
            RAISE EXCEPTION 'quest reward must consume exact carried item count' USING ERRCODE='23514';
        END IF;
    END LOOP;$carried$;
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
    -- Retain actual for main 2709, which later adds its Ground-offering loop.
    IF (length(body)-length(replace(body,'actual BIGINT','')))/length('actual BIGINT')<>1
        OR (length(body)-length(replace(body,carried_proof,'')))/length(carried_proof)<>1 THEN
        RAISE EXCEPTION 'native claim requires the exact carried input proof block';
    END IF;
    body:=replace(body,'actual BIGINT','actual BIGINT; native_consumed_count BIGINT');
    -- Main2709 may already have installed a separate Ground counter. Rename
    -- only this complete carried block;2020 still requires one native triple.
    body:=replace(body,carried_proof,
        replace(replace(carried_proof,'INTO actual FROM','INTO native_consumed_count FROM'),
            'IF actual IS DISTINCT','IF native_consumed_count IS DISTINCT'));
    -- The receipt proof is the command-or-world-callback arm. The repeatable
    -- counter proof also names r.cause_kind='command' and stays command-only.
    anchor:='(r.cause_kind=''command'' OR (r.cause_kind=''use''';
    IF (length(body)-length(replace(body,anchor,'')))/length(anchor)<>1 THEN
        RAISE EXCEPTION 'native claim requires the exact quest receipt cause proof'; END IF;
    body:=replace(body,anchor,$proof$((r.cause_kind='command' AND NOT NEW.plan ? 'world_claim')
        OR (r.cause_kind='use' AND game_quest_native_claim_plan(NEW.plan)) OR (r.cause_kind='use'$proof$);
    anchor:='m.claim_production_key=game_quest_reward_mint_claim_key(NEW.character_id,NEW.cause_id,NEW.cause_ordinal,NEW.transition_key,NEW.plan)';
    IF (length(body)-length(replace(body,anchor,'')))/length(anchor)<>1 THEN
        RAISE EXCEPTION 'native claim requires the exact existing native MINT proof'; END IF;
    replacement:=anchor || $proof$ AND (NOT NEW.plan ? 'world_claim' OR
        (game_quest_native_claim_plan(NEW.plan) AND m.claim_family='RewardClaim'
         AND m.claim_revision_ref=NEW.plan #>> '{world_claim,revision}'))$proof$;
    EXECUTE replace(body,anchor,replacement);
END $migration$;
