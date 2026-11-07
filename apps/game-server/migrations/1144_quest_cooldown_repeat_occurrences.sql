-- A free repeat proves an elapsed source clock and its new database timestamp
-- in the same immutable quest receipt. Costed repeats retain every 1141 check.
DO $migration$
DECLARE body TEXT; original TEXT := 'jsonb_array_length(NEW.plan->''take'')=0'; replacement TEXT;
BEGIN
    body := pg_get_functiondef('game_quest_reward_complete()'::regprocedure);
    IF (length(body)-length(replace(body,original,'')))/length(original)<>1 THEN
        RAISE EXCEPTION 'unexpected costed repeat completion body';
    END IF;
    replacement := $clock$(jsonb_array_length(NEW.plan->'take')=0 AND NOT coalesce((
        jsonb_typeof(NEW.plan->'cooldown')='object'
        AND jsonb_typeof(NEW.plan #> '{cooldown,seconds}')='number'
        AND (NEW.plan #>> '{cooldown,seconds}') ~ '^[0-9]+$'
        AND (NEW.plan #>> '{cooldown,seconds}')::numeric BETWEEN 1 AND 31536000
        AND EXISTS (
            SELECT 1 FROM game_character_quest_receipts r
            CROSS JOIN LATERAL unnest(r.track_keys,r.values_before,r.values_after)
                AS clock(track_key,value_before,value_after)
            JOIN game_character_quest_tracks t ON t.character_id=r.character_id
                AND t.track_key=clock.track_key AND t.quest_key=r.quest_key
            WHERE r.character_id=NEW.character_id AND r.cause_id=NEW.cause_id
                AND r.cause_ordinal=NEW.cause_ordinal AND r.transition_key=NEW.transition_key
                AND r.quest_key=NEW.plan->>'quest' AND r.created_xact_id=pg_current_xact_id()
                AND clock.track_key=NEW.plan #>> '{cooldown,track}'
                AND t.value=clock.value_after
                AND clock.value_after>=floor(extract(epoch FROM transaction_timestamp()))
                AND clock.value_after<=floor(extract(epoch FROM clock_timestamp()))
                AND (clock.value_before<=0 OR
                    clock.value_after::numeric-clock.value_before::numeric >=
                        (NEW.plan #>> '{cooldown,seconds}')::numeric)
        )
    ),false))$clock$;
    EXECUTE replace(body,original,replacement);
    EXECUTE format('ALTER FUNCTION game_quest_reward_complete() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $migration$;
