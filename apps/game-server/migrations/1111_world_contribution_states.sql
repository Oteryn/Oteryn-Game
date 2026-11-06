-- WCONTRIB: immutable shared effects chosen by the existing server-save owner.
CREATE TABLE game_world_change_states (
    world_id UUID NOT NULL,
    reset_epoch NUMERIC(20,0) NOT NULL,
    state JSONB NOT NULL CHECK (
        jsonb_typeof(state) = 'object' AND octet_length(state::text) <= 4096
    ),
    PRIMARY KEY (world_id, reset_epoch),
    FOREIGN KEY (world_id, reset_epoch) REFERENCES game_world_change_days
);
CREATE TRIGGER game_world_change_states_immutable BEFORE UPDATE OR DELETE
 ON game_world_change_states FOR EACH ROW EXECUTE FUNCTION game_world_reset_history_immutable();

REVOKE ALL ON game_world_change_states FROM PUBLIC;
GRANT SELECT, INSERT ON game_world_change_states TO oteryn_game_runtime;
GRANT SELECT ON game_world_change_states TO oteryn_game_control;
