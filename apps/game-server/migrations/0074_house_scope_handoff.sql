-- SCOPE-HANDOFF-1: the house runtime scope and the recoverable entry handoff (packet
-- `reviews/OTERYN_GAME_ARCH_BATCH_PREMIUM_SOCIAL_PACKETS_2026-10-04.md` §2.3; decision
-- `reviews/OTERYN_GAME_HOUSE_RUNTIME0_HOUSE_INTERIOR_RUNTIME_DECISION_2026-09-30.md` §4.1-§4.3
-- with its ADMIT-0 amendment; HOUSE-CUSTODY-0 §3.4 direction; ADR-0001 §10). Scope of this
-- migration only:
--   * runtime-scope assignments gain the house kind: `scope_kind` 1 Channel / 2 House, the
--     `house_key`, a nullable `channel_id`, a tagged `scope_key` (`\x01` world channel, `\x02`
--     world house key), and the immutability guard compares with IS DISTINCT FROM;
--   * `game_control_house_scope_grants`: exact-house control grants (1 assign, 2 replace,
--     3 revoke), checked by the receipt and effect guards for a house scope exactly as the
--     Channel grants are for a Channel scope;
--   * reconnect sessions of scope kind 2 may name the house (`runtime_scope_house_key`) and the
--     origin Channel (`origin_channel_id`); the instance id of a house session is derived from
--     the HouseId by `game_house_scope_instance_id`, so one house has one instance id; a
--     terminal replacement of a house session inherits its predecessor's house and origin
--     Channel, and no other session may take a house instance id without them;
--   * `game_house_scope_handoffs`: one row per entry handoff, PREPARED (1) while the source
--     session stays live, COMMITTED (2) in the transaction that makes the source terminal and
--     admits the house session. Abort deletes a PREPARED row and its tile reservation; a
--     COMMITTED row is retained history and immutable. Only the entry direction (1) exists:
--     the exit into a Channel scope stays refused until ADMIT-0 (HOUSE-RUNTIME-0 amendment).
-- ADR-0021 reset step 2 has no implementation on main; its writer, when built, assigns Channel
-- scopes only (`scope_kind = 1`).
--
-- Rollback: applied migrations are immutable, so rollback is a new migration. Before any house
-- scope is assigned it drops the handoff and grant tables, the session columns and the
-- assignment columns and restores the 0003, 0006 and 0008 guard bodies. After a house scope or
-- a house session exists they are authoritative runtime and admission history: a rollback keeps
-- them and may only stop new writes by revoking the grants below.

CREATE FUNCTION game_house_scope_key_valid(p_house_key TEXT) RETURNS BOOLEAN
LANGUAGE sql IMMUTABLE PARALLEL SAFE AS $$
    SELECT p_house_key IS NOT NULL AND octet_length(p_house_key) <= 512
       AND p_house_key ~ '^oteryn:content\.house\.[a-z0-9_]+$'
$$;

-- The instance id of a HouseId: the first 16 bytes of SHA-256 over a fixed domain tag, the
-- World and the house key, with the UUIDv7 version and RFC variant bits set. The Rust runtime
-- derives the same bytes (`house_scope::HouseId::instance_id`).
CREATE FUNCTION game_house_scope_instance_id(p_world_id UUID, p_house_key TEXT) RETURNS UUID
LANGUAGE sql IMMUTABLE STRICT PARALLEL SAFE AS $$
    SELECT encode(set_byte(set_byte(d, 6, (get_byte(d, 6) & 15) | 112),
                           8, (get_byte(d, 8) & 63) | 128), 'hex')::uuid
      FROM (SELECT substring(sha256('oteryn:house-scope-instance:v1'::BYTEA
                                    || uuid_send(p_world_id) || textsend(p_house_key))
                             FROM 1 FOR 16) AS d) AS digest
$$;

CREATE FUNCTION game_house_scope_key(p_world_id UUID, p_house_key TEXT) RETURNS BYTEA
LANGUAGE sql IMMUTABLE STRICT PARALLEL SAFE AS $$
    SELECT '\x02'::BYTEA || uuid_send(p_world_id) || textsend(p_house_key)
$$;

-- Assignments: the house kind.
ALTER TABLE game_runtime_scope_assignments
    ADD COLUMN scope_kind SMALLINT NOT NULL DEFAULT 1 CHECK (scope_kind IN (1, 2)),
    ADD COLUMN house_key TEXT NULL,
    ALTER COLUMN channel_id DROP NOT NULL,
    DROP CONSTRAINT game_runtime_scope_assignments_channel_id_check,
    ADD CONSTRAINT game_runtime_scope_assignment_channel_id_v7
        CHECK (channel_id IS NULL OR game_node_is_uuid_v7(channel_id)),
    DROP CONSTRAINT game_runtime_scope_assignments_check,
    ADD CONSTRAINT game_runtime_scope_assignment_kind_key CHECK (
        (scope_kind = 1 AND channel_id IS NOT NULL AND house_key IS NULL
         AND scope_key = '\x01'::BYTEA || uuid_send(world_id) || uuid_send(channel_id))
        OR
        (scope_kind = 2 AND channel_id IS NULL AND game_house_scope_key_valid(house_key)
         AND scope_key = game_house_scope_key(world_id, house_key))
    );

CREATE OR REPLACE FUNCTION game_runtime_scope_assignment_guard() RETURNS trigger
LANGUAGE plpgsql AS $$ BEGIN
    IF TG_OP = 'DELETE'
       OR NEW.scope_key IS DISTINCT FROM OLD.scope_key
       OR NEW.scope_kind IS DISTINCT FROM OLD.scope_kind
       OR NEW.world_id IS DISTINCT FROM OLD.world_id
       OR NEW.channel_id IS DISTINCT FROM OLD.channel_id
       OR NEW.house_key IS DISTINCT FROM OLD.house_key
       OR NEW.ownership_generation <= OLD.ownership_generation
       OR NEW.source_revision <= OLD.source_revision THEN
        RAISE EXCEPTION 'runtime-scope assignment cannot roll back, reuse a generation or be deleted' USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END; $$;

-- Exact-house control authorization: one row per control login role, house scope and
-- permitted operation (1 assign, 2 replace, 3 revoke). Only the database owner writes it.
CREATE TABLE game_control_house_scope_grants (
    control_role TEXT NOT NULL CHECK (
        octet_length(control_role) BETWEEN 1 AND 63
        AND control_role !~ '[^A-Za-z0-9._:-]'
    ),
    world_id UUID NOT NULL CHECK (game_node_is_uuid_v7(world_id)),
    house_key TEXT NOT NULL CHECK (game_house_scope_key_valid(house_key)),
    operation SMALLINT NOT NULL CHECK (operation IN (1, 2, 3)),
    PRIMARY KEY (control_role, world_id, house_key, operation)
);

-- The 0008 receipt guard, with the house grant for a `\x02` scope key.
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
       ) THEN
        RAISE EXCEPTION 'runtime-scope assignment is not granted to this control role' USING ERRCODE = '42501';
    END IF;
    RETURN NEW;
END; $$;

-- The 0006 effect guard, reading the grant of the row's own kind.
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
    ) THEN
        RAISE EXCEPTION 'runtime-scope assignment effect is not granted to this control role' USING ERRCODE = '42501';
    END IF;
    RETURN NEW;
END; $$;

-- Sessions: a house session names its house and its origin Channel.
ALTER TABLE game_durability_reconnect_sessions
    ADD COLUMN runtime_scope_house_key TEXT NULL,
    ADD COLUMN origin_channel_id UUID NULL,
    ADD CONSTRAINT game_durability_house_scope_session CHECK (
        (runtime_scope_house_key IS NULL AND origin_channel_id IS NULL)
        OR
        (runtime_scope_kind = 2
         AND game_house_scope_key_valid(runtime_scope_house_key)
         AND runtime_scope_instance_id
             = game_house_scope_instance_id(runtime_scope_world_id, runtime_scope_house_key)
         AND origin_channel_id IS NOT NULL
         AND (get_byte(uuid_send(origin_channel_id), 6) >> 4) = 7
         AND (get_byte(uuid_send(origin_channel_id), 8) & 192) = 128)
    );

-- A terminal replacement (0001 receipt, then the candidate row) inserts the candidate with the
-- predecessor's runtime scope but no house columns: it inherits them from the predecessor named
-- by its replacement receipt. Any other session of a house instance id without them is refused.
CREATE INDEX game_durability_house_scope_instance_sessions
    ON game_durability_reconnect_sessions (runtime_scope_world_id, runtime_scope_instance_id)
    WHERE runtime_scope_house_key IS NOT NULL;
CREATE FUNCTION game_house_scope_session_inherit() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.runtime_scope_kind = 2 AND NEW.runtime_scope_house_key IS NULL THEN
        SELECT p.runtime_scope_house_key, p.origin_channel_id
          INTO NEW.runtime_scope_house_key, NEW.origin_channel_id
          FROM game_durability_session_replacements r
          JOIN game_durability_reconnect_sessions p
            ON p.game_session_id = r.predecessor_game_session_id
         WHERE r.character_id = NEW.character_id
           AND r.candidate_game_session_id = NEW.game_session_id
           AND p.character_id = NEW.character_id
           AND p.runtime_scope_kind = 2
           AND p.runtime_scope_world_id = NEW.runtime_scope_world_id
           AND p.runtime_scope_instance_id = NEW.runtime_scope_instance_id
           AND p.runtime_scope_house_key IS NOT NULL;
        IF NEW.runtime_scope_house_key IS NULL AND EXISTS (
                SELECT 1 FROM game_durability_reconnect_sessions s
                WHERE s.runtime_scope_world_id = NEW.runtime_scope_world_id
                  AND s.runtime_scope_instance_id = NEW.runtime_scope_instance_id
                  AND s.runtime_scope_house_key IS NOT NULL) THEN
            RAISE EXCEPTION 'a house scope session must name its house and origin Channel'
                USING ERRCODE = '23514';
        END IF;
    END IF;
    RETURN NEW;
END; $$;
CREATE TRIGGER game_house_scope_session_inherit BEFORE INSERT
    ON game_durability_reconnect_sessions FOR EACH ROW
    EXECUTE FUNCTION game_house_scope_session_inherit();

-- Entry handoffs. state: 1 PREPARED, 2 COMMITTED. direction: 1 entry (the only one admitted).
CREATE TABLE game_house_scope_handoffs (
    handoff_id UUID PRIMARY KEY
        CHECK ((get_byte(uuid_send(handoff_id), 6) >> 4) = 7)
        CHECK ((get_byte(uuid_send(handoff_id), 8) & 192) = 128),
    direction SMALLINT NOT NULL CHECK (direction = 1),
    character_id UUID NOT NULL
        CHECK ((get_byte(uuid_send(character_id), 6) >> 4) = 7)
        CHECK ((get_byte(uuid_send(character_id), 8) & 192) = 128),
    account_id UUID NOT NULL,
    world_id UUID NOT NULL
        CHECK ((get_byte(uuid_send(world_id), 6) >> 4) = 7)
        CHECK ((get_byte(uuid_send(world_id), 8) & 192) = 128),
    origin_channel_id UUID NOT NULL
        CHECK ((get_byte(uuid_send(origin_channel_id), 6) >> 4) = 7)
        CHECK ((get_byte(uuid_send(origin_channel_id), 8) & 192) = 128),
    source_game_session_id UUID NOT NULL UNIQUE
        REFERENCES game_durability_reconnect_sessions (game_session_id),
    source_connection_generation NUMERIC(20, 0) NOT NULL
        CHECK (source_connection_generation BETWEEN 1 AND 18446744073709551615),
    source_character_lease_generation NUMERIC(20, 0) NOT NULL
        CHECK (source_character_lease_generation BETWEEN 1 AND 18446744073709551614),
    source_scope_ownership_generation NUMERIC(20, 0) NOT NULL
        CHECK (source_scope_ownership_generation BETWEEN 1 AND 18446744073709551615),
    house_key TEXT NOT NULL CHECK (game_house_scope_key_valid(house_key)),
    destination_scope_ownership_generation NUMERIC(20, 0) NOT NULL
        CHECK (destination_scope_ownership_generation BETWEEN 1 AND 18446744073709551615),
    acl_revision NUMERIC(20, 0) NOT NULL
        CHECK (acl_revision BETWEEN 0 AND 18446744073709551615),
    -- The guild revisions the pre-check used for a guild-entry grant, else NULL.
    guild_revisions JSONB NULL,
    -- The reserved tile, in the HouseInterior `spatial_position` encoding (0025).
    reserved_tile BYTEA NOT NULL CHECK (octet_length(reserved_tile) BETWEEN 1 AND 128),
    state SMALLINT NOT NULL CHECK (state IN (1, 2)),
    destination_game_session_id UUID NULL UNIQUE
        REFERENCES game_durability_reconnect_sessions (game_session_id),
    prepared_at BIGINT NOT NULL CHECK (prepared_at >= 0),
    committed_at BIGINT NULL CHECK (committed_at IS NULL OR committed_at >= prepared_at),
    CHECK ((state = 2) = (destination_game_session_id IS NOT NULL)),
    CHECK ((state = 2) = (committed_at IS NOT NULL)),
    CHECK (destination_game_session_id IS DISTINCT FROM source_game_session_id)
);

CREATE UNIQUE INDEX game_house_scope_one_open_handoff_per_character
    ON game_house_scope_handoffs (character_id) WHERE state = 1;
CREATE UNIQUE INDEX game_house_scope_one_reservation_per_tile
    ON game_house_scope_handoffs (world_id, house_key, reserved_tile) WHERE state = 1;
CREATE INDEX game_house_scope_open_handoffs_by_house
    ON game_house_scope_handoffs (world_id, house_key) WHERE state = 1;
CREATE INDEX game_durability_house_scope_sessions
    ON game_durability_reconnect_sessions (runtime_scope_world_id, runtime_scope_house_key)
    WHERE runtime_scope_house_key IS NOT NULL AND session_state IN (1, 2);

-- A handoff is born PREPARED; its only update is PREPARED -> COMMITTED, which sets exactly the
-- destination session and the commit time; only a PREPARED row is deleted (abort).
CREATE FUNCTION game_house_scope_handoff_guard() RETURNS trigger
LANGUAGE plpgsql AS $$ BEGIN
    IF TG_OP = 'INSERT' THEN
        IF NEW.state <> 1 THEN
            RAISE EXCEPTION 'a house scope handoff is prepared before it commits' USING ERRCODE = '23514';
        END IF;
        RETURN NEW;
    END IF;
    IF OLD.state <> 1 THEN
        RAISE EXCEPTION 'a committed house scope handoff is immutable' USING ERRCODE = '23514';
    END IF;
    IF TG_OP = 'DELETE' THEN
        RETURN OLD;
    END IF;
    IF NEW.state <> 2
       OR (NEW.handoff_id, NEW.direction, NEW.character_id, NEW.account_id, NEW.world_id,
           NEW.origin_channel_id, NEW.source_game_session_id, NEW.source_connection_generation,
           NEW.source_character_lease_generation, NEW.source_scope_ownership_generation,
           NEW.house_key, NEW.destination_scope_ownership_generation, NEW.acl_revision,
           NEW.guild_revisions, NEW.reserved_tile, NEW.prepared_at)
          IS DISTINCT FROM
          (OLD.handoff_id, OLD.direction, OLD.character_id, OLD.account_id, OLD.world_id,
           OLD.origin_channel_id, OLD.source_game_session_id, OLD.source_connection_generation,
           OLD.source_character_lease_generation, OLD.source_scope_ownership_generation,
           OLD.house_key, OLD.destination_scope_ownership_generation, OLD.acl_revision,
           OLD.guild_revisions, OLD.reserved_tile, OLD.prepared_at) THEN
        RAISE EXCEPTION 'a house scope handoff only commits' USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END; $$;
CREATE TRIGGER game_house_scope_handoff_guard BEFORE INSERT OR UPDATE OR DELETE
    ON game_house_scope_handoffs FOR EACH ROW EXECUTE FUNCTION game_house_scope_handoff_guard();

-- At commit time a COMMITTED handoff is proven by its sessions: the source session is
-- terminal, and the destination is a house session of the same Character, Account, World,
-- house, origin Channel and scope generation, at a later lease generation.
CREATE FUNCTION game_house_scope_handoff_commit_proven() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    v_handoff game_house_scope_handoffs%ROWTYPE;
BEGIN
    SELECT * INTO v_handoff FROM game_house_scope_handoffs WHERE handoff_id = NEW.handoff_id;
    IF NOT FOUND OR v_handoff.state <> 2 THEN
        RETURN NULL;
    END IF;
    IF NOT EXISTS (
            SELECT 1 FROM game_durability_reconnect_sessions s
            WHERE s.game_session_id = v_handoff.source_game_session_id
              AND s.character_id = v_handoff.character_id
              AND s.account_id = v_handoff.account_id
              AND s.world_id = v_handoff.world_id
              AND s.runtime_scope_kind = 1
              AND s.runtime_scope_channel_id = v_handoff.origin_channel_id
              AND s.session_state = 3)
       OR NOT EXISTS (
            SELECT 1 FROM game_durability_reconnect_sessions d
            WHERE d.game_session_id = v_handoff.destination_game_session_id
              AND d.character_id = v_handoff.character_id
              AND d.account_id = v_handoff.account_id
              AND d.world_id = v_handoff.world_id
              AND d.runtime_scope_kind = 2
              AND d.runtime_scope_world_id = v_handoff.world_id
              AND d.runtime_scope_house_key = v_handoff.house_key
              AND d.origin_channel_id = v_handoff.origin_channel_id
              AND d.scope_ownership_generation = v_handoff.destination_scope_ownership_generation
              AND d.character_lease_generation > v_handoff.source_character_lease_generation)
    THEN
        RAISE EXCEPTION 'a committed house scope handoff requires its terminal source and house session'
            USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END; $$;
CREATE CONSTRAINT TRIGGER game_house_scope_handoff_commit_proven
    AFTER UPDATE ON game_house_scope_handoffs
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
    EXECUTE FUNCTION game_house_scope_handoff_commit_proven();

-- House access (OTERYN_GAME_HOUSE_RT_INBOX_PACKETS_2026-10-04 §1.2): the role (OWNER, SUBOWNER,
-- GUEST) or no row, whether the content fence is set, the access-list revision and, for a
-- guild-entry grant, the guild revisions used. The admission commit compares it with its
-- pre-check and refuses on any difference. This stub returns no row, so every house refuses
-- NO_ACCESS (HOUSE-RUNTIME-0 §5.1: a house with no owner admits nobody). HOUSE-1a replaces the
-- body once by CREATE OR REPLACE with this signature; the body takes the §4.1 FOR SHARE locks.
CREATE FUNCTION game_house_access(p_world_id UUID, p_house_key TEXT, p_character_id UUID)
RETURNS TABLE (
    role TEXT,
    content_fenced BOOLEAN,
    acl_revision NUMERIC(20, 0),
    guild_revisions JSONB
)
LANGUAGE sql AS $$
    SELECT NULL::TEXT, NULL::BOOLEAN, NULL::NUMERIC(20, 0), NULL::JSONB WHERE false
$$;

DO $$ BEGIN
    EXECUTE format('ALTER FUNCTION game_house_access(UUID, TEXT, UUID) SET search_path = %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_runtime_scope_assignment_guard() SET search_path = %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_control_scope_grant_guard() SET search_path = %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_control_scope_effect_guard() SET search_path = %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_house_scope_handoff_guard() SET search_path = %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_house_scope_handoff_commit_proven() SET search_path = %I, pg_temp', current_schema());
END $$;

REVOKE ALL ON TABLE game_control_house_scope_grants, game_house_scope_handoffs FROM PUBLIC;
REVOKE ALL ON FUNCTION
    game_house_scope_handoff_guard(),
    game_house_scope_handoff_commit_proven(),
    game_house_access(UUID, TEXT, UUID)
FROM PUBLIC;

-- The GameNode runtime writes handoffs inside its fenced admission transactions; the control
-- plane reads the house grants for its own decisions and may inspect handoffs.
GRANT SELECT, INSERT, UPDATE, DELETE ON game_house_scope_handoffs TO oteryn_game_runtime;
-- The admission commit calls the access function; the runtime holds no property row grant.
GRANT EXECUTE ON FUNCTION game_house_access(UUID, TEXT, UUID) TO oteryn_game_runtime;
GRANT SELECT ON game_control_house_scope_grants, game_house_scope_handoffs TO oteryn_game_control;
