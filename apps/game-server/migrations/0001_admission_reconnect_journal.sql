CREATE TABLE game_durability_reconnect_sessions (
    game_session_id UUID PRIMARY KEY
        CHECK ((get_byte(uuid_send(game_session_id), 6) >> 4) = 7)
        CHECK ((get_byte(uuid_send(game_session_id), 8) & 192) = 128),
    control_loss_epoch BIGINT NOT NULL CHECK (control_loss_epoch > 0),
    predecessor_generation BIGINT NOT NULL CHECK (predecessor_generation > 0),
    character_lease_generation BIGINT NOT NULL CHECK (character_lease_generation > 0),
    scope_ownership_generation BIGINT NOT NULL CHECK (scope_ownership_generation > 0),
    current_generation BIGINT NOT NULL CHECK (current_generation > 0),
    current_transport_ref BYTEA NULL CHECK (octet_length(current_transport_ref) = 16),
    session_state SMALLINT NOT NULL DEFAULT 1 CHECK (session_state BETWEEN 1 AND 2),
    attempt_count SMALLINT NOT NULL DEFAULT 0 CHECK (attempt_count BETWEEN 0 AND 8),
    prepared_attempt_ref BYTEA NULL CHECK (octet_length(prepared_attempt_ref) = 8)
);

CREATE TABLE game_durability_transport_ref_reservations (
    transport_ref BYTEA PRIMARY KEY CHECK (octet_length(transport_ref) = 16),
    game_session_id UUID NOT NULL
        CHECK ((get_byte(uuid_send(game_session_id), 6) >> 4) = 7)
        CHECK ((get_byte(uuid_send(game_session_id), 8) & 192) = 128),
    reconnect_attempt_ref BYTEA NOT NULL CHECK (octet_length(reconnect_attempt_ref) = 8),
    reserved_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE game_durability_recovery_grant_consumptions (
    recovery_grant_nonce BYTEA PRIMARY KEY CHECK (octet_length(recovery_grant_nonce) = 32),
    game_session_id UUID NOT NULL
        CHECK ((get_byte(uuid_send(game_session_id), 6) >> 4) = 7)
        CHECK ((get_byte(uuid_send(game_session_id), 8) & 192) = 128),
    reconnect_attempt_ref BYTEA NOT NULL CHECK (octet_length(reconnect_attempt_ref) = 8),
    consumed_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE game_durability_reconnect_attempts (
    game_session_id UUID NOT NULL
        CHECK ((get_byte(uuid_send(game_session_id), 6) >> 4) = 7)
        CHECK ((get_byte(uuid_send(game_session_id), 8) & 192) = 128),
    reconnect_attempt_ref BYTEA NOT NULL CHECK (octet_length(reconnect_attempt_ref) = 8),
    control_loss_epoch BIGINT NOT NULL CHECK (control_loss_epoch > 0),
    transport_ref BYTEA NOT NULL CHECK (octet_length(transport_ref) = 16),
    account_id UUID NOT NULL,
    character_id UUID NOT NULL
        CHECK ((get_byte(uuid_send(character_id), 6) >> 4) = 7)
        CHECK ((get_byte(uuid_send(character_id), 8) & 192) = 128),
    world_id UUID NOT NULL
        CHECK ((get_byte(uuid_send(world_id), 6) >> 4) = 7)
        CHECK ((get_byte(uuid_send(world_id), 8) & 192) = 128),
    runtime_scope_kind SMALLINT NOT NULL CHECK (runtime_scope_kind IN (1, 2)),
    runtime_scope_world_id UUID NOT NULL
        CHECK ((get_byte(uuid_send(runtime_scope_world_id), 6) >> 4) = 7)
        CHECK ((get_byte(uuid_send(runtime_scope_world_id), 8) & 192) = 128),
    runtime_scope_channel_id UUID NULL,
    runtime_scope_instance_id UUID NULL,
    fnd02_next_command_id NUMERIC(20, 0) NOT NULL
        CHECK (fnd02_next_command_id BETWEEN 1 AND 18446744073709551615),
    record_json TEXT NOT NULL,
    state SMALLINT NOT NULL CHECK (state BETWEEN 1 AND 5),
    CHECK (
        (runtime_scope_kind = 1 AND runtime_scope_channel_id IS NOT NULL AND runtime_scope_instance_id IS NULL)
        OR
        (runtime_scope_kind = 2 AND runtime_scope_channel_id IS NULL AND runtime_scope_instance_id IS NOT NULL)
    ),
    CHECK (
        runtime_scope_channel_id IS NULL
        OR ((get_byte(uuid_send(runtime_scope_channel_id), 6) >> 4) = 7
            AND (get_byte(uuid_send(runtime_scope_channel_id), 8) & 192) = 128)
    ),
    CHECK (
        runtime_scope_instance_id IS NULL
        OR ((get_byte(uuid_send(runtime_scope_instance_id), 6) >> 4) = 7
            AND (get_byte(uuid_send(runtime_scope_instance_id), 8) & 192) = 128)
    ),
    PRIMARY KEY (game_session_id, reconnect_attempt_ref)
);

CREATE TABLE game_durability_reconnect_pending_commands (
    game_session_id UUID NOT NULL,
    reconnect_attempt_ref BYTEA NOT NULL CHECK (octet_length(reconnect_attempt_ref) = 8),
    command_id NUMERIC(20, 0) NOT NULL
        CHECK (command_id BETWEEN 1 AND 18446744073709551615),
    disposition SMALLINT NOT NULL CHECK (disposition IN (1, 2)),
    PRIMARY KEY (game_session_id, reconnect_attempt_ref, command_id),
    FOREIGN KEY (game_session_id, reconnect_attempt_ref)
        REFERENCES game_durability_reconnect_attempts (game_session_id, reconnect_attempt_ref)
        ON DELETE RESTRICT
);

CREATE INDEX game_durability_reconnect_attempts_epoch_idx
    ON game_durability_reconnect_attempts (game_session_id, control_loss_epoch);
