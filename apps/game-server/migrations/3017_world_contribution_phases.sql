-- Separate ship fuel from the shared fuel pool of the three Awash pumps.
ALTER TABLE game_world_change_tallies
    DROP CONSTRAINT game_world_change_tallies_resource_check,
    ADD CHECK (resource IN ('awash_coal','coal','medicine','votes'));
ALTER TABLE game_world_change_contributions
    DROP CONSTRAINT game_world_change_contributions_resource_check,
    ADD CHECK (resource IN ('awash_coal','coal','medicine','votes'));

-- The save decision is immutable. Live contributions remain in the existing ledger.
CREATE TABLE game_world_contribution_days (
    world_id UUID NOT NULL,
    reset_epoch NUMERIC(20,0) NOT NULL,
    ship_carry BIGINT NOT NULL CHECK (ship_carry >= 0),
    ship_saves_remaining SMALLINT NOT NULL CHECK (ship_saves_remaining BETWEEN 0 AND 7),
    pump_carry BIGINT NOT NULL CHECK (pump_carry BETWEEN 0 AND 201),
    awash_drained BOOLEAN NOT NULL,
    PRIMARY KEY (world_id, reset_epoch),
    FOREIGN KEY (world_id, reset_epoch) REFERENCES game_world_change_days
);

CREATE TRIGGER game_world_contribution_days_immutable BEFORE UPDATE OR DELETE
    ON game_world_contribution_days FOR EACH ROW EXECUTE FUNCTION game_world_reset_history_immutable();

REVOKE ALL ON game_world_contribution_days FROM PUBLIC;
GRANT SELECT, INSERT ON game_world_contribution_days TO oteryn_game_runtime;
GRANT SELECT ON game_world_contribution_days TO oteryn_game_control;
