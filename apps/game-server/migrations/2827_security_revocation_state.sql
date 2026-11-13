-- Platform owns revocations. Game retains their identities and captures affected sessions.
CREATE TABLE game_security_revocations (
    revocation_id UUID PRIMARY KEY CHECK (get_byte(uuid_send(revocation_id),6)>>4=7 AND get_byte(uuid_send(revocation_id),8)&192=128),
    account_id UUID NOT NULL CHECK (get_byte(uuid_send(account_id),6)>>4=7 AND get_byte(uuid_send(account_id),8)&192=128),
    native_security_generation BIGINT NOT NULL CHECK (native_security_generation>0),
    cause SMALLINT NOT NULL CHECK (cause BETWEEN 1 AND 4),
    effective_at_ms BIGINT NOT NULL CHECK (effective_at_ms>0)
);
CREATE INDEX game_security_revocations_account ON game_security_revocations(account_id);
CREATE TABLE game_security_revocation_floors (
    account_id UUID PRIMARY KEY,
    native_security_generation BIGINT NOT NULL CHECK (native_security_generation>0)
);
ALTER TABLE game_durability_reconnect_sessions
    ADD COLUMN admission_security_generation NUMERIC(20,0) CHECK (admission_security_generation>0),
    ADD COLUMN security_revoked BOOLEAN NOT NULL DEFAULT false;
REVOKE ALL ON game_security_revocations,game_security_revocation_floors FROM PUBLIC;
GRANT SELECT,INSERT ON game_security_revocations TO oteryn_game_runtime;
GRANT SELECT,INSERT,UPDATE ON game_security_revocation_floors TO oteryn_game_runtime;
GRANT SELECT ON game_security_revocations,game_security_revocation_floors TO oteryn_game_control;

-- Includes native successors of a captured activity session; these never revive authority.
CREATE FUNCTION game_security_session_revoked(p_session UUID) RETURNS BOOLEAN
LANGUAGE sql STABLE SET search_path=pg_catalog,public AS $$
    SELECT EXISTS(SELECT 1 FROM public.game_durability_reconnect_sessions s
        WHERE s.game_session_id=p_session AND s.security_revoked)
        OR EXISTS(SELECT 1 FROM public.game_activity_runtime_returns a
        JOIN public.game_durability_reconnect_sessions s ON s.game_session_id=a.source_game_session_id
        WHERE a.destination_game_session_id=p_session AND s.security_revoked)
$$;
REVOKE ALL ON FUNCTION game_security_session_revoked(UUID) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_security_session_revoked(UUID) TO oteryn_game_runtime;

-- Admission and stream application both hold the canonical admission relation fence.
-- Defend PREPARE and COMMIT (current and candidate proofs) at the durable mutation boundary.
CREATE FUNCTION game_security_session_fence() RETURNS TRIGGER
LANGUAGE plpgsql SET search_path=pg_catalog,public AS $$
BEGIN
    IF TG_OP='UPDATE' THEN
        IF OLD.security_revoked AND NOT NEW.security_revoked THEN
            RAISE EXCEPTION 'security revocation is terminal';
        END IF;
        IF public.game_security_session_revoked(OLD.game_session_id) AND NEW.session_state<>3
            AND (NEW.current_transport_ref IS DISTINCT FROM OLD.current_transport_ref AND NEW.current_transport_ref IS NOT NULL
                OR NEW.current_generation<>OLD.current_generation
                OR NEW.prepared_attempt_ref IS DISTINCT FROM OLD.prepared_attempt_ref AND NEW.prepared_attempt_ref IS NOT NULL) THEN
            RAISE EXCEPTION 'security revoked reconnect';
        END IF;
    ELSE
        IF NEW.admission_security_generation IS NOT NULL AND EXISTS(
            SELECT 1 FROM public.game_security_revocation_floors f WHERE f.account_id=NEW.account_id
            AND f.native_security_generation>NEW.admission_security_generation) THEN
            RAISE EXCEPTION 'security generation revoked';
        END IF;
    END IF;
    RETURN NEW;
END $$;
REVOKE ALL ON FUNCTION game_security_session_fence() FROM PUBLIC;
CREATE TRIGGER game_security_session_fence BEFORE INSERT OR UPDATE ON game_durability_reconnect_sessions
    FOR EACH ROW EXECUTE FUNCTION game_security_session_fence();
