-- Crystal 201189f8: data-global/scripts/quests/forgotten_knowledge/actions_lantern.lua:1-4.
-- Admit the two explicit lantern pairs through the existing House custody guard.
ALTER FUNCTION game_item_action_house_pair(TEXT,TEXT) RENAME TO game_item_action_house_balloon_pair;

CREATE FUNCTION game_item_action_house_pair(p_before TEXT,p_after TEXT) RETURNS BOOLEAN
LANGUAGE sql IMMUTABLE STRICT AS $$
 SELECT (p_before,p_after) IN (
    ('oteryn:item.tibia.i23734','oteryn:item.tibia.i23738'),
    ('oteryn:item.tibia.i23738','oteryn:item.tibia.i23734')
 ) OR game_item_action_house_balloon_pair(p_before,p_after)
$$;

DO $migration$
BEGIN
 EXECUTE format('ALTER FUNCTION game_item_action_house_pair(TEXT,TEXT) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $migration$;

REVOKE ALL ON FUNCTION game_item_action_house_pair(TEXT,TEXT) FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
