-- Crystal201189f brutus_the_baker.lua: annual, independent furniture/cupcake
-- choices. The current Character owner applies the normal audited MINT and
-- acknowledges its entitlement in the SAME physical type-2 transaction.
DO $$
DECLARE body TEXT; original TEXT;
BEGIN
 SELECT pg_get_functiondef('game_cake_reward_record(uuid,uuid,integer,uuid,smallint)'::regprocedure) INTO original;
 body := replace(original,
  'claim:=CASE p_category WHEN 1 THEN ''oteryn:reward-claim.cake-brutus-item'' ELSE ''oteryn:reward-claim.cake-brutus-cupcake'' END;',
  'claim:=(CASE p_category WHEN 1 THEN ''oteryn:reward-claim.cake-brutus-item'' ELSE ''oteryn:reward-claim.cake-brutus-cupcake'' END)||''.''||p_year::text;');
 IF body=original THEN RAISE EXCEPTION 'unexpected Cake reward owner definition'; END IF;
 EXECUTE body;
 SELECT pg_get_functiondef('game_cake_lock_entitlement(uuid,integer,uuid,smallint)'::regprocedure) INTO original;
 body := replace(original, 'OR NOT p.brutus_present OR p.event_year',
  'OR NOT p.brutus_present OR p.event_end_cleaned_up OR p.event_year');
 IF body=original THEN RAISE EXCEPTION 'unexpected Cake entitlement definition'; END IF;
 EXECUTE body;
END $$;
REVOKE ALL ON FUNCTION game_cake_reward_record(UUID,UUID,INTEGER,UUID,SMALLINT),game_cake_lock_entitlement(UUID,INTEGER,UUID,SMALLINT) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_cake_reward_record(UUID,UUID,INTEGER,UUID,SMALLINT),game_cake_lock_entitlement(UUID,INTEGER,UUID,SMALLINT) TO oteryn_game_runtime;

CREATE FUNCTION game_cake_brutus_status(p_world UUID,p_character UUID,p_start BOOLEAN)
RETURNS TABLE(event_year INTEGER,cities SMALLINT,was_started BOOLEAN,item_claimed BOOLEAN,cupcake_claimed BOOLEAN)
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE p game_cake_phases; e game_cake_errands;
BEGIN
 SELECT * INTO p FROM game_cake_phases f WHERE f.world_id=p_world AND f.event_year=
  (SELECT max(w.event_year) FROM game_cake_world_progress w WHERE w.world_id=p_world) FOR UPDATE;
 IF NOT FOUND OR p.stage<>3 OR NOT p.brutus_present OR p.event_end_cleaned_up THEN RETURN; END IF;
 SELECT * INTO e FROM game_cake_errands f WHERE f.world_id=p_world AND f.event_year=p.event_year AND f.character_id=p_character FOR UPDATE;
 event_year:=p.event_year; cities:=COALESCE(e.cities,0); was_started:=COALESCE(e.started,FALSE);
 item_claimed:=COALESCE(e.item_claimed,FALSE); cupcake_claimed:=COALESCE(e.cupcake_claimed,FALSE);
 IF p_start AND cities<>31 AND NOT was_started THEN
  INSERT INTO game_cake_errands(world_id,event_year,character_id,started)
   VALUES(p_world,p.event_year,p_character,TRUE)
   ON CONFLICT ON CONSTRAINT game_cake_errands_pkey DO UPDATE SET started=TRUE;
 END IF;
 RETURN NEXT;
END $$;
REVOKE ALL ON FUNCTION game_cake_brutus_status(UUID,UUID,BOOLEAN) FROM PUBLIC;
DO $$ BEGIN EXECUTE format('ALTER FUNCTION game_cake_brutus_status(UUID,UUID,BOOLEAN) SET search_path=pg_catalog,%I,pg_temp',current_schema()); END $$;
GRANT EXECUTE ON FUNCTION game_cake_brutus_status(UUID,UUID,BOOLEAN) TO oteryn_game_runtime;
