-- Crystal 201189f8 items.xml:13782,13788,13793,16650.
-- The existing corpse scheduler supplies 10s/300s/300s/60s for this chain.
DO $elephant_corpse_chain$
DECLARE
    root CONSTANT TEXT := 'oteryn:item.tibia.i6052';
    anchor CONSTANT TEXT := 'WHEN ''oteryn:item.tibia.i54678'' THEN';
    body TEXT;
BEGIN
    IF game_summer_corpse_chain(root) IS NOT NULL THEN
        RAISE EXCEPTION 'Elephant corpse root already has a source owner';
    END IF;
    body := pg_get_functiondef('game_summer_corpse_chain(text)'::regprocedure);
    IF (length(body)-length(replace(body,anchor,'')))/length(anchor) <> 1 THEN
        RAISE EXCEPTION 'Existing corpse registry anchor changed';
    END IF;
    EXECUTE replace(body, anchor,
        'WHEN ''oteryn:item.tibia.i6052'' THEN ARRAY[6052,4360,4361,4362] ' || anchor);
    IF game_summer_corpse_chain(root) IS DISTINCT FROM ARRAY[6052,4360,4361,4362]
        OR game_source_corpse_durations_v1(root) IS DISTINCT FROM ARRAY[10000,300000,300000,60000]::BIGINT[] THEN
        RAISE EXCEPTION 'Elephant corpse chain diverges from its source policy';
    END IF;
END $elephant_corpse_chain$;
