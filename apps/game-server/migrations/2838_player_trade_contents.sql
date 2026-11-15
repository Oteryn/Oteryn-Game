-- An offered container retains its nested edges: moving the root moves custody
-- of the whole tree. Freeze and conserve every descendant's complete snapshot.
ALTER TABLE game_player_trade_receipts ADD COLUMN before_contents JSONB NOT NULL DEFAULT '[]';

CREATE FUNCTION game_player_trade_contents(p_root UUID) RETURNS JSONB
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
   (SELECT to_jsonb(t) FROM game_item_text_revisions t WHERE t.item_instance_id=e.item_instance_id) AS text_revision
  FROM edges e LEFT JOIN game_item_instances i USING(item_instance_id)
  LEFT JOIN game_item_nested_entries n USING(item_instance_id)
 )
 SELECT coalesce(jsonb_agg(jsonb_build_object('item',item,'entry',entry,'key',key,
   'timed',timed,'text',text,'text_revision',text_revision)
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

-- Preserve the current proof chains and their generation/account/custody checks.
-- Replace only the old nested-leaf restriction, adding complete before and after
-- tree equality to the existing fresh receipt and deferred conservation guards.
DO $migration$
DECLARE body TEXT; needle TEXT;
BEGIN
 body:=pg_get_functiondef('game_player_trade_line_admit()'::regprocedure);
 needle:='    OR EXISTS(SELECT 1 FROM game_item_nested_entries WHERE parent_item_instance_id=i.item_instance_id)';
 IF position(needle IN body)=0 THEN RAISE EXCEPTION 'missing player trade leaf guard'; END IF;
 body:=replace(body,needle,'    OR game_player_trade_contents(i.item_instance_id) IS DISTINCT FROM NEW.before_contents');
 EXECUTE body;
 body:=pg_get_functiondef('game_player_trade_complete(UUID)'::regprocedure);
 needle:='  IF l.created_xact_id<>pg_current_xact_id()';
 IF position(needle IN body)=0 THEN RAISE EXCEPTION 'missing player trade conservation guard'; END IF;
 body:=replace(body,needle,needle||E'\n     OR game_player_trade_contents(l.item_instance_id) IS DISTINCT FROM l.before_contents');
 EXECUTE body;
END $migration$;

