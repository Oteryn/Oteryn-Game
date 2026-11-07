-- Exact Crystal201189 golden_outfit_display.lua1–18 and28; USE only.
CREATE TABLE game_house_golden_display_pairs(before_id INTEGER PRIMARY KEY,after_id INTEGER NOT NULL);
INSERT INTO game_house_golden_display_pairs VALUES
(31509,31500),(31500,31505),(31505,31513),(31513,31509),
(31510,31502),(31502,31506),(31506,31514),(31514,31510),
(31511,31503),(31503,31507),(31507,31515),(31515,31511),
(31512,31504),(31504,31508),(31508,31516),(31516,31512);
REVOKE ALL ON game_house_golden_display_pairs FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
DO $migration$ DECLARE body TEXT; needle TEXT; BEGIN
 body:=pg_get_functiondef('game_item_action_house_admit(game_item_consume_transform_records)'::regprocedure);
 needle:=$old$OR r.effects->>'kind' IS DISTINCT FROM 'house_transform'$old$;
 IF position(needle IN body)=0 THEN RAISE EXCEPTION 'SRV7 missing Golden display source action guard'; END IF;
 EXECUTE replace(body,needle,needle||$new$
 OR (coalesce((r.effects->>'house_rotate')::boolean,FALSE)=FALSE
  AND EXISTS(SELECT 1 FROM game_house_golden_display_pairs golden_pair
   WHERE r.before_item->>'definition_production_key'='oteryn:item.tibia.i'||golden_pair.before_id::text)
  AND NOT EXISTS(SELECT 1 FROM game_character_quest_tracks golden_track
   WHERE golden_track.character_id=r.character_id
    AND golden_track.quest_key='oteryn:quest.golden_outfits_quest'
    AND golden_track.track_key='oteryn:quest-progress/crystalserver/outfit_quest/golden_outfit'
    AND golden_track.value=3))$new$);
END $migration$;
