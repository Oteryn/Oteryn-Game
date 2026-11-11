-- 1287 already installs the exact world-USE receipt arm. Verify that proof
-- without replacing its target-set, payment or selected-instance proof arms.
DO $migration$
DECLARE body TEXT; expected TEXT;
BEGIN
    SELECT pg_get_functiondef('game_quest_reward_complete()'::regprocedure) INTO STRICT body;
    expected := $proof$r.quest_key=NEW.plan->>'quest'
                AND (r.cause_kind='command' OR (r.cause_kind='use'
                    AND NEW.plan #>> '{world_callback,key}'=NEW.transition_key
                    AND NEW.plan #>> '{world_callback,quest}'=NEW.plan->>'quest'
                    AND NEW.plan #>> '{world_callback,trigger}'='use'))$proof$;
    IF (length(body)-length(replace(body,expected,'')))/length(expected)<>1 THEN
        RAISE EXCEPTION 'quest world reward receipt proof shape changed';
    END IF;
    EXECUTE format('ALTER FUNCTION game_quest_reward_complete() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $migration$;
REVOKE ALL ON FUNCTION game_quest_reward_complete() FROM PUBLIC;
CREATE INDEX game_quest_reward_receipt_evidence
    ON game_character_quest_receipts(character_id,quest_key,transition_key);
