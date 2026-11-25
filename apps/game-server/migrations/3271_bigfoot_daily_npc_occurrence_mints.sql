-- Keep the existing NPC MINT proof and give the authored Bigfoot daily
-- supplies and rewards the same command-bound identity as repeatable quest exchanges.
CREATE FUNCTION game_quest_npc_reward_claim_key(
    p_plan JSONB, p_transition TEXT, p_character UUID, p_cause UUID, p_ordinal NUMERIC
) RETURNS TEXT LANGUAGE SQL IMMUTABLE AS $function$
    SELECT CASE WHEN p_plan->>'quest'='oteryn:quest.bigfoot_s_burden_quest' AND (
        p_transition ~ '^oteryn:quest-npc/bigfoot_s_burden_quest/commander_stone/branch-(73-4-1-1|110-11-1-1|182-25-1-1|(81-6-1-1|118-13-1-1|154-20-1-1|195-27-1-1)(-rank-(26|27|28|30))?)$'
        OR p_transition ~ '^oteryn:quest-npc/bigfoot_s_burden_quest/gnomeral/branch-(77-4-1-1|119-11-1-1|161-18-1-1|(90-6-1-1|132-13-1-1|174-20-1-1|214-27-1-1)(-rank-(26|27|28|30))?)$'
    ) THEN game_quest_reward_mint_claim_key(
        p_character, p_cause, p_ordinal, p_transition, '{"per_occurrence_mint":true}'::JSONB
    ) ELSE game_quest_npc_reward_claim_key(p_plan,p_transition) END
$function$;

DO $migration$
DECLARE
    body TEXT := pg_get_functiondef('game_quest_npc_item_complete()'::regprocedure);
    old_key TEXT := 'game_quest_npc_reward_claim_key(NEW.plan,NEW.transition_key)';
    new_key TEXT := 'game_quest_npc_reward_claim_key(NEW.plan,NEW.transition_key,NEW.character_id,NEW.cause_id,NEW.cause_ordinal)';
BEGIN
    IF length(body)-length(replace(body,old_key,'')) <> length(old_key) THEN
        RAISE EXCEPTION 'unexpected NPC reward claim identity proof';
    END IF;
    EXECUTE replace(body,old_key,new_key);
    EXECUTE format(
        'ALTER FUNCTION game_quest_npc_reward_claim_key(JSONB,TEXT,UUID,UUID,NUMERIC) SET search_path=pg_catalog,%I,pg_temp',
        current_schema()
    );
END $migration$;

REVOKE ALL ON FUNCTION game_quest_npc_reward_claim_key(JSONB,TEXT,UUID,UUID,NUMERIC) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_quest_npc_reward_claim_key(JSONB,TEXT,UUID,UUID,NUMERIC) TO oteryn_game_runtime;
