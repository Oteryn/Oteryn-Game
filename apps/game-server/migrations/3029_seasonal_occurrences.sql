-- Seasonal activation belongs to the same transaction as the server save.
CREATE TABLE game_world_seasonal_occurrences (
    world_id UUID NOT NULL,
    reset_epoch NUMERIC(20,0) NOT NULL,
    event_key TEXT NOT NULL CHECK (event_key IN (
        'kingsday', 'last_creep_standing', 'spring_into_life', 'tinder_box'
    )),
    occurrence_id BYTEA NOT NULL CHECK (octet_length(occurrence_id) = 32),
    PRIMARY KEY (world_id, reset_epoch, event_key),
    FOREIGN KEY (world_id, reset_epoch) REFERENCES game_world_event_epochs
);

CREATE TRIGGER game_world_seasonal_occurrences_immutable BEFORE UPDATE OR DELETE
    ON game_world_seasonal_occurrences FOR EACH ROW EXECUTE FUNCTION game_world_reset_history_immutable();

REVOKE ALL ON game_world_seasonal_occurrences FROM PUBLIC;
GRANT SELECT, INSERT ON game_world_seasonal_occurrences TO oteryn_game_runtime;
GRANT SELECT ON game_world_seasonal_occurrences TO oteryn_game_control;
