-- Source-qualified shared charge windows are changed by the Channel quest writer.
-- A charge debit commits alongside the selected tool burn and Character quest receipt.
CREATE TABLE game_quest_world_charge_windows (
    world_id UUID NOT NULL,
    map_revision TEXT NOT NULL CHECK (octet_length(map_revision) BETWEEN 1 AND 128),
    reset_epoch NUMERIC(20,0) NOT NULL CHECK (reset_epoch BETWEEN 0 AND 18446744073709551615),
    charge_group TEXT NOT NULL CHECK (octet_length(charge_group) BETWEEN 1 AND 128),
    opening_session UUID NOT NULL,
    opening_command NUMERIC(20,0) NOT NULL CHECK (opening_command BETWEEN 1 AND 18446744073709551615),
    ready_at BIGINT NOT NULL CHECK (ready_at > 0),
    expires_at BIGINT NOT NULL CHECK (expires_at > ready_at),
    remaining SMALLINT NOT NULL CHECK (remaining BETWEEN 0 AND 64),
    PRIMARY KEY (world_id, map_revision, reset_epoch, charge_group)
);
REVOKE ALL ON game_quest_world_charge_windows FROM PUBLIC;
GRANT SELECT, INSERT, UPDATE ON game_quest_world_charge_windows TO oteryn_game_runtime;
