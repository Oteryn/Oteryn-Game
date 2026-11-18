-- Freeze qualified descendant state in the existing admission and conservation guards.
CREATE OR REPLACE FUNCTION game_player_trade_contents(p_root UUID) RETURNS JSONB
LANGUAGE plpgsql STABLE SECURITY DEFINER AS $$
DECLARE result JSONB; invalid BOOLEAN; size BIGINT; root_world UUID;
BEGIN
 SELECT world_id INTO root_world FROM game_item_instances WHERE item_instance_id=p_root;
 IF root_world IS NULL THEN RAISE EXCEPTION 'missing player trade root' USING ERRCODE='23514'; END IF;
 WITH edges AS MATERIALIZED (
  SELECT * FROM game_item_loop_subtree_edges(p_root) WHERE depth>0 LIMIT 100
 ), snapshots AS (
  SELECT e.*,to_jsonb(i) AS item,to_jsonb(n) AS entry,
   (SELECT to_jsonb(k) FROM game_item_keys k WHERE k.item_instance_id=e.item_instance_id) AS key,
   (SELECT to_jsonb(t) FROM game_item_timed_states t WHERE t.item_instance_id=e.item_instance_id) AS timed,
   (SELECT jsonb_build_object('sha256',encode(sha256(convert_to(to_jsonb(t)::text,'UTF8')),'hex')) FROM game_item_texts t WHERE t.item_instance_id=e.item_instance_id) AS text,
   (SELECT to_jsonb(t) FROM game_item_text_revisions t WHERE t.item_instance_id=e.item_instance_id) AS text_revision,
   NULLIF(jsonb_strip_nulls(jsonb_build_object(
    'fluid',(SELECT to_jsonb(f) FROM game_item_fluid_states f WHERE f.item_instance_id=e.item_instance_id),
    'tier',(SELECT to_jsonb(f) FROM game_item_tiers f WHERE f.item_instance_id=e.item_instance_id),
    'imbuements',(SELECT jsonb_agg(to_jsonb(b) ORDER BY b.slot) FROM game_item_imbuements b WHERE b.item_instance_id=e.item_instance_id)
   )), '{}'::jsonb) AS attributes
  FROM edges e LEFT JOIN game_item_instances i USING(item_instance_id)
  LEFT JOIN game_item_nested_entries n USING(item_instance_id)
 )
 SELECT coalesce(jsonb_agg(jsonb_build_object('item',item,'entry',entry,'key',key,
   'timed',timed,'text',text,'text_revision',text_revision)
   || CASE WHEN attributes IS NULL THEN '{}'::jsonb ELSE jsonb_build_object('attributes',attributes) END
   ORDER BY depth,parent_item_instance_id,placement_ordinal DESC),'[]'::jsonb),
  coalesce(bool_or(cyclic OR depth>7 OR source_kind<>2 OR item IS NULL OR entry IS NULL
   OR world_id<>root_world OR (item->>'lifecycle')::INT<>1 OR (item->>'world_id')::UUID<>world_id),false), count(*)
 INTO result,invalid,size FROM snapshots;
 IF invalid OR size>=100 OR size<>(SELECT count(DISTINCT value->'item'->>'item_instance_id') FROM jsonb_array_elements(result)) THEN
  RAISE EXCEPTION 'invalid player trade contents' USING ERRCODE='23514';
 END IF;
 RETURN result;
END $$;
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_player_trade_contents(UUID) SET search_path = pg_catalog, %I, pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_player_trade_contents(UUID) FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
