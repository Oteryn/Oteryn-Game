-- First loot slice corpse chains from Crystal 201189f data/items/items.xml.
-- The two container bodies have 10s/300s/300s transformations and a 60s terminal body.
-- Glitterscale stays a plain Ground body for 500s.
DO $loot_slice_corpse$
DECLARE
    body TEXT;
    anchor TEXT;
BEGIN
    body := pg_get_functiondef('game_summer_corpse_chain(text)'::regprocedure);
    anchor := 'WHEN ''oteryn:item.tibia.i54678'' THEN';
    IF (length(body)-length(replace(body,anchor,'')))/length(anchor) <> 1 THEN
        RAISE EXCEPTION 'Corpse chain registry layout changed';
    END IF;
    EXECUTE replace(body, anchor, $chains$
 WHEN 'oteryn:item.tibia.i5972' THEN ARRAY[5972,4024,4156,4157]
 WHEN 'oteryn:item.tibia.i22742' THEN ARRAY[22742,22743,22744,22745]
 WHEN 'oteryn:item.tibia.i11589' THEN ARRAY[11589]
 $chains$ || anchor);

    body := pg_get_functiondef('game_source_corpse_durations_v1(text)'::regprocedure);
    anchor := 'WHEN game_summer_corpse_chain(p_key) IS NOT NULL';
    IF (length(body)-length(replace(body,anchor,'')))/length(anchor) <> 1 THEN
        RAISE EXCEPTION 'Corpse duration registry layout changed';
    END IF;
    EXECUTE replace(body, anchor,
        'WHEN p_key=''oteryn:item.tibia.i11589'' THEN ARRAY[500000]::BIGINT[] ' || anchor);

    body := pg_get_functiondef('game_item_plain_corpse_source(uuid)'::regprocedure);
    anchor := 'IN (''oteryn:item.tibia.i33792''';
    IF (length(body)-length(replace(body,anchor,'')))/length(anchor) <> 1 THEN
        RAISE EXCEPTION 'Plain corpse Ground provenance layout changed';
    END IF;
    EXECUTE replace(body, anchor, 'IN (''oteryn:item.tibia.i11589'',''oteryn:item.tibia.i33792''');
END $loot_slice_corpse$;
