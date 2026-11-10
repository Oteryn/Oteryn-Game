-- T135: Canary 99902524 src/map/house/house.cpp:340-356.
-- Depositing an ordinary item does not make it owner-bound. Explicit item
-- bindings override the seller; the existing Guild reclaim rules stay intact.
ALTER FUNCTION game_house_disposition_container_sources(UUID,TEXT)
 RENAME TO game_house_disposition_container_sources_before_t135;
CREATE FUNCTION game_house_disposition_container_sources(w UUID,h TEXT)
RETURNS TABLE(item_instance_id UUID,reclaim_subject_character_id UUID,source_location JSONB,source_provenance JSONB,source_item JSONB)
LANGUAGE sql STABLE SECURITY DEFINER AS $$
 SELECT s.item_instance_id,
  CASE WHEN o.owner_guild_id IS NULL AND o.owner_character_id IS NOT NULL
   THEN coalesce(b.character_id,o.owner_character_id) ELSE s.reclaim_subject_character_id END,
  s.source_location,s.source_provenance,s.source_item
 FROM game_house_disposition_container_sources_before_t135(w,h) s
 JOIN game_house_ownership o ON o.world_id=w AND o.house_key=h
 LEFT JOIN game_item_bindings b ON b.item_instance_id=s.item_instance_id AND b.world_id=w
$$;
-- Lock the actual recipients, and check those same subjects after property lock.
-- Keep all existing Node, ownership, content, census and delivery proof branches.
DO $patch$
DECLARE body TEXT;old TEXT;replacement TEXT;
BEGIN
 body:=pg_get_functiondef('game_house_disposition_advance(UUID,INT,INT,INT)'::regprocedure);
 old:='(SELECT item_instance_id,world_id,house_key,reclaim_subject_character_id FROM game_item_house_reclaim_provenance UNION ALL SELECT item_instance_id,world_id,house_key,reclaim_subject_character_id FROM game_house_nested_reclaim_provenance)';
 replacement:='(SELECT item_instance_id,o.world_id AS world_id,o.house_key AS house_key,reclaim_subject_character_id FROM game_house_disposition_container_sources(o.world_id,o.house_key))';
 IF (length(body)-length(replace(body,old,'')))/length(old)<>2 THEN
  RAISE EXCEPTION 'T135 disposition recipient lock/check seams changed';
 END IF;
 EXECUTE replace(body,old,replacement);
 EXECUTE format('ALTER FUNCTION game_house_disposition_container_sources(UUID,TEXT) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $patch$;
REVOKE ALL ON FUNCTION game_house_disposition_container_sources(UUID,TEXT),
 game_house_disposition_container_sources_before_t135(UUID,TEXT)
 FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
