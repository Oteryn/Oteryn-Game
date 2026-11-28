-- A Creature STEP may move a whole live Ground item within its current scope.
-- The existing Ground owner lock and exact before/after receipt guard both addresses.
CREATE TABLE game_monster_item_push_receipts (
    source_ordinal bigint PRIMARY KEY,
    destination_ordinal bigint NOT NULL UNIQUE CHECK (destination_ordinal > source_ordinal),
    item_instance_id uuid NOT NULL,
    actor_placement bytea NOT NULL CHECK (octet_length(actor_placement) = 16),
    step_due numeric(20,0) NOT NULL CHECK (step_due >= 0),
    state_revision numeric(20,0) NOT NULL,
    reset_epoch numeric(20,0) NOT NULL CHECK (reset_epoch BETWEEN 0 AND 18446744073709551615),
    ground_before jsonb NOT NULL,
    ground_after jsonb NOT NULL,
    created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id(),
    CHECK (ground_before->'spatial_position' <> ground_after->'spatial_position'),
    CHECK (ground_before - ARRAY['spatial_position','stack_ordinal'] =
           ground_after - ARRAY['spatial_position','stack_ordinal'])
);

CREATE TRIGGER game_monster_item_push_receipt_immutable BEFORE UPDATE OR DELETE
    ON game_monster_item_push_receipts FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_monster_item_push_receipt_no_truncate BEFORE TRUNCATE
    ON game_monster_item_push_receipts FOR EACH STATEMENT EXECUTE FUNCTION game_item_immutable();

DO $migration$
DECLARE name text; body text; at integer;
BEGIN
    FOREACH name IN ARRAY ARRAY['game_native_map_ground_rebind','game_item_ground_owner_lock'] LOOP
        body := pg_get_functiondef((name || '()')::regprocedure);
        at := regexp_instr(body, E'\\mBEGIN\\M', 1, 1, 0, 'i');
        IF at = 0 THEN RAISE EXCEPTION 'missing Ground trigger predecessor %', name; END IF;
        EXECUTE overlay(body PLACING E'BEGIN\n' || $arm$
            IF TG_OP = 'UPDATE' AND EXISTS (
                SELECT 1 FROM game_monster_item_push_receipts r
                WHERE r.source_ordinal = OLD.stack_ordinal AND r.destination_ordinal = NEW.stack_ordinal
                  AND r.item_instance_id = OLD.item_instance_id AND r.created_xact_id = pg_current_xact_id()
                  AND r.ground_before = to_jsonb(OLD) AND r.ground_after = to_jsonb(NEW)
            ) THEN RETURN NEW; END IF;
        $arm$ FROM at FOR 5);
    END LOOP;
END
$migration$;

CREATE FUNCTION game_push_monster_ground_item(
    p_item uuid, p_world uuid, p_kind smallint, p_channel uuid, p_instance uuid,
    p_generation numeric, p_scope_key bytea, p_actor bytea, p_due numeric, p_revision numeric,
    p_ordinal bigint, p_source bytea, p_destination bytea, p_map text, p_content text, p_frame bytea,
    p_node uuid, p_node_revision numeric, p_node_secret bytea, p_reset numeric
) RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE before_row record;
DECLARE destination_ordinal bigint;
BEGIN
    IF p_kind = 1 AND p_channel IS NOT NULL AND p_instance IS NULL THEN
        PERFORM pg_advisory_xact_lock(hashtextextended(encode(uuid_send(p_world),'hex') ||
            encode(uuid_send(p_channel),'hex'),33));
    ELSIF p_kind = 2 AND p_channel IS NULL AND p_instance IS NOT NULL THEN
        PERFORM pg_advisory_xact_lock(hashtextextended(encode(p_scope_key,'hex'),33));
    ELSE
        RETURN false;
    END IF;
    -- A reset already taking its exclusive job fence wins this owner turn.
    LOCK TABLE game_world_reset_jobs IN SHARE MODE NOWAIT;
    IF octet_length(p_source) <> 10 OR octet_length(p_destination) <> 10
       OR p_source = p_destination OR substring(p_source from 9) <> substring(p_destination from 9)
       OR NOT EXISTS (
           SELECT 1 FROM game_runtime_scope_assignments s
           JOIN game_durability_admission_runtime_guards a USING (scope_key)
           WHERE s.scope_key = p_scope_key AND s.world_id = p_world
             AND s.channel_id IS NOT DISTINCT FROM p_channel
             AND s.activity_instance_id IS NOT DISTINCT FROM p_instance
             AND s.state = 1 AND a.ready AND s.holder_node_id = p_node
             AND s.holder_registration_revision = p_node_revision
             AND s.ownership_generation = p_generation AND a.ownership_generation = p_generation
           FOR SHARE OF s, a
       )
       OR game_node_prove_current_incarnation(p_node, p_node_revision, p_node_secret) IS DISTINCT FROM true
       OR game_world_reset_map_epoch_current(p_world, p_reset) IS DISTINCT FROM true
       OR abs((('x' || encode(substring(p_source from 1 for 4),'hex'))::bit(32)::integer)::bigint -
              (('x' || encode(substring(p_destination from 1 for 4),'hex'))::bit(32)::integer)::bigint)
        + abs((('x' || encode(substring(p_source from 5 for 4),'hex'))::bit(32)::integer)::bigint -
              (('x' || encode(substring(p_destination from 5 for 4),'hex'))::bit(32)::integer)::bigint) <> 1
       THEN RETURN false; END IF;

    -- Replays use the original append ordinal, even after a later move.
    IF EXISTS (SELECT 1 FROM game_monster_item_push_receipts r WHERE r.source_ordinal = p_ordinal) THEN
        RETURN EXISTS (
            SELECT 1 FROM game_monster_item_push_receipts r
            WHERE r.source_ordinal = p_ordinal AND r.item_instance_id = p_item
              AND r.actor_placement = p_actor AND r.step_due = p_due AND r.state_revision = p_revision
              AND r.reset_epoch = p_reset
              AND r.ground_before->>'world_id' = p_world::text
              AND r.ground_before->>'scope_kind' = p_kind::text
              AND r.ground_before->>'channel_id' IS NOT DISTINCT FROM p_channel::text
              AND r.ground_before->>'instance_id' IS NOT DISTINCT FROM p_instance::text
              AND r.ground_before->>'runtime_scope_ownership_generation' = p_generation::text
              AND r.ground_before->>'map_revision' = p_map
              AND r.ground_before->>'content_revision' = p_content
              AND r.ground_before->'native_room_placement_context' = to_jsonb(p_frame)
              AND r.ground_before->'spatial_position' = to_jsonb(p_source)
              AND r.ground_after->'spatial_position' = to_jsonb(p_destination)
        );
    END IF;

    PERFORM 1 FROM game_item_instances i WHERE i.item_instance_id = p_item
      AND i.world_id = p_world AND i.lifecycle = 1 AND i.state_revision = p_revision
      AND (
          EXISTS (
              SELECT 1 FROM game_native_map_item_receipts m
              WHERE m.item_instance_id = i.item_instance_id AND m.transaction_id = i.minted_transaction_id
                AND m.blocks_movement AND NOT m.immovable_block_solid
                AND coalesce(m.source_attributes->>'4','0') = '0'
                AND coalesce(m.source_attributes->>'5','0') = '0'
          ) OR EXISTS (
              SELECT 1 FROM game_spell_item_lines l
              WHERE l.item_instance_id = i.item_instance_id AND l.transaction_id = i.minted_transaction_id
                AND l.operation_kind = 1 AND l.blocks_movement AND NOT l.immovable_block_solid
                AND l.expires_at_unix_ms IS NULL
          )
      )
      FOR UPDATE;
    IF NOT FOUND THEN RETURN false; END IF;
    SELECT g.* INTO before_row FROM game_item_ground_locations g
      WHERE g.item_instance_id = p_item AND g.world_id = p_world
        AND g.scope_kind = p_kind AND g.channel_id IS NOT DISTINCT FROM p_channel
        AND g.instance_id IS NOT DISTINCT FROM p_instance AND g.runtime_scope_ownership_generation = p_generation
        AND g.stack_ordinal = p_ordinal AND g.spatial_position = p_source
        AND g.map_revision = p_map AND g.content_revision = p_content
        AND g.native_room_placement_context = p_frame;
    IF NOT FOUND THEN RETURN false; END IF;

    destination_ordinal := nextval('game_item_ground_stack_ordinal_seq');
    INSERT INTO game_monster_item_push_receipts
        (source_ordinal, destination_ordinal, item_instance_id, actor_placement, step_due,
         state_revision, reset_epoch, ground_before, ground_after)
    VALUES (p_ordinal, destination_ordinal, p_item, p_actor, p_due, p_revision, p_reset,
            to_jsonb(before_row), to_jsonb(before_row) ||
                jsonb_build_object('spatial_position', p_destination, 'stack_ordinal', destination_ordinal));
    UPDATE game_item_ground_locations g
       SET spatial_position = p_destination, stack_ordinal = destination_ordinal
       WHERE g.item_instance_id = p_item AND g.stack_ordinal = p_ordinal;
    IF NOT FOUND THEN RAISE EXCEPTION 'Ground push lost its locked item' USING ERRCODE = '23514'; END IF;
    RETURN true;
END $$;

-- A native-map generation handoff preserves the moved address and order.
CREATE FUNCTION game_monster_item_push_current(
    p_ground game_item_ground_locations, p_revision numeric
) RETURNS boolean LANGUAGE sql STABLE AS $$
    SELECT EXISTS (
        SELECT 1 FROM game_monster_item_push_receipts r
        WHERE r.destination_ordinal = (p_ground).stack_ordinal
          AND r.item_instance_id = (p_ground).item_instance_id AND r.state_revision = p_revision
          AND (r.ground_after = to_jsonb(p_ground) OR (
              r.ground_after - 'runtime_scope_ownership_generation' =
                  to_jsonb(p_ground) - 'runtime_scope_ownership_generation'
              AND EXISTS (
                  SELECT 1 FROM game_native_map_scope_adoptions a
                  JOIN game_native_map_item_receipts m ON m.transaction_id = a.source_transaction_id
                  WHERE m.item_instance_id = r.item_instance_id
                    AND a.ownership_generation = (p_ground).runtime_scope_ownership_generation
                    AND a.ground_before - 'runtime_scope_ownership_generation' =
                        to_jsonb(p_ground) - 'runtime_scope_ownership_generation'
              )
          ))
    )
$$;

DO $migration$
BEGIN
    EXECUTE format('ALTER FUNCTION game_monster_item_push_current(game_item_ground_locations,numeric) SET search_path = %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_push_monster_ground_item(uuid,uuid,smallint,uuid,uuid,numeric,bytea,bytea,numeric,numeric,bigint,bytea,bytea,text,text,bytea,uuid,numeric,bytea,numeric) SET search_path = %I, pg_temp', current_schema());
END
$migration$;

REVOKE ALL ON game_monster_item_push_receipts FROM PUBLIC;
REVOKE ALL ON FUNCTION game_push_monster_ground_item(uuid,uuid,smallint,uuid,uuid,numeric,bytea,bytea,numeric,numeric,bigint,bytea,bytea,text,text,bytea,uuid,numeric,bytea,numeric) FROM PUBLIC;
GRANT SELECT ON game_monster_item_push_receipts TO oteryn_game_runtime;
GRANT EXECUTE ON FUNCTION game_push_monster_ground_item(uuid,uuid,smallint,uuid,uuid,numeric,bytea,bytea,numeric,numeric,bigint,bytea,bytea,text,text,bytea,uuid,numeric,bytea,numeric) TO oteryn_game_runtime;
REVOKE ALL ON FUNCTION game_monster_item_push_current(game_item_ground_locations,numeric) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_monster_item_push_current(game_item_ground_locations,numeric) TO oteryn_game_runtime;
