-- Crystal game.cpp2762 and items.cpp237-242: ordinary NPCs also deliver
-- official wrappable shop targets as23398, retaining the exact original target.
ALTER TABLE game_house_furniture_kit_catalogue
 DROP CONSTRAINT game_house_furniture_kit_catalogue_source_row_check,
 DROP CONSTRAINT game_house_furniture_kit_catalogue_target_key_key,
 ADD COLUMN npc_key TEXT CHECK(npc_key IS NULL OR npc_key ~ '^oteryn:npc[.][a-z0-9_]+$'),
 ADD CONSTRAINT game_house_furniture_kit_catalogue_source_row_check
 CHECK(source_row BETWEEN 0 AND 1674 OR source_row BETWEEN 1000000 AND __NPC_FURNITURE_LAST__);
INSERT INTO game_house_furniture_kit_catalogue(source_row,target_key,price_gold,source_line,npc_key) VALUES
__NPC_FURNITURE_ROWS__;
DO $migration$ DECLARE body TEXT; needle TEXT; BEGIN
 body:=pg_get_functiondef('game_house_furniture_kit_origin_proven()'::regprocedure);
 needle:=$old$AND t.npc_key ~ '^oteryn:npc\.hireling:[0-9a-f]{32}$' AND t.house_key IS NOT NULL$old$;
 IF position(needle IN body)=0 THEN RAISE EXCEPTION 'SRV7 missing original NPC-kit origin proof'; END IF;
 EXECUTE replace(body,needle,$new$AND ((c.npc_key IS NULL AND t.npc_key ~ '^oteryn:npc\.hireling:[0-9a-f]{32}$' AND t.house_key IS NOT NULL)
  OR (c.npc_key IS NOT NULL AND t.npc_key=c.npc_key AND t.house_key IS NULL))$new$);
END $migration$;
