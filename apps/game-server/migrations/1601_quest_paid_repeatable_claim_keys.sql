-- Main's 1550 replaces the shared helper after the package's 1097/1286
-- extensions. Restore those claim identities around the final main body;
-- retain its timed world-use, per-occurrence and carried-exchange branches.
DO $migration$
DECLARE
    body TEXT;
    anchor TEXT := $anchor$SELECT CASE WHEN v_plan->'per_occurrence_mint'='true'::jsonb OR ($anchor$;
    replacement TEXT := $replacement$SELECT CASE WHEN v_plan ? 'counter' THEN
        v_transition || '/occurrence/' || replace(v_cause::text,'-','') || '/' || v_ordinal::text
    WHEN (
        v_plan->'repeatable'='true'::jsonb
        AND jsonb_typeof(v_plan->'give')='object'
        AND (v_plan #>> '{fee,gold}')::numeric BETWEEN 1 AND 1000000000000
        AND NOT coalesce((v_plan->>'completes')::boolean,false)
        AND jsonb_array_length(coalesce(v_plan->'state_ops','[]'::jsonb))=0
        AND jsonb_array_length(coalesce(v_plan->'participants','[]'::jsonb))=0
        AND NOT EXISTS(SELECT 1 FROM jsonb_array_elements(coalesce(v_plan->'storage','[]'::jsonb)) s
            WHERE s->>'preserve' IS DISTINCT FROM 'true')
    ) OR v_plan->'per_occurrence_mint'='true'::jsonb OR ($replacement$;
BEGIN
    SELECT pg_get_functiondef('game_quest_reward_mint_claim_key(uuid,uuid,numeric,text,jsonb)'::regprocedure)
        INTO STRICT body;
    IF (length(body)-length(replace(body,anchor,'')))/length(anchor)<>1
        OR (length(body)-length(replace(body,'oteryn:quest-exchange:v1','')))/length('oteryn:quest-exchange:v1')<>1
        OR position('v_transition || ''/occurrence/''' IN body)<>0 THEN
        RAISE EXCEPTION 'paid repeatable claims require the exact final main claim-key helper';
    END IF;
    -- Only the pure identity derivation changes. All deferred source, bounded
    -- counter, same-xid MINT and exact fee proofs continue to run independently.
    EXECUTE replace(body,anchor,replacement);
END $migration$;
