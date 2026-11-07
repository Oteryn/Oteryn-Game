-- Extend main's per-command MINT identity for source-bound NPCFIX repeats.
-- Main's paid-exchange arm and hash bytes remain unchanged. Every additional
-- occurrence is still proven by the reward completion, fee and clock owners.
DO $migration$
DECLARE body TEXT; original TEXT := 'CASE WHEN jsonb_array_length(v_plan -> ''storage''::text) = 0';
BEGIN
    body := pg_get_functiondef('game_quest_reward_mint_claim_key(uuid,uuid,numeric,text,jsonb)'::regprocedure);
    -- PostgreSQL deparses SQL function bodies verbatim on supported versions;
    -- replace only the complete CASE predicate, never the shared proof function.
    IF position('CASE WHEN jsonb_array_length(v_plan->''storage'')=0' IN body)>0 THEN
        original := 'CASE WHEN jsonb_array_length(v_plan->''storage'')=0';
    END IF;
    IF position(original IN body)=0 THEN
        RAISE EXCEPTION 'missing main quest exchange identity predecessor';
    END IF;
    body := replace(body,original,
        'CASE WHEN (coalesce((v_plan->>''repeatable'')::boolean,false) '
        ||'AND jsonb_typeof(v_plan->''give'')=''object'' '
        ||'AND (jsonb_array_length(v_plan->''take'')>0 '
        ||'OR jsonb_typeof(v_plan->''fee'')=''object'' '
        ||'OR jsonb_typeof(v_plan->''cooldown'')=''object'')) OR jsonb_array_length(v_plan->''storage'')=0');
    EXECUTE body;
    EXECUTE format('ALTER FUNCTION game_quest_reward_mint_claim_key(UUID,UUID,NUMERIC,TEXT,JSONB) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $migration$;
