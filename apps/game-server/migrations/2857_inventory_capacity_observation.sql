-- INVCAP-1: advisory HUD observation checks of the node and assignment history the observed
-- fence depends on, in one READ ONLY, REPEATABLE READ transaction. They are an advisory
-- observation only: they never authorize a durable mutation, and they check less than the
-- locking writer predicates (full history scans), which remain unchanged and authoritative.
-- Every lookup here is bounded by a unique or scope index, so an observation costs the same
-- whatever the history length.
-- Definer privileges keep private node-history tables inaccessible to runtime callers.

-- Latest-receipt lookup of one scope (`ORDER BY source_revision DESC LIMIT 1`).
CREATE INDEX game_runtime_scope_assignment_receipts_scope_revision
    ON game_runtime_scope_assignment_receipts (scope_key, source_revision);

-- Checks the registration revision namespace only at its base and its tip: revision 1
-- exists, a registration or ending exists at the writer high-water, and nothing lies beyond
-- it. A restore that loses the newest registration or ending (a revoke or supersede of the
-- proven node included) fails here. It does not check the count, distinctness or gaps of
-- the revisions in between; only the writer predicates do.
CREATE FUNCTION game_node_registration_observation_valid() RETURNS BOOLEAN
LANGUAGE plpgsql STABLE SECURITY DEFINER SET search_path = pg_catalog, public, pg_temp AS $$
DECLARE
    v_high_water NUMERIC(20, 0);
BEGIN
    SELECT registration_revision_high_water INTO v_high_water
        FROM game_node_registration_writer WHERE writer_id = 1;
    IF NOT FOUND OR v_high_water < 1 THEN
        RETURN FALSE;
    END IF;
    RETURN EXISTS (SELECT 1 FROM game_node_registrations WHERE registration_revision = 1)
       AND (EXISTS (SELECT 1 FROM game_node_registrations
                        WHERE registration_revision = v_high_water)
            OR EXISTS (SELECT 1 FROM game_node_registration_endings
                           WHERE ending_revision = v_high_water))
       AND NOT EXISTS (SELECT 1 FROM game_node_registrations
                           WHERE registration_revision > v_high_water)
       AND NOT EXISTS (SELECT 1 FROM game_node_registration_endings
                           WHERE ending_revision > v_high_water);
END; $$;

-- The proven incarnation itself: CURRENT, unended, its own secret, and its row agrees
-- with the (absent) immutable ending.
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
          AND g.ended_at IS NULL AND g.superseded_by IS NULL
          AND g.authorization_digest = sha256(p_secret)
          AND NOT EXISTS (SELECT 1 FROM game_node_registration_endings e WHERE e.node_id = g.node_id)
       ;
    RETURN FOUND;
END; $$;

-- Checks the assignment revision namespace only at its base and its tip (receipt 1 and the
-- writer high-water exist, nothing lies beyond it) and that the observed scope's row exactly
-- reproduces that scope's latest receipt. A superseded or rolled-back row of the observed
-- scope (a newer receipt for it) fails here. The guarantee covers the observed scope only:
-- other scopes and the receipts between base and tip are not checked.
CREATE FUNCTION game_runtime_scope_assignment_observation_valid(p_scope_key BYTEA)
RETURNS BOOLEAN
LANGUAGE plpgsql STABLE SECURITY DEFINER SET search_path = pg_catalog, public, pg_temp AS $$
DECLARE
    v_high_water NUMERIC(20, 0);
BEGIN
    SELECT source_revision_high_water INTO v_high_water
        FROM game_runtime_scope_assignment_writer WHERE writer_id = 1;
    IF NOT FOUND OR v_high_water < 1 OR p_scope_key IS NULL THEN
        RETURN FALSE;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM game_runtime_scope_assignment_receipts WHERE source_revision = 1)
       OR NOT EXISTS (SELECT 1 FROM game_runtime_scope_assignment_receipts
                          WHERE source_revision = v_high_water)
       OR EXISTS (SELECT 1 FROM game_runtime_scope_assignment_receipts
                      WHERE source_revision > v_high_water) THEN
        RETURN FALSE;
    END IF;
    RETURN EXISTS (
        SELECT 1
        FROM game_runtime_scope_assignments a
        JOIN LATERAL (
            SELECT r.* FROM game_runtime_scope_assignment_receipts r
            WHERE r.scope_key = a.scope_key
            ORDER BY r.source_revision DESC LIMIT 1
        ) r ON TRUE
        WHERE a.scope_key = p_scope_key
          AND a.ownership_generation = r.ownership_generation
          AND a.state = r.state
          AND a.holder_node_id IS NOT DISTINCT FROM r.holder_node_id
          AND a.holder_registration_revision IS NOT DISTINCT FROM r.holder_registration_revision
          AND a.source_revision = r.source_revision
          AND a.decision_identity = r.decision_identity
          AND a.operation_key = r.operation_key
          AND a.decided_at = r.decided_at
    );
END; $$;

REVOKE ALL ON FUNCTION game_node_registration_observation_valid(),
    game_node_prove_observed_incarnation(UUID, NUMERIC, BYTEA),
    game_runtime_scope_assignment_observation_valid(BYTEA) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_node_prove_observed_incarnation(UUID, NUMERIC, BYTEA),
    game_runtime_scope_assignment_observation_valid(BYTEA) TO oteryn_game_runtime;

DO $$ DECLARE function_name TEXT; BEGIN
    FOREACH function_name IN ARRAY ARRAY[
        'game_node_registration_observation_valid()',
        'game_node_prove_observed_incarnation(uuid,numeric,bytea)',
        'game_runtime_scope_assignment_observation_valid(bytea)'
    ] LOOP
        EXECUTE format('ALTER FUNCTION %s SET search_path = pg_catalog, %I, pg_temp',
            function_name, current_schema());
    END LOOP;
END $$;
