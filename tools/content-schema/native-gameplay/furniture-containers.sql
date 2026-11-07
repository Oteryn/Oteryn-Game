-- A rotation keeps a container and every child instance in place. Admit it
-- only when the source-authored target capacity contains all current children.
CREATE TABLE game_house_furniture_capacities (
 item_id INTEGER PRIMARY KEY CHECK(item_id>0),capacity INTEGER NOT NULL CHECK(capacity BETWEEN 1 AND 65535)
);
INSERT INTO game_house_furniture_capacities VALUES
__CAPACITY_ROWS__;
REVOKE ALL ON game_house_furniture_capacities FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
GRANT SELECT ON game_house_furniture_capacities TO oteryn_game_runtime,oteryn_game_control;
CREATE FUNCTION game_house_furniture_container_fits(r game_item_consume_transform_records) RETURNS BOOLEAN
LANGUAGE sql STABLE SECURITY DEFINER AS $$
 SELECT (SELECT count(*) FROM (
  SELECT item_instance_id FROM game_item_container_entries WHERE parent_item_instance_id=r.source_item_instance_id
  UNION ALL SELECT item_instance_id FROM game_item_nested_entries WHERE parent_item_instance_id=r.source_item_instance_id
  UNION ALL SELECT item_instance_id FROM game_item_corpse_container_entries WHERE parent_item_instance_id=r.source_item_instance_id
 ) children)<=coalesce((SELECT capacity FROM game_house_furniture_capacities
  WHERE 'oteryn:item.tibia.i'||item_id::text=r.effects->'house_after'->>'key'),0)
$$;
DO $migration$ DECLARE body TEXT; needle TEXT; BEGIN
 body:=pg_get_functiondef('game_item_action_house_admit(game_item_consume_transform_records)'::regprocedure);
 needle:=$old$OR EXISTS(SELECT 1 FROM game_item_container_entries WHERE parent_item_instance_id=i.item_instance_id)
  OR EXISTS(SELECT 1 FROM game_item_nested_entries WHERE parent_item_instance_id=i.item_instance_id)$old$;
 IF position(needle IN body)=0 THEN RAISE EXCEPTION 'SRV7 missing House container admission'; END IF;
 EXECUTE replace(body,needle,'OR game_house_furniture_container_fits(r) IS DISTINCT FROM TRUE');
 body:=pg_get_functiondef('game_item_action_house_complete(UUID)'::regprocedure);
 needle:='OR game_item_action_house_custody(r) IS DISTINCT FROM TRUE';
 IF position(needle IN body)=0 THEN RAISE EXCEPTION 'SRV7 missing House container completion'; END IF;
 EXECUTE replace(body,needle,needle||' OR game_house_furniture_container_fits(r) IS DISTINCT FROM TRUE');
END $migration$;
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_house_furniture_container_fits(game_item_consume_transform_records) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_house_furniture_container_fits(game_item_consume_transform_records)
 FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
