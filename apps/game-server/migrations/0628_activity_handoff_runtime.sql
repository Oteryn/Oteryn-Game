-- W02: the existing 0265 group owner now admits genuine kind-2 Instance sessions.
-- Only preallocated kind-3 control assignments are consumed. This migration grants
-- no assignment, control grant, registration or Runtime-readiness publication rights.
-- All rows below are retained receipts; rollback is fix-forward, never history deletion.

CREATE TABLE game_activity_runtime_entries (
    handoff_id UUID PRIMARY KEY REFERENCES game_activity_handoff_groups(handoff_id),
    initiating_character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    initiating_game_session_id UUID NOT NULL REFERENCES game_durability_reconnect_sessions(game_session_id),
    initiating_connection_generation NUMERIC(20,0) NOT NULL
        CHECK (initiating_connection_generation BETWEEN 1 AND 18446744073709551615),
    command_id NUMERIC(20,0) NOT NULL CHECK (command_id BETWEEN 1 AND 18446744073709551615),
    request_binding BYTEA NOT NULL CHECK (octet_length(request_binding) = 32),
    holder_node_id UUID NOT NULL CHECK (game_node_is_uuid_v7(holder_node_id)),
    holder_registration_revision NUMERIC(20,0) NOT NULL
        CHECK (holder_registration_revision BETWEEN 1 AND 18446744073709551615),
    assignment_source_revision NUMERIC(20,0) NOT NULL
        CHECK (assignment_source_revision BETWEEN 1 AND 18446744073709551615),
    destination_coordinate_frame_digest BYTEA NOT NULL CHECK (octet_length(destination_coordinate_frame_digest) = 32),
    destination_map_revision_digest BYTEA NOT NULL CHECK (octet_length(destination_map_revision_digest) = 32),
    destination_server_artifact_digest BYTEA NOT NULL CHECK (octet_length(destination_server_artifact_digest) = 32),
    deadline BIGINT NOT NULL CHECK (deadline >= 0),
    UNIQUE (initiating_character_id, initiating_game_session_id, initiating_connection_generation, command_id)
);

CREATE TABLE game_activity_runtime_placements (
    handoff_id UUID NOT NULL,
    character_id UUID NOT NULL,
    destination_x INTEGER NOT NULL,
    destination_y INTEGER NOT NULL,
    destination_floor SMALLINT NOT NULL,
    exit_x INTEGER NOT NULL,
    exit_y INTEGER NOT NULL,
    exit_floor SMALLINT NOT NULL,
    -- Exit interpretation belongs to the immutable origin Channel snapshot.
    exit_coordinate_frame_digest BYTEA NOT NULL CHECK (octet_length(exit_coordinate_frame_digest) = 32),
    exit_map_revision_digest BYTEA NOT NULL CHECK (octet_length(exit_map_revision_digest) = 32),
    exit_server_artifact_digest BYTEA NOT NULL CHECK (octet_length(exit_server_artifact_digest) = 32),
    saved_health INTEGER CHECK (saved_health > 0),
    saved_mana INTEGER CHECK (saved_mana >= 0),
    saved_rest INTEGER[],
    PRIMARY KEY (handoff_id, character_id),
    FOREIGN KEY (handoff_id, character_id)
        REFERENCES game_activity_handoff_participants(handoff_id, character_id),
    CHECK ((saved_health IS NULL) = (saved_mana IS NULL)),
    CHECK (saved_rest IS NULL OR (array_length(saved_rest, 1) = 4 AND 0 <= ALL(saved_rest)))
);

CREATE TABLE game_activity_runtime_returns (
    return_id UUID PRIMARY KEY CHECK (game_node_is_uuid_v7(return_id)),
    handoff_id UUID NOT NULL,
    character_id UUID NOT NULL,
    source_game_session_id UUID NOT NULL REFERENCES game_durability_reconnect_sessions(game_session_id),
    source_connection_generation NUMERIC(20,0) NOT NULL
        CHECK (source_connection_generation BETWEEN 1 AND 18446744073709551615),
    source_lease_generation NUMERIC(20,0) NOT NULL
        CHECK (source_lease_generation BETWEEN 1 AND 18446744073709551614),
    -- NULL is a server-owned lifecycle occurrence, identified by return_id.
    command_id NUMERIC(20,0) NULL CHECK (command_id BETWEEN 1 AND 18446744073709551615),
    request_binding BYTEA NOT NULL CHECK (octet_length(request_binding) = 32),
    destination_game_session_id UUID NOT NULL UNIQUE
        REFERENCES game_durability_reconnect_sessions(game_session_id),
    -- Server return causes below constrain this field at table creation time.
    destination_transport_ref BYTEA CHECK (octet_length(destination_transport_ref) = 16),
    destination_ownership_generation NUMERIC(20,0) NOT NULL
        CHECK (destination_ownership_generation BETWEEN 1 AND 18446744073709551615),
    returned_at BIGINT NOT NULL CHECK (returned_at >= 0),
    -- Explicit exit, timeout, defeat, proven owner loss, or a retained death obligation.
    reason SMALLINT NOT NULL CHECK (reason BETWEEN 1 AND 9),
    release_transport_ref BYTEA NULL CHECK (octet_length(release_transport_ref)=16),
    release_loss_epoch NUMERIC(20,0) NULL CHECK (release_loss_epoch BETWEEN 1 AND 18446744073709551615),
    death_occurrence_id UUID NULL REFERENCES game_character_death_receipts(death_occurrence_id),
    return_x INTEGER NOT NULL,
    return_y INTEGER NOT NULL,
    return_floor SMALLINT NOT NULL,
    return_coordinate_frame_digest BYTEA NOT NULL CHECK (octet_length(return_coordinate_frame_digest)=32),
    return_map_revision_digest BYTEA NOT NULL CHECK (octet_length(return_map_revision_digest)=32),
    return_server_artifact_digest BYTEA NOT NULL CHECK (octet_length(return_server_artifact_digest)=32),
    CHECK (command_id IS NOT NULL OR reason IN (2,3,4,5,6,7,8,9)),
    CHECK ((reason=5)=(death_occurrence_id IS NOT NULL)),
    CHECK (reason<>5 OR command_id IS NULL),
    CHECK (reason<>7 OR (command_id IS NULL AND destination_transport_ref IS NOT NULL)),
    CHECK ((reason=8)=(release_transport_ref IS NOT NULL)),
    CHECK ((reason=9)=(release_loss_epoch IS NOT NULL)),
    CHECK (reason<>8 OR (command_id IS NULL AND destination_transport_ref IS NOT NULL)),
    CHECK (reason<>9 OR (command_id IS NULL AND destination_transport_ref IS NULL)),
    UNIQUE (handoff_id, character_id),
    FOREIGN KEY (handoff_id, character_id)
        REFERENCES game_activity_handoff_participants(handoff_id, character_id),
    CHECK (destination_game_session_id <> source_game_session_id)
);

CREATE FUNCTION game_activity_runtime_receipt_immutable() RETURNS trigger
LANGUAGE plpgsql SET search_path = pg_catalog, public, pg_temp AS $$ BEGIN
    RAISE EXCEPTION 'activity runtime receipts are immutable' USING ERRCODE = '23514';
END; $$;
CREATE TRIGGER game_activity_runtime_entry_immutable BEFORE UPDATE OR DELETE
    ON game_activity_runtime_entries FOR EACH ROW EXECUTE FUNCTION game_activity_runtime_receipt_immutable();
CREATE TRIGGER game_activity_runtime_placement_immutable BEFORE UPDATE OR DELETE
    ON game_activity_runtime_placements FOR EACH ROW EXECUTE FUNCTION game_activity_runtime_receipt_immutable();
CREATE TRIGGER game_activity_runtime_return_immutable BEFORE UPDATE OR DELETE
    ON game_activity_runtime_returns FOR EACH ROW EXECUTE FUNCTION game_activity_runtime_receipt_immutable();

ALTER TABLE game_durability_reconnect_sessions
    DROP CONSTRAINT game_durability_house_scope_session,
    ADD CONSTRAINT game_durability_house_scope_session CHECK (
        (runtime_scope_house_key IS NULL AND origin_channel_id IS NULL)
        OR (runtime_scope_kind = 2 AND runtime_scope_house_key IS NULL
            AND origin_channel_id IS NOT NULL AND game_node_is_uuid_v7(origin_channel_id))
        OR (runtime_scope_kind = 2
            AND runtime_scope_house_key IS NOT NULL
            AND game_house_scope_key_valid(runtime_scope_house_key)
            AND runtime_scope_instance_id = game_house_scope_instance_id(runtime_scope_world_id, runtime_scope_house_key)
            AND origin_channel_id IS NOT NULL AND game_node_is_uuid_v7(origin_channel_id))
    );

-- The permissive CHECK above never independently admits an activity. Every live
-- non-house Instance with an origin must be this exact committed destination.
CREATE OR REPLACE FUNCTION game_activity_scope_session_guard() RETURNS trigger
LANGUAGE plpgsql SET search_path = pg_catalog, public, pg_temp AS $$
DECLARE v_allocated BOOLEAN;
BEGIN
    IF NEW.runtime_scope_kind <> 2 OR NEW.session_state NOT IN (1, 2) THEN
        RETURN NEW;
    END IF;
    PERFORM game_house_scope_instance_lock(NEW.runtime_scope_world_id, NEW.runtime_scope_instance_id);
    SELECT EXISTS (SELECT 1 FROM game_runtime_scope_assignments a
        WHERE a.scope_kind = 3 AND a.world_id = NEW.runtime_scope_world_id
          AND a.activity_instance_id = NEW.runtime_scope_instance_id) INTO v_allocated;
    IF NOT v_allocated AND NOT (NEW.runtime_scope_house_key IS NULL AND NEW.origin_channel_id IS NOT NULL) THEN
        RETURN NEW;
    END IF;
    IF NEW.runtime_scope_house_key IS NOT NULL OR NOT game_runtime_scope_assignment_history_valid() THEN
        RAISE EXCEPTION 'activity admission requires valid assignment history' USING ERRCODE = '23514';
    END IF;
    PERFORM 1 FROM game_activity_handoff_groups g
        JOIN game_activity_handoff_participants p ON p.handoff_id = g.handoff_id
        JOIN game_activity_runtime_entries e ON e.handoff_id = g.handoff_id
        JOIN game_activity_runtime_placements q ON (q.handoff_id, q.character_id) = (p.handoff_id, p.character_id)
        JOIN game_runtime_scope_assignments a
          ON a.scope_key = '\x03'::bytea || uuid_send(g.world_id) || uuid_send(g.instance_id)
        JOIN game_durability_admission_runtime_guards r
          ON r.scope_key = '\x02'::bytea || uuid_send(g.world_id) || uuid_send(g.instance_id)
        WHERE g.state = 2 AND g.world_id = NEW.world_id AND g.world_id = NEW.runtime_scope_world_id
          AND g.instance_id = NEW.runtime_scope_instance_id
          AND g.ownership_generation = NEW.scope_ownership_generation
          AND p.character_id = NEW.character_id AND p.account_id = NEW.account_id
          AND p.destination_game_session_id = NEW.game_session_id
          AND p.origin_channel_id = NEW.origin_channel_id
          AND NEW.character_lease_generation = p.source_lease_generation + 1
          AND NEW.current_generation >= 1
          AND a.scope_kind = 3 AND a.state = 1 AND a.ownership_generation = g.ownership_generation
          AND a.holder_node_id = e.holder_node_id
          AND a.holder_registration_revision = e.holder_registration_revision
          AND a.source_revision = e.assignment_source_revision
          AND r.ready AND r.ownership_generation = a.ownership_generation
          AND game_node_lock_current_registration(a.holder_node_id, a.holder_registration_revision)
        FOR SHARE OF g, p, e, q, a, r;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'activity session requires a committed activity handoff with its current group destination' USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END; $$;

-- Return receipts cannot manufacture a live Channel destination or be attached
-- to a different Character. Issuance and current owner proofs are checked again
-- by the generic writer in the same semantic transaction.
CREATE FUNCTION game_activity_runtime_return_proven() RETURNS trigger
LANGUAGE plpgsql SET search_path = pg_catalog, public, pg_temp AS $$ BEGIN
    PERFORM 1 FROM game_activity_handoff_participants p
        JOIN game_activity_handoff_groups g ON g.handoff_id = p.handoff_id
        JOIN game_activity_runtime_placements q ON (q.handoff_id,q.character_id)=(p.handoff_id,p.character_id)
        JOIN game_durability_reconnect_sessions s ON s.game_session_id = NEW.source_game_session_id
        JOIN game_durability_reconnect_sessions d ON d.game_session_id = NEW.destination_game_session_id
        WHERE p.handoff_id = NEW.handoff_id AND p.character_id = NEW.character_id AND g.state = 2
          AND s.game_session_id = p.destination_game_session_id
          AND s.character_id = p.character_id AND s.account_id = p.account_id
          AND s.runtime_scope_kind = 2 AND s.runtime_scope_instance_id = g.instance_id
          AND s.world_id = g.world_id AND s.runtime_scope_world_id = g.world_id
          AND s.scope_ownership_generation = g.ownership_generation
          AND s.current_generation = NEW.source_connection_generation
          AND s.character_lease_generation = NEW.source_lease_generation
          AND s.session_state = 3 AND s.current_transport_ref IS NULL AND s.prepared_attempt_ref IS NULL
          AND d.character_id = p.character_id AND d.account_id = p.account_id
          AND d.world_id = p.world_id AND d.runtime_scope_world_id = p.world_id
          AND d.runtime_scope_kind = 1 AND d.runtime_scope_channel_id = p.origin_channel_id
          AND d.runtime_scope_instance_id IS NULL AND d.runtime_scope_house_key IS NULL AND d.origin_channel_id IS NULL
          AND d.scope_ownership_generation = NEW.destination_ownership_generation
          AND d.character_lease_generation = NEW.source_lease_generation + 1 AND d.current_generation = 1
          AND (NEW.return_coordinate_frame_digest,NEW.return_map_revision_digest,NEW.return_server_artifact_digest)
              =(q.exit_coordinate_frame_digest,q.exit_map_revision_digest,q.exit_server_artifact_digest)
          AND (NEW.reason=5 OR (NEW.return_x,NEW.return_y,NEW.return_floor)=(q.exit_x,q.exit_y,q.exit_floor))
        FOR SHARE OF p, g, q, s, d;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'activity return requires its terminal source and fresh origin destination' USING ERRCODE = '23514';
    END IF;
    IF NEW.reason=7 THEN
        -- Existing IDLEKICK timer/input authority (connection/serve/idle.rs),
        -- distinct from recorded control loss and from a client command id.
        PERFORM 1 FROM game_durability_reconnect_sessions d
            WHERE d.game_session_id=NEW.destination_game_session_id AND d.session_state=2
              AND d.current_transport_ref=NEW.destination_transport_ref
              AND d.control_loss_epoch IS NULL AND d.original_grace_deadline IS NULL
            FOR SHARE OF d;
        IF NOT FOUND THEN
            RAISE EXCEPTION 'idle activity logout requires its active fresh destination' USING ERRCODE='23514';
        END IF;
    END IF;
    IF NEW.reason=8 THEN
        -- Existing abandoned-transport release. The writer checks ACTIVE plus
        -- this exact source transport before issue_successor terminalizes it;
        -- the immutable audit keeps its issued session identity afterward.
        PERFORM 1 FROM game_durability_transport_ref_reservations t
            JOIN game_durability_reconnect_sessions d ON d.game_session_id=NEW.destination_game_session_id
            WHERE t.transport_ref=NEW.release_transport_ref AND t.game_session_id=NEW.source_game_session_id
              AND d.session_state=2 AND d.current_transport_ref=NEW.destination_transport_ref
              AND d.control_loss_epoch IS NULL AND d.original_grace_deadline IS NULL
            FOR SHARE OF t,d;
        IF NOT FOUND THEN
            RAISE EXCEPTION 'abandoned activity return requires its exact issued transport and active fresh destination' USING ERRCODE='23514';
        END IF;
    END IF;
    IF NEW.reason=9 THEN
        -- Existing D449 independently recorded loss, still inside its original
        -- grace. The fresh offline origin inherits that same fixed deadline.
        PERFORM 1 FROM game_durability_reconnect_sessions s
            JOIN game_durability_reconnect_sessions d ON d.game_session_id=NEW.destination_game_session_id
            JOIN game_durability_admission_lifecycle_receipts l
              ON l.operation_key=convert_to('owning-loss-v1','UTF8')||uuid_send(s.game_session_id)
                ||decode(lpad(to_hex(trunc(NEW.release_loss_epoch/4294967296)::bigint),8,'0')
                    ||lpad(to_hex(mod(NEW.release_loss_epoch,4294967296)::bigint),8,'0'),'hex')
            WHERE s.game_session_id=NEW.source_game_session_id AND s.control_loss_epoch=NEW.release_loss_epoch
              AND s.original_grace_deadline>NEW.returned_at/1000
              AND d.session_state=1 AND d.current_transport_ref IS NULL
              AND d.original_grace_deadline=s.original_grace_deadline AND d.control_loss_epoch=1
            FOR SHARE OF s,d,l;
        IF NOT FOUND THEN
            RAISE EXCEPTION 'activity capability mismatch requires its recorded epoch and inherited grace' USING ERRCODE='23514';
        END IF;
    END IF;
    IF NEW.reason=6 AND NEW.command_id IS NULL THEN
        -- Autonomous logout is the already recorded, expired loss. A client
        -- logout instead carries its real non-zero command id; no synthetic
        -- command or unexpected-loss observation is manufactured.
        PERFORM 1 FROM game_durability_reconnect_sessions s
            JOIN game_durability_reconnect_sessions d ON d.game_session_id=NEW.destination_game_session_id
            WHERE s.game_session_id=NEW.source_game_session_id AND s.control_loss_epoch IS NOT NULL
              AND s.original_grace_deadline IS NOT NULL AND s.original_grace_deadline<=NEW.returned_at/1000
              AND NEW.destination_transport_ref IS NULL AND d.current_transport_ref IS NULL
              AND d.control_loss_epoch IS NOT NULL AND d.original_grace_deadline=s.original_grace_deadline
            FOR SHARE OF s,d;
        IF NOT FOUND THEN
            RAISE EXCEPTION 'autonomous activity logout requires its recorded expired loss' USING ERRCODE='23514';
        END IF;
    END IF;
    IF NEW.reason=5 THEN
        -- 0630 extends the existing death receipt with exact Instance identity.
        -- The full migration set is applied before any runtime handoff is served.
        PERFORM 1 FROM game_character_pending_respawns p
            JOIN game_character_death_receipts d ON (d.death_occurrence_id,d.character_id,d.respawn_position)
                =(p.death_occurrence_id,p.character_id,p.respawn_position)
            JOIN game_activity_handoff_groups g ON g.handoff_id=NEW.handoff_id
            WHERE p.character_id=NEW.character_id AND p.death_occurrence_id=NEW.death_occurrence_id
              AND d.death_scope_kind=2 AND d.death_world_id=g.world_id AND d.death_instance_id=g.instance_id
              AND d.death_scope_generation=g.ownership_generation
              AND p.respawn_position=int4send(NEW.return_x)||int4send(NEW.return_y)||int2send(NEW.return_floor)
              AND game_player_death_items_complete(p.death_occurrence_id)
            FOR SHARE OF p,d,g;
        IF NOT FOUND THEN
            RAISE EXCEPTION 'activity death return requires its complete retained death and pending temple' USING ERRCODE='23514';
        END IF;
    END IF;
    RETURN NULL;
END; $$;
CREATE CONSTRAINT TRIGGER game_activity_runtime_return_proven AFTER INSERT
    ON game_activity_runtime_returns DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW EXECUTE FUNCTION game_activity_runtime_return_proven();

-- Crystal boss_lever.lua:66 defaults timeAfterKill to 60 seconds and :67
-- defaults level to zero. The content importer remains the only catalogue writer.
ALTER TABLE game_boss_room_catalogue
    DROP CONSTRAINT game_boss_room_catalogue_minimum_level_check,
    ADD CONSTRAINT game_boss_room_catalogue_minimum_level_check CHECK (minimum_level BETWEEN 0 AND 4294967295),
    ADD COLUMN after_kill_seconds BIGINT NOT NULL DEFAULT 60 CHECK (after_kill_seconds BETWEEN 0 AND 9223372036854775);
ALTER TABLE game_boss_room_activities ADD COLUMN exit_deadline BIGINT CHECK (exit_deadline >= opened_at);

REVOKE ALL ON game_activity_runtime_entries, game_activity_runtime_placements,
    game_activity_runtime_returns FROM PUBLIC;
REVOKE ALL ON FUNCTION game_activity_runtime_receipt_immutable(), game_activity_runtime_return_proven() FROM PUBLIC;
GRANT SELECT, INSERT ON game_activity_runtime_entries, game_activity_runtime_placements,
    game_activity_runtime_returns TO oteryn_game_runtime;
GRANT INSERT ON game_activity_handoff_groups, game_activity_handoff_participants TO oteryn_game_runtime;
GRANT UPDATE (state, decided_at) ON game_activity_handoff_groups TO oteryn_game_runtime;
GRANT INSERT ON game_boss_room_activities, game_boss_room_participants TO oteryn_game_runtime;
GRANT UPDATE (state, terminal_at, exit_deadline) ON game_boss_room_activities TO oteryn_game_runtime;
GRANT UPDATE (returned_at) ON game_boss_room_participants TO oteryn_game_runtime;

-- Fresh activity sessions retain their original fresh-admission interpretation,
-- while their immediate entry/return origin is explicit and immutable.
ALTER TABLE game_activity_handoff_participants
    ADD CONSTRAINT game_activity_destination_identity UNIQUE (handoff_id, destination_game_session_id);
ALTER TABLE game_activity_runtime_returns
    ADD CONSTRAINT game_activity_return_destination_identity UNIQUE (return_id, destination_game_session_id);
ALTER TABLE game_activity_runtime_placements
    ADD COLUMN destination_transport_ref BYTEA CHECK (octet_length(destination_transport_ref) = 16);
ALTER TABLE game_durability_reconnect_sessions
    ADD COLUMN activity_handoff_id UUID,
    ADD COLUMN activity_return_id UUID,
    ADD COLUMN activity_recovery_protection BYTEA,
    ADD CONSTRAINT game_activity_session_protection CHECK (
        ((activity_handoff_id IS NULL AND activity_return_id IS NULL)
            AND activity_recovery_protection IS NULL)
        OR ((activity_handoff_id IS NOT NULL OR activity_return_id IS NOT NULL)
            AND activity_recovery_protection IS NOT NULL
            AND octet_length(activity_recovery_protection) BETWEEN 12 AND 52)
    ),
    ADD CONSTRAINT game_activity_session_entry_origin FOREIGN KEY (activity_handoff_id, game_session_id)
        REFERENCES game_activity_handoff_participants(handoff_id, destination_game_session_id) DEFERRABLE INITIALLY DEFERRED,
    ADD CONSTRAINT game_activity_session_return_origin FOREIGN KEY (activity_return_id, game_session_id)
        REFERENCES game_activity_runtime_returns(return_id, destination_game_session_id) DEFERRABLE INITIALLY DEFERRED,
    ADD CONSTRAINT game_activity_session_one_origin CHECK (activity_handoff_id IS NULL OR activity_return_id IS NULL),
    DROP CONSTRAINT game_truthful_session_continuity,
    ADD CONSTRAINT game_truthful_session_continuity CHECK (
        (control_loss_epoch IS NOT NULL AND original_grace_deadline IS NOT NULL AND predecessor_generation IS NOT NULL)
        OR (control_loss_epoch IS NULL AND original_grace_deadline IS NULL AND predecessor_generation IS NULL
            AND (fresh_replay_key IS NOT NULL OR (initial_fresh_replay_key IS NOT NULL
                AND (activity_handoff_id IS NOT NULL OR activity_return_id IS NOT NULL)))
            AND session_state IN (2, 3) AND prepared_attempt_ref IS NULL AND attempt_count = 0)
    );

CREATE FUNCTION game_activity_session_origin_immutable() RETURNS trigger
LANGUAGE plpgsql SET search_path = pg_catalog, public, pg_temp AS $$ BEGIN
    IF (OLD.activity_handoff_id IS NOT NULL OR OLD.activity_return_id IS NOT NULL)
       AND (NEW.activity_handoff_id, NEW.activity_return_id, NEW.initial_fresh_replay_key,
            NEW.character_id, NEW.account_id, NEW.world_id, NEW.runtime_scope_world_id,
            NEW.runtime_scope_kind, NEW.runtime_scope_channel_id, NEW.runtime_scope_instance_id,
            NEW.origin_channel_id, NEW.character_lease_generation, NEW.scope_ownership_generation,
            NEW.activity_recovery_protection)
           IS DISTINCT FROM
           (OLD.activity_handoff_id, OLD.activity_return_id, OLD.initial_fresh_replay_key,
            OLD.character_id, OLD.account_id, OLD.world_id, OLD.runtime_scope_world_id,
            OLD.runtime_scope_kind, OLD.runtime_scope_channel_id, OLD.runtime_scope_instance_id,
            OLD.origin_channel_id, OLD.character_lease_generation, OLD.scope_ownership_generation,
            OLD.activity_recovery_protection) THEN
        RAISE EXCEPTION 'activity session origin is immutable' USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END; $$;
CREATE TRIGGER game_activity_session_origin_immutable BEFORE UPDATE
    ON game_durability_reconnect_sessions FOR EACH ROW EXECUTE FUNCTION game_activity_session_origin_immutable();

ALTER TABLE game_durability_transport_ref_reservations
    ADD COLUMN activity_handoff_id UUID,
    ADD COLUMN activity_return_id UUID,
    ADD CONSTRAINT game_activity_transport_entry_origin FOREIGN KEY (activity_handoff_id, game_session_id)
        REFERENCES game_activity_handoff_participants(handoff_id, destination_game_session_id) DEFERRABLE INITIALLY DEFERRED,
    ADD CONSTRAINT game_activity_transport_return_origin FOREIGN KEY (activity_return_id, game_session_id)
        REFERENCES game_activity_runtime_returns(return_id, destination_game_session_id) DEFERRABLE INITIALLY DEFERRED,
    DROP CONSTRAINT game_transport_exact_origin,
    ADD CONSTRAINT game_transport_exact_origin CHECK (
        (reservation_owner = 1 AND reconnect_attempt_ref IS NOT NULL AND fresh_replay_key IS NULL
            AND activity_handoff_id IS NULL AND activity_return_id IS NULL)
        OR (reservation_owner = 2 AND reconnect_attempt_ref IS NULL AND fresh_replay_key IS NOT NULL
            AND activity_handoff_id IS NULL AND activity_return_id IS NULL)
        OR (reservation_owner = 3 AND reconnect_attempt_ref IS NULL AND fresh_replay_key IS NULL
            AND ((activity_handoff_id IS NOT NULL AND activity_return_id IS NULL)
                OR (activity_handoff_id IS NULL AND activity_return_id IS NOT NULL)))
    );

-- This historical proof is separate from the current owner/session/lease gates.
-- It resolves by the retained destination identity as well as live origin columns,
-- so erasing an origin pointer can never turn an activity into a legacy session.
CREATE FUNCTION game_activity_session_origin_valid(p_session UUID, p_character UUID) RETURNS BOOLEAN
LANGUAGE sql STABLE SET search_path = pg_catalog, public, pg_temp AS $$
    SELECT CASE
      WHEN s.activity_handoff_id IS NOT NULL OR EXISTS (
          SELECT 1 FROM game_activity_handoff_participants p WHERE p.destination_game_session_id=s.game_session_id)
      THEN EXISTS (
        SELECT 1 FROM game_activity_handoff_participants p
        JOIN game_activity_handoff_groups g USING(handoff_id)
        JOIN game_activity_runtime_entries e USING(handoff_id)
        JOIN game_activity_runtime_placements q USING(handoff_id,character_id)
        JOIN game_durability_reconnect_sessions origin ON origin.game_session_id=p.source_game_session_id
        JOIN game_durability_session_use_memberships m ON m.game_session_id=s.game_session_id AND m.character_id=s.character_id
        WHERE p.destination_game_session_id=s.game_session_id AND s.activity_handoff_id=p.handoff_id AND s.activity_return_id IS NULL
          AND g.state=2 AND p.character_id=s.character_id AND p.account_id=s.account_id AND g.world_id=s.world_id
          AND s.runtime_scope_kind=2 AND s.runtime_scope_instance_id=g.instance_id AND s.origin_channel_id=p.origin_channel_id
          AND s.scope_ownership_generation=g.ownership_generation AND s.character_lease_generation=p.source_lease_generation+1
          AND m.operation_binding=uuid_send(g.handoff_id)
          AND origin.session_state=3 AND origin.character_id=s.character_id AND origin.account_id=s.account_id
          AND s.initial_fresh_replay_key=COALESCE(origin.fresh_replay_key,origin.initial_fresh_replay_key)
          AND (q.destination_transport_ref IS NULL OR EXISTS (
              SELECT 1 FROM game_durability_transport_ref_reservations t
              WHERE t.transport_ref=q.destination_transport_ref AND t.game_session_id=s.game_session_id
                AND t.reservation_owner=3 AND t.activity_handoff_id=g.handoff_id AND t.activity_return_id IS NULL)))
      WHEN s.activity_return_id IS NOT NULL OR EXISTS (
          SELECT 1 FROM game_activity_runtime_returns r WHERE r.destination_game_session_id=s.game_session_id)
      THEN EXISTS (
        SELECT 1 FROM game_activity_runtime_returns r
        JOIN game_activity_handoff_participants p USING(handoff_id,character_id)
        JOIN game_durability_reconnect_sessions origin ON origin.game_session_id=r.source_game_session_id
        JOIN game_durability_session_use_memberships m ON m.game_session_id=s.game_session_id AND m.character_id=s.character_id
        WHERE r.destination_game_session_id=s.game_session_id AND s.activity_return_id=r.return_id AND s.activity_handoff_id IS NULL
          AND r.character_id=s.character_id AND s.account_id=p.account_id AND s.world_id=p.world_id
          AND s.runtime_scope_kind=1 AND s.runtime_scope_channel_id=p.origin_channel_id
          AND s.scope_ownership_generation=r.destination_ownership_generation AND s.character_lease_generation=r.source_lease_generation+1
          AND m.operation_binding=uuid_send(r.return_id)
          AND origin.session_state=3 AND origin.character_id=s.character_id AND origin.account_id=s.account_id
          AND origin.activity_handoff_id=r.handoff_id AND origin.game_session_id=p.destination_game_session_id
          AND s.initial_fresh_replay_key=COALESCE(origin.fresh_replay_key,origin.initial_fresh_replay_key)
          AND (r.destination_transport_ref IS NULL OR EXISTS (
              SELECT 1 FROM game_durability_transport_ref_reservations t
              WHERE t.transport_ref=r.destination_transport_ref AND t.game_session_id=s.game_session_id
                AND t.reservation_owner=3 AND t.activity_return_id=r.return_id AND t.activity_handoff_id IS NULL)))
      ELSE TRUE END
      AND EXISTS (SELECT 1 FROM game_durability_session_use_ledgers l
        WHERE l.character_id=s.character_id AND l.version=1 AND l.complete
          AND l.revision=l.revision_floor
          AND l.revision=(SELECT count(*) FROM game_durability_session_use_memberships m WHERE m.character_id=s.character_id)
          AND l.revision=COALESCE((SELECT max(membership_revision) FROM game_durability_session_use_memberships m WHERE m.character_id=s.character_id),0))
    FROM game_durability_reconnect_sessions s WHERE s.game_session_id=p_session AND s.character_id=p_character;
$$;
REVOKE ALL ON FUNCTION game_activity_session_origin_immutable(), game_activity_session_origin_valid(UUID,UUID) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_activity_session_origin_valid(UUID,UUID) TO oteryn_game_runtime;

-- No unsourced placement defaults. Content must explicitly author both ends.
ALTER TABLE game_boss_room_catalogue
    ADD COLUMN template_revision TEXT CHECK (template_revision ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'),
    ADD COLUMN entry_positions JSONB CHECK (jsonb_typeof(entry_positions) = 'array'),
    ADD COLUMN exit_position JSONB CHECK (jsonb_typeof(exit_position) = 'object');
ALTER TABLE game_boss_room_catalogue ADD COLUMN boss_position JSONB CHECK (jsonb_typeof(boss_position) = 'object');
ALTER TABLE game_boss_room_activities
    ADD COLUMN after_kill_ms BIGINT NOT NULL DEFAULT 60000 CHECK (after_kill_ms >= 0),
    ADD COLUMN boss_creature_key TEXT,
    ADD COLUMN defeat_id UUID CHECK (defeat_id IS NULL OR defeat_id <> '00000000-0000-0000-0000-000000000000'::uuid);
GRANT UPDATE (defeat_id) ON game_boss_room_activities TO oteryn_game_runtime;

CREATE FUNCTION game_boss_room_runtime_guard() RETURNS trigger
LANGUAGE plpgsql SET search_path = pg_catalog, public, pg_temp AS $$ BEGIN
    IF TG_OP = 'INSERT' THEN
        PERFORM 1 FROM game_activity_handoff_groups g JOIN game_activity_runtime_entries e USING(handoff_id)
            WHERE g.handoff_id=NEW.admission AND g.state=2 AND g.world_id=NEW.world_id AND g.instance_id=NEW.instance_id
              AND g.activity_key=NEW.boss_key AND g.content_revision=NEW.content_revision
              AND e.deadline=NEW.deadline AND g.prepared_at=NEW.opened_at;
        IF NOT FOUND OR NEW.state<>1 OR NEW.boss_creature_key IS NULL OR NEW.defeat_id IS NOT NULL THEN
            RAISE EXCEPTION 'boss room requires its complete committed activity' USING ERRCODE='23514';
        END IF;
    ELSIF TG_OP='UPDATE' THEN
        IF (to_jsonb(NEW)-'state'-'terminal_at'-'exit_deadline'-'defeat_id') IS DISTINCT FROM
           (to_jsonb(OLD)-'state'-'terminal_at'-'exit_deadline'-'defeat_id') THEN
            RAISE EXCEPTION 'boss admission identity and policy are immutable' USING ERRCODE='23514';
        END IF;
        IF OLD.state=1 AND NEW.state=2 AND NEW.defeat_id IS NOT NULL
           AND NEW.terminal_at BETWEEN OLD.opened_at AND OLD.deadline
           AND ((OLD.after_kill_ms=0 AND NEW.exit_deadline IS NULL)
             OR (OLD.after_kill_ms>0 AND NEW.exit_deadline=NEW.terminal_at+OLD.after_kill_ms)) THEN RETURN NEW; END IF;
        IF OLD.state=1 AND NEW.state=3 AND NEW.defeat_id IS NULL
           AND NEW.terminal_at>=OLD.deadline AND NEW.exit_deadline=NEW.terminal_at THEN RETURN NEW; END IF;
        IF OLD.state IN(1,2,3) AND NEW.state=4 AND NOT EXISTS(
            SELECT 1 FROM game_activity_handoff_participants p WHERE p.handoff_id=OLD.admission
              AND NOT EXISTS(SELECT 1 FROM game_activity_runtime_returns r WHERE (r.handoff_id,r.character_id)=(p.handoff_id,p.character_id)))
           AND (OLD.defeat_id IS NOT DISTINCT FROM NEW.defeat_id) THEN RETURN NEW; END IF;
        RAISE EXCEPTION 'boss room transition is not current or complete' USING ERRCODE='23514';
    ELSE
        RAISE EXCEPTION 'boss room history is retained' USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END; $$;
CREATE TRIGGER game_boss_room_runtime_guard BEFORE INSERT OR UPDATE OR DELETE
    ON game_boss_room_activities FOR EACH ROW EXECUTE FUNCTION game_boss_room_runtime_guard();
REVOKE ALL ON FUNCTION game_boss_room_runtime_guard() FROM PUBLIC;

-- Lost activity acknowledgement recovery extends the same handoff/session owner.
-- A participant's selected capabilities are source connection facts, distinct
-- from later client support. MAX_CAPABILITY_COUNT is 128 in protocol-oteryn.
CREATE FUNCTION game_activity_selection_valid(p_selection BIGINT[]) RETURNS BOOLEAN
LANGUAGE sql IMMUTABLE SET search_path = pg_catalog, public, pg_temp AS $$
    SELECT p_selection IS NOT NULL AND cardinality(p_selection)<=128
       AND (cardinality(p_selection)=0 OR (array_ndims(p_selection)=1 AND array_lower(p_selection,1)=1))
       AND NOT EXISTS(SELECT 1 FROM unnest(p_selection) WITH ORDINALITY v(id,n)
          WHERE id IS NULL OR id NOT BETWEEN 1 AND 4294967295
             OR (n>1 AND p_selection[n-1]>=id));
$$;
ALTER TABLE game_activity_runtime_placements
    ADD COLUMN selected_capabilities BIGINT[] NOT NULL
        CHECK (game_activity_selection_valid(selected_capabilities));
REVOKE ALL ON FUNCTION game_activity_selection_valid(BIGINT[]) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_activity_selection_valid(BIGINT[]) TO oteryn_game_runtime;

-- These are immutable recovery receipts, not a second allocation authority.
-- The ceiling reuses admission_journal::MAX_ATTEMPTS_PER_EPOCH (8). Ordinary
-- loss-epoch attempt_count and its protection budget are never reset here.
CREATE TABLE game_activity_transfer_recoveries (
    recovery_grant_nonce BYTEA PRIMARY KEY
        REFERENCES game_durability_recovery_grant_consumptions(recovery_grant_nonce),
    game_session_id UUID NOT NULL REFERENCES game_durability_reconnect_sessions(game_session_id),
    handoff_id UUID NOT NULL REFERENCES game_activity_handoff_groups(handoff_id),
    return_id UUID NULL REFERENCES game_activity_runtime_returns(return_id),
    recovery_ordinal SMALLINT NOT NULL CHECK (recovery_ordinal BETWEEN 1 AND 8),
    reconnect_attempt_ref BYTEA NOT NULL CHECK (octet_length(reconnect_attempt_ref)=8),
    predecessor_generation NUMERIC(20,0) NOT NULL CHECK (predecessor_generation BETWEEN 1 AND 18446744073709551614),
    connection_generation NUMERIC(20,0) NOT NULL CHECK (connection_generation=predecessor_generation+1),
    predecessor_transport_ref BYTEA NULL CHECK (octet_length(predecessor_transport_ref)=16),
    transport_ref BYTEA NOT NULL UNIQUE REFERENCES game_durability_transport_ref_reservations(transport_ref),
    predecessor_session_state SMALLINT NOT NULL CHECK (predecessor_session_state IN(1,2)),
    retained_grace_deadline BIGINT NULL CHECK (retained_grace_deadline>=0),
    node_id UUID NOT NULL CHECK (game_node_is_uuid_v7(node_id)),
    node_registration_revision NUMERIC(20,0) NOT NULL CHECK (node_registration_revision BETWEEN 1 AND 18446744073709551615),
    decided_at BIGINT NOT NULL CHECK (decided_at>=0),
    UNIQUE(game_session_id,recovery_ordinal),
    UNIQUE(game_session_id,connection_generation),
    UNIQUE(game_session_id,reconnect_attempt_ref),
    CHECK ((predecessor_session_state=2 AND predecessor_transport_ref IS NOT NULL AND retained_grace_deadline IS NULL)
        OR (predecessor_session_state=1 AND predecessor_transport_ref IS NULL AND retained_grace_deadline>decided_at))
);

CREATE FUNCTION game_activity_transfer_recovery_guard() RETURNS trigger
LANGUAGE plpgsql SET search_path = pg_catalog, public, pg_temp AS $$
DECLARE s game_durability_reconnect_sessions%ROWTYPE; v_count BIGINT; v_last NUMERIC;
BEGIN
    SELECT * INTO STRICT s FROM game_durability_reconnect_sessions
        WHERE game_session_id=NEW.game_session_id FOR UPDATE;
    IF s.session_state<>NEW.predecessor_session_state OR s.current_generation<>NEW.predecessor_generation
       OR s.current_transport_ref IS DISTINCT FROM NEW.predecessor_transport_ref
       OR s.prepared_attempt_ref IS NOT NULL OR s.attempt_count<>0
       OR NOT COALESCE(game_activity_session_origin_valid(s.game_session_id,s.character_id),false)
       OR ((NEW.return_id IS NULL AND s.activity_handoff_id=NEW.handoff_id AND s.activity_return_id IS NULL)
           OR (NEW.return_id IS NOT NULL AND s.activity_return_id=NEW.return_id AND s.activity_handoff_id IS NULL)) IS NOT TRUE
       OR EXISTS(SELECT 1 FROM game_durability_admission_lifecycle_receipts
           WHERE operation_key=convert_to('activity-transfer-input-v1','UTF8')||uuid_send(s.game_session_id))
       OR EXISTS(SELECT 1 FROM game_character_pending_respawns WHERE character_id=s.character_id)
       OR EXISTS(SELECT 1 FROM game_character_pending_arrivals WHERE character_id=s.character_id)
       OR EXISTS(SELECT 1 FROM game_durability_reconnect_attempts
           WHERE game_session_id=s.game_session_id AND reconnect_attempt_ref=NEW.reconnect_attempt_ref) THEN
        RAISE EXCEPTION 'activity recovery requires its unused current destination' USING ERRCODE='23514';
    END IF;
    IF NEW.return_id IS NOT NULL AND NOT EXISTS(SELECT 1 FROM game_activity_runtime_returns
        WHERE return_id=NEW.return_id AND handoff_id=NEW.handoff_id AND destination_game_session_id=s.game_session_id) THEN
        RAISE EXCEPTION 'activity recovery return binding differs' USING ERRCODE='23514';
    END IF;
    SELECT count(*),max(connection_generation) INTO v_count,v_last FROM game_activity_transfer_recoveries
        WHERE game_session_id=s.game_session_id;
    IF NEW.recovery_ordinal<>v_count+1 OR NEW.predecessor_generation<>COALESCE(v_last,1)
       OR NEW.predecessor_generation<>NEW.recovery_ordinal OR NEW.decided_at>floor(extract(epoch FROM clock_timestamp()))::bigint THEN
        RAISE EXCEPTION 'activity recovery generation or budget differs' USING ERRCODE='23514';
    END IF;
    IF s.session_state=2 THEN
        IF s.control_loss_epoch IS NOT NULL OR s.original_grace_deadline IS NOT NULL OR s.predecessor_generation IS NOT NULL THEN
            RAISE EXCEPTION 'recorded loss belongs to ordinary reconnect' USING ERRCODE='23514';
        END IF;
    ELSE
        -- The new offline return inherited the predecessor's expiry. It has no
        -- owning-loss receipt of its own. This proven recovery closes only that
        -- awaiting-controller state; the original predecessor history remains.
        IF v_count<>0 OR s.current_generation<>1 OR s.control_loss_epoch<>1 OR s.predecessor_generation<>1
           OR s.original_grace_deadline IS DISTINCT FROM NEW.retained_grace_deadline
           OR s.original_grace_deadline<=NEW.decided_at OR NEW.return_id IS NULL
           OR NOT EXISTS(SELECT 1 FROM game_activity_runtime_returns r WHERE r.return_id=NEW.return_id
                AND r.destination_game_session_id=s.game_session_id AND r.destination_transport_ref IS NULL)
           OR EXISTS(SELECT 1 FROM game_durability_admission_lifecycle_receipts
                WHERE operation_key=convert_to('owning-loss-v1','UTF8')||uuid_send(s.game_session_id)||int8send(1)) THEN
            RAISE EXCEPTION 'offline activity recovery requires retained unexpired transfer continuity' USING ERRCODE='23514';
        END IF;
    END IF;
    PERFORM 1 FROM game_runtime_scope_assignments a JOIN game_durability_admission_runtime_guards g
        ON g.scope_key=(CASE WHEN s.runtime_scope_kind=1 THEN '\x01'::bytea ELSE '\x02'::bytea END)
            ||uuid_send(s.world_id)||uuid_send(COALESCE(s.runtime_scope_channel_id,s.runtime_scope_instance_id))
        JOIN game_durability_admission_character_guards c ON c.character_id=s.character_id
        JOIN game_durability_admission_account_guards p ON p.account_id=s.account_id
        WHERE a.scope_key=(CASE WHEN s.runtime_scope_kind=1 THEN '\x01'::bytea ELSE '\x03'::bytea END)
            ||uuid_send(s.world_id)||uuid_send(COALESCE(s.runtime_scope_channel_id,s.runtime_scope_instance_id))
          AND a.state=1 AND a.ownership_generation=s.scope_ownership_generation
          AND a.holder_node_id=NEW.node_id AND a.holder_registration_revision=NEW.node_registration_revision
          AND g.ready AND g.ownership_generation=a.ownership_generation
          AND c.eligible AND c.account_id=s.account_id AND c.world_id=s.world_id
          AND c.holder_game_session_id=s.game_session_id AND c.lease_generation=s.character_lease_generation
          AND p.holder_game_session_id=s.game_session_id AND p.presence_character_id=s.character_id
          AND game_runtime_scope_assignment_history_valid()
          AND game_node_lock_current_registration(NEW.node_id,NEW.node_registration_revision)
        FOR SHARE OF a,g,c,p;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'activity recovery lost its current scope or claim holder' USING ERRCODE='23514';
    END IF;
    PERFORM 1 FROM game_durability_transport_ref_reservations t
        JOIN game_durability_recovery_grant_consumptions g ON g.recovery_grant_nonce=NEW.recovery_grant_nonce
        WHERE t.transport_ref=NEW.transport_ref AND t.game_session_id=s.game_session_id
          AND t.reservation_owner=1 AND t.reconnect_attempt_ref=NEW.reconnect_attempt_ref
          AND g.game_session_id=s.game_session_id AND g.reconnect_attempt_ref=NEW.reconnect_attempt_ref;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'activity recovery requires its single-use credential and transport' USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END; $$;
CREATE TRIGGER game_activity_transfer_recovery_guard BEFORE INSERT ON game_activity_transfer_recoveries
    FOR EACH ROW EXECUTE FUNCTION game_activity_transfer_recovery_guard();
CREATE TRIGGER game_activity_transfer_recovery_immutable BEFORE UPDATE OR DELETE ON game_activity_transfer_recoveries
    FOR EACH ROW EXECUTE FUNCTION game_activity_runtime_receipt_immutable();

CREATE FUNCTION game_activity_transfer_recovery_committed() RETURNS trigger
LANGUAGE plpgsql SET search_path = pg_catalog, public, pg_temp AS $$ BEGIN
    PERFORM 1 FROM game_durability_reconnect_sessions s WHERE s.game_session_id=NEW.game_session_id
        AND s.session_state=2 AND s.current_generation=NEW.connection_generation AND s.current_transport_ref=NEW.transport_ref
        AND s.control_loss_epoch IS NULL AND s.original_grace_deadline IS NULL AND s.predecessor_generation IS NULL
        AND s.prepared_attempt_ref IS NULL AND s.attempt_count=0;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'activity recovery receipt has no exact active successor' USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END; $$;
CREATE CONSTRAINT TRIGGER game_activity_transfer_recovery_committed AFTER INSERT ON game_activity_transfer_recoveries
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_activity_transfer_recovery_committed();

CREATE FUNCTION game_activity_recovery_attempt_collision() RETURNS trigger
LANGUAGE plpgsql SET search_path = pg_catalog, public, pg_temp AS $$ BEGIN
    IF EXISTS(SELECT 1 FROM game_activity_transfer_recoveries WHERE game_session_id=NEW.game_session_id
        AND reconnect_attempt_ref=NEW.reconnect_attempt_ref) THEN
        RAISE EXCEPTION 'reconnect attempt identity already names activity recovery' USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END; $$;
CREATE TRIGGER game_activity_recovery_attempt_collision BEFORE INSERT ON game_durability_reconnect_attempts
    FOR EACH ROW EXECUTE FUNCTION game_activity_recovery_attempt_collision();
REVOKE ALL ON game_activity_transfer_recoveries FROM PUBLIC;
GRANT SELECT,INSERT ON game_activity_transfer_recoveries TO oteryn_game_runtime;
REVOKE ALL ON FUNCTION game_activity_transfer_recovery_guard(), game_activity_transfer_recovery_committed(),
    game_activity_recovery_attempt_collision() FROM PUBLIC;
