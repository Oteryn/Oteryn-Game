-- C16d: the live Rat body lasts 10 seconds before the first visible Dead Rat
-- stage. The three visible stage lifetimes are from TibiaWiki's Dead Rat page
-- (Fandom archive); the pinned Canary Item identities bind those three stages.
DO $migration$
DECLARE
    body TEXT;
    anchor CONSTANT TEXT := 'WHEN ''oteryn:item.tibia.i54678'' THEN';
    expected CONSTANT INTEGER[] := ARRAY[5964,3994,3995,3996];
BEGIN
    IF game_summer_corpse_chain('oteryn:item.tibia.i5964') IS NULL THEN
        body := pg_get_functiondef('game_summer_corpse_chain(text)'::regprocedure);
        IF (length(body)-length(replace(body,anchor,'')))/length(anchor) <> 1 THEN
            RAISE EXCEPTION 'Rat corpse chain registry layout changed';
        END IF;
        EXECUTE replace(body, anchor,
            format('WHEN %L THEN %L::INTEGER[]', 'oteryn:item.tibia.i5964', expected) || E'\n ' || anchor);
    END IF;
    IF game_summer_corpse_chain('oteryn:item.tibia.i5964') IS DISTINCT FROM expected THEN
        RAISE EXCEPTION 'Rat corpse stage identities differ from Global';
    END IF;
    IF game_source_corpse_durations_v1('oteryn:item.tibia.i5964')
       IS DISTINCT FROM ARRAY[10000,300000,300000,60000]::BIGINT[] THEN
        RAISE EXCEPTION 'Rat corpse durations differ from Global';
    END IF;
END $migration$;
