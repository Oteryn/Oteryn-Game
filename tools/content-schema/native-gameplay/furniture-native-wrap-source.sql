-- Policy population only. No per-Item state or clock is introduced.
CREATE TABLE game_house_furniture_portable_wrap_sources (
 item_id INTEGER PRIMARY KEY CHECK(item_id>0),
 stack_max INTEGER NOT NULL CHECK(stack_max BETWEEN 1 AND 100)
);
INSERT INTO game_house_furniture_portable_wrap_sources VALUES
__PORTABLE_WRAP_ROWS__;
REVOKE ALL ON game_house_furniture_portable_wrap_sources FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
GRANT SELECT ON game_house_furniture_portable_wrap_sources TO oteryn_game_runtime;

-- Generic House action source stays exactly quantity1. Only source-qualified
-- portable Wrap stacks receive this fallback, under the mature lock/fence order.
CREATE FUNCTION game_house_furniture_wrap_source(p_session UUID,p_character UUID,p_connection NUMERIC,
 p_lease NUMERIC,p_scope NUMERIC,p_node UUID,p_node_revision NUMERIC,p_world UUID,p_house TEXT,p_item UUID)
RETURNS TABLE(before_item JSONB,location JSONB,provenance JSONB,spatial_position BYTEA,stack_ordinal TEXT)
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE i game_item_instances%ROWTYPE; e game_item_container_entries%ROWTYPE;
 n game_item_nested_entries%ROWTYPE;
 floor_item game_item_house_interior_locations%ROWTYPE;
 floor_proof game_item_house_reclaim_provenance%ROWTYPE;
BEGIN
 RETURN QUERY SELECT * FROM game_house_construction_source(p_session,p_character,p_connection,p_lease,p_scope,
  p_node,p_node_revision,p_world,p_house,p_item);
 IF FOUND THEN RETURN; END IF;
 SELECT candidate.* INTO i FROM game_item_instances candidate
 JOIN game_house_furniture_portable_wrap_sources policy
  ON candidate.definition_production_key='oteryn:item.tibia.i'||policy.item_id::text
 WHERE candidate.item_instance_id=p_item AND candidate.world_id=p_world
  AND candidate.lifecycle=1 AND candidate.definition_family='Item' AND candidate.definition_revision_ref='definition-r1'
  AND candidate.quantity BETWEEN 2 AND policy.stack_max FOR UPDATE OF candidate;
 IF NOT FOUND THEN RETURN; END IF;
 -- The generic source above has already acquired the mature House/Character
 -- fences. Only an exact closed stack policy admits this quantity2..max arm.
 SELECT placed.* INTO floor_item FROM game_item_house_interior_locations placed
  WHERE placed.item_instance_id=p_item AND placed.world_id=p_world AND placed.house_key=p_house FOR UPDATE;
 IF FOUND THEN
  SELECT proof.* INTO floor_proof FROM game_item_house_reclaim_provenance proof
   WHERE proof.item_instance_id=p_item AND proof.world_id=p_world AND proof.house_key=p_house
    AND proof.placement_transaction_id=floor_item.placed_transaction_id FOR UPDATE;
  IF NOT FOUND OR game_house_access_tile(p_world,p_house,floor_item.spatial_position) IS DISTINCT FROM TRUE
   THEN RETURN; END IF;
  RETURN QUERY SELECT to_jsonb(i),jsonb_build_object('house',to_jsonb(floor_item),'custody_kind','house','custody',to_jsonb(floor_item)),
   to_jsonb(floor_proof),floor_item.spatial_position,floor_item.stack_ordinal::text;
  RETURN;
 END IF;
 SELECT entry.* INTO e FROM game_item_container_entries entry
 JOIN game_item_container_slots bag ON bag.character_id=entry.character_id
  AND bag.item_instance_id=entry.parent_item_instance_id AND bag.world_id=entry.world_id
 WHERE entry.item_instance_id=p_item AND entry.world_id=p_world AND entry.character_id=p_character FOR UPDATE OF entry;
 IF FOUND THEN
  RETURN QUERY SELECT to_jsonb(i),jsonb_build_object('custody_kind','container','custody',to_jsonb(e)),
   'null'::jsonb,NULL::bytea,e.placement_ordinal::text;
  RETURN;
 END IF;
 SELECT entry.* INTO n FROM game_item_nested_entries entry,
 LATERAL game_item_loop_owner(entry.parent_item_instance_id) owner
 WHERE entry.item_instance_id=p_item AND entry.world_id=p_world
  AND owner.owner_character=p_character AND owner.depth BETWEEN 0 AND 7 FOR UPDATE OF entry;
 IF FOUND THEN
  RETURN QUERY SELECT to_jsonb(i),jsonb_build_object('custody_kind','nested','custody',to_jsonb(n)),
   'null'::jsonb,NULL::bytea,n.placement_ordinal::text;
 END IF;
END $$;
DO $path$ BEGIN
 EXECUTE format('ALTER FUNCTION game_house_furniture_wrap_source(uuid,uuid,numeric,numeric,numeric,uuid,numeric,uuid,text,uuid) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $path$;
REVOKE ALL ON FUNCTION game_house_furniture_wrap_source(UUID,UUID,NUMERIC,NUMERIC,NUMERIC,UUID,NUMERIC,UUID,TEXT,UUID)
 FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
GRANT EXECUTE ON FUNCTION game_house_furniture_wrap_source(UUID,UUID,NUMERIC,NUMERIC,NUMERIC,UUID,NUMERIC,UUID,TEXT,UUID)
 TO oteryn_game_runtime;
