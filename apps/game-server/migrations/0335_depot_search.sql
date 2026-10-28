-- DEPOT-SEARCH-1 extends migrations 0151/0189 without another custody or stash owner.
-- Support Character-scoped custody scans without walking every World's holdings.
-- No runtime privileges, value writes or receipt semantics change.
CREATE INDEX IF NOT EXISTS game_depot_search_character_world
    ON game_item_character_depot_locations (character_id, world_id, item_instance_id);
CREATE INDEX IF NOT EXISTS game_inbox_search_character_world
    ON game_item_character_inbox_locations (character_id, world_id, item_instance_id);
