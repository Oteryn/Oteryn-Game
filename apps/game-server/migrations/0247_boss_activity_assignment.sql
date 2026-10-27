-- BOSS-ENCOUNTER-1 / owner decision 24: a fresh activity instance per admission.
-- Assignment kind 3 is (WorldId, InstanceId); kind 2 remains HouseId.
-- Runtime/session kind 2 remains Instance. Origin Channel is participant metadata.
-- This migration installs allocation fencing only. It deliberately refuses a
-- live activity session until the committed activity handoff owner is installed.
-- Rollback is a forward migration: retain all assignment receipts/high water.

CREATE FUNCTION game_activity_scope_key(p_world_id UUID, p_instance_id UUID)
RETURNS BYTEA LANGUAGE sql IMMUTABLE STRICT PARALLEL SAFE AS $$
    SELECT '\x03'::BYTEA || uuid_send(p_world_id) || uuid_send(p_instance_id)
$$;

ALTER TABLE game_runtime_scope_assignments
    ADD COLUMN activity_instance_id UUID NULL
        CHECK (activity_instance_id IS NULL OR game_node_is_uuid_v7(activity_instance_id)),
    DROP CONSTRAINT game_runtime_scope_assignments_scope_kind_check,
    ADD CONSTRAINT game_runtime_scope_assignments_scope_kind_check CHECK (scope_kind IN (1, 2, 3)),
    DROP CONSTRAINT game_runtime_scope_assignment_kind_key,
    ADD CONSTRAINT game_runtime_scope_assignment_kind_key CHECK (
        (scope_kind = 1 AND channel_id IS NOT NULL AND house_key IS NULL
         AND activity_instance_id IS NULL
         AND scope_key = '\x01'::BYTEA || uuid_send(world_id) || uuid_send(channel_id))
        OR
        (scope_kind = 2 AND channel_id IS NULL AND game_house_scope_key_valid(house_key)
         AND activity_instance_id IS NULL AND scope_key = game_house_scope_key(world_id, house_key))
        OR
        (scope_kind = 3 AND channel_id IS NULL AND house_key IS NULL
         AND activity_instance_id IS NOT NULL
         AND scope_key = game_activity_scope_key(world_id, activity_instance_id))
    );
CREATE UNIQUE INDEX game_runtime_activity_instances
    ON game_runtime_scope_assignments(world_id, activity_instance_id) WHERE scope_kind = 3;

CREATE TABLE game_control_activity_scope_grants (
    control_role TEXT NOT NULL CHECK (
        octet_length(control_role) BETWEEN 1 AND 63 AND control_role !~ '[^A-Za-z0-9._:-]'),
    world_id UUID NOT NULL CHECK (game_node_is_uuid_v7(world_id)),
    instance_id UUID NOT NULL CHECK (game_node_is_uuid_v7(instance_id)),
    operation SMALLINT NOT NULL CHECK (operation IN (1, 2, 3)),
    PRIMARY KEY (control_role, world_id, instance_id, operation)
);
REVOKE ALL ON game_control_activity_scope_grants FROM PUBLIC;
GRANT SELECT ON game_control_activity_scope_grants TO oteryn_game_control;


CREATE OR REPLACE FUNCTION game_runtime_scope_assignment_guard() RETURNS trigger
LANGUAGE plpgsql AS $$ BEGIN
    IF TG_OP = 'DELETE'
       OR NEW.scope_key IS DISTINCT FROM OLD.scope_key
       OR NEW.scope_kind IS DISTINCT FROM OLD.scope_kind
       OR NEW.world_id IS DISTINCT FROM OLD.world_id
       OR NEW.channel_id IS DISTINCT FROM OLD.channel_id
       OR NEW.house_key IS DISTINCT FROM OLD.house_key
       OR NEW.activity_instance_id IS DISTINCT FROM OLD.activity_instance_id
       OR NEW.ownership_generation <= OLD.ownership_generation
       OR NEW.source_revision <= OLD.source_revision THEN
        RAISE EXCEPTION 'runtime-scope assignment cannot roll back, reuse a generation or be deleted' USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END; $$;

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
       OR NOT (
            EXISTS (
                SELECT 1 FROM game_control_scope_grants g
                WHERE g.control_role = session_user
                  AND '\x01'::BYTEA || uuid_send(g.world_id) || uuid_send(g.channel_id) = NEW.scope_key
                  AND g.operation = v_kind)
            OR EXISTS (
                SELECT 1 FROM game_control_house_scope_grants g
                WHERE g.control_role = session_user
                  AND game_house_scope_key(g.world_id, g.house_key) = NEW.scope_key
                  AND g.operation = v_kind)
            OR EXISTS (
                SELECT 1 FROM game_control_activity_scope_grants g
                WHERE g.control_role = session_user
                  AND game_activity_scope_key(g.world_id, g.instance_id) = NEW.scope_key
                  AND g.operation = v_kind)
       ) THEN
        RAISE EXCEPTION 'runtime-scope assignment is not granted to this control role' USING ERRCODE = '42501';
    END IF;
    RETURN NEW;
END; $$;

CREATE OR REPLACE FUNCTION game_control_scope_effect_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    v_operation SMALLINT;
BEGIN
    IF TG_OP = 'INSERT' THEN
        IF EXISTS (SELECT 1 FROM game_runtime_scope_assignments WHERE scope_key = NEW.scope_key) THEN
            RETURN NEW;
        END IF;
        v_operation := CASE WHEN NEW.state = 1 THEN 1 ELSE 0 END;
    ELSE
        v_operation := CASE WHEN NEW.state = 2 THEN 3 ELSE 2 END;
    END IF;
    IF NOT (
        (NEW.scope_kind = 1 AND EXISTS (
            SELECT 1 FROM game_control_scope_grants g
            WHERE g.control_role = session_user AND g.world_id = NEW.world_id
              AND g.channel_id = NEW.channel_id AND g.operation = v_operation))
        OR
        (NEW.scope_kind = 2 AND EXISTS (
            SELECT 1 FROM game_control_house_scope_grants g
            WHERE g.control_role = session_user AND g.world_id = NEW.world_id
              AND g.house_key = NEW.house_key AND g.operation = v_operation))
        OR
        (NEW.scope_kind = 3 AND EXISTS (
            SELECT 1 FROM game_control_activity_scope_grants g
            WHERE g.control_role = session_user AND g.world_id = NEW.world_id
              AND g.instance_id = NEW.activity_instance_id AND g.operation = v_operation))
    ) THEN
        RAISE EXCEPTION 'runtime-scope assignment effect is not granted to this control role' USING ERRCODE = '42501';
    END IF;
    RETURN NEW;
END; $$;

-- Preserve the house writer's typed ScopeInUse refusal for an activity collision.
CREATE OR REPLACE FUNCTION game_house_scope_bare_session_exists(p_world_id UUID, p_house_key TEXT)
RETURNS BOOLEAN LANGUAGE plpgsql VOLATILE SECURITY DEFINER AS $$
DECLARE
    v_instance UUID := game_house_scope_instance_id(p_world_id, p_house_key);
BEGIN
    PERFORM game_house_scope_instance_lock(p_world_id, v_instance);
    RETURN EXISTS (
        SELECT 1 FROM game_durability_reconnect_sessions s
        WHERE s.runtime_scope_kind = 2 AND s.runtime_scope_house_key IS NULL
          AND s.session_state IN (1, 2) AND s.runtime_scope_world_id = p_world_id
          AND s.runtime_scope_instance_id = v_instance)
       OR EXISTS (
        SELECT 1 FROM game_runtime_scope_assignments a
        WHERE a.scope_kind = 3 AND a.world_id = p_world_id AND a.activity_instance_id = v_instance);
END; $$;

-- Assignment and bare-session admission use the existing shared instance lock.
-- The identity of an ended activity is retained and cannot become a house.
CREATE FUNCTION game_activity_scope_available(p_world_id UUID, p_instance_id UUID)
RETURNS BOOLEAN LANGUAGE plpgsql VOLATILE SECURITY DEFINER AS $$
BEGIN
    PERFORM game_house_scope_instance_lock(p_world_id, p_instance_id);
    RETURN NOT EXISTS (
        SELECT 1 FROM game_runtime_scope_assignments a WHERE a.world_id = p_world_id
          AND a.scope_kind = 2
          AND game_house_scope_instance_id(a.world_id, a.house_key) = p_instance_id)
       AND NOT EXISTS (
        SELECT 1 FROM game_durability_reconnect_sessions s
        WHERE s.runtime_scope_world_id = p_world_id AND s.runtime_scope_instance_id = p_instance_id
          AND s.runtime_scope_kind = 2 AND s.session_state IN (1, 2));
END; $$;

CREATE FUNCTION game_activity_scope_assignment_admission() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    v_instance UUID;
BEGIN
    IF NEW.scope_kind = 3 THEN
        v_instance := NEW.activity_instance_id;
        -- The availability test is only for a new identity; successor assignments
        -- keep the same identity and fence all earlier generations.
        IF TG_OP = 'INSERT' AND NOT game_activity_scope_available(NEW.world_id, v_instance) THEN
            RAISE EXCEPTION 'activity instance identity is already in use' USING ERRCODE = '23514';
        END IF;
    ELSIF NEW.scope_kind = 2 THEN
        v_instance := game_house_scope_instance_id(NEW.world_id, NEW.house_key);
        PERFORM game_house_scope_instance_lock(NEW.world_id, v_instance);
        IF EXISTS (SELECT 1 FROM game_runtime_scope_assignments a
                   WHERE a.world_id = NEW.world_id AND a.scope_kind = 3
                     AND a.activity_instance_id = v_instance) THEN
            RAISE EXCEPTION 'house instance identity belongs to an activity' USING ERRCODE = '23514';
        END IF;
    END IF;
    RETURN NEW;
END; $$;
CREATE TRIGGER game_activity_scope_assignment_admission BEFORE INSERT OR UPDATE
    ON game_runtime_scope_assignments FOR EACH ROW
    EXECUTE FUNCTION game_activity_scope_assignment_admission();

-- No bare session may acquire an activity identity before the handoff contract
-- can prove terminal source + fresh destination + origin return binding.
CREATE FUNCTION game_activity_scope_session_guard() RETURNS trigger
LANGUAGE plpgsql AS $$ BEGIN
    IF NEW.runtime_scope_kind = 2 AND NEW.session_state IN (1, 2) THEN
        PERFORM game_house_scope_instance_lock(NEW.runtime_scope_world_id, NEW.runtime_scope_instance_id);
        IF EXISTS (SELECT 1 FROM game_runtime_scope_assignments a
                   WHERE a.scope_kind = 3 AND a.world_id = NEW.runtime_scope_world_id
                     AND a.activity_instance_id = NEW.runtime_scope_instance_id) THEN
            RAISE EXCEPTION 'activity session requires a committed activity handoff' USING ERRCODE = '23514';
        END IF;
    END IF;
    RETURN NEW;
END; $$;
CREATE TRIGGER game_activity_scope_session_guard BEFORE INSERT OR UPDATE
    ON game_durability_reconnect_sessions FOR EACH ROW
    EXECUTE FUNCTION game_activity_scope_session_guard();


CREATE OR REPLACE FUNCTION game_runtime_guard_requires_current_assignment() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, public, pg_temp AS $$
DECLARE
    v_assignment game_runtime_scope_assignments%ROWTYPE;
BEGIN
    SELECT * INTO v_assignment FROM game_runtime_scope_assignments
        WHERE scope_key = NEW.scope_key FOR SHARE;
    IF NOT FOUND AND octet_length(NEW.scope_key) = 33 AND get_byte(NEW.scope_key, 0) = 2 THEN
        SELECT * INTO v_assignment FROM game_runtime_scope_assignments
        WHERE scope_key = '\x03'::BYTEA || substring(NEW.scope_key FROM 2) AND scope_kind = 3
        FOR SHARE;
    END IF;
    IF v_assignment.scope_key IS NULL THEN
        RETURN NEW;
    END IF;
    -- A partially restored assignment row is never trusted for any Runtime
    -- publication: authoritative retained history must validate first.
    IF NOT game_runtime_scope_assignment_history_valid() THEN
        RAISE EXCEPTION 'runtime publication requires valid assignment history' USING ERRCODE = '23514';
    END IF;
    IF v_assignment.ownership_generation <> NEW.ownership_generation THEN
        RAISE EXCEPTION 'runtime publication requires the current assignment generation' USING ERRCODE = '23514';
    END IF;
    IF NEW.ready THEN
        IF NOT (
            v_assignment.state = 1
            AND game_node_lock_current_registration(
                v_assignment.holder_node_id, v_assignment.holder_registration_revision)
        ) THEN
            RAISE EXCEPTION 'runtime readiness requires the current scope assignment' USING ERRCODE = '23514';
        END IF;
        -- Checked, not consumed: INSERT .. ON CONFLICT DO UPDATE fires both
        -- the insert and update row triggers. The attesting publisher
        -- removes its own attestation before commit.
        PERFORM 1 FROM game_runtime_readiness_attestations
            WHERE attested_xact = pg_current_xact_id() AND scope_key = v_assignment.scope_key
              AND holder_node_id = v_assignment.holder_node_id
              AND holder_registration_revision = v_assignment.holder_registration_revision
              AND ownership_generation = v_assignment.ownership_generation;
        IF NOT FOUND THEN
            RAISE EXCEPTION 'runtime readiness requires an attested current holder' USING ERRCODE = '23514';
        END IF;
    END IF;
    RETURN NEW;
END; $$;

-- Pin every SECURITY DEFINER and replaced trigger to this migration's schema.
DO $$ BEGIN
    EXECUTE format('ALTER FUNCTION game_runtime_scope_assignment_guard() SET search_path = %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_control_scope_grant_guard() SET search_path = %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_control_scope_effect_guard() SET search_path = %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_house_scope_bare_session_exists(UUID, TEXT) SET search_path = %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_activity_scope_available(UUID, UUID) SET search_path = %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_activity_scope_assignment_admission() SET search_path = %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_activity_scope_session_guard() SET search_path = %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_runtime_guard_requires_current_assignment() SET search_path = %I, pg_temp', current_schema());
END $$;
REVOKE ALL ON FUNCTION game_activity_scope_available(UUID, UUID),
    game_activity_scope_assignment_admission(), game_activity_scope_session_guard() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_activity_scope_available(UUID, UUID) TO oteryn_game_control;
