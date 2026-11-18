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

-- Geode contributions persist for the live World reset epoch and Channel.
CREATE TABLE game_warzone_geodes (
    world_id UUID NOT NULL,
    channel_id UUID NOT NULL,
    geode_key TEXT NOT NULL CHECK (geode_key IN (
        'oteryn:quest-callback/dangerous-depth-geode-iv',
        'oteryn:quest-callback/dangerous-depth-geode-v',
        'oteryn:quest-callback/dangerous-depth-geode-vi'
    )),
    reset_epoch NUMERIC(20,0) NOT NULL CHECK (reset_epoch >= 0),
    fragments SMALLINT NOT NULL CHECK (fragments BETWEEN 1 AND 30),
    opened_at TIMESTAMPTZ,
    last_session UUID NOT NULL,
    last_command NUMERIC(20,0) NOT NULL CHECK (last_command BETWEEN 1 AND 18446744073709551615),
    contribution_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
    PRIMARY KEY (world_id, channel_id, geode_key),
    CHECK ((fragments = 30) = (opened_at IS NOT NULL))
);

REVOKE ALL ON game_warzone_geodes FROM PUBLIC;
GRANT SELECT, INSERT, UPDATE ON game_warzone_geodes TO oteryn_game_runtime;
GRANT SELECT ON game_warzone_geodes TO oteryn_game_control;

-- Geode USE writes a receipt and an exact carried BURN without personal quest progress.
DO $migration$
DECLARE predicate TEXT; body TEXT;
    keys TEXT := $keys$ARRAY[
        'oteryn:quest-callback/dangerous-depth-geode-iv',
        'oteryn:quest-callback/dangerous-depth-geode-v',
        'oteryn:quest-callback/dangerous-depth-geode-vi']$keys$;
    needle TEXT := 'IF cardinality(NEW.track_keys)<>0 OR NEW.completes THEN RETURN NULL; END IF;';
BEGIN
    SELECT pg_get_expr(conbin,conrelid) INTO STRICT predicate FROM pg_constraint
      WHERE conrelid='game_character_quest_receipts'::regclass
        AND conname='game_character_quest_receipt_effects_shape';
    ALTER TABLE game_character_quest_receipts DROP CONSTRAINT game_character_quest_receipt_effects_shape;
    EXECUTE format($shape$ALTER TABLE game_character_quest_receipts
      ADD CONSTRAINT game_character_quest_receipt_effects_shape CHECK ((%s) OR (
        transition_key=ANY(%s) AND quest_key='oteryn:quest.dangerous_depths_quest'
        AND cause_kind='use' AND NOT completes AND cardinality(track_keys)=0
        AND cardinality(values_before)=0 AND cardinality(values_after)=0
        AND level_after=level_before AND experience_after=experience_before))$shape$, predicate, keys);

    body := pg_get_functiondef('game_quest_reward_value_only_complete()'::regprocedure);
    IF (length(body)-length(replace(body,needle,'')))/length(needle)<>1 THEN
        RAISE EXCEPTION 'missing or ambiguous canonical value-only quest guard';
    END IF;
    body := replace(body,needle,needle || format($arm$
      IF NEW.transition_key=ANY(%s) THEN
        IF NEW.quest_key='oteryn:quest.dangerous_depths_quest' AND NEW.cause_kind='use'
          AND NEW.created_xact_id=pg_current_xact_id()
          AND EXISTS (
            SELECT 1 FROM game_quest_callback_consumptions p
            JOIN game_warzone_geodes g ON g.geode_key=p.transition_key
              AND g.world_id=(p.before_item->>'world_id')::uuid
              AND g.last_session=p.cause_id AND g.last_command=p.cause_ordinal
            WHERE p.character_id=NEW.character_id AND p.cause_id=NEW.cause_id
              AND p.cause_ordinal=NEW.cause_ordinal AND p.transition_key=NEW.transition_key
              AND p.created_xact_id=pg_current_xact_id() AND g.contribution_xact_id=pg_current_xact_id()
              AND p.quantity=1 AND p.replacement IS NULL AND p.location_kind<>'ground'
              AND p.before_item->>'definition_family'='Item'
              AND p.before_item->>'definition_production_key'='oteryn:item.tibia.i27509'
              AND p.before_item->>'definition_revision_ref'='definition-r1'
              AND convert_from(p.proof,'UTF8')::jsonb #>> '{callback,selector,native,appearance}'='27510'
              AND convert_from(p.proof,'UTF8')::jsonb #>> '{callback,item_on_target,selected_instance}'='true'
          ) THEN RETURN NULL; END IF;
        RAISE EXCEPTION 'Warzone geode receipt lacks contribution and carried fragment' USING ERRCODE='23514';
      END IF;
    $arm$, keys));
    EXECUTE body;
END $migration$;
