-- Preserve an immutable pre-use snapshot for the reusable selected replica.
-- Older corpse samples keep their original, consumed-instance proof.
ALTER TABLE game_quest_corpse_samples ADD COLUMN before_source JSONB
    CHECK(before_source IS NULL OR (
        jsonb_typeof(before_source)='object' AND octet_length(before_source::TEXT)<=16384));

DO $migration$
DECLARE body TEXT; anchor TEXT;
BEGIN
    body:=pg_get_functiondef('game_quest_carried_use_complete()'::regprocedure);
    anchor:='IF NOT (NEW.plan ? ''carried_item'') THEN';
    IF position(anchor IN body)=0 THEN
        RAISE EXCEPTION 'Wrath retained tool: carried-use proof layout changed';
    END IF;
    EXECUTE replace(body,anchor,$guard$
    IF NEW.plan->>'carried_item'='oteryn:item.tibia.i11362'
       AND NEW.transition_key LIKE 'oteryn:quest-reward/wrath/banish-%'
       AND NEW.plan ? 'corpse_target'
       AND NEW.plan->>'quest'='oteryn:quest.wrath_of_the_emperor_quest'
       AND NEW.plan->'give'='null'::JSONB
       AND NEW.plan->'take'='[]'::JSONB THEN
        -- The corpse-sample constraint trigger independently requires this
        -- same-transaction snapshot, effective tool custody and unchanged body.
        RETURN NULL;
    END IF;
    $guard$||anchor);

    body:=pg_get_functiondef('game_quest_corpse_sample_complete()'::regprocedure);
    anchor:='AND p.race_key=source_row.plan->>''creature_target''';
    IF position(anchor IN body)=0 THEN
        RAISE EXCEPTION 'Wrath retained tool: corpse species proof layout changed';
    END IF;
    body:=replace(body,anchor,$race$
    AND (p.race_key=source_row.plan->>'creature_target'
        OR (sample_row.before_source IS NOT NULL AND sample_row.appearance=11361
            AND p.race_key IN ('oteryn:creature.fury_of_the_emperor',
                'oteryn:creature.scorn_of_the_emperor','oteryn:creature.spite_of_the_emperor',
                'oteryn:creature.wrath_of_the_emperor')))
    $race$);
    anchor:='SELECT * INTO input_row FROM game_quest_callback_consumptions c';
    IF position(anchor IN body)=0 THEN
        RAISE EXCEPTION 'Wrath retained tool: corpse input proof layout changed';
    END IF;
    EXECUTE replace(body,anchor,$guard$
    IF source_row.plan->>'carried_item'='oteryn:item.tibia.i11362'
       AND source_row.transition_key LIKE 'oteryn:quest-reward/wrath/banish-%'
       AND source_row.plan->>'quest'='oteryn:quest.wrath_of_the_emperor_quest'
       AND source_row.plan->'give'='null'::JSONB
       AND source_row.plan->'take'='[]'::JSONB THEN
        IF sample_row.appearance NOT IN (11361,11429)
          OR sample_row.before_source IS NULL
          OR source_row.plan->'present' IS DISTINCT FROM
               '[{"item":"oteryn:item.tibia.i11362","count":1}]'::JSONB
          OR NOT EXISTS (
            SELECT 1 FROM game_quest_callback_carried(sample_row.character_id,sample_row.world_id) c
            JOIN game_item_instances i ON i.item_instance_id=c.item
            WHERE i.item_instance_id=sample_row.source_item
              AND i.world_id=sample_row.world_id AND i.lifecycle=1
              AND i.definition_family='Item' AND i.definition_production_key='oteryn:item.tibia.i11362'
              AND to_jsonb(i)=sample_row.before_source)
          OR EXISTS (
            SELECT 1 FROM game_quest_callback_consumptions c
            WHERE (c.character_id,c.cause_id,c.cause_ordinal,c.transition_key)=
              (sample_row.character_id,sample_row.cause_id,sample_row.cause_ordinal,sample_row.transition_key))
        THEN RAISE EXCEPTION 'Wrath corpse sample lacks retained exact replica' USING ERRCODE='23514'; END IF;
        RETURN NULL;
    ELSIF sample_row.before_source IS NOT NULL THEN
        RAISE EXCEPTION 'ordinary corpse sample cannot use retained tool proof' USING ERRCODE='23514';
    END IF;
    $guard$||anchor);
END $migration$;
