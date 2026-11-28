-- Tibia Global Dead Rat / Animate Dead Rune: the immovable death appearance
-- lasts ten seconds, followed by two five-minute stages and a one-minute stage.
-- The existing corpse owner schedules those exact four durations at mint.
DO $rat_corpse_chain$
DECLARE
    root CONSTANT TEXT := 'oteryn:item.tibia.i5964';
    chain CONSTANT INTEGER[] := ARRAY[5964,3994,3995,3996];
    anchor CONSTANT TEXT := 'WHEN ''oteryn:item.tibia.i54678'' THEN';
    body TEXT;
BEGIN
    IF game_summer_corpse_chain(root) IS NOT NULL THEN
        RAISE EXCEPTION 'Rat corpse root already has a source owner';
    END IF;
    body := pg_get_functiondef('game_summer_corpse_chain(text)'::regprocedure);
    IF (length(body)-length(replace(body,anchor,'')))/length(anchor) <> 1 THEN
        RAISE EXCEPTION 'Existing corpse registry anchor changed';
    END IF;
    EXECUTE replace(body, anchor,
        format('WHEN %L THEN %L::INTEGER[] ', root, chain) || anchor);
    IF game_summer_corpse_chain(root) IS DISTINCT FROM chain
        OR game_source_corpse_durations_v1(root) IS DISTINCT FROM ARRAY[10000,300000,300000,60000]::BIGINT[] THEN
        RAISE EXCEPTION 'Rat corpse chain diverges from its source policy';
    END IF;
END $rat_corpse_chain$;
