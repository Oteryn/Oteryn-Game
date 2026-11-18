-- One independently expiring replica mechanism of each kind per World.
CREATE TABLE game_forgotten_knowledge_replica_windows (
    world_id uuid NOT NULL,
    replica_kind smallint NOT NULL CHECK (replica_kind IN (1, 2)),
    expires_at timestamptz NOT NULL,
    kills integer NOT NULL DEFAULT 0 CHECK (kills BETWEEN 0 AND 5),
    PRIMARY KEY (world_id, replica_kind)
);
REVOKE ALL ON game_forgotten_knowledge_replica_windows FROM PUBLIC;
GRANT SELECT, INSERT, UPDATE ON game_forgotten_knowledge_replica_windows TO oteryn_game_runtime;
GRANT SELECT ON game_forgotten_knowledge_replica_windows TO oteryn_game_control;

-- Only one selected-phial activation can own the tree per World.
CREATE TABLE game_forgotten_knowledge_tree_windows (
    world_id uuid PRIMARY KEY,
    expires_at timestamptz NOT NULL
);
REVOKE ALL ON game_forgotten_knowledge_tree_windows FROM PUBLIC;
GRANT SELECT, INSERT, UPDATE ON game_forgotten_knowledge_tree_windows TO oteryn_game_runtime;
GRANT SELECT ON game_forgotten_knowledge_tree_windows TO oteryn_game_control;
