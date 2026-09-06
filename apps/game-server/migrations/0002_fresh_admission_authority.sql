-- Fresh admission extends the canonical session owner. No source truth is seeded.
-- All FKs are immediate: callers acquire their relation/key/row protections before L.
CREATE TABLE game_durability_fresh_admission_receipts (
    replay_key BYTEA PRIMARY KEY CHECK (octet_length(replay_key) = 33 AND get_byte(replay_key, 0) = 1),
    game_session_id UUID NOT NULL UNIQUE,
    account_id UUID NOT NULL,
    character_id UUID NOT NULL,
    world_id UUID NOT NULL,
    channel_id UUID NOT NULL,
    character_lease_generation NUMERIC(20, 0) NOT NULL CHECK (character_lease_generation BETWEEN 1 AND 18446744073709551615),
    scope_ownership_generation NUMERIC(20, 0) NOT NULL CHECK (scope_ownership_generation BETWEEN 1 AND 18446744073709551615),
    connection_generation NUMERIC(20, 0) NOT NULL CHECK (connection_generation = 1),
    transport_ref BYTEA NOT NULL CHECK (octet_length(transport_ref) = 16),
    semantic_version SMALLINT NOT NULL CHECK (semantic_version = 1),
    operation_json TEXT NOT NULL CHECK (jsonb_typeof(operation_json::jsonb) = 'object'),
    authorization_decided_at BIGINT NOT NULL CHECK (authorization_decided_at >= 0),
    UNIQUE (replay_key, game_session_id),
    CHECK ((get_byte(uuid_send(game_session_id), 6) >> 4) = 7 AND (get_byte(uuid_send(game_session_id), 8) & 192) = 128),
    CHECK ((get_byte(uuid_send(character_id), 6) >> 4) = 7 AND (get_byte(uuid_send(character_id), 8) & 192) = 128),
    CHECK ((get_byte(uuid_send(world_id), 6) >> 4) = 7 AND (get_byte(uuid_send(world_id), 8) & 192) = 128),
    CHECK ((get_byte(uuid_send(channel_id), 6) >> 4) = 7 AND (get_byte(uuid_send(channel_id), 8) & 192) = 128)
);

ALTER TABLE game_durability_reconnect_sessions
    ALTER COLUMN control_loss_epoch DROP NOT NULL,
    ALTER COLUMN original_grace_deadline DROP NOT NULL,
    ALTER COLUMN predecessor_generation DROP NOT NULL,
    ADD COLUMN fresh_replay_key BYTEA NULL,
    ADD CONSTRAINT game_fresh_session_origin FOREIGN KEY (fresh_replay_key, game_session_id)
        REFERENCES game_durability_fresh_admission_receipts (replay_key, game_session_id),
    ADD CONSTRAINT game_truthful_session_continuity CHECK (
        (control_loss_epoch IS NOT NULL AND original_grace_deadline IS NOT NULL AND predecessor_generation IS NOT NULL)
        OR
        (control_loss_epoch IS NULL AND original_grace_deadline IS NULL AND predecessor_generation IS NULL
         AND fresh_replay_key IS NOT NULL AND session_state IN (2, 3)
         AND prepared_attempt_ref IS NULL AND attempt_count = 0)
    );

-- Conflicting legacy rows stop migration; no winner is chosen or history rewritten.
CREATE UNIQUE INDEX game_durability_one_nonterminal_session_per_account
    ON game_durability_reconnect_sessions (account_id) WHERE session_state IN (1, 2);

ALTER TABLE game_durability_transport_ref_reservations
    ALTER COLUMN reconnect_attempt_ref DROP NOT NULL,
    ADD COLUMN reservation_owner SMALLINT NOT NULL DEFAULT 1,
    ADD COLUMN fresh_replay_key BYTEA NULL,
    ADD CONSTRAINT game_transport_exact_origin CHECK (
        (reservation_owner = 1 AND reconnect_attempt_ref IS NOT NULL AND fresh_replay_key IS NULL)
        OR
        (reservation_owner = 2 AND reconnect_attempt_ref IS NULL AND fresh_replay_key IS NOT NULL)
    ),
    ADD CONSTRAINT game_fresh_transport_origin FOREIGN KEY (fresh_replay_key, game_session_id)
        REFERENCES game_durability_fresh_admission_receipts (replay_key, game_session_id);

-- Immutable history retains every accepted decision and full effect, including denials.
-- Canonical domain-tagged keys are compared in full; advisory hashes never grant identity.
CREATE TABLE game_durability_admission_guard_history (
    guard_key BYTEA NOT NULL,
    publication_revision NUMERIC(20, 0) NOT NULL CHECK (publication_revision BETWEEN 1 AND 18446744073709551615),
    source_authority TEXT NOT NULL CHECK (length(source_authority) > 0),
    source_revision NUMERIC(20, 0) NOT NULL CHECK (source_revision BETWEEN 1 AND 18446744073709551615),
    decision_identity TEXT NOT NULL CHECK (length(decision_identity) > 0),
    change_json TEXT NOT NULL CHECK (jsonb_typeof(change_json::jsonb) = 'object'),
    PRIMARY KEY (guard_key, publication_revision),
    UNIQUE (guard_key, source_authority, source_revision),
    UNIQUE (guard_key, source_authority, decision_identity)
);

CREATE TABLE game_durability_admission_account_guards (
    account_id UUID PRIMARY KEY,
    presence_character_id UUID NULL,
    holder_game_session_id UUID NULL REFERENCES game_durability_reconnect_sessions (game_session_id),
    publication_revision NUMERIC(20, 0) NOT NULL CHECK (publication_revision BETWEEN 1 AND 18446744073709551615),
    source_authority TEXT NOT NULL CHECK (length(source_authority) > 0),
    source_revision NUMERIC(20, 0) NOT NULL CHECK (source_revision BETWEEN 1 AND 18446744073709551615),
    decision_identity TEXT NOT NULL CHECK (length(decision_identity) > 0),
    source_observed_at BIGINT NOT NULL CHECK (source_observed_at >= 0),
    clock_uncertainty_seconds NUMERIC(20, 0) NOT NULL CHECK (clock_uncertainty_seconds BETWEEN 0 AND 18446744073709551615),
    change_json TEXT NOT NULL CHECK (jsonb_typeof(change_json::jsonb) = 'object'),
    CHECK ((presence_character_id IS NULL) = (holder_game_session_id IS NULL))
);

CREATE TABLE game_durability_admission_character_guards (
    character_id UUID PRIMARY KEY,
    account_id UUID NOT NULL,
    world_id UUID NOT NULL,
    eligible BOOLEAN NOT NULL,
    lease_generation NUMERIC(20, 0) NOT NULL CHECK (lease_generation BETWEEN 1 AND 18446744073709551615),
    holder_game_session_id UUID NULL REFERENCES game_durability_reconnect_sessions (game_session_id),
    publication_revision NUMERIC(20, 0) NOT NULL CHECK (publication_revision BETWEEN 1 AND 18446744073709551615),
    source_authority TEXT NOT NULL CHECK (length(source_authority) > 0),
    source_revision NUMERIC(20, 0) NOT NULL CHECK (source_revision BETWEEN 1 AND 18446744073709551615),
    decision_identity TEXT NOT NULL CHECK (length(decision_identity) > 0),
    source_observed_at BIGINT NOT NULL CHECK (source_observed_at >= 0),
    clock_uncertainty_seconds NUMERIC(20, 0) NOT NULL CHECK (clock_uncertainty_seconds BETWEEN 0 AND 18446744073709551615),
    change_json TEXT NOT NULL CHECK (jsonb_typeof(change_json::jsonb) = 'object'),
    CHECK (holder_game_session_id IS NULL OR lease_generation > 0)
);

CREATE TABLE game_durability_admission_runtime_guards (
    scope_key BYTEA PRIMARY KEY,
    ownership_generation NUMERIC(20, 0) NOT NULL CHECK (ownership_generation BETWEEN 1 AND 18446744073709551615),
    ready BOOLEAN NOT NULL,
    publication_revision NUMERIC(20, 0) NOT NULL CHECK (publication_revision BETWEEN 1 AND 18446744073709551615),
    source_authority TEXT NOT NULL CHECK (length(source_authority) > 0),
    source_revision NUMERIC(20, 0) NOT NULL CHECK (source_revision BETWEEN 1 AND 18446744073709551615),
    decision_identity TEXT NOT NULL CHECK (length(decision_identity) > 0),
    source_observed_at BIGINT NOT NULL CHECK (source_observed_at >= 0),
    clock_uncertainty_seconds NUMERIC(20, 0) NOT NULL CHECK (clock_uncertainty_seconds BETWEEN 0 AND 18446744073709551615),
    change_json TEXT NOT NULL CHECK (jsonb_typeof(change_json::jsonb) = 'object')
);

CREATE TABLE game_durability_admission_signing_trust_guards (
    key_id TEXT NOT NULL,
    profile TEXT NOT NULL,
    public_key BYTEA NOT NULL CHECK (octet_length(public_key) = 32),
    trusted BOOLEAN NOT NULL,
    publication_revision NUMERIC(20, 0) NOT NULL CHECK (publication_revision BETWEEN 1 AND 18446744073709551615),
    source_authority TEXT NOT NULL CHECK (length(source_authority) > 0),
    source_revision NUMERIC(20, 0) NOT NULL CHECK (source_revision BETWEEN 1 AND 18446744073709551615),
    decision_identity TEXT NOT NULL CHECK (length(decision_identity) > 0),
    source_observed_at BIGINT NOT NULL CHECK (source_observed_at >= 0),
    clock_uncertainty_seconds NUMERIC(20, 0) NOT NULL CHECK (clock_uncertainty_seconds BETWEEN 0 AND 18446744073709551615),
    change_json TEXT NOT NULL CHECK (jsonb_typeof(change_json::jsonb) = 'object'),
    PRIMARY KEY (key_id, profile)
);

CREATE TABLE game_durability_admission_lifecycle_receipts (
    operation_key BYTEA PRIMARY KEY,
    operation_json TEXT NOT NULL CHECK (jsonb_typeof(operation_json::jsonb) = 'object'),
    decided_at BIGINT NOT NULL CHECK (decided_at >= 0)
);

CREATE FUNCTION game_durability_reject_history_mutation() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION 'immutable Game durability history cannot be changed' USING ERRCODE = '23514';
END;
$$;

CREATE TRIGGER game_fresh_receipt_immutable BEFORE UPDATE OR DELETE
    ON game_durability_fresh_admission_receipts FOR EACH ROW
    EXECUTE FUNCTION game_durability_reject_history_mutation();
CREATE TRIGGER game_guard_history_immutable BEFORE UPDATE OR DELETE
    ON game_durability_admission_guard_history FOR EACH ROW
    EXECUTE FUNCTION game_durability_reject_history_mutation();
CREATE TRIGGER game_lifecycle_receipt_immutable BEFORE UPDATE OR DELETE
    ON game_durability_admission_lifecycle_receipts FOR EACH ROW
    EXECUTE FUNCTION game_durability_reject_history_mutation();
CREATE TRIGGER game_transport_reservation_immutable BEFORE UPDATE OR DELETE
    ON game_durability_transport_ref_reservations FOR EACH ROW
    EXECUTE FUNCTION game_durability_reject_history_mutation();

-- One stable logical executor: row0 is its generation; rows1/2 are the only
-- durable pending slots. These are custody checkpoints, never owner authority.
CREATE TABLE game_durability_executor_custody (
    slot SMALLINT PRIMARY KEY CHECK (slot BETWEEN 0 AND 2),
    generation NUMERIC(20, 0) NOT NULL CHECK (generation BETWEEN 0 AND 18446744073709551615),
    operation_kind SMALLINT NULL CHECK (operation_kind BETWEEN 1 AND 8),
    operation_json TEXT NULL CHECK (octet_length(operation_json) BETWEEN 1 AND 65536),
    CHECK ((operation_kind IS NULL) = (operation_json IS NULL)),
    CHECK (slot <> 0 OR operation_json IS NULL)
);
INSERT INTO game_durability_executor_custody (slot, generation) VALUES (0, 0), (1, 0), (2, 0);
