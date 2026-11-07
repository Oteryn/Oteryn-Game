DO $migration$ DECLARE body TEXT; needle TEXT; BEGIN
 body:=pg_get_functiondef('game_cake_world_state_valid(jsonb)'::regprocedure);
 needle:='BETWEEN 0 AND 1116'; IF strpos(body,needle)=0 THEN RAISE EXCEPTION 'Cake goal state anchor changed'; END IF;
 EXECUTE replace(body,needle,'BETWEEN 0 AND 1066');
 body:=pg_get_functiondef('game_cake_apply_terrain_tool_transform(uuid,numeric,uuid,uuid,bytea,uuid,uuid,bytea,text,bytea,bigint,bigint,bigint,bigint,bigint[],bigint[])'::regprocedure);
 needle:=$text$(w.state->>'layer_progress')::integer>=1116$text$;
 IF strpos(body,needle)=0 THEN RAISE EXCEPTION 'Cake CAS goal anchor changed'; END IF;
 body:=replace(body,needle,$text$(w.state->>'layer_progress')::integer>=game_cake_edible_goal()$text$);
 needle:=$text$ OR ((w.state->>'window_end') IS NOT NULL AND c.now_seconds>=(w.state->>'window_end')::bigint)$text$;
 IF strpos(body,needle)=0 THEN RAISE EXCEPTION 'Cake CAS fixed-window anchor changed'; END IF;
 body:=replace(body,needle,$text$ OR (w.state->>'window_start') IS NULL OR (w.state->>'window_end') IS NULL OR c.now_seconds<(w.state->>'window_start')::bigint OR c.now_seconds>=(w.state->>'window_end')::bigint$text$);
 needle:=' SELECT * INTO STRICT w FROM game_seasonal_cake_worlds';
 IF strpos(body,needle)=0 THEN RAISE EXCEPTION 'Cake CAS footprint anchor changed'; END IF;
 body:=replace(body,needle,$guard$ IF NOT EXISTS(SELECT 1 FROM game_cake_edible_cells cell WHERE cell.x=get_byte(p_placement,34)*256+get_byte(p_placement,35)
  AND cell.y=get_byte(p_placement,36)*256+get_byte(p_placement,37) AND cell.appearance=p_before_item_id AND cell.stack=get_byte(p_placement,39)) THEN
 RAISE EXCEPTION 'Cake source is not an actual edible floor7 cell' USING ERRCODE='23514'; END IF;
 SELECT * INTO STRICT w FROM game_seasonal_cake_worlds$guard$);
 EXECUTE body;
END $migration$;
