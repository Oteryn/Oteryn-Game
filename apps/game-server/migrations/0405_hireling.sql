CREATE TABLE game_house_hirelings (
    world_id uuid NOT NULL,
    house_key text NOT NULL,
    owner_character_id uuid NOT NULL REFERENCES game_character_roots(character_id),
    x integer NOT NULL CHECK (x BETWEEN 0 AND 65535),
    y integer NOT NULL CHECK (y BETWEEN 0 AND 65535),
    floor smallint NOT NULL CHECK (floor BETWEEN -15 AND 0),
    PRIMARY KEY(world_id, house_key)
);
CREATE TABLE game_house_hireling_receipts (
    game_session_id uuid NOT NULL,
    command_id numeric(20,0) NOT NULL CHECK(command_id > 0),
    active boolean NOT NULL,
    x integer NOT NULL,
    y integer NOT NULL,
    floor smallint NOT NULL,
    PRIMARY KEY(game_session_id, command_id)
);
REVOKE ALL ON game_house_hirelings, game_house_hireling_receipts FROM PUBLIC;
GRANT SELECT, INSERT, UPDATE, DELETE ON game_house_hirelings TO oteryn_game_runtime;
GRANT SELECT, INSERT ON game_house_hireling_receipts TO oteryn_game_runtime;
GRANT SELECT ON game_house_hirelings, game_house_hireling_receipts TO oteryn_game_control;
