-- S17 P1: absent rows and new preferences both mean no bystander consent.
-- The fenced owner locks the Character root before inserting/updating this row.
CREATE TABLE game_character_cast_consent (
    character_id UUID PRIMARY KEY REFERENCES game_character_roots(character_id),
    allowed BOOLEAN NOT NULL DEFAULT FALSE,
    revision BIGINT NOT NULL DEFAULT 1 CHECK (revision > 0)
);
REVOKE ALL ON game_character_cast_consent FROM PUBLIC;
GRANT SELECT, INSERT, UPDATE ON game_character_cast_consent TO oteryn_game_runtime;
GRANT SELECT ON game_character_cast_consent TO oteryn_game_control;

-- A false preference cannot commit while a live media worker can still release
-- its old buffered revision. Fresh empty workers start at the current prefix;
-- pending floors also deny new captures until the preference write completes.
CREATE TABLE game_cast_consent_control (
    singleton BOOLEAN PRIMARY KEY CHECK (singleton),
    last_seq BIGINT NOT NULL CHECK (last_seq >= 0)
);
INSERT INTO game_cast_consent_control VALUES(TRUE,0);
CREATE TABLE game_cast_consent_floors (
    character_id UUID PRIMARY KEY,
    minimum_revision BIGINT NOT NULL CHECK (minimum_revision > 0),
    seq BIGINT NOT NULL CHECK (seq > 0)
);
CREATE TABLE game_cast_consent_invalidations (
    seq BIGINT PRIMARY KEY CHECK (seq > 0),
    character_id UUID NOT NULL,
    minimum_revision BIGINT NOT NULL CHECK (minimum_revision > 0),
    created_at_ms BIGINT NOT NULL DEFAULT (floor(extract(epoch FROM clock_timestamp())*1000)::bigint)
);
CREATE TABLE game_cast_consent_workers (
    instance UUID PRIMARY KEY,
    node_id UUID NOT NULL,
    registration_revision NUMERIC(20,0) NOT NULL CHECK (registration_revision > 0),
    applied_seq BIGINT NOT NULL CHECK (applied_seq >= 0),
    expires_at_ms BIGINT NOT NULL CHECK (expires_at_ms >= 0)
);
CREATE INDEX game_cast_consent_workers_pending ON game_cast_consent_workers(applied_seq, expires_at_ms);
REVOKE ALL ON game_cast_consent_control,game_cast_consent_floors,game_cast_consent_invalidations,game_cast_consent_workers FROM PUBLIC;
GRANT SELECT,UPDATE ON game_cast_consent_control TO oteryn_game_runtime;
GRANT SELECT,INSERT,UPDATE ON game_cast_consent_floors,game_cast_consent_workers TO oteryn_game_runtime;
GRANT SELECT,INSERT ON game_cast_consent_invalidations TO oteryn_game_runtime;
GRANT SELECT ON game_cast_consent_control,game_cast_consent_floors,game_cast_consent_invalidations,game_cast_consent_workers TO oteryn_game_control;

-- Technical delivery cursors/events use the same P30D operational window as
-- the audit. Current denial floors are preference-lifetime state, not history.
CREATE INDEX game_cast_consent_invalidations_expiry ON game_cast_consent_invalidations(created_at_ms,seq);
CREATE FUNCTION game_expire_cast_consent_metadata(p_limit INTEGER) RETURNS INTEGER
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,public,pg_temp AS $$
DECLARE removed INTEGER; more INTEGER;
BEGIN
    IF p_limit IS NULL OR p_limit<1 OR p_limit>128 THEN RAISE EXCEPTION 'invalid cast consent expiry limit'; END IF;
    WITH selected AS (
        SELECT e.seq FROM public.game_cast_consent_invalidations e
        WHERE e.created_at_ms<floor(extract(epoch FROM clock_timestamp())*1000)::bigint-2592000000
          AND NOT EXISTS(SELECT 1 FROM public.game_cast_consent_workers w WHERE w.applied_seq<e.seq AND w.expires_at_ms>floor(extract(epoch FROM clock_timestamp())*1000)::bigint)
        ORDER BY e.seq LIMIT p_limit FOR UPDATE SKIP LOCKED
    ) DELETE FROM public.game_cast_consent_invalidations e USING selected s WHERE e.seq=s.seq;
    GET DIAGNOSTICS removed=ROW_COUNT;
    WITH selected AS (
        SELECT instance FROM public.game_cast_consent_workers
        WHERE expires_at_ms<floor(extract(epoch FROM clock_timestamp())*1000)::bigint-2592000000
        ORDER BY expires_at_ms LIMIT (p_limit-removed) FOR UPDATE SKIP LOCKED
    ) DELETE FROM public.game_cast_consent_workers w USING selected s WHERE w.instance=s.instance;
    GET DIAGNOSTICS more=ROW_COUNT;
    RETURN removed+more;
END $$;
REVOKE ALL ON FUNCTION game_expire_cast_consent_metadata(INTEGER) FROM PUBLIC,oteryn_game_runtime;
GRANT EXECUTE ON FUNCTION game_expire_cast_consent_metadata(INTEGER) TO oteryn_game_control;
