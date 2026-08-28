CREATE TABLE game_durability_reconnect_sessions (
    game_session_id UUID PRIMARY KEY
        CHECK ((get_byte(uuid_send(game_session_id), 6) >> 4) = 7)
        CHECK ((get_byte(uuid_send(game_session_id), 8) & 192) = 128),
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
    control_loss_epoch NUMERIC(20, 0) NOT NULL
        CHECK (control_loss_epoch BETWEEN 1 AND 18446744073709551615),
    original_grace_deadline BIGINT NOT NULL,
    predecessor_generation NUMERIC(20, 0) NOT NULL
        CHECK (predecessor_generation BETWEEN 1 AND 18446744073709551615),
    character_lease_generation NUMERIC(20, 0) NOT NULL
        CHECK (character_lease_generation BETWEEN 1 AND 18446744073709551615),
    scope_ownership_generation NUMERIC(20, 0) NOT NULL
        CHECK (scope_ownership_generation BETWEEN 1 AND 18446744073709551615),
    current_generation NUMERIC(20, 0) NOT NULL
        CHECK (current_generation BETWEEN 1 AND 18446744073709551615),
    current_transport_ref BYTEA NULL CHECK (octet_length(current_transport_ref) = 16),
    session_state SMALLINT NOT NULL DEFAULT 1 CHECK (session_state BETWEEN 1 AND 3),
    attempt_count SMALLINT NOT NULL DEFAULT 0 CHECK (attempt_count BETWEEN 0 AND 8),
    prepared_attempt_ref BYTEA NULL CHECK (octet_length(prepared_attempt_ref) = 8),
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
    )
);

CREATE UNIQUE INDEX game_durability_one_nonterminal_session_per_character
    ON game_durability_reconnect_sessions (character_id)
    WHERE session_state IN (1, 2);

CREATE TABLE game_durability_session_replacements (
    character_id UUID NOT NULL
        CHECK ((get_byte(uuid_send(character_id), 6) >> 4) = 7)
        CHECK ((get_byte(uuid_send(character_id), 8) & 192) = 128),
    predecessor_game_session_id UUID NOT NULL
        CHECK ((get_byte(uuid_send(predecessor_game_session_id), 6) >> 4) = 7)
        CHECK ((get_byte(uuid_send(predecessor_game_session_id), 8) & 192) = 128),
    candidate_game_session_id UUID NOT NULL
        CHECK ((get_byte(uuid_send(candidate_game_session_id), 6) >> 4) = 7)
        CHECK ((get_byte(uuid_send(candidate_game_session_id), 8) & 192) = 128),
    predecessor_connection_generation NUMERIC(20, 0) NOT NULL
        CHECK (predecessor_connection_generation BETWEEN 1 AND 18446744073709551615),
    predecessor_character_lease_generation NUMERIC(20, 0) NOT NULL
        CHECK (predecessor_character_lease_generation BETWEEN 1 AND 18446744073709551615),
    predecessor_scope_ownership_generation NUMERIC(20, 0) NOT NULL
        CHECK (predecessor_scope_ownership_generation BETWEEN 1 AND 18446744073709551615),
    replaced_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (character_id, predecessor_game_session_id, candidate_game_session_id),
    UNIQUE (character_id, candidate_game_session_id)
);

CREATE TABLE game_durability_control_loss_continuity (
    character_id UUID NOT NULL
        CHECK ((get_byte(uuid_send(character_id), 6) >> 4) = 7)
        CHECK ((get_byte(uuid_send(character_id), 8) & 192) = 128),
    control_loss_epoch NUMERIC(20, 0) NOT NULL
        CHECK (control_loss_epoch BETWEEN 1 AND 18446744073709551615),
    account_id UUID NOT NULL,
    world_id UUID NOT NULL
        CHECK ((get_byte(uuid_send(world_id), 6) >> 4) = 7)
        CHECK ((get_byte(uuid_send(world_id), 8) & 192) = 128),
    context_game_session_id UUID NOT NULL
        CHECK ((get_byte(uuid_send(context_game_session_id), 6) >> 4) = 7)
        CHECK ((get_byte(uuid_send(context_game_session_id), 8) & 192) = 128),
    original_grace_deadline BIGINT NOT NULL,
    protection_entitlement_state SMALLINT NOT NULL CHECK (protection_entitlement_state IN (1, 2)),
    protection_fenced_generation NUMERIC(20, 0) NULL
        CHECK (protection_fenced_generation BETWEEN 1 AND 18446744073709551615),
    protection_activated_at TIMESTAMPTZ NULL,
    protection_expires_at TIMESTAMPTZ NULL,
    protection_rearm_state SMALLINT NOT NULL CHECK (protection_rearm_state IN (1, 2)),
    protection_rearm_deadline TIMESTAMPTZ NULL,
    PRIMARY KEY (character_id, control_loss_epoch),
    CHECK (
        (protection_entitlement_state = 1
            AND protection_fenced_generation IS NULL
            AND protection_activated_at IS NULL
            AND protection_expires_at IS NULL
            AND protection_rearm_state = 1
            AND protection_rearm_deadline IS NULL)
        OR
        (protection_entitlement_state = 2
            AND protection_fenced_generation IS NOT NULL
            AND protection_rearm_state = 2
            AND (
                (protection_activated_at IS NULL AND protection_expires_at IS NULL)
                OR
                (protection_activated_at IS NOT NULL
                    AND protection_expires_at = protection_activated_at + INTERVAL '4 seconds')
            ))
    )
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
    control_loss_epoch NUMERIC(20, 0) NOT NULL
        CHECK (control_loss_epoch BETWEEN 1 AND 18446744073709551615),
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
