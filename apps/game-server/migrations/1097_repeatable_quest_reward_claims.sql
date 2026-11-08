-- Paid repeatable exchanges keep the existing once-only MINT owner: each exact
-- CommandRef gets its own claim, coupled to the original source and payment.
DO $migration$
DECLARE body TEXT;
    original TEXT := 'm.claim_production_key=NEW.transition_key AND m.created_xact_id=pg_current_xact_id()';
BEGIN
    SELECT pg_get_functiondef('game_quest_reward_complete()'::regprocedure) INTO STRICT body;
    IF position(original IN body)=0 THEN
        RAISE EXCEPTION 'repeatable quest MINT requires exact source proof anchor';
    END IF;
    body := replace(body, original, $proof$
        m.claim_production_key=CASE WHEN NEW.plan->>'repeatable'='true'
            THEN NEW.transition_key||':command:'||replace(NEW.cause_id::text,'-','')||':'||NEW.cause_ordinal::text
            ELSE NEW.transition_key END
        AND m.created_xact_id=pg_current_xact_id()
    $proof$);
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
