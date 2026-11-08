-- The independent-output participant uses the SQL range alias `actual`.
-- Keep its proof intact and disambiguate the older PL/pgSQL quantity variable.
-- Exact replacements also work when 2100 has already been applied.
DO $migration$
DECLARE definition TEXT; anchor TEXT; replacement TEXT;
BEGIN
    definition := pg_get_functiondef('game_quest_reward_complete()'::regprocedure);
    FOR anchor, replacement IN SELECT * FROM (VALUES
        ('actual BIGINT;', 'v_actual_quantity BIGINT;'),
        ('INTO actual FROM game_quest_callback_consumptions',
         'INTO v_actual_quantity FROM game_quest_callback_consumptions'),
        ('IF actual IS DISTINCT FROM', 'IF v_actual_quantity IS DISTINCT FROM')
    ) replacements(before_text, after_text) LOOP
        IF (length(definition)-length(replace(definition,anchor,'')))/length(anchor) <> 1 THEN
            RAISE EXCEPTION 'quest reward quantity variable requires exact guard anchor: %', anchor;
        END IF;
        definition := replace(definition,anchor,replacement);
    END LOOP;
    EXECUTE definition;
END $migration$;
