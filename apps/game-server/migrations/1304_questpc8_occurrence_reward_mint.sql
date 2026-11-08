-- QUESTPC8: reuse the shared unambiguous occurrence key for explicitly recurring
-- portable rewards. Every occurrence still uses the original one-shot MINT owner.
CREATE OR REPLACE FUNCTION game_quest_reward_mint_claim_key(v_character UUID,v_cause UUID,v_ordinal NUMERIC,
    v_transition TEXT,v_plan JSONB) RETURNS TEXT LANGUAGE SQL IMMUTABLE AS $function$
    SELECT CASE WHEN v_plan->'per_occurrence_mint'='true'::jsonb OR (
        jsonb_array_length(v_plan->'storage')=0
        AND jsonb_array_length(coalesce(v_plan->'state_ops','[]'::jsonb))=0
        AND jsonb_array_length(coalesce(v_plan->'participants','[]'::jsonb))=0
        AND jsonb_array_length(v_plan->'take')>0 AND jsonb_typeof(v_plan->'give')='object')
    THEN 'oteryn:quest-exchange/'||encode(sha256(
        convert_to('oteryn:quest-exchange:v1','UTF8')||decode('00','hex')
        ||uuid_send(v_character)||uuid_send(v_cause)
        ||convert_to(v_ordinal::text||':'||v_transition,'UTF8')),'hex')
    ELSE v_transition END
$function$;

CREATE FUNCTION game_quest_reward_occurrence_effects_valid(v_plan JSONB,v_keys TEXT[],v_before BIGINT[],v_after BIGINT[])
RETURNS BOOLEAN LANGUAGE SQL STABLE AS $function$
    SELECT coalesce(jsonb_typeof(v_plan)='object'
        AND v_plan->'per_occurrence_mint'='true'::jsonb
        AND jsonb_typeof(v_plan->'give')='object'
        AND coalesce(v_plan->'outfit','null'::jsonb)='null'::jsonb
        AND coalesce(v_plan->'outfits','[]'::jsonb)='[]'::jsonb
        AND coalesce(v_plan->'achievement','null'::jsonb)='null'::jsonb
        AND v_before IS DISTINCT FROM v_after
        AND game_quest_reward_effects_valid(v_plan,v_keys,v_before,v_after),false)
$function$;

CREATE FUNCTION game_quest_reward_occurrence_complete() RETURNS trigger LANGUAGE plpgsql AS $function$
DECLARE receipt game_character_quest_receipts%ROWTYPE;
BEGIN
    IF NOT (NEW.plan ? 'per_occurrence_mint') THEN RETURN NULL; END IF;
    IF jsonb_typeof(NEW.plan->'per_occurrence_mint') IS DISTINCT FROM 'boolean' THEN
        RAISE EXCEPTION 'quest MINT occurrence option must be boolean' USING ERRCODE='23514';
    END IF;
    IF NEW.plan->'per_occurrence_mint'='false'::jsonb THEN RETURN NULL; END IF;
    SELECT * INTO receipt FROM game_character_quest_receipts
        WHERE character_id=NEW.character_id AND cause_id=NEW.cause_id
            AND cause_ordinal=NEW.cause_ordinal AND transition_key=NEW.transition_key;
    IF NOT FOUND OR receipt.cause_kind<>'command'
        OR receipt.quest_key IS DISTINCT FROM NEW.plan->>'quest'
        OR receipt.created_xact_id<>pg_current_xact_id()
        OR NEW.created_xact_id<>pg_current_xact_id() OR NEW.mint_transaction_id IS NULL
        OR NOT game_quest_reward_occurrence_effects_valid(NEW.plan,receipt.track_keys,receipt.values_before,receipt.values_after) THEN
        RAISE EXCEPTION 'quest MINT occurrence lacks exact changed-state receipt' USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END
$function$;
CREATE CONSTRAINT TRIGGER game_quest_reward_occurrence_complete AFTER INSERT ON game_quest_reward_sources
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_quest_reward_occurrence_complete();
DO $migration$
DECLARE function_name TEXT;
BEGIN
    FOREACH function_name IN ARRAY ARRAY[
        'game_quest_reward_mint_claim_key(uuid,uuid,numeric,text,jsonb)',
        'game_quest_reward_occurrence_effects_valid(jsonb,text[],bigint[],bigint[])',
        'game_quest_reward_occurrence_complete()'] LOOP
        EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',function_name,current_schema());
        EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC',function_name);
    END LOOP;
END
$migration$;
GRANT EXECUTE ON FUNCTION game_quest_reward_mint_claim_key(UUID,UUID,NUMERIC,TEXT,JSONB) TO oteryn_game_runtime;
GRANT EXECUTE ON FUNCTION game_quest_reward_occurrence_effects_valid(JSONB,TEXT[],BIGINT[],BIGINT[]) TO oteryn_game_runtime,oteryn_game_control;
