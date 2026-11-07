-- S14a: the leader's Team Finder listing (Crystal 201189f8 protocolgame.cpp:2973-3036).
-- One listing per leader character, kept across logout; browse/join (S14b) and
-- accept/deny (S14c) reference it by leader_character_id.
CREATE TABLE IF NOT EXISTS game_team_finder_leader_listings (
    leader_character_id UUID PRIMARY KEY REFERENCES game_character_roots(character_id) ON DELETE CASCADE,
    min_level INTEGER NOT NULL CHECK (min_level BETWEEN 1 AND 65535),
    max_level INTEGER NOT NULL CHECK (max_level BETWEEN min_level AND 65535),
    vocation_mask SMALLINT NOT NULL CHECK (vocation_mask BETWEEN 1 AND 255),
    team_slots INTEGER NOT NULL CHECK (team_slots BETWEEN 1 AND 65535),
    free_slots INTEGER NOT NULL CHECK (free_slots BETWEEN 0 AND team_slots),
    party_members_join BOOLEAN NOT NULL,
    client_timestamp BIGINT NOT NULL CHECK (client_timestamp BETWEEN 0 AND 4294967295),
    team_type SMALLINT NOT NULL CHECK (team_type BETWEEN 1 AND 3),
    target_id INTEGER NOT NULL CHECK (target_id BETWEEN 0 AND 65535),
    hunt_area INTEGER CHECK (hunt_area BETWEEN 0 AND 65535),
    CHECK ((team_type = 2) = (hunt_area IS NOT NULL))
);
REVOKE ALL ON game_team_finder_leader_listings FROM PUBLIC;
GRANT SELECT, INSERT, UPDATE, DELETE ON game_team_finder_leader_listings TO oteryn_game_runtime;
GRANT SELECT ON game_team_finder_leader_listings TO oteryn_game_control;

-- A replayed command returns its first result and never re-applies an older edit.
CREATE TABLE IF NOT EXISTS game_team_finder_leader_receipts (
    game_session_id UUID NOT NULL,
    command_id NUMERIC(20,0) NOT NULL CHECK (command_id > 0),
    leader_character_id UUID NOT NULL REFERENCES game_character_roots(character_id) ON DELETE CASCADE,
    accepted BOOLEAN NOT NULL,
    PRIMARY KEY (game_session_id, command_id)
);
CREATE INDEX IF NOT EXISTS game_team_finder_leader_receipts_leader ON game_team_finder_leader_receipts(leader_character_id);
REVOKE ALL ON game_team_finder_leader_receipts FROM PUBLIC;
GRANT SELECT, INSERT ON game_team_finder_leader_receipts TO oteryn_game_runtime;
GRANT SELECT ON game_team_finder_leader_receipts TO oteryn_game_control;
