-- Crystal 201189f8 items.xml:20777–20802. The final remains do not decay.
DO $koshei_corpse_chain$
DECLARE
    root CONSTANT TEXT := 'oteryn:item.tibia.i7535';
    chain_anchor CONSTANT TEXT := 'WHEN ''oteryn:item.tibia.i54678'' THEN';
    duration_anchor CONSTANT TEXT := 'WHEN game_summer_corpse_chain(p_key) IS NOT NULL THEN';
    body TEXT;
BEGIN
    IF game_summer_corpse_chain(root) IS NOT NULL
        OR game_source_corpse_durations_v1(root) IS NOT NULL THEN
        RAISE EXCEPTION 'Koshei corpse root already has a source owner';
    END IF;

    body := pg_get_functiondef('game_summer_corpse_chain(text)'::regprocedure);
    IF (length(body)-length(replace(body,chain_anchor,'')))/length(chain_anchor) <> 1 THEN
        RAISE EXCEPTION 'Existing corpse registry anchor changed';
    END IF;
    EXECUTE replace(body, chain_anchor,
        'WHEN ''oteryn:item.tibia.i7535'' THEN ARRAY[7535,7537,7538,7539,7540] ' || chain_anchor);

    body := pg_get_functiondef('game_source_corpse_durations_v1(text)'::regprocedure);
    IF (length(body)-length(replace(body,duration_anchor,'')))/length(duration_anchor) <> 1 THEN
        RAISE EXCEPTION 'Existing corpse duration registry anchor changed';
    END IF;
    EXECUTE replace(body, duration_anchor,
        'WHEN p_key=''oteryn:item.tibia.i7535'' THEN ARRAY[10000,10000,300000,300000,0]::BIGINT[] ' || duration_anchor);

    IF game_summer_corpse_chain(root) IS DISTINCT FROM ARRAY[7535,7537,7538,7539,7540]
        OR game_source_corpse_durations_v1(root) IS DISTINCT FROM ARRAY[10000,10000,300000,300000,0]::BIGINT[] THEN
        RAISE EXCEPTION 'Koshei corpse chain diverges from its source policy';
    END IF;
END $koshei_corpse_chain$;
