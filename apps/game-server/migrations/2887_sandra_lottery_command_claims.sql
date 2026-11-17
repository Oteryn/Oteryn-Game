-- Sandra's repeatable exchanges retain the existing atomic NPC proof. Only
-- their MINT identity gains the already-fenced session and command occurrence.
CREATE FUNCTION game_quest_npc_reward_claim_key(
    p_plan JSONB, p_transition TEXT, p_cause_id UUID, p_cause_ordinal NUMERIC
) RETURNS TEXT LANGUAGE plpgsql IMMUTABLE AS $$
BEGIN
    IF p_transition = ANY(ARRAY[
        'oteryn:quest-npc/sandra/deposit-283',
        'oteryn:quest-npc/sandra/deposit-284',
        'oteryn:quest-npc/sandra/deposit-285',
        'oteryn:quest-npc/sandra/lottery'
    ]) THEN
        IF p_plan->>'npc' IS DISTINCT FROM 'oteryn:npc.sandra'
           OR p_plan->>'quest' IS DISTINCT FROM 'oteryn:quest.outfit_and_addon_quests'
           OR p_cause_id IS NULL OR p_cause_ordinal IS NULL
           OR p_cause_ordinal < 0 OR p_cause_ordinal > 18446744073709551615
           OR p_cause_ordinal <> trunc(p_cause_ordinal) THEN
            RAISE EXCEPTION 'Sandra exchange requires its exact source and command'
                USING ERRCODE='23514';
        END IF;
        RETURN 'oteryn:reward.sandra-lottery.'
            || replace(p_cause_id::TEXT,'-','') || '.' || trunc(p_cause_ordinal)::TEXT;
    END IF;
    RETURN game_quest_npc_reward_claim_key(p_plan,p_transition);
END $$;

DO $migration$
DECLARE
    definition TEXT;
    original_call TEXT := 'game_quest_npc_reward_claim_key(NEW.plan,NEW.transition_key)';
    occurrence_call TEXT := 'game_quest_npc_reward_claim_key(NEW.plan,NEW.transition_key,NEW.cause_id,NEW.cause_ordinal)';
BEGIN
    EXECUTE format('ALTER FUNCTION game_quest_npc_reward_claim_key(JSONB,TEXT,UUID,NUMERIC) SET search_path=pg_catalog,%I,pg_temp',current_schema());
    definition := pg_get_functiondef('game_quest_npc_item_complete()'::regprocedure);
    IF length(definition)-length(replace(definition,original_call,'')) <> length(original_call) THEN
        RAISE EXCEPTION 'expected one independent NPC claim identity proof';
    END IF;
    -- Preserve every existing source, quest, consumption, sibling-output and
    -- same-transaction MINT predicate; bind only the claim-key call to its cause.
    EXECUTE replace(definition,original_call,occurrence_call);
END $migration$;

REVOKE ALL ON FUNCTION game_quest_npc_reward_claim_key(JSONB,TEXT,UUID,NUMERIC) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_quest_npc_reward_claim_key(JSONB,TEXT,UUID,NUMERIC) TO oteryn_game_runtime;
