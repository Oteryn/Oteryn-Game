-- INVCAP-1: advisory HUD observations prove the same retained node and assignment
-- history in one READ ONLY, REPEATABLE READ transaction. These snapshot predicates
-- never authorize a durable mutation. The locking writer predicates remain unchanged.
-- Definer privileges keep private node-history tables inaccessible to runtime callers.

CREATE FUNCTION game_node_registration_observation_valid() RETURNS BOOLEAN
LANGUAGE plpgsql STABLE SECURITY DEFINER SET search_path = pg_catalog, public, pg_temp AS $$
DECLARE
    v_high_water NUMERIC(20, 0);
BEGIN
    SELECT registration_revision_high_water INTO v_high_water
        FROM game_node_registration_writer WHERE writer_id = 1;
    IF NOT FOUND THEN
        RETURN FALSE;
    END IF;
    IF (SELECT count(*)::numeric FROM game_node_registrations)
           + (SELECT count(*)::numeric FROM game_node_registration_endings) <> v_high_water
       OR (SELECT count(*)::numeric FROM (
               SELECT registration_revision FROM game_node_registrations
               UNION SELECT ending_revision FROM game_node_registration_endings) revisions) <> v_high_water
       OR (SELECT coalesce(min(revision), 1) FROM (
               SELECT registration_revision AS revision FROM game_node_registrations
               UNION ALL SELECT ending_revision FROM game_node_registration_endings) revisions) <> 1
       OR (SELECT coalesce(max(revision), 0) FROM (
               SELECT registration_revision AS revision FROM game_node_registrations
               UNION ALL SELECT ending_revision FROM game_node_registration_endings) revisions) <> v_high_water THEN
        RETURN FALSE;
    END IF;
    RETURN NOT EXISTS (
        SELECT 1 FROM game_node_registrations g
        LEFT JOIN game_node_registration_endings e USING (node_id)
        WHERE (e.node_id IS NULL) <> (g.state = 1)
           OR (e.node_id IS NOT NULL AND (
                  e.state <> g.state
                  OR e.superseded_by IS DISTINCT FROM g.superseded_by
                  OR e.ended_at IS DISTINCT FROM g.ended_at))
    );
END; $$;

CREATE FUNCTION game_node_prove_observed_incarnation(
    p_node_id UUID, p_registration_revision NUMERIC, p_secret BYTEA)
RETURNS BOOLEAN
LANGUAGE plpgsql STABLE SECURITY DEFINER SET search_path = pg_catalog, public, pg_temp AS $$
BEGIN
    IF p_secret IS NULL OR octet_length(p_secret) <> 32
       OR NOT game_node_registration_observation_valid() THEN
        RETURN FALSE;
    END IF;
    PERFORM 1 FROM game_node_registrations g
        WHERE g.node_id = p_node_id AND g.registration_revision = p_registration_revision AND g.state = 1
          AND g.authorization_digest = sha256(p_secret)
          AND NOT EXISTS (SELECT 1 FROM game_node_registration_endings e WHERE e.node_id = g.node_id)
       ;
    RETURN FOUND;
END; $$;

CREATE FUNCTION game_runtime_scope_assignment_observation_valid() RETURNS BOOLEAN
LANGUAGE plpgsql STABLE SECURITY DEFINER SET search_path = pg_catalog, public, pg_temp AS $$
DECLARE
    v_high_water NUMERIC(20, 0);
BEGIN
    SELECT source_revision_high_water INTO v_high_water
        FROM game_runtime_scope_assignment_writer WHERE writer_id = 1;
    IF NOT FOUND THEN
        RETURN FALSE;
    END IF;
    IF (SELECT count(*)::numeric FROM game_runtime_scope_assignment_receipts) <> v_high_water
       OR (SELECT coalesce(min(source_revision), 1) FROM game_runtime_scope_assignment_receipts) <> 1
       OR (SELECT coalesce(max(source_revision), 0) FROM game_runtime_scope_assignment_receipts) <> v_high_water
       OR EXISTS (
            SELECT 1 FROM game_runtime_scope_assignment_receipts r
            LEFT JOIN game_runtime_scope_assignments a USING (scope_key)
            WHERE a.scope_key IS NULL
       ) THEN
        RETURN FALSE;
    END IF;
    RETURN NOT EXISTS (
        SELECT 1
        FROM game_runtime_scope_assignments a
        LEFT JOIN LATERAL (
            SELECT r.* FROM game_runtime_scope_assignment_receipts r
            WHERE r.scope_key = a.scope_key
            ORDER BY r.source_revision DESC LIMIT 1
        ) r ON TRUE
        WHERE r.source_revision IS NULL
           OR a.ownership_generation IS DISTINCT FROM r.ownership_generation
           OR a.state IS DISTINCT FROM r.state
           OR a.holder_node_id IS DISTINCT FROM r.holder_node_id
           OR a.holder_registration_revision IS DISTINCT FROM r.holder_registration_revision
           OR a.source_revision IS DISTINCT FROM r.source_revision
           OR a.decision_identity IS DISTINCT FROM r.decision_identity
           OR a.operation_key IS DISTINCT FROM r.operation_key
           OR a.decided_at IS DISTINCT FROM r.decided_at
    );
END; $$;

REVOKE ALL ON FUNCTION game_node_registration_observation_valid(),
    game_node_prove_observed_incarnation(UUID, NUMERIC, BYTEA),
    game_runtime_scope_assignment_observation_valid() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_node_prove_observed_incarnation(UUID, NUMERIC, BYTEA),
    game_runtime_scope_assignment_observation_valid() TO oteryn_game_runtime;

DO $$ DECLARE function_name TEXT; BEGIN
    FOREACH function_name IN ARRAY ARRAY[
        'game_node_registration_observation_valid()',
        'game_node_prove_observed_incarnation(uuid,numeric,bytea)',
        'game_runtime_scope_assignment_observation_valid()'
    ] LOOP
        EXECUTE format('ALTER FUNCTION %s SET search_path = pg_catalog, %I, pg_temp',
            function_name, current_schema());
    END LOOP;
END $$;
