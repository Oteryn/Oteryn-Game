CREATE TABLE game_team_finder_leader_listings (
    character_id UUID PRIMARY KEY REFERENCES game_character_roots(character_id) ON DELETE CASCADE,
    min_level INTEGER NOT NULL CHECK (min_level BETWEEN 1 AND 65535),
    max_level INTEGER NOT NULL CHECK (max_level BETWEEN min_level AND 65535),
    vocation_mask SMALLINT NOT NULL CHECK (vocation_mask BETWEEN 1 AND 255),
    team_slots INTEGER NOT NULL CHECK (team_slots BETWEEN 1 AND 65535),
    free_slots INTEGER NOT NULL CHECK (free_slots BETWEEN 1 AND team_slots),
    team_type SMALLINT NOT NULL CHECK (team_type BETWEEN 1 AND 3),
    target_id INTEGER NOT NULL CHECK (target_id BETWEEN 0 AND 65535),
    hunt_area INTEGER CHECK (hunt_area BETWEEN 0 AND 65535),
    listed_at BIGINT NOT NULL CHECK (listed_at BETWEEN 0 AND 4294967295),
    CHECK ((team_type = 2) = (hunt_area IS NOT NULL))
);
REVOKE ALL ON game_team_finder_leader_listings FROM PUBLIC;
GRANT SELECT, INSERT, UPDATE, DELETE ON game_team_finder_leader_listings TO oteryn_game_runtime;
GRANT SELECT ON game_team_finder_leader_listings TO oteryn_game_control;
