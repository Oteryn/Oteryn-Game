-- Limit the existing committed membership exception to the Crystal Warzone window.
CREATE OR REPLACE FUNCTION game_activity_handoff_participant_guard() RETURNS trigger
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
    IF NOT FOUND OR (v_group.state <> 1 AND NOT (
        v_group.state = 2 AND v_group.admission_closes_at IS NOT NULL
        AND v_group.admission_window_ms = 8000
        AND v_group.activity_key IN (
            'oteryn:boss.the_baron_from_below',
            'oteryn:boss.the_count_of_the_core',
            'oteryn:boss.the_duke_of_the_depths'
        )
        AND clock_timestamp() < v_group.admission_closes_at
        AND EXISTS (
            SELECT 1 FROM game_boss_room_activities b
            WHERE b.admission = v_group.handoff_id AND b.state = 1
              AND b.boss_key = v_group.activity_key
              AND b.origin_channel_id = NEW.origin_channel_id
            FOR SHARE
        )
    )) THEN
        RAISE EXCEPTION 'activity membership requires a prepared group' USING ERRCODE = '23514';
    END IF;
    IF (NEW.enrollment_command_id IS NOT NULL) <> (v_group.state = 2) THEN
        RAISE EXCEPTION 'activity enrollment receipt requires an open committed window'
            USING ERRCODE = '23514';
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
