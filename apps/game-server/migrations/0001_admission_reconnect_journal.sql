CREATE TABLE game_durability_reconnect_sessions (
    game_session_id BYTEA PRIMARY KEY CHECK (octet_length(game_session_id) = 16),
    control_loss_epoch BIGINT NOT NULL CHECK (control_loss_epoch > 0),
    predecessor_generation BIGINT NOT NULL CHECK (predecessor_generation > 0),
    character_lease_generation BIGINT NOT NULL CHECK (character_lease_generation > 0),
    scope_ownership_generation BIGINT NOT NULL CHECK (scope_ownership_generation > 0),
    attempt_count SMALLINT NOT NULL DEFAULT 0 CHECK (attempt_count BETWEEN 0 AND 8),
    prepared_attempt_ref BYTEA NULL CHECK (octet_length(prepared_attempt_ref) = 8)
);

CREATE TABLE game_durability_transport_ref_reservations (
    transport_ref BYTEA PRIMARY KEY CHECK (octet_length(transport_ref) = 16),
    game_session_id BYTEA NOT NULL CHECK (octet_length(game_session_id) = 16),
    reconnect_attempt_ref BYTEA NOT NULL CHECK (octet_length(reconnect_attempt_ref) = 8),
    reserved_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE game_durability_reconnect_attempts (
    game_session_id BYTEA NOT NULL CHECK (octet_length(game_session_id) = 16),
    reconnect_attempt_ref BYTEA NOT NULL CHECK (octet_length(reconnect_attempt_ref) = 8),
    control_loss_epoch BIGINT NOT NULL CHECK (control_loss_epoch > 0),
    transport_ref BYTEA NOT NULL CHECK (octet_length(transport_ref) = 16),
    record_json TEXT NOT NULL,
    state SMALLINT NOT NULL CHECK (state BETWEEN 1 AND 4),
    PRIMARY KEY (game_session_id, reconnect_attempt_ref)
);

CREATE INDEX game_durability_reconnect_attempts_epoch_idx
    ON game_durability_reconnect_attempts (game_session_id, control_loss_epoch);
