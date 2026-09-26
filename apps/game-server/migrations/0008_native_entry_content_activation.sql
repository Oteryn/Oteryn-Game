-- NATIVE_ENTRY_CONTENT_ACTIVATION_V1 (first-entry decision #935 "Activation issuer and
-- current pin"; #162 allocation 5849898542). One immutable control-plane activation
-- issuance per Channel scope and monotonic sequence. The newest row of a scope is its
-- current activation and its floor: a node activates only that row, and a new issuance
-- must name it as its predecessor (or name no predecessor for the scope's first one).

-- Operation 4 authorizes native entry Content activation issuance for one scope.
ALTER TABLE game_control_scope_grants DROP CONSTRAINT game_control_scope_grants_operation_check;
ALTER TABLE game_control_scope_grants ADD CONSTRAINT game_control_scope_grants_operation_check
    CHECK (operation IN (1, 2, 3, 4));

-- The assignment-receipt guard (0006) matched the command kind against any granted
-- operation. With operation 4 grantable, it must admit only the assignment kinds, so a
-- Content-activation grant can never authorize an assignment receipt.
CREATE OR REPLACE FUNCTION game_control_scope_grant_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    v_kind INTEGER := get_byte(NEW.command, 1);
    v_actor_len INTEGER := get_byte(NEW.command, 34);
BEGIN
    IF v_kind NOT IN (1, 2, 3)
       OR convert_from(substring(NEW.command FROM 36 FOR v_actor_len), 'UTF8') <> session_user
       -- The command kind must match the committed state (3 revoke <=> REVOKED).
       OR (v_kind = 3) <> (NEW.state = 2)
       OR NOT EXISTS (
            SELECT 1 FROM game_control_scope_grants g
            WHERE g.control_role = session_user
              AND '\x01'::BYTEA || uuid_send(g.world_id) || uuid_send(g.channel_id) = NEW.scope_key
              AND g.operation = v_kind
       ) THEN
        RAISE EXCEPTION 'runtime-scope assignment is not granted to this control role' USING ERRCODE = '42501';
    END IF;
    RETURN NEW;
END; $$;

CREATE TABLE game_content_activations (
    world_id UUID NOT NULL CHECK (game_node_is_uuid_v7(world_id)),
    channel_id UUID NOT NULL CHECK (game_node_is_uuid_v7(channel_id)),
    activation_sequence NUMERIC(20, 0) NOT NULL
        CHECK (activation_sequence BETWEEN 1 AND 18446744073709551615),
    previous_sequence NUMERIC(20, 0) NULL,
    server_artifact_digest BYTEA NOT NULL CHECK (octet_length(server_artifact_digest) = 32),
    client_artifact_digest BYTEA NOT NULL CHECK (octet_length(client_artifact_digest) = 32),
    frame_binding_digest BYTEA NOT NULL CHECK (octet_length(frame_binding_digest) = 32),
    issued_by TEXT NOT NULL,
    issued_at BIGINT NOT NULL CHECK (issued_at >= 0),
    PRIMARY KEY (world_id, channel_id, activation_sequence),
    CHECK (world_id <> channel_id),
    CHECK (previous_sequence IS NULL OR previous_sequence < activation_sequence)
);

CREATE FUNCTION game_content_activation_immutable() RETURNS trigger
LANGUAGE plpgsql AS $$ BEGIN
    RAISE EXCEPTION 'content activation issuances are immutable' USING ERRCODE = '23514';
END; $$;
CREATE TRIGGER game_content_activation_immutable BEFORE UPDATE OR DELETE
    ON game_content_activations FOR EACH ROW EXECUTE FUNCTION game_content_activation_immutable();

-- Record one issuance for a scope granted operation 4 to the session role. An exact
-- replay of a recorded sequence succeeds. A conflicting replay, a predecessor that is
-- not the scope's current sequence, or a sequence not above it rejects (OTC01).
CREATE FUNCTION game_content_record_activation(
    p_world_id UUID, p_channel_id UUID, p_sequence NUMERIC, p_previous NUMERIC,
    p_server BYTEA, p_client BYTEA, p_frame BYTEA
) RETURNS VOID LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE
    v_existing game_content_activations%ROWTYPE;
    v_current NUMERIC;
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM game_control_scope_grants g
        WHERE g.control_role = session_user AND g.world_id = p_world_id
          AND g.channel_id = p_channel_id AND g.operation = 4
    ) THEN
        RAISE EXCEPTION 'content activation is not granted to this control role' USING ERRCODE = '42501';
    END IF;
    PERFORM pg_advisory_xact_lock(hashtextextended(
        'oteryn:content-activation:' || p_world_id::text || ':' || p_channel_id::text, 0));
    SELECT * INTO v_existing FROM game_content_activations
        WHERE world_id = p_world_id AND channel_id = p_channel_id
          AND activation_sequence = p_sequence;
    IF FOUND THEN
        IF v_existing.previous_sequence IS NOT DISTINCT FROM p_previous
           AND (v_existing.server_artifact_digest, v_existing.client_artifact_digest,
                v_existing.frame_binding_digest) = (p_server, p_client, p_frame) THEN
            RETURN;
        END IF;
        RAISE EXCEPTION 'content activation conflicts with the recorded sequence' USING ERRCODE = 'OTC01';
    END IF;
    SELECT max(activation_sequence) INTO v_current FROM game_content_activations
        WHERE world_id = p_world_id AND channel_id = p_channel_id;
    IF p_previous IS DISTINCT FROM v_current OR p_sequence <= coalesce(v_current, 0) THEN
        RAISE EXCEPTION 'content activation predecessor is stale or its sequence is not newer'
            USING ERRCODE = 'OTC01';
    END IF;
    INSERT INTO game_content_activations (world_id, channel_id, activation_sequence,
        previous_sequence, server_artifact_digest, client_artifact_digest, frame_binding_digest,
        issued_by, issued_at)
    VALUES (p_world_id, p_channel_id, p_sequence, p_previous, p_server, p_client, p_frame,
        session_user, floor(extract(epoch FROM statement_timestamp()))::BIGINT);
END; $$;

DO $$
BEGIN
    EXECUTE format(
        'ALTER FUNCTION game_content_record_activation(uuid, uuid, numeric, numeric, bytea, bytea, bytea) SET search_path = %I, pg_temp',
        current_schema());
END $$;

REVOKE ALL ON FUNCTION game_content_record_activation(UUID, UUID, NUMERIC, NUMERIC, BYTEA, BYTEA, BYTEA)
    FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_content_record_activation(UUID, UUID, NUMERIC, NUMERIC, BYTEA, BYTEA, BYTEA)
    TO oteryn_game_control;
GRANT SELECT ON game_content_activations TO oteryn_game_runtime, oteryn_game_control;
