-- Admit only the three already-sealed toy schedules to the existing named-House
-- absolute deadline owner. Its current Node/House/custody/CAS/audit chain is retained.
CREATE FUNCTION game_santa_toy_schedule_matches(toy_schedule game_spell_item_temporal_schedules)
RETURNS BOOLEAN LANGUAGE sql STABLE AS $$
 SELECT toy_schedule.definition_family='Item' AND toy_schedule.target_family='Item'
  AND toy_schedule.target_revision=toy_schedule.definition_revision
  AND EXISTS(SELECT 1 FROM game_santa_toy_lifetimes toy_schedule_life
   WHERE toy_schedule_life.item_key=toy_schedule.definition_production_key
    AND toy_schedule_life.target_key=toy_schedule.target_production_key
    AND toy_schedule_life.duration_ms=toy_schedule.duration_millis)
$$;
DO $migration$ DECLARE body TEXT; needle TEXT; BEGIN
 body:=pg_get_functiondef('game_house_furniture_absolute_expiry_complete(uuid)'::regprocedure);
 needle:=$old$p.definition_production_key NOT IN('oteryn:item.tibia.i9034','oteryn:item.tibia.i9039',
   'oteryn:item.tibia.i35949','oteryn:item.tibia.i35951','oteryn:item.tibia.i35969','oteryn:item.tibia.i35971')$old$;
 IF strpos(body,needle)=0 THEN RAISE EXCEPTION 'missing original House expiry completion source predicate'; END IF;
 EXECUTE replace(body,needle,'('||needle||' AND game_santa_toy_schedule_matches(p) IS DISTINCT FROM TRUE)');
 body:=pg_get_functiondef('game_house_furniture_absolute_expire(uuid,uuid,numeric,numeric,numeric,uuid,numeric,uuid,text)'::regprocedure);
 needle:=$old$i.definition_production_key IN('oteryn:item.tibia.i9034','oteryn:item.tibia.i9039',
    'oteryn:item.tibia.i35949','oteryn:item.tibia.i35951','oteryn:item.tibia.i35969','oteryn:item.tibia.i35971')$old$;
 IF strpos(body,needle)=0 THEN RAISE EXCEPTION 'missing original House expiry candidate source predicate'; END IF;
 EXECUTE replace(body,needle,'('||needle||' OR game_santa_toy_schedule_matches(schedule) IS TRUE)');
END $migration$;
DO $$ BEGIN EXECUTE format('ALTER FUNCTION game_santa_toy_schedule_matches(game_spell_item_temporal_schedules) SET search_path=pg_catalog,%I,pg_temp',current_schema()); END $$;
REVOKE ALL ON FUNCTION game_santa_toy_schedule_matches(game_spell_item_temporal_schedules) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_santa_toy_schedule_matches(game_spell_item_temporal_schedules) TO oteryn_game_runtime,oteryn_game_control;
