-- Dead Rat (TibiaWiki): two five-minute stages followed by a 60-second terminal stage.
-- Extend the current definitions (last amended in 3200), retaining their grants and settings.
DO $dead_rat_corpse_chain$
DECLARE
    body TEXT;
    anchor TEXT;
BEGIN
    body := pg_get_functiondef('game_summer_corpse_chain(text)'::regprocedure);
    anchor := 'WHEN ''oteryn:item.tibia.i54678'' THEN';
    IF (length(body) - length(replace(body, anchor, ''))) / length(anchor) <> 1 THEN
        RAISE EXCEPTION 'Corpse chain registry layout changed';
    END IF;
    EXECUTE replace(body, anchor,
        $chain$WHEN 'oteryn:item.tibia.i3994' THEN ARRAY[3994,3995,3996]
 $chain$ || anchor);

    body := pg_get_functiondef('game_source_corpse_durations_v1(text)'::regprocedure);
    anchor := 'WHEN game_summer_corpse_chain(p_key) IS NOT NULL';
    IF (length(body) - length(replace(body, anchor, ''))) / length(anchor) <> 1 THEN
        RAISE EXCEPTION 'Corpse duration registry layout changed';
    END IF;
    EXECUTE replace(body, anchor,
        $durations$WHEN p_key='oteryn:item.tibia.i3994' THEN ARRAY[300000,300000,60000]::BIGINT[]
 $durations$ || anchor);
END $dead_rat_corpse_chain$;
