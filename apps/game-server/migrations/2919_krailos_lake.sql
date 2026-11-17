CREATE TABLE game_krailos_lake (
    world_id uuid PRIMARY KEY,
    reset_epoch numeric(20, 0) NOT NULL CHECK (reset_epoch BETWEEN 0 AND 18446744073709551615),
    progress smallint NOT NULL CHECK (progress BETWEEN 0 AND 36)
);

REVOKE ALL ON game_krailos_lake FROM PUBLIC;
GRANT SELECT, INSERT, UPDATE ON game_krailos_lake TO oteryn_game_runtime;
GRANT SELECT ON game_krailos_lake TO oteryn_game_control;

CREATE TABLE game_krailos_captured_corpses (
    item_instance_id uuid PRIMARY KEY REFERENCES game_item_instances ON DELETE CASCADE
);

REVOKE ALL ON game_krailos_captured_corpses FROM PUBLIC;
GRANT SELECT, INSERT ON game_krailos_captured_corpses TO oteryn_game_runtime;
GRANT SELECT ON game_krailos_captured_corpses TO oteryn_game_control;
