-- Crystal 201189f8 actions_corpse.lua:2–3 names these two plain death items.
DO $$
DECLARE
    body text := pg_get_functiondef('game_item_plain_corpse_source(uuid)'::regprocedure);
    old text := '''oteryn:item.tibia.i33984''';
BEGIN
    IF (length(body) - length(replace(body, old, ''))) / length(old) <> 1 THEN
        RAISE EXCEPTION 'Father corpse Ground admission layout changed';
    END IF;
    EXECUTE replace(body, old, old || ',''oteryn:item.tibia.i11589'',''oteryn:item.tibia.i11590''');
END
$$;
REVOKE ALL ON FUNCTION game_item_plain_corpse_source(uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_item_plain_corpse_source(uuid) TO oteryn_game_runtime;
GRANT EXECUTE ON FUNCTION game_item_plain_corpse_source(uuid) TO oteryn_game_control;
