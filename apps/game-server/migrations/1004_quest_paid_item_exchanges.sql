-- Each paid occurrence remains a one-shot claim in the existing MINT owner.
-- Fixed-width UUIDs and a colon-terminated decimal CommandId make the hash
-- unambiguous over the complete unsigned CommandId domain.
CREATE FUNCTION game_quest_reward_mint_claim_key(v_character UUID,v_cause UUID,v_ordinal NUMERIC,
    v_transition TEXT,v_plan JSONB) RETURNS TEXT LANGUAGE SQL IMMUTABLE AS $function$
    SELECT CASE WHEN jsonb_array_length(v_plan->'storage')=0
        AND jsonb_array_length(coalesce(v_plan->'state_ops','[]'::jsonb))=0
        AND jsonb_array_length(coalesce(v_plan->'participants','[]'::jsonb))=0
        AND jsonb_array_length(v_plan->'take')>0 AND jsonb_typeof(v_plan->'give')='object'
    THEN 'oteryn:quest-exchange/'||encode(sha256(
        convert_to('oteryn:quest-exchange:v1','UTF8')||decode('00','hex')
        ||uuid_send(v_character)||uuid_send(v_cause)
        ||convert_to(v_ordinal::text||':'||v_transition,'UTF8')),'hex')
    ELSE v_transition END
$function$;
DO $migration$
DECLARE v_definition TEXT; v_old TEXT:='m.claim_production_key=NEW.transition_key';
BEGIN
    SELECT pg_get_functiondef('game_quest_reward_complete()'::regprocedure) INTO v_definition;
    IF (length(v_definition)-length(replace(v_definition,v_old,'')))/length(v_old)<>1 THEN
        RAISE EXCEPTION 'quest MINT completion proof shape changed'; END IF;
    EXECUTE replace(v_definition,v_old,
        'm.claim_production_key=game_quest_reward_mint_claim_key(NEW.character_id,NEW.cause_id,NEW.cause_ordinal,NEW.transition_key,NEW.plan)');
    EXECUTE format('ALTER FUNCTION game_quest_reward_mint_claim_key(UUID,UUID,NUMERIC,TEXT,JSONB) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $migration$;
REVOKE ALL ON FUNCTION game_quest_reward_mint_claim_key(UUID,UUID,NUMERIC,TEXT,JSONB) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_quest_reward_mint_claim_key(UUID,UUID,NUMERIC,TEXT,JSONB) TO oteryn_game_runtime;

-- Reuse exact same-R quest receipts, consumption proofs and native MINT for
-- repeatable source NPC exchanges. Empty gifts and free grants remain refused.
CREATE OR REPLACE FUNCTION game_quest_reward_value_only_complete() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE v_source game_quest_reward_sources%ROWTYPE; v_targets JSONB;
BEGIN
    IF cardinality(NEW.track_keys)<>0 OR NEW.completes THEN RETURN NULL; END IF;
    SELECT * INTO v_source FROM game_quest_reward_sources WHERE character_id=NEW.character_id
        AND cause_id=NEW.cause_id AND cause_ordinal=NEW.cause_ordinal AND transition_key=NEW.transition_key;
    IF NOT FOUND OR NEW.cause_kind<>'command' OR NEW.created_xact_id<>pg_current_xact_id()
        OR v_source.created_xact_id<>pg_current_xact_id() OR v_source.plan->>'quest' IS DISTINCT FROM NEW.quest_key
        OR jsonb_array_length(v_source.plan->'storage')<>0
        OR jsonb_array_length(coalesce(v_source.plan->'state_ops','[]'::jsonb))<>0 THEN
        RAISE EXCEPTION 'value-only quest lacks exact same-transaction source' USING ERRCODE='23514'; END IF;
    -- The source completion trigger independently proves every exact carried
    -- consumption and the exact native backpack MINT in this same transaction.
    -- Positive costs make repeated source trades legitimate without fake flags.
    IF jsonb_typeof(v_source.plan->'give')='object'
        AND (v_source.plan #>> '{give,count}')::bigint>0
        AND jsonb_array_length(v_source.plan->'take') BETWEEN 1 AND 16
        AND NOT EXISTS(SELECT 1 FROM jsonb_array_elements(v_source.plan->'take') e
            WHERE (e->>'count') IS NULL OR (e->>'count')::bigint<=0)
        AND v_source.mint_transaction_id IS NOT NULL THEN
        RETURN NULL;
    END IF;
    IF cardinality(game_quest_reward_grant_keys(v_source.plan))=0 THEN
        RAISE EXCEPTION 'value-only quest lacks paid MINT or one-shot grant' USING ERRCODE='23514'; END IF;
    v_targets:=CASE WHEN v_source.plan->'outfit'<>'null'::jsonb THEN jsonb_build_array(v_source.plan->'outfit')
        ELSE coalesce(v_source.plan->'outfits','[]'::jsonb) END;
    IF NOT EXISTS(SELECT 1 FROM jsonb_array_elements(v_targets) t
        CROSS JOIN jsonb_array_elements(coalesce(v_source.plan->'forbids_addons','[]'::jsonb)) g
        WHERE t->>'outfit'=g->>'outfit' AND ((t->>'addons')::integer & (g->>'addons')::integer)<>0
            AND (t->>'sex' IS NULL OR t->>'sex'=coalesce(g->>'sex',v_source.plan->>'creation_sex'))) THEN
        RAISE EXCEPTION 'value-only quest lacks one-shot earned predicate' USING ERRCODE='23514'; END IF;
    RETURN NULL;
END $$;
DO $migration$
BEGIN
    EXECUTE format('ALTER FUNCTION game_quest_reward_value_only_complete() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $migration$;
REVOKE ALL ON FUNCTION game_quest_reward_value_only_complete() FROM PUBLIC;
