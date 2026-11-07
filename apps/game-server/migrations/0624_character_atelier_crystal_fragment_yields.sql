-- Crystal 201189f player_wheel.cpp:1308-1318: revealed lesser/regular/greater
-- gems yield 1..5 / 2..10 / 1..5 fragments. Keep historical receipts and all
-- existing custody/fee/Character proof arms; only the current mint bounds change.
DO $migration$
DECLARE body TEXT; old_bound TEXT; new_bound TEXT;
BEGIN
    old_bound := '(line->>''count'')::int NOT BETWEEN 1 AND 5';
    new_bound := '(line->>''count'')::int NOT BETWEEN 1 AND 10';
    SELECT pg_get_functiondef('game_atelier_item_admit()'::regprocedure) INTO STRICT body;
    IF position(new_bound IN body) = 0 THEN
        IF position(old_bound IN body) = 0 THEN
            RAISE EXCEPTION 'missing Atelier fragment admission bound';
        END IF;
        EXECUTE replace(body, old_bound, new_bound);
    END IF;

    old_bound := '(CASE WHEN quality=''Regular'' THEN 5 ELSE 3 END)';
    new_bound := '(CASE WHEN quality=''Regular'' THEN 10 ELSE 5 END)';
    SELECT pg_get_functiondef('game_atelier_assets_guard()'::regprocedure) INTO STRICT body;
    IF position(new_bound IN body) = 0 THEN
        IF position(old_bound IN body) = 0 THEN
            RAISE EXCEPTION 'missing Atelier quality-specific yield proof';
        END IF;
        EXECUTE replace(body, old_bound, new_bound);
    END IF;

    EXECUTE format('ALTER FUNCTION game_atelier_item_admit() SET search_path=pg_catalog,%I,pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_atelier_assets_guard() SET search_path=pg_catalog,%I,pg_temp', current_schema());
END
$migration$;

REVOKE ALL ON FUNCTION game_atelier_item_admit(), game_atelier_assets_guard() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_atelier_item_admit(), game_atelier_assets_guard() TO oteryn_game_runtime;
