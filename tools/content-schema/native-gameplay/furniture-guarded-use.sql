-- Guarded deterministic House USE: carpets retain the accepted single-furnishing
-- cell model; the caller additionally checks the complete pinned base collision.
CREATE TABLE game_house_furniture_guarded_uses(item_id INTEGER PRIMARY KEY,kind SMALLINT NOT NULL CHECK(kind IN(1,2,3)));
INSERT INTO game_house_furniture_guarded_uses(item_id,kind) VALUES
__GUARDED_USE_ROWS__;
REVOKE ALL ON game_house_furniture_guarded_uses FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
CREATE FUNCTION game_house_furniture_single_item(p_item UUID) RETURNS BOOLEAN
LANGUAGE sql STABLE SECURITY DEFINER AS $$
 SELECT EXISTS(SELECT 1 FROM game_item_house_interior_locations source WHERE source.item_instance_id=p_item
  AND (SELECT count(*) FROM game_item_house_interior_locations h WHERE h.world_id=source.world_id
   AND h.house_key=source.house_key AND h.spatial_position=source.spatial_position)=1)
$$;
DO $migration$ DECLARE body TEXT; needle TEXT; BEGIN
 body:=pg_get_functiondef('game_item_action_house_admit(game_item_consume_transform_records)'::regprocedure);
 needle:=$old$OR r.effects->>'kind' IS DISTINCT FROM 'house_transform'$old$;
 IF position(needle IN body)=0 THEN RAISE EXCEPTION 'SRV7 missing current House action discriminator'; END IF;
 EXECUTE replace(body,needle,needle||$new$
  OR (EXISTS(SELECT 1 FROM game_house_furniture_guarded_uses g WHERE g.kind=1
    AND r.before_item->>'definition_production_key'='oteryn:item.tibia.i'||g.item_id::text)
   AND (coalesce(r.source_location->>'custody_kind','house')<>'house'
    OR game_house_furniture_single_item(r.source_item_instance_id) IS DISTINCT FROM TRUE))$new$);
END $migration$;
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_house_furniture_single_item(UUID) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_house_furniture_single_item(UUID) FROM PUBLIC,oteryn_game_control;
GRANT EXECUTE ON FUNCTION game_house_furniture_single_item(UUID) TO oteryn_game_runtime;
