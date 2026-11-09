-- T135: Canary 99902524 game.cpp:5593-5621 stores a wrapped cask's
-- remaining amount; unwrapping restores that amount, never a fresh full charge.
-- Keep Oteryn's existing timed row on the same item identity. This path writes
-- no timed state and grants no charge refill, duration reset or deadline change.
CREATE FUNCTION game_house_charged_wrap_preserves(i UUID,b TEXT,a TEXT,k SMALLINT)
RETURNS BOOLEAN LANGUAGE sql STABLE STRICT SECURITY DEFINER AS $$
 SELECT ((k=1 AND a='oteryn:item.tibia.i23398' AND game_house_native_wrap_eligible(b))
  OR (k=2 AND b='oteryn:item.tibia.i23398' AND game_house_native_wrap_eligible(a)))
  AND EXISTS(SELECT 1 FROM game_item_timed_states t WHERE t.item_instance_id=i
   AND t.charges IS NOT NULL AND t.remaining_ms IS NULL AND t.deadline_at IS NULL)
$$;
DO $patch$
DECLARE body TEXT;needle TEXT;
BEGIN
 body:=pg_get_functiondef('game_house_furniture_wrap(uuid,uuid,numeric,numeric,numeric,uuid,numeric,uuid,text,numeric,uuid,numeric,numeric,smallint,bytea,bytea,text,integer,text)'::regprocedure);
 needle:='AND NOT game_house_timed_decoration_wrap_pair(i.definition_production_key,p_after,p_action)';
 IF strpos(body,needle)=0 THEN RAISE EXCEPTION 'missing charged furniture admission seam'; END IF;
 EXECUTE replace(body,needle,needle||' AND NOT game_house_charged_wrap_preserves(p_item,i.definition_production_key,p_after,p_action)');
 EXECUTE format('ALTER FUNCTION game_house_charged_wrap_preserves(UUID,TEXT,TEXT,SMALLINT) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $patch$;
REVOKE ALL ON FUNCTION game_house_charged_wrap_preserves(UUID,TEXT,TEXT,SMALLINT)
 FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
