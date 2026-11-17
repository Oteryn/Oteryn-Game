-- Disambiguate the lane's deferred XP proof without changing its admission rules.
DO $migration$
DECLARE body TEXT;
BEGIN
    SELECT pg_get_functiondef('game_father_values_proven()'::regprocedure) INTO STRICT body;
    IF strpos(body,'amount BIGINT')=0 OR strpos(body,'x.amount=amount')=0 THEN
        RAISE EXCEPTION 'Father reward amount proof shape changed';
    END IF;
    body:=replace(body,'amount BIGINT','expected_amount BIGINT');
    body:=replace(body,'amount:=','expected_amount:=');
    body:=replace(body,'x.amount=amount','x.amount=expected_amount');
    EXECUTE body;
    EXECUTE format('ALTER FUNCTION game_father_values_proven() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $migration$;
REVOKE ALL ON FUNCTION game_father_values_proven() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_father_values_proven() TO oteryn_game_runtime;
