-- The death owner freezes its Channel observation. Retain World changes so a
-- settlement can advance a stale observation only through the time of death.
CREATE TABLE game_party_shared_xp_history (
    party_id UUID NOT NULL REFERENCES game_parties(party_id) ON DELETE CASCADE,
    changed_at TIMESTAMPTZ NOT NULL,
    revision NUMERIC(20,0) NOT NULL,
    enabled BOOLEAN NOT NULL,
    PRIMARY KEY (party_id, changed_at, revision)
);

INSERT INTO game_party_shared_xp_history (party_id, changed_at, revision, enabled)
SELECT party_id, clock_timestamp(), revision, shared_xp_enabled FROM game_parties;

CREATE FUNCTION game_record_party_shared_xp() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
    IF TG_OP = 'INSERT' OR NEW.shared_xp_enabled IS DISTINCT FROM OLD.shared_xp_enabled THEN
        INSERT INTO game_party_shared_xp_history (party_id, changed_at, revision, enabled)
        SELECT NEW.party_id, clock_timestamp(), NEW.revision, NEW.shared_xp_enabled
        WHERE EXISTS (SELECT 1 FROM game_parties WHERE party_id = NEW.party_id);
    END IF;
    RETURN NULL;
END $$;

DO $migration$
BEGIN
    EXECUTE format('ALTER FUNCTION game_record_party_shared_xp() SET search_path = pg_catalog, %I, pg_temp', current_schema());
END $migration$;

CREATE CONSTRAINT TRIGGER game_party_shared_xp_history_record
    AFTER INSERT OR UPDATE OF shared_xp_enabled ON game_parties
    DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW EXECUTE FUNCTION game_record_party_shared_xp();

REVOKE ALL ON TABLE game_party_shared_xp_history FROM PUBLIC;
REVOKE ALL ON FUNCTION game_record_party_shared_xp() FROM PUBLIC;
GRANT SELECT ON TABLE game_party_shared_xp_history TO oteryn_game_runtime, oteryn_game_control;
