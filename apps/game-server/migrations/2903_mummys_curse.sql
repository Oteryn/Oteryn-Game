CREATE TABLE game_mummys_curse_progress (
    world_id uuid PRIMARY KEY,
    broken_jars smallint NOT NULL DEFAULT 0 CHECK (broken_jars BETWEEN 0 AND 5)
);

CREATE TABLE game_mummys_curse_retries (
    world_id uuid NOT NULL REFERENCES game_mummys_curse_progress(world_id),
    character_id uuid NOT NULL,
    action_id integer NOT NULL CHECK (action_id BETWEEN 50006 AND 50009),
    retry_at bigint NOT NULL CHECK (retry_at >= 0),
    PRIMARY KEY (world_id, character_id, action_id)
);

REVOKE ALL ON game_mummys_curse_progress, game_mummys_curse_retries FROM PUBLIC;
GRANT SELECT, INSERT, UPDATE ON game_mummys_curse_progress, game_mummys_curse_retries TO oteryn_game_runtime;
GRANT SELECT ON game_mummys_curse_progress, game_mummys_curse_retries TO oteryn_game_control;
