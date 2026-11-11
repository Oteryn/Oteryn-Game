-- Bind physical native identities to the immutable daily decision in the same reset transaction.
-- Existing decisions are upgraded from their stored bytes, never redrawn.
ALTER TABLE game_world_change_days
    ADD COLUMN native_binding_required BOOLEAN NOT NULL DEFAULT FALSE;
-- Legacy writers can finish a save during rollout. The new owner explicitly
-- inserts TRUE and captures the binding in the same transaction.

CREATE TABLE game_world_change_native_bindings (
    world_id UUID NOT NULL,
    reset_epoch NUMERIC(20,0) NOT NULL,
    bindings BYTEA NOT NULL CHECK (octet_length(bindings) BETWEEN 1 AND 65536),
    bindings_digest BYTEA NOT NULL CHECK (octet_length(bindings_digest) = 32),
    PRIMARY KEY (world_id, reset_epoch),
    FOREIGN KEY (world_id, reset_epoch) REFERENCES game_world_change_days
);
CREATE TRIGGER game_world_change_native_bindings_immutable BEFORE UPDATE OR DELETE
 ON game_world_change_native_bindings FOR EACH ROW EXECUTE FUNCTION game_world_reset_history_immutable();

-- A legacy decision can acquire its binding requirement once. The immutable
-- World/epoch/decision itself remains protected, including against DELETE.
CREATE FUNCTION game_world_change_days_binding_upgrade() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'UPDATE' AND NOT OLD.native_binding_required AND NEW.native_binding_required
       AND (NEW.world_id, NEW.reset_epoch, NEW.decision) IS NOT DISTINCT FROM
           (OLD.world_id, OLD.reset_epoch, OLD.decision)
       AND EXISTS (SELECT 1 FROM game_world_change_native_bindings b
                   WHERE (b.world_id,b.reset_epoch)=(OLD.world_id,OLD.reset_epoch)) THEN
        RETURN NEW;
    END IF;
    RAISE EXCEPTION 'world change decision is immutable';
END $$;
REVOKE ALL ON FUNCTION game_world_change_days_binding_upgrade() FROM PUBLIC;
DROP TRIGGER game_world_change_days_immutable ON game_world_change_days;
CREATE TRIGGER game_world_change_days_immutable BEFORE UPDATE OR DELETE
 ON game_world_change_days FOR EACH ROW EXECUTE FUNCTION game_world_change_days_binding_upgrade();

REVOKE ALL ON game_world_change_native_bindings FROM PUBLIC;
GRANT SELECT, INSERT ON game_world_change_native_bindings TO oteryn_game_runtime;
GRANT UPDATE(native_binding_required) ON game_world_change_days TO oteryn_game_runtime;
GRANT SELECT ON game_world_change_native_bindings TO oteryn_game_control;
