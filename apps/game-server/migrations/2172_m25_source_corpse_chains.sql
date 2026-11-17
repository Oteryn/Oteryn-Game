-- M25: retain the existing corpse MINT/deadline owner for two exact source chains.
-- Crystal201189f items.xml:74406-74427 (Ahau); Canary47df
-- items.xml:77058-77078 (Mitmah's selected canonical Creature donor).
-- Both are 10s/300s/300s/60s. No new timer, custody, clock or audit namespace.
DO $m25_corpse_chains$
DECLARE body TEXT; anchor TEXT; prior_primal INTEGER[]; prior_summer INTEGER[];
BEGIN
    IF game_summer_corpse_chain('oteryn:item.tibia.i42069') IS NOT NULL
        OR game_summer_corpse_chain('oteryn:item.tibia.i44687') IS NOT NULL THEN
        RAISE EXCEPTION 'M25 corpse roots already have a source owner';
    END IF;
    prior_primal:=game_summer_corpse_chain('oteryn:item.tibia.i39530');
    prior_summer:=game_summer_corpse_chain('oteryn:item.tibia.i54678');
    body:=pg_get_functiondef('game_summer_corpse_chain(text)'::regprocedure);
    anchor:='WHEN ''oteryn:item.tibia.i54678'' THEN';
    IF (length(body)-length(replace(body,anchor,'')))/length(anchor)<>1 THEN
        RAISE EXCEPTION 'M25 existing corpse registry anchor changed';
    END IF;
    EXECUTE replace(body,anchor,
        'WHEN ''oteryn:item.tibia.i42069'' THEN ARRAY[42069,42072,42071,42070] '
        ||'WHEN ''oteryn:item.tibia.i44687'' THEN ARRAY[44687,44686,44685,44684] '
        ||anchor);
    IF game_summer_corpse_chain('oteryn:item.tibia.i42069') IS DISTINCT FROM ARRAY[42069,42072,42071,42070]
        OR game_summer_corpse_chain('oteryn:item.tibia.i44687') IS DISTINCT FROM ARRAY[44687,44686,44685,44684]
        OR game_source_corpse_durations_v1('oteryn:item.tibia.i42069') IS DISTINCT FROM ARRAY[10000,300000,300000,60000]::BIGINT[]
        OR game_source_corpse_durations_v1('oteryn:item.tibia.i44687') IS DISTINCT FROM ARRAY[10000,300000,300000,60000]::BIGINT[]
        OR game_summer_corpse_chain('oteryn:item.tibia.i39530') IS DISTINCT FROM prior_primal
        OR game_summer_corpse_chain('oteryn:item.tibia.i54678') IS DISTINCT FROM prior_summer THEN
        RAISE EXCEPTION 'M25 source corpse chain or preserved predecessor differs';
    END IF;
END $m25_corpse_chains$;
