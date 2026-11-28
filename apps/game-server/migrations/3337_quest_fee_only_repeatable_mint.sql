-- Fee-only repeatable NPC rewards use the existing paid MINT transaction.
-- The source and fee completion triggers still prove the exact MINT and burn.
DO $migration$
DECLARE
    body TEXT;
    anchor TEXT := '    IF cardinality(game_quest_reward_grant_keys(v_source.plan))=0 THEN';
BEGIN
    SELECT pg_get_functiondef('game_quest_reward_value_only_complete()'::regprocedure)
        INTO STRICT body;
    IF (length(body)-length(replace(body,anchor,'')))/length(anchor)<>1 THEN
        RAISE EXCEPTION 'fee-only quest MINT requires the exact value-only quest guard';
    END IF;
    EXECUTE replace(body,anchor,$arm$
    IF v_source.plan->'repeatable'='true'::jsonb
        AND jsonb_typeof(v_source.plan->'give')='object'
        AND (v_source.plan #>> '{give,count}')::bigint>0
        AND jsonb_array_length(v_source.plan->'take')=0
        AND v_source.mint_transaction_id IS NOT NULL
        AND EXISTS(SELECT 1 FROM game_quest_reward_fees q
            WHERE q.character_id=NEW.character_id AND q.cause_id=NEW.cause_id
                AND q.cause_ordinal=NEW.cause_ordinal AND q.transition_key=NEW.transition_key
                AND q.committed_character_revision=NEW.committed_character_revision
                AND q.created_xact_id=pg_current_xact_id()
                AND q.fee_gold_units::text=v_source.plan #>> '{fee,gold}'
                AND q.include_bank::text=v_source.plan #>> '{fee,include_bank}')
    THEN RETURN NULL; END IF;
$arm$||anchor);
END $migration$;
