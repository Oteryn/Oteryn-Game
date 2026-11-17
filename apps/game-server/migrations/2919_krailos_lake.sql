CREATE TABLE game_krailos_lake (
    world_id uuid PRIMARY KEY,
    reset_epoch numeric(20, 0) NOT NULL CHECK (reset_epoch >= 0),
    progress smallint NOT NULL CHECK (progress BETWEEN 0 AND 36)
);

REVOKE ALL ON game_krailos_lake FROM PUBLIC;
GRANT SELECT, INSERT, UPDATE ON game_krailos_lake TO oteryn_game_runtime;
GRANT SELECT ON game_krailos_lake TO oteryn_game_control;
