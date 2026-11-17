-- S17 P2. Existing typed outboxes require unrelated Character/item receipts
-- and incompatible retention tuples; none accepts ephemeral cast facts.
CREATE TABLE game_cast_audit_streams (
    incarnation UUID PRIMARY KEY CHECK (game_character_is_uuid_v7(incarnation)),
    node_id UUID NOT NULL,
    registration_revision NUMERIC(20,0) NOT NULL,
    last_ordinal BIGINT NOT NULL DEFAULT 0 CHECK (last_ordinal >= 0)
);
CREATE TABLE game_cast_audit_outbox (
    event_id UUID PRIMARY KEY DEFAULT game_character_uuid_v7() CHECK (game_character_is_uuid_v7(event_id)),
    incarnation UUID NOT NULL REFERENCES game_cast_audit_streams(incarnation),
    ordinal BIGINT NOT NULL CHECK (ordinal > 0),
    payload BYTEA NOT NULL CHECK (octet_length(payload) BETWEEN 1 AND 256),
    envelope BYTEA NOT NULL CHECK (octet_length(envelope) BETWEEN 1 AND 1024),
    envelope_sha256 BYTEA NOT NULL CHECK (envelope_sha256 = sha256(envelope)),
    occurred_at BIGINT NOT NULL CHECK (occurred_at > 0),
    expires_at BIGINT NOT NULL CHECK (expires_at = occurred_at + 2592000000),
    UNIQUE (incarnation, ordinal)
);
CREATE INDEX game_cast_audit_expiry ON game_cast_audit_outbox(expires_at,event_id);
REVOKE ALL ON game_cast_audit_streams,game_cast_audit_outbox FROM PUBLIC;
GRANT SELECT,INSERT ON game_cast_audit_streams,game_cast_audit_outbox TO oteryn_game_runtime;
GRANT UPDATE(last_ordinal) ON game_cast_audit_streams TO oteryn_game_runtime;
GRANT SELECT ON game_cast_audit_streams,game_cast_audit_outbox TO oteryn_game_control;

-- Restricted bounded retention entry point; runtime cannot delete audit.
CREATE FUNCTION game_cast_audit_expire(p_limit INTEGER) RETURNS INTEGER
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE removed INTEGER;
BEGIN
    IF p_limit < 1 OR p_limit > 128 OR p_limit IS NULL THEN
        RAISE EXCEPTION 'invalid cast audit purge limit';
    END IF;
    WITH expired AS (
        SELECT event_id FROM game_cast_audit_outbox
        WHERE expires_at < floor(extract(epoch FROM clock_timestamp())*1000)::bigint
        ORDER BY expires_at,event_id LIMIT p_limit FOR UPDATE SKIP LOCKED
    ) DELETE FROM game_cast_audit_outbox WHERE event_id IN (SELECT event_id FROM expired);
    GET DIAGNOSTICS removed = ROW_COUNT;
    RETURN removed;
END;
$$;
DO $$ BEGIN
    EXECUTE format('ALTER FUNCTION game_cast_audit_expire(INTEGER) SET search_path = pg_catalog, %I, pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_cast_audit_expire(INTEGER) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_cast_audit_expire(INTEGER) TO oteryn_game_control;
