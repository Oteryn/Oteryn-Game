-- Native selector rewards retain the NPC owner's exact same-transaction
-- quest receipt, carried debit, backpack MINT and earned-outfit checks.
DO $migration$
DECLARE body TEXT; original TEXT:='r.cause_kind=''command''';
BEGIN
    body:=pg_get_functiondef('game_quest_reward_complete()'::regprocedure);
    IF (length(body)-length(replace(body,original,'')))/length(original)<>1 THEN
        RAISE EXCEPTION 'unexpected quest reward cause proof body';
    END IF;
    EXECUTE replace(body,original,'r.cause_kind IN (''command'',''use'',''step'')');
    EXECUTE format('ALTER FUNCTION game_quest_reward_complete() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $migration$;
