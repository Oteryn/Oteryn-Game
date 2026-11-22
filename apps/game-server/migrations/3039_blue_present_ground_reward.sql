-- Keep the existing CRAFT-1 debit, output identity, outbox and command replay.
-- Crystal 201189f8 surprise_bags.lua:17-27 uses Player:addItem's map fallback.
CREATE FUNCTION game_blue_present_ground_matches(
 r game_item_craft_records, o JSONB, g game_item_ground_locations
) RETURNS BOOLEAN LANGUAGE sql STABLE AS $$
 SELECT r.recipe='oteryn:reward-bag.event_6570'
  AND r.created_xact_id=pg_current_xact_id()
  AND g.item_instance_id=(o->>'item')::uuid
  AND g.stack_ordinal=(o->>'ordinal')::bigint
  AND (to_jsonb(g)-ARRAY['item_instance_id','stack_ordinal'])=((o->'ground')-'parent_capacity')
$$;

DO $migration$
DECLARE body TEXT; anchor TEXT; arm TEXT; begin_at INT; name TEXT;
BEGIN
 body:=pg_get_functiondef('game_item_craft_admit()'::regprocedure);
 anchor:='OR ((o->>''ordinal'')::numeric<=(SELECT coalesce(max(placement_ordinal),0)';
 IF strpos(body,anchor)=0 THEN RAISE EXCEPTION 'missing craft output ordinal proof'; END IF;
 body:=replace(body,anchor,
  'OR (NOT (o ? ''ground'') AND (o->>''ordinal'')::numeric<=(SELECT coalesce(max(placement_ordinal),0)');
 anchor:='FOR o IN SELECT value FROM jsonb_array_elements(NEW.outputs) LOOP';
 IF strpos(body,anchor)=0 THEN RAISE EXCEPTION 'missing craft output admission'; END IF;
 arm:=$arm$
  IF o ? 'ground' AND (
      NEW.recipe<>'oteryn:reward-bag.event_6570' OR NOT NEW.consume_source
      OR NEW.ingredient_item_instance_id IS NOT NULL
      OR i.definition_production_key<>'oteryn:item.tibia.i6570'
      OR jsonb_array_length(NEW.outputs)<>1
      OR (o->>'ordinal')::bigint IS DISTINCT FROM currval('game_item_ground_stack_ordinal_seq')
      OR (o->'ground'->>'world_id')::uuid IS DISTINCT FROM NEW.world_id
      OR o->'ground'->>'scope_kind' IS DISTINCT FROM '1'
      OR o->'ground'->'instance_id' IS DISTINCT FROM 'null'::jsonb
      OR octet_length((o->'ground'->>'spatial_position')::bytea) IS DISTINCT FROM 10
      OR (o->'ground'->>'corpse_ref')::bytea IS DISTINCT FROM
          convert_to('OTERYN_ITEM_LOOP_DROP/v1:', 'UTF8') || uuid_send(NEW.transaction_id)
      OR NOT COALESCE((o->'ground'->>'parent_capacity')::int BETWEEN 1 AND 20, FALSE)
      OR (SELECT count(*) FROM game_item_container_entries
          WHERE parent_item_instance_id=NEW.parent_item_instance_id)
          < (o->'ground'->>'parent_capacity')::int
      OR NOT EXISTS(SELECT 1 FROM game_durability_reconnect_sessions s
          WHERE s.game_session_id=NEW.game_session_id AND s.character_id=NEW.character_id
           AND s.session_state=2 AND s.runtime_scope_kind=1 AND s.runtime_scope_world_id=NEW.world_id
           AND s.runtime_scope_channel_id=(o->'ground'->>'channel_id')::uuid
           AND s.scope_ownership_generation=(o->'ground'->>'runtime_scope_ownership_generation')::numeric)
  ) THEN RAISE EXCEPTION 'Blue Present drop requires a carried present, a full backpack and current Channel' USING ERRCODE='23514'; END IF;
$arm$;
 EXECUTE replace(body,anchor,anchor||arm);

 body:=pg_get_functiondef('game_item_craft_complete(uuid)'::regprocedure);
 anchor:='SELECT 1 FROM game_item_instances n JOIN game_item_container_entries e USING(item_instance_id)';
 IF strpos(body,anchor)=0 THEN RAISE EXCEPTION 'missing craft output custody join'; END IF;
 body:=replace(body,anchor,
     'SELECT 1 FROM game_item_instances n LEFT JOIN game_item_container_entries e USING(item_instance_id)');
 anchor:=$old$AND e.world_id=r.world_id AND e.character_id=r.character_id AND e.parent_item_instance_id=r.parent_item_instance_id
       AND e.placement_ordinal=(o->>'ordinal')::numeric AND e.placed_transaction_id=r.transaction_id)$old$;
 IF strpos(body,anchor)=0 THEN RAISE EXCEPTION 'missing craft output custody proof'; END IF;
 arm:=$arm$AND (
       (NOT (o ? 'ground') AND e.world_id=r.world_id AND e.character_id=r.character_id
        AND e.parent_item_instance_id=r.parent_item_instance_id AND e.placement_ordinal=(o->>'ordinal')::numeric
        AND e.placed_transaction_id=r.transaction_id)
       OR (o ? 'ground' AND e.item_instance_id IS NULL AND EXISTS(
        SELECT 1 FROM game_item_ground_locations g WHERE g.item_instance_id=n.item_instance_id
         AND game_blue_present_ground_matches(r,o,g)))))$arm$;
 EXECUTE replace(body,anchor,arm);

 -- The ordinary Ground owner assigns all other ordinals. The craft receipt
 -- can pre-reserve this one for its audit and must prove the same insert.
 FOREACH name IN ARRAY ARRAY['game_item_ground_insertion_guard','game_item_ground_stack_stamp'] LOOP
  body:=pg_get_functiondef((name||'()')::regprocedure);
  begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
  IF begin_at=0 THEN RAISE EXCEPTION 'missing Blue Present Ground proof: %',name; END IF;
  arm:=$arm$
   IF NEW.stack_ordinal IS NOT NULL AND EXISTS(
     SELECT 1 FROM game_item_instances i JOIN game_item_craft_records r ON r.transaction_id=i.minted_transaction_id,
     LATERAL jsonb_array_elements(r.outputs) o
     WHERE i.item_instance_id=NEW.item_instance_id AND game_blue_present_ground_matches(r,o,NEW)) THEN
$arm$;
  arm:=arm||CASE WHEN name='game_item_ground_stack_stamp' THEN $arm$
     IF NEW.stack_ordinal IS DISTINCT FROM currval('game_item_ground_stack_ordinal_seq')
         OR EXISTS(SELECT 1 FROM game_item_ground_locations WHERE stack_ordinal=NEW.stack_ordinal)
     THEN RAISE EXCEPTION 'Blue Present Ground ordinal was not freshly reserved' USING ERRCODE='23514'; END IF;
     RETURN NEW;
$arm$ ELSE ' RETURN NULL;' END||' END IF;';
  EXECUTE overlay(body placing E'BEGIN\n'||arm from begin_at for 5);
 END LOOP;
 EXECUTE format('ALTER FUNCTION game_blue_present_ground_matches(game_item_craft_records,jsonb,game_item_ground_locations) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $migration$;

REVOKE ALL ON FUNCTION game_blue_present_ground_matches(game_item_craft_records,JSONB,game_item_ground_locations) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_blue_present_ground_matches(game_item_craft_records,JSONB,game_item_ground_locations) TO oteryn_game_runtime;
GRANT EXECUTE ON FUNCTION game_blue_present_ground_matches(game_item_craft_records,JSONB,game_item_ground_locations) TO oteryn_game_control;
