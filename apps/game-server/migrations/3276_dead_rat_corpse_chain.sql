-- Dead Rat (TibiaWiki): two five-minute stages followed by a 60-second terminal stage.
-- pg_get_functiondef returns CREATE OR REPLACE, retaining the installed bodies and settings
-- from 3200 and earlier. Replacing each function in place preserves its grants.
DO $dead_rat_corpse_chain$
DECLARE
    root CONSTANT TEXT := 'oteryn:item.tibia.i3994';
    expected_chain CONSTANT INTEGER[] := ARRAY[3994,3995,3996];
    expected_durations CONSTANT BIGINT[] := ARRAY[300000,300000,60000]::BIGINT[];
    body TEXT;
    anchor TEXT;
BEGIN
    IF game_summer_corpse_chain(root) IS NOT NULL THEN
        IF game_summer_corpse_chain(root) IS DISTINCT FROM expected_chain
            OR game_source_corpse_durations_v1(root) IS DISTINCT FROM expected_durations THEN
            RAISE EXCEPTION 'Dead Rat already has a different corpse policy';
        END IF;
        RETURN;
    END IF;

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
    IF game_summer_corpse_chain(root) IS DISTINCT FROM expected_chain
        OR game_source_corpse_durations_v1(root) IS DISTINCT FROM expected_durations THEN
        RAISE EXCEPTION 'Dead Rat corpse policy did not install';
    END IF;
END $dead_rat_corpse_chain$;
