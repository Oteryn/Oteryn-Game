-- Paid repeatable exchanges keep the existing once-only MINT owner: each exact
-- CommandRef gets its own claim, coupled to the original source and payment.
DO $migration$
DECLARE body TEXT; helper TEXT;
    original TEXT := 'm.claim_production_key=game_quest_reward_mint_claim_key(NEW.character_id,NEW.cause_id,NEW.cause_ordinal,NEW.transition_key,NEW.plan) AND m.created_xact_id=pg_current_xact_id()';
    hash_anchor TEXT := 'THEN ''oteryn:quest-exchange/''';
BEGIN
    SELECT pg_get_functiondef('game_quest_reward_complete()'::regprocedure) INTO STRICT body;
    IF (length(body)-length(replace(body,original,'')))/length(original)<>1
        OR (length(body)-length(replace(body,'RETURN NULL;','')))/length('RETURN NULL;')<>1 THEN
        RAISE EXCEPTION 'repeatable quest MINT requires exact source proof anchor';
    END IF;
    -- Main's 1004 already owns the source-bound claim derivation. Extend its
    -- exact paid-exchange hash to fee-paid repeatable rewards, matching the
    -- native MINT caller; retain ordinary exchanges and fixed claims unchanged.
    SELECT pg_get_functiondef('game_quest_reward_mint_claim_key(uuid,uuid,numeric,text,jsonb)'::regprocedure)
        INTO STRICT helper;
    IF (length(helper)-length(replace(helper,hash_anchor,'')))/length(hash_anchor)<>1 THEN
        RAISE EXCEPTION 'repeatable quest MINT requires exact main claim-key helper';
    END IF;
    EXECUTE replace(helper,hash_anchor,'OR v_plan->>''repeatable''=''true'' '||hash_anchor);
    EXECUTE replace(body, 'RETURN NULL;', $guard$
        IF NEW.plan->>'repeatable'='true' AND (
            NEW.mint_transaction_id IS NULL
            OR coalesce((NEW.plan->>'completes')::boolean,false)
            OR NOT EXISTS(SELECT 1 FROM game_quest_reward_fees q
                WHERE q.character_id=NEW.character_id AND q.cause_id=NEW.cause_id
                    AND q.cause_ordinal=NEW.cause_ordinal AND q.transition_key=NEW.transition_key
                    AND q.fee_gold_units>0 AND q.created_xact_id=pg_current_xact_id())
        ) THEN
            RAISE EXCEPTION 'repeatable quest MINT lacks its exact atomic payment' USING ERRCODE='23514';
        END IF;
        RETURN NULL;
    $guard$);
END $migration$;
