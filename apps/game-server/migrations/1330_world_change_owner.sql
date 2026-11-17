-- WORLDCHG-Q extends the existing reset/event owner; no second reset scheduler.
CREATE TABLE game_world_change_days (
    world_id UUID NOT NULL,
    reset_epoch NUMERIC(20,0) NOT NULL,
    decision JSONB NOT NULL CHECK (jsonb_typeof(decision) = 'object' AND octet_length(decision::text) <= 1024),
    PRIMARY KEY (world_id, reset_epoch),
    FOREIGN KEY (world_id, reset_epoch) REFERENCES game_world_reset_epochs
);
CREATE TRIGGER game_world_change_days_immutable BEFORE UPDATE OR DELETE
 ON game_world_change_days FOR EACH ROW EXECUTE FUNCTION game_world_reset_history_immutable();

CREATE TABLE game_world_change_tallies (
    world_id UUID NOT NULL,
    reset_epoch NUMERIC(20,0) NOT NULL,
    resource TEXT NOT NULL CHECK (resource IN ('coal','medicine','votes')),
    tally NUMERIC(20,0) NOT NULL CHECK (tally BETWEEN 0 AND 18446744073709551615),
    PRIMARY KEY (world_id, reset_epoch, resource),
    FOREIGN KEY (world_id, reset_epoch) REFERENCES game_world_change_days
);
CREATE TABLE game_world_change_contributions (
    world_id UUID NOT NULL,
    reset_epoch NUMERIC(20,0) NOT NULL,
    operation_id BYTEA NOT NULL CHECK (octet_length(operation_id) = 16),
    resource TEXT NOT NULL CHECK (resource IN ('coal','medicine','votes')),
    amount BIGINT NOT NULL CHECK (amount BETWEEN 1 AND 4294967295),
    resulting_tally NUMERIC(20,0) NOT NULL CHECK (resulting_tally BETWEEN 1 AND 18446744073709551615),
    PRIMARY KEY (world_id, reset_epoch, operation_id),
    FOREIGN KEY (world_id, reset_epoch) REFERENCES game_world_change_days
);
CREATE TRIGGER game_world_change_contributions_immutable BEFORE UPDATE OR DELETE
 ON game_world_change_contributions FOR EACH ROW EXECUTE FUNCTION game_world_reset_history_immutable();

REVOKE ALL ON game_world_change_days, game_world_change_tallies, game_world_change_contributions FROM PUBLIC;
GRANT SELECT, INSERT ON game_world_change_days, game_world_change_contributions TO oteryn_game_runtime;
GRANT SELECT, INSERT, UPDATE ON game_world_change_tallies TO oteryn_game_runtime;
GRANT SELECT ON game_world_change_days, game_world_change_tallies, game_world_change_contributions TO oteryn_game_control;
