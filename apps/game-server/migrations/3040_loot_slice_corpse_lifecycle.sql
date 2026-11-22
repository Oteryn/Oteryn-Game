-- First loot slice corpse chains from Crystal 201189f data/items/items.xml.
-- The two container bodies have 10s/300s/300s transformations and a 60s terminal body.
-- Glitterscale stays a plain Ground body for 500s.
-- The other admitted loot bodies use the committed native Item temporal chains.
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
 WHEN 'oteryn:item.tibia.i4024' THEN ARRAY[4024,4156,4157]
 WHEN 'oteryn:item.tibia.i4097' THEN ARRAY[4097,4098,4099]
 WHEN 'oteryn:item.tibia.i5977' THEN ARRAY[5977,4038,4039,4040]
 WHEN 'oteryn:item.tibia.i6006' THEN ARRAY[6006,4137,4138,4139]
 WHEN 'oteryn:item.tibia.i6011' THEN ARRAY[6011,4160,4161,4128]
 WHEN 'oteryn:item.tibia.i6078' THEN ARRAY[6078,5931,5932,5933]
 WHEN 'oteryn:item.tibia.i8105' THEN ARRAY[8105,8106,8107,8108]
 WHEN 'oteryn:item.tibia.i8136' THEN ARRAY[8136,8141]
 WHEN 'oteryn:item.tibia.i8138' THEN ARRAY[8138,8139]
 WHEN 'oteryn:item.tibia.i18099' THEN ARRAY[18099,18100,18101]
 WHEN 'oteryn:item.tibia.i20972' THEN ARRAY[20972,20973,20974,20975]
 WHEN 'oteryn:item.tibia.i21910' THEN ARRAY[21910,21911,21912]
 WHEN 'oteryn:item.tibia.i22067' THEN ARRAY[22067,22071,22072,22073]
 WHEN 'oteryn:item.tibia.i25065' THEN ARRAY[25065,25066,25067]
 WHEN 'oteryn:item.tibia.i25085' THEN ARRAY[25085,25086]
 WHEN 'oteryn:item.tibia.i27714' THEN ARRAY[27714,27715,27716,27717]
 WHEN 'oteryn:item.tibia.i27718' THEN ARRAY[27718,27720,27721]
 WHEN 'oteryn:item.tibia.i31453' THEN ARRAY[31453,31452,31451,31450]
 WHEN 'oteryn:item.tibia.i31599' THEN ARRAY[31599,31598,31597,31596]
 WHEN 'oteryn:item.tibia.i31611' THEN ARRAY[31611,31610,31609,31608]
 $chains$ || anchor);

    body := pg_get_functiondef('game_source_corpse_durations_v1(text)'::regprocedure);
    anchor := 'WHEN game_summer_corpse_chain(p_key) IS NOT NULL';
    IF (length(body)-length(replace(body,anchor,'')))/length(anchor) <> 1 THEN
        RAISE EXCEPTION 'Corpse duration registry layout changed';
    END IF;
    EXECUTE replace(body, anchor,
        $durations$
 WHEN p_key='oteryn:item.tibia.i11589' THEN ARRAY[500000]::BIGINT[]
 WHEN p_key='oteryn:item.tibia.i4024' THEN ARRAY[300000,300000,60000]::BIGINT[]
 WHEN p_key='oteryn:item.tibia.i4097' THEN ARRAY[300000,300000,60000]::BIGINT[]
 WHEN p_key='oteryn:item.tibia.i8136' THEN ARRAY[10000,500000]::BIGINT[]
 WHEN p_key='oteryn:item.tibia.i8138' THEN ARRAY[10000,500000]::BIGINT[]
 WHEN p_key='oteryn:item.tibia.i18099' THEN ARRAY[300000,300000,60000]::BIGINT[]
 WHEN p_key='oteryn:item.tibia.i21910' THEN ARRAY[10000,300000,60000]::BIGINT[]
 WHEN p_key='oteryn:item.tibia.i25065' THEN ARRAY[10000,500000,60000]::BIGINT[]
 WHEN p_key='oteryn:item.tibia.i25085' THEN ARRAY[10000,600000]::BIGINT[]
 WHEN p_key='oteryn:item.tibia.i27718' THEN ARRAY[300000,300000,60000]::BIGINT[]
 $durations$ || anchor);

    body := pg_get_functiondef('game_item_plain_corpse_source(uuid)'::regprocedure);
    anchor := 'IN (''oteryn:item.tibia.i33792''';
    IF (length(body)-length(replace(body,anchor,'')))/length(anchor) <> 1 THEN
        RAISE EXCEPTION 'Plain corpse Ground provenance layout changed';
    END IF;
    EXECUTE replace(body, anchor, 'IN (''oteryn:item.tibia.i11589'',''oteryn:item.tibia.i33792''');

    -- Transform schedules are sealed at mint. A corpse already on the ground
    -- when its chain is admitted keeps the legacy retirement deadline.
    body := pg_get_functiondef('game_corpse_retirement_deadline(uuid)'::regprocedure);
    anchor := 'SELECT CASE WHEN original.d[array_length(original.d,1)]=0 THEN NULL';
    IF (length(body)-length(replace(body,anchor,'')))/length(anchor) <> 1 THEN
        RAISE EXCEPTION 'Corpse retirement deadline layout changed';
    END IF;
    EXECUTE replace(body, anchor, $deadline$
 SELECT CASE WHEN original.d[array_length(original.d,1)]=0 THEN NULL
 WHEN cardinality(original.d)>1 AND NOT EXISTS (
   SELECT 1 FROM game_spell_field_temporal_schedules s
   WHERE s.item_instance_id=m.item_instance_id AND s.source_transaction_id=m.transaction_id
     AND s.source_ordinal=0 AND s.source_stage=1
 ) THEN m.materialized_at+60000
 $deadline$);
END $loot_slice_corpse$;
