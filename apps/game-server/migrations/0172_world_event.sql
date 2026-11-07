-- EVENTS-WORLD-1 basic reset-selected announcement. Installation activates nothing.
CREATE TABLE game_world_event_epochs (
    world_id UUID NOT NULL,
    reset_epoch NUMERIC(20,0) NOT NULL CHECK (reset_epoch BETWEEN 1 AND 18446744073709551615),
    active BOOLEAN NOT NULL,
    announcement TEXT NOT NULL CHECK (octet_length(announcement) <= 128),
    CHECK (active = (announcement <> '')),
    PRIMARY KEY (world_id, reset_epoch),
    FOREIGN KEY (world_id, reset_epoch) REFERENCES game_world_reset_epochs
);

CREATE TABLE game_world_event_query_receipts (
    game_session_id UUID NOT NULL REFERENCES game_durability_reconnect_sessions(game_session_id),
    command_id NUMERIC(20,0) NOT NULL CHECK (command_id BETWEEN 1 AND 18446744073709551615),
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    world_id UUID NOT NULL CHECK (game_node_is_uuid_v7(world_id)),
    result BYTEA NOT NULL CHECK (octet_length(result) <= 144),
    PRIMARY KEY (game_session_id, command_id)
);

CREATE TRIGGER game_world_event_epochs_immutable BEFORE UPDATE OR DELETE
    ON game_world_event_epochs FOR EACH ROW EXECUTE FUNCTION game_world_reset_history_immutable();
CREATE TRIGGER game_world_event_query_receipts_immutable BEFORE UPDATE OR DELETE
    ON game_world_event_query_receipts FOR EACH ROW EXECUTE FUNCTION game_world_reset_history_immutable();

REVOKE ALL ON game_world_event_epochs, game_world_event_query_receipts FROM PUBLIC;
GRANT SELECT, INSERT ON game_world_event_epochs, game_world_event_query_receipts TO oteryn_game_runtime;
GRANT SELECT ON game_world_event_epochs TO oteryn_game_control;
