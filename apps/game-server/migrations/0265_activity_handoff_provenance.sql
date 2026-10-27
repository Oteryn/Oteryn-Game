-- BOSS-ENCOUNTER-1: retained activity handoff provenance and strict group barrier.
-- This is storage qualification, not activity admission. In particular, neither
-- the 0074 house-session CHECK nor the 0247 activity-session refusal is relaxed.
-- The runtime receives read access only until the handoff writer, fresh-session
-- issuance, return/recovery owner and InstanceRuntime are qualified together.
-- Rollback is forward-only: preserve snapshots and committed receipts.

CREATE TABLE game_activity_handoff_groups (
    handoff_id UUID PRIMARY KEY CHECK (game_node_is_uuid_v7(handoff_id)),
    world_id UUID NOT NULL CHECK (game_node_is_uuid_v7(world_id)),
    instance_id UUID NOT NULL CHECK (game_node_is_uuid_v7(instance_id)),
    activity_key TEXT NOT NULL CHECK (octet_length(activity_key) BETWEEN 1 AND 512),
    template_revision TEXT NOT NULL
        CHECK (template_revision ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'),
    content_revision TEXT NOT NULL
        CHECK (content_revision ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'),
    ownership_generation NUMERIC(20,0) NOT NULL
        CHECK (ownership_generation BETWEEN 1 AND 18446744073709551615),
    -- Storage bound only; activity content must impose its own smaller group limit.
    participant_count SMALLINT NOT NULL CHECK (participant_count > 0),
    -- PREPARED, COMMITTED, ABORTED. Terminal outcomes remain retained history.
    state SMALLINT NOT NULL CHECK (state IN (1, 2, 3)),
    prepared_at BIGINT NOT NULL CHECK (prepared_at >= 0),
    decided_at BIGINT NULL CHECK (decided_at >= prepared_at),
    CHECK ((state = 1) = (decided_at IS NULL)),
    UNIQUE (world_id, instance_id),
    UNIQUE (handoff_id, world_id)
);

CREATE TABLE game_activity_handoff_participants (
    handoff_id UUID NOT NULL,
    world_id UUID NOT NULL,
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    account_id UUID NOT NULL CHECK (game_node_is_uuid_v7(account_id)),
    origin_channel_id UUID NOT NULL CHECK (game_node_is_uuid_v7(origin_channel_id)),
    source_game_session_id UUID NOT NULL
        REFERENCES game_durability_reconnect_sessions(game_session_id),
    destination_game_session_id UUID NOT NULL UNIQUE
        CHECK (game_node_is_uuid_v7(destination_game_session_id)),
    source_connection_generation NUMERIC(20,0) NOT NULL
        CHECK (source_connection_generation BETWEEN 1 AND 18446744073709551615),
    source_lease_generation NUMERIC(20,0) NOT NULL
        CHECK (source_lease_generation BETWEEN 1 AND 18446744073709551614),
    source_ownership_generation NUMERIC(20,0) NOT NULL
        CHECK (source_ownership_generation BETWEEN 1 AND 18446744073709551615),
    profile_revision TEXT NOT NULL
        CHECK (profile_revision ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'),
    ruleset_revision TEXT NOT NULL
        CHECK (ruleset_revision ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'),
    content_revision TEXT NOT NULL
        CHECK (content_revision ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'),
    starter_template_revision TEXT NOT NULL
        CHECK (starter_template_revision ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'),
    -- Saved Channel entry cell and its interpretation; never Instance authority.
    entry_x INTEGER NOT NULL,
    entry_y INTEGER NOT NULL,
    entry_floor SMALLINT NOT NULL,
    coordinate_frame_digest BYTEA NOT NULL CHECK (octet_length(coordinate_frame_digest) = 32),
    map_revision_digest BYTEA NOT NULL CHECK (octet_length(map_revision_digest) = 32),
    server_artifact_digest BYTEA NOT NULL CHECK (octet_length(server_artifact_digest) = 32),
    PRIMARY KEY (handoff_id, character_id),
    UNIQUE (handoff_id, account_id),
    FOREIGN KEY (handoff_id, world_id)
        REFERENCES game_activity_handoff_groups(handoff_id, world_id),
    CHECK (destination_game_session_id <> source_game_session_id)
);
CREATE INDEX game_activity_handoff_source_history
    ON game_activity_handoff_participants(source_game_session_id, handoff_id);

-- Serialize membership with group decisions; membership is append-only and its
-- origin/content snapshot is immutable even when a preparation is aborted.
CREATE FUNCTION game_activity_handoff_participant_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    v_group game_activity_handoff_groups%ROWTYPE;
    v_count BIGINT;
BEGIN
    IF TG_OP <> 'INSERT' THEN
        RAISE EXCEPTION 'activity participant provenance is immutable' USING ERRCODE = '23514';
    END IF;
    SELECT * INTO v_group FROM game_activity_handoff_groups
        WHERE handoff_id = NEW.handoff_id FOR UPDATE;
    IF NOT FOUND OR v_group.state <> 1 THEN
        RAISE EXCEPTION 'activity membership requires a prepared group' USING ERRCODE = '23514';
    END IF;
    SELECT count(*) INTO v_count FROM (
        SELECT 1 FROM game_activity_handoff_participants
        WHERE handoff_id = NEW.handoff_id LIMIT v_group.participant_count
    ) members;
    IF v_count >= v_group.participant_count THEN
        RAISE EXCEPTION 'activity group membership is full' USING ERRCODE = '23514';
    END IF;
    -- Serialize competing preparations of this source, while allowing a retry
    -- with a fresh Instance/session after a retained ABORTED outcome.
    PERFORM 1 FROM game_durability_reconnect_sessions
        WHERE game_session_id = NEW.source_game_session_id FOR UPDATE;
    IF EXISTS (
        SELECT 1 FROM game_activity_handoff_participants p
        JOIN game_activity_handoff_groups g ON g.handoff_id = p.handoff_id
        WHERE p.source_game_session_id = NEW.source_game_session_id AND g.state IN (1, 2)
        LIMIT 1
    ) THEN
        RAISE EXCEPTION 'activity source already has a pending or committed group' USING ERRCODE = '23514';
    END IF;
    -- These are snapshot consistency checks, not a replacement for the writer's
    -- current recovery, node possession, readiness and participant/root fences.
    PERFORM 1 FROM game_durability_reconnect_sessions s
        JOIN game_character_roots r ON r.character_id = s.character_id
        JOIN game_character_last_positions p ON p.character_id = s.character_id
        WHERE s.game_session_id = NEW.source_game_session_id
          AND s.session_state IN (1, 2) AND s.runtime_scope_kind = 1
          AND s.runtime_scope_house_key IS NULL AND s.origin_channel_id IS NULL
          AND s.character_id = NEW.character_id AND s.account_id = NEW.account_id
          AND s.world_id = NEW.world_id AND s.runtime_scope_world_id = NEW.world_id
          AND s.runtime_scope_channel_id = NEW.origin_channel_id
          AND s.current_generation = NEW.source_connection_generation
          AND s.character_lease_generation = NEW.source_lease_generation
          AND s.scope_ownership_generation = NEW.source_ownership_generation
          AND r.account_id = NEW.account_id AND r.world_id = NEW.world_id
          AND (r.profile_revision, r.ruleset_revision, r.content_revision, r.starter_template_revision)
              = (NEW.profile_revision, NEW.ruleset_revision, NEW.content_revision, NEW.starter_template_revision)
          AND NEW.content_revision = v_group.content_revision
          AND p.world_id = NEW.world_id
          AND p.character_lease_generation = NEW.source_lease_generation
          AND (p.tile_x, p.tile_y, p.tile_floor) = (NEW.entry_x, NEW.entry_y, NEW.entry_floor)
          AND p.coordinate_frame_digest = NEW.coordinate_frame_digest
          AND p.map_revision_digest = NEW.map_revision_digest
          AND p.server_artifact_digest = NEW.server_artifact_digest
        FOR SHARE OF s, r, p;
    IF NOT FOUND OR EXISTS (
        SELECT 1 FROM game_durability_reconnect_sessions
        WHERE game_session_id = NEW.destination_game_session_id
    ) THEN
        RAISE EXCEPTION 'activity provenance requires its exact source and fresh destination'
            USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END; $$;
CREATE TRIGGER game_activity_handoff_participant_guard BEFORE INSERT OR UPDATE OR DELETE
    ON game_activity_handoff_participants FOR EACH ROW
    EXECUTE FUNCTION game_activity_handoff_participant_guard();

CREATE FUNCTION game_activity_handoff_group_guard() RETURNS trigger
LANGUAGE plpgsql AS $$ BEGIN
    IF TG_OP = 'INSERT' THEN
        IF NEW.state = 1 THEN
            RETURN NEW;
        END IF;
    ELSIF TG_OP = 'UPDATE' THEN
        IF OLD.state = 1 AND NEW.state IN (2, 3)
           AND (to_jsonb(NEW) - 'state' - 'decided_at')
               = (to_jsonb(OLD) - 'state' - 'decided_at') THEN
            RETURN NEW;
        END IF;
    END IF;
    RAISE EXCEPTION 'activity group only prepares then commits or aborts once' USING ERRCODE = '23514';
END; $$;
CREATE TRIGGER game_activity_handoff_group_guard BEFORE INSERT OR UPDATE OR DELETE
    ON game_activity_handoff_groups FOR EACH ROW
    EXECUTE FUNCTION game_activity_handoff_group_guard();

-- A committed group proves every declared participant in the same transaction.
-- No session is admitted by this trigger. The 0247 refusal remains authoritative.
CREATE FUNCTION game_activity_handoff_group_proven() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    v_group game_activity_handoff_groups%ROWTYPE;
    v_member game_activity_handoff_participants%ROWTYPE;
    v_count BIGINT;
BEGIN
    SELECT * INTO STRICT v_group FROM game_activity_handoff_groups
        WHERE handoff_id = NEW.handoff_id FOR UPDATE;
    IF v_group.state <> 2 THEN
        RETURN NULL;
    END IF;
    SELECT count(*) INTO v_count FROM (
        SELECT 1 FROM game_activity_handoff_participants
        WHERE handoff_id = v_group.handoff_id LIMIT v_group.participant_count
    ) members;
    IF v_count <> v_group.participant_count THEN
        RAISE EXCEPTION 'activity commit requires the complete group' USING ERRCODE = '23514';
    END IF;
    FOR v_member IN SELECT * FROM game_activity_handoff_participants
        WHERE handoff_id = v_group.handoff_id ORDER BY character_id
        LIMIT v_group.participant_count
    LOOP
        PERFORM 1 FROM game_durability_reconnect_sessions s
            JOIN game_durability_reconnect_sessions d
              ON d.game_session_id = v_member.destination_game_session_id
            WHERE s.game_session_id = v_member.source_game_session_id
              AND s.character_id = v_member.character_id AND s.account_id = v_member.account_id
              AND s.world_id = v_member.world_id AND s.runtime_scope_world_id = v_member.world_id
              AND s.runtime_scope_kind = 1 AND s.runtime_scope_channel_id = v_member.origin_channel_id
              AND s.current_generation = v_member.source_connection_generation
              AND s.character_lease_generation = v_member.source_lease_generation
              AND s.scope_ownership_generation = v_member.source_ownership_generation
              AND s.session_state = 3 AND s.current_transport_ref IS NULL
              AND s.prepared_attempt_ref IS NULL
              AND d.character_id = v_member.character_id AND d.account_id = v_member.account_id
              AND d.world_id = v_member.world_id AND d.runtime_scope_world_id = v_member.world_id
              AND d.runtime_scope_kind = 2 AND d.runtime_scope_house_key IS NULL
              AND d.runtime_scope_instance_id = v_group.instance_id
              AND d.origin_channel_id = v_member.origin_channel_id
              AND d.scope_ownership_generation = v_group.ownership_generation
              AND d.character_lease_generation = v_member.source_lease_generation + 1
              AND d.current_generation = 1
            FOR SHARE OF s, d;
        IF NOT FOUND THEN
            RAISE EXCEPTION 'activity commit requires every terminal source and bound destination'
                USING ERRCODE = '23514';
        END IF;
    END LOOP;
    RETURN NULL;
END; $$;
CREATE CONSTRAINT TRIGGER game_activity_handoff_group_proven AFTER UPDATE
    ON game_activity_handoff_groups DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW WHEN (OLD.state = 1 AND NEW.state = 2)
    EXECUTE FUNCTION game_activity_handoff_group_proven();

DO $$ BEGIN
    EXECUTE format('ALTER FUNCTION game_activity_handoff_participant_guard() SET search_path = %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_activity_handoff_group_guard() SET search_path = %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_activity_handoff_group_proven() SET search_path = %I, pg_temp', current_schema());
END $$;
REVOKE ALL ON game_activity_handoff_groups, game_activity_handoff_participants FROM PUBLIC;
REVOKE ALL ON FUNCTION game_activity_handoff_participant_guard(),
    game_activity_handoff_group_guard(), game_activity_handoff_group_proven() FROM PUBLIC;
GRANT SELECT ON game_activity_handoff_groups, game_activity_handoff_participants
    TO oteryn_game_runtime, oteryn_game_control;
