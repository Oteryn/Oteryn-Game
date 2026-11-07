-- Keep shared QBASE1205 unchanged; disambiguate only its local XP variable.
-- All eleven tracks, item hand-ins, cape and XP remain one atomic programme.
DO $migration$
DECLARE body TEXT; old TEXT; new TEXT;
BEGIN
    SELECT pg_get_functiondef('game_father_values_proven()'::regprocedure) INTO STRICT body;
    FOR old,new IN SELECT * FROM (VALUES
        ('expected_rule TEXT; amount BIGINT;', 'expected_rule TEXT; required_xp BIGINT;'),
        ('amount:=8000;', 'required_xp:=8000;'),
        ('amount:=2500;', 'required_xp:=2500;'),
        ('AND x.amount=amount AND x.xmin=', 'AND x.amount=required_xp AND x.xmin=')) AS edits(old,new)
    LOOP
        IF (length(body)-length(replace(body,old,'')))/length(old)<>1 THEN
            RAISE EXCEPTION 'shared Father XP proof shape changed'; END IF;
        body:=replace(body,old,new);
    END LOOP;
    EXECUTE body;
END $migration$;
