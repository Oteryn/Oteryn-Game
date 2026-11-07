-- L2 explicit stateful repeats extend canonical QENGINE2 paid exchanges.
-- Keep every target-set, source receipt, MINT, input and earned-value proof.
DO $migration$
DECLARE body TEXT; anchor TEXT:=$old$FOR expected IN SELECT value FROM jsonb_array_elements(NEW.plan->'take') LOOP$old$;
BEGIN
    body:=pg_get_functiondef('game_quest_reward_complete()'::regprocedure);
    IF (length(body)-length(replace(body,anchor,'')))/length(anchor)<>1 THEN
        RAISE EXCEPTION 'quest repeat completion anchor changed'; END IF;
    EXECUTE replace(body,anchor,$new$
    IF NEW.plan ? 'repeatable' AND jsonb_typeof(NEW.plan->'repeatable') IS DISTINCT FROM 'boolean' THEN
        RAISE EXCEPTION 'quest repeat flag must be boolean' USING ERRCODE='23514'; END IF;
    IF coalesce((NEW.plan->>'repeatable')::boolean,false) AND (
        (jsonb_array_length(NEW.plan->'take')=0 AND NOT EXISTS(
            SELECT 1 FROM game_quest_reward_fees f WHERE f.character_id=NEW.character_id
                AND f.cause_id=NEW.cause_id AND f.cause_ordinal=NEW.cause_ordinal
                AND f.transition_key=NEW.transition_key AND f.fee_gold_units>0
                AND f.created_xact_id=pg_current_xact_id()))
        OR jsonb_typeof(NEW.plan->'give') IS DISTINCT FROM 'object'
        OR NEW.plan->'outfit' IS DISTINCT FROM 'null'::jsonb
        OR jsonb_array_length(coalesce(NEW.plan->'outfits','[]'::jsonb))<>0
        OR NEW.plan->'achievement' IS DISTINCT FROM 'null'::jsonb) THEN
        RAISE EXCEPTION 'quest repeat must be a costed item exchange' USING ERRCODE='23514'; END IF;
    FOR expected IN SELECT value FROM jsonb_array_elements(NEW.plan->'take') LOOP$new$);
    body:=pg_get_functiondef('game_quest_reward_mint_claim_key(uuid,uuid,numeric,text,jsonb)'::regprocedure);
    anchor:='SELECT CASE WHEN jsonb_array_length';
    IF position(anchor IN body)=0 OR position('ELSE v_transition END' IN body)=0 THEN
        RAISE EXCEPTION 'quest repeat claim occurrence anchor changed'; END IF;
    body:=replace(body,anchor,$new$SELECT CASE WHEN coalesce((v_plan->>'repeatable')::boolean,false)
        THEN v_transition||'/occurrence/'||replace(v_cause::text,'-','')||'/'||v_ordinal::text
        ELSE CASE WHEN jsonb_array_length$new$);
    EXECUTE replace(body,'ELSE v_transition END','ELSE v_transition END END');
END $migration$;
