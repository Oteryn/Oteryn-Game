-- The move transaction owns the claim generation. Existing moves keep their
-- original publication bytes when replayed.
ALTER TABLE game_item_loop_move_receipts ADD COLUMN inbox_recovery_generation NUMERIC(20,0)
    CHECK (inbox_recovery_generation BETWEEN 1 AND 18446744073709551615);
GRANT INSERT (inbox_recovery_generation) ON game_item_loop_move_receipts TO oteryn_game_runtime;

-- A restored claim may lose its local claim-operation mapping. Retain the
-- delivery identity alongside a new conflict so every Inbox route can find it.
ALTER TABLE game_store_restore_conflicts ADD COLUMN delivery_operation_id UUID
    CHECK (game_character_is_uuid_v7(delivery_operation_id));
CREATE INDEX game_store_restore_conflicts_delivery
    ON game_store_restore_conflicts(delivery_operation_id) WHERE delivery_operation_id IS NOT NULL;

-- Web Premium Time was already granted by Platform. Game records only the
-- registration decision so retries preserve its binding, generation and time.
CREATE TABLE game_store_premium_registrations (
    operation_id UUID PRIMARY KEY CHECK (game_character_is_uuid_v7(operation_id)),
    binding_digest BYTEA NOT NULL CHECK (octet_length(binding_digest)=32),
    recovery_generation NUMERIC(20,0) NOT NULL REFERENCES game_character_recovery_admissions,
    decided_at BIGINT NOT NULL CHECK (decided_at>0)
);
CREATE INDEX game_store_premium_registration_generation
    ON game_store_premium_registrations(recovery_generation);
REVOKE ALL ON game_store_premium_registrations FROM PUBLIC;
GRANT SELECT, INSERT ON game_store_premium_registrations TO oteryn_game_runtime, oteryn_game_control;
