-- TOOLS-2: fixed-map state is Channel-owned; a reset epoch starts a fresh placement.
CREATE TABLE game_terrain_tool_transforms (
    world_id UUID NOT NULL,
    channel_id UUID NOT NULL,
    placement BYTEA NOT NULL CHECK (octet_length(placement) = 40),
    reset_epoch BIGINT NOT NULL CHECK (reset_epoch >= 0),
    map_revision TEXT NOT NULL CHECK (octet_length(map_revision) BETWEEN 1 AND 512),
    content_generation BYTEA NOT NULL CHECK (octet_length(content_generation) = 32),
    state_item_id BIGINT NOT NULL CHECK (state_item_id BETWEEN 1 AND 4294967295),
    object_revision BIGINT NOT NULL CHECK (object_revision >= 1),
    decay_states BIGINT[] NOT NULL,
    decay_delays BIGINT[] NOT NULL,
    decay_step INTEGER NOT NULL DEFAULT 0,
    decay_at_unix_ms BIGINT,
    CHECK (cardinality(decay_states) = cardinality(decay_delays)
        AND cardinality(decay_states) <= 8
        AND array_position(decay_states, NULL) IS NULL
        AND array_position(decay_delays, NULL) IS NULL
        AND decay_step BETWEEN 0 AND cardinality(decay_states)
        AND 0 < ALL(decay_states) AND 4294967295 >= ALL(decay_states)
        AND 0 < ALL(decay_delays)
        AND (decay_at_unix_ms IS NULL) = (decay_step = cardinality(decay_states))),
    PRIMARY KEY (world_id, channel_id, placement, reset_epoch)
);
CREATE INDEX game_terrain_tool_transforms_due
    ON game_terrain_tool_transforms (world_id, channel_id, decay_at_unix_ms)
    WHERE decay_at_unix_ms IS NOT NULL;

CREATE TABLE game_terrain_tool_receipts (
    game_session_id UUID NOT NULL,
    command_id NUMERIC(20,0) NOT NULL CHECK (command_id BETWEEN 1 AND 18446744073709551615),
    character_id UUID NOT NULL,
    tool_item_instance_id UUID NOT NULL,
    intent BYTEA NOT NULL CHECK (octet_length(intent) BETWEEN 1 AND 529),
    world_id UUID NOT NULL,
    channel_id UUID NOT NULL,
    placement BYTEA NOT NULL,
    reset_epoch BIGINT NOT NULL,
    transformed_item_id BIGINT NOT NULL CHECK (transformed_item_id BETWEEN 1 AND 4294967295),
    committed_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    PRIMARY KEY (game_session_id, command_id),
    FOREIGN KEY (world_id, channel_id, placement, reset_epoch)
        REFERENCES game_terrain_tool_transforms (world_id, channel_id, placement, reset_epoch)
);

-- Called only inside the durable owner's current-session/custody transaction. The database
-- clock starts decay at commit admission, never at a client-provided timestamp.
CREATE FUNCTION game_apply_terrain_tool_transform(
    p_game_session_id UUID, p_command_id NUMERIC, p_character_id UUID,
    p_tool_item_instance_id UUID, p_intent BYTEA, p_world_id UUID, p_channel_id UUID,
    p_placement BYTEA, p_map_revision TEXT, p_content_generation BYTEA, p_reset_epoch BIGINT,
    p_expected_revision BIGINT, p_before_item_id BIGINT, p_after_item_id BIGINT,
    p_decay_states BIGINT[], p_decay_delays BIGINT[]
) RETURNS BOOLEAN LANGUAGE plpgsql AS $$
DECLARE
    prior game_terrain_tool_receipts%ROWTYPE;
    current_state game_terrain_tool_transforms%ROWTYPE;
    now_ms BIGINT;
BEGIN
    -- Serializes first insertion, revision CAS and the per-scope resource bound.
    PERFORM pg_advisory_xact_lock(hashtextextended(
        'oteryn:terrain-scope:' || p_world_id::text || ':' || p_channel_id::text, 0));
    SELECT * INTO prior FROM game_terrain_tool_receipts
        WHERE game_session_id=p_game_session_id AND command_id=p_command_id;
    IF FOUND THEN
        RETURN prior.character_id=p_character_id AND prior.tool_item_instance_id=p_tool_item_instance_id
            AND prior.intent=p_intent AND prior.world_id=p_world_id AND prior.channel_id=p_channel_id
            AND prior.placement=p_placement AND prior.reset_epoch=p_reset_epoch;
    END IF;
    SELECT * INTO current_state FROM game_terrain_tool_transforms
        WHERE world_id=p_world_id AND channel_id=p_channel_id
            AND placement=p_placement AND reset_epoch=p_reset_epoch FOR UPDATE;
    now_ms := floor(extract(epoch FROM clock_timestamp()) * 1000)::bigint;
    IF FOUND THEN
        IF current_state.object_revision<>p_expected_revision OR current_state.state_item_id<>p_before_item_id
            OR current_state.map_revision<>p_map_revision OR current_state.content_generation<>p_content_generation THEN
            RETURN FALSE;
        END IF;
        UPDATE game_terrain_tool_transforms SET state_item_id=p_after_item_id,
            object_revision=object_revision+1, decay_states=p_decay_states, decay_delays=p_decay_delays,
            decay_step=0, decay_at_unix_ms=CASE WHEN cardinality(p_decay_delays)>0 THEN now_ms+p_decay_delays[1] END
            WHERE world_id=p_world_id AND channel_id=p_channel_id AND placement=p_placement AND reset_epoch=p_reset_epoch;
    ELSE
        IF p_expected_revision<>0 OR (SELECT count(*) FROM (SELECT 1 FROM game_terrain_tool_transforms
            WHERE world_id=p_world_id AND channel_id=p_channel_id AND map_revision=p_map_revision
                AND reset_epoch=p_reset_epoch LIMIT 1024) bounded)>=1024 THEN
            RETURN FALSE;
        END IF;
        INSERT INTO game_terrain_tool_transforms VALUES (
            p_world_id,p_channel_id,p_placement,p_reset_epoch,p_map_revision,p_content_generation,
            p_after_item_id,1,p_decay_states,p_decay_delays,0,
            CASE WHEN cardinality(p_decay_delays)>0 THEN now_ms+p_decay_delays[1] END);
    END IF;
    INSERT INTO game_terrain_tool_receipts (
        game_session_id,command_id,character_id,tool_item_instance_id,intent,world_id,channel_id,
        placement,reset_epoch,transformed_item_id
    ) VALUES (p_game_session_id,p_command_id,p_character_id,p_tool_item_instance_id,p_intent,
        p_world_id,p_channel_id,p_placement,p_reset_epoch,p_after_item_id);
    RETURN TRUE;
END;
$$;

GRANT SELECT, INSERT, UPDATE ON game_terrain_tool_transforms TO oteryn_game_runtime;
GRANT SELECT, INSERT ON game_terrain_tool_receipts TO oteryn_game_runtime;
REVOKE ALL ON FUNCTION game_apply_terrain_tool_transform(UUID,NUMERIC,UUID,UUID,BYTEA,UUID,UUID,BYTEA,TEXT,BYTEA,BIGINT,BIGINT,BIGINT,BIGINT,BIGINT[],BIGINT[]) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_apply_terrain_tool_transform(UUID,NUMERIC,UUID,UUID,BYTEA,UUID,UUID,BYTEA,TEXT,BYTEA,BIGINT,BIGINT,BIGINT,BIGINT,BIGINT[],BIGINT[]) TO oteryn_game_runtime;
