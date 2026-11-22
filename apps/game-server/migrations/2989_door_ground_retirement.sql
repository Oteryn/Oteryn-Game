-- M06: a closing doorway retires stranded Ground trees and authored placements.
CREATE TABLE game_door_ground_retirements (
    item_instance_id UUID PRIMARY KEY REFERENCES game_item_instances,
    root_item_instance_id UUID NOT NULL REFERENCES game_item_instances,
    transaction_id UUID NOT NULL CHECK (game_character_is_uuid_v7(transaction_id)),
    game_session_id UUID NOT NULL,
    state_revision_before NUMERIC(20,0) NOT NULL CHECK (state_revision_before > 0),
    quantity_before BIGINT NOT NULL CHECK (quantity_before > 0),
    location_before JSONB NOT NULL,
    created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id()
);
CREATE TABLE game_door_authored_retirements (
    world_id UUID NOT NULL,
    channel_id UUID NOT NULL,
    base_bundle_digest BYTEA NOT NULL CHECK (octet_length(base_bundle_digest) = 32),
    placement_key NUMERIC(20,0) NOT NULL CHECK (placement_key > 0),
    reset_epoch NUMERIC(20,0) NOT NULL CHECK (reset_epoch >= 0),
    door_key NUMERIC(20,0) NOT NULL CHECK (door_key > 0 AND door_key <> placement_key),
    game_session_id UUID NOT NULL,
    PRIMARY KEY (world_id, channel_id, base_bundle_digest, reset_epoch, placement_key)
);
CREATE TRIGGER game_door_ground_retirements_immutable
    BEFORE UPDATE OR DELETE ON game_door_ground_retirements
    FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_door_ground_retirements_no_truncate
    BEFORE TRUNCATE ON game_door_ground_retirements
    EXECUTE FUNCTION game_item_reject_truncate();
CREATE TRIGGER game_door_authored_retirements_immutable
    BEFORE UPDATE OR DELETE ON game_door_authored_retirements
    FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_door_authored_retirements_no_truncate
    BEFORE TRUNCATE ON game_door_authored_retirements
    EXECUTE FUNCTION game_item_reject_truncate();

CREATE FUNCTION game_door_ground_receipt_admit() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF current_user <> pg_get_userbyid((SELECT relowner FROM pg_class WHERE oid=TG_RELID)) THEN
        RAISE EXCEPTION 'door Ground receipts require the retirement owner' USING ERRCODE='42501';
    END IF;
    NEW.created_xact_id:=pg_current_xact_id();
    RETURN NEW;
END $$;
CREATE TRIGGER game_door_ground_receipt_admit
    BEFORE INSERT ON game_door_ground_retirements
    FOR EACH ROW EXECUTE FUNCTION game_door_ground_receipt_admit();

-- Runs inside the Ground owner's current-session, reset and scope transaction.
-- Each live root and its ordinary nested contents retire together. A committed
-- receipt for this root and source is terminal, including after a lost reply.
CREATE FUNCTION game_retire_door_ground(
    p_item UUID, p_session UUID, p_world UUID, p_channel UUID, p_instance UUID,
    p_generation NUMERIC, p_position BYTEA, p_map_revision TEXT, p_content_revision TEXT,
    p_native_position BYTEA, p_native_map_revision TEXT
) RETURNS BOOLEAN LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE
    prior game_door_ground_retirements%ROWTYPE;
    ground game_item_ground_locations%ROWTYPE;
    tree UUID[];
    retirement_id UUID := game_character_uuid_v7();
    written INTEGER;
BEGIN
    -- Fresh admission and reconnect commit publish Active (2).
    PERFORM 1 FROM game_durability_reconnect_sessions s
      WHERE s.game_session_id=p_session AND s.session_state=2
        AND s.runtime_scope_world_id=p_world
        AND s.runtime_scope_channel_id IS NOT DISTINCT FROM p_channel
        AND s.runtime_scope_instance_id IS NOT DISTINCT FROM p_instance
        AND s.scope_ownership_generation=p_generation
      FOR SHARE;
    IF NOT FOUND THEN RETURN FALSE; END IF;
    SELECT * INTO prior FROM game_door_ground_retirements WHERE item_instance_id=p_item;
    IF FOUND THEN
        RETURN prior.root_item_instance_id=p_item AND prior.game_session_id=p_session
            AND (prior.location_before->>'world_id')::uuid=p_world
            AND (prior.location_before->>'channel_id')::uuid IS NOT DISTINCT FROM p_channel
            AND (prior.location_before->>'instance_id')::uuid IS NOT DISTINCT FROM p_instance
            AND ((prior.location_before->>'spatial_position')::bytea,
                 prior.location_before->>'map_revision')
                IN ((p_position,p_map_revision),(p_native_position,p_native_map_revision))
            AND prior.location_before->>'content_revision'=p_content_revision;
    END IF;
    SELECT g.* INTO ground FROM game_item_ground_locations g
      JOIN game_item_instances i USING(item_instance_id,world_id)
     WHERE g.item_instance_id=p_item AND g.world_id=p_world
       AND g.channel_id IS NOT DISTINCT FROM p_channel
       AND g.instance_id IS NOT DISTINCT FROM p_instance
       AND g.runtime_scope_ownership_generation<=p_generation
       AND (g.spatial_position,g.map_revision)
           IN ((p_position,p_map_revision),(p_native_position,p_native_map_revision))
       AND g.content_revision=p_content_revision AND i.lifecycle=1
     FOR UPDATE OF i;
    IF NOT FOUND THEN RETURN FALSE; END IF;
    -- A corpse moved by the loop can appear in the ordinary Ground view.
    -- Recheck its durable provenance independently of the projection.
    IF game_item_plain_corpse_source(p_item)
       OR EXISTS (SELECT 1 FROM game_item_mint_receipts m WHERE m.item_instance_id=p_item
                   AND m.loot_purpose_key='CORPSE_MATERIALIZATION') THEN
        RETURN TRUE;
    END IF;
    WITH RECURSIVE members(id) AS (
        SELECT p_item
        UNION
        SELECT e.item_instance_id FROM members m JOIN game_item_nested_entries e
          ON e.parent_item_instance_id=m.id AND e.world_id=p_world
    )
    SELECT array_agg(id ORDER BY id) INTO tree FROM (SELECT id FROM members LIMIT 501) bounded;
    IF cardinality(tree)>500 THEN RETURN FALSE; END IF;
    PERFORM 1 FROM game_item_instances WHERE item_instance_id=ANY(tree)
        AND world_id=p_world AND lifecycle=1 ORDER BY item_instance_id FOR UPDATE;
    IF EXISTS (SELECT 1 FROM game_item_corpse_container_entries
                WHERE parent_item_instance_id=ANY(tree))
       OR EXISTS (SELECT 1 FROM game_item_container_entries
                   WHERE parent_item_instance_id=ANY(tree)) THEN
        RETURN FALSE;
    END IF;
    INSERT INTO game_door_ground_retirements
        (item_instance_id,root_item_instance_id,transaction_id,game_session_id,
         state_revision_before,quantity_before,location_before)
    SELECT i.item_instance_id,p_item,retirement_id,p_session,i.state_revision,i.quantity,
           CASE WHEN i.item_instance_id=p_item THEN to_jsonb(ground) ELSE to_jsonb(e) END
      FROM game_item_instances i LEFT JOIN game_item_nested_entries e USING(item_instance_id,world_id)
     WHERE i.item_instance_id=ANY(tree) AND i.world_id=p_world AND i.lifecycle=1;
    GET DIAGNOSTICS written = ROW_COUNT;
    IF written<>cardinality(tree) THEN
        RAISE EXCEPTION 'door retirement lost a member of its Ground tree' USING ERRCODE='23514';
    END IF;
    DELETE FROM game_item_nested_entries WHERE item_instance_id=ANY(tree);
    DELETE FROM game_item_ground_locations WHERE item_instance_id=p_item;
    UPDATE game_item_instances SET quantity=0,lifecycle=2,last_transaction_id=retirement_id
        WHERE item_instance_id=ANY(tree);
    RETURN TRUE;
END $$;

-- Append one same-transaction receipt alternative to each installed custody
-- guard. The other mutation families retain their current proofs.
DO $migration$
DECLARE n TEXT; body TEXT; arm TEXT; at_begin INTEGER;
BEGIN
 FOREACH n IN ARRAY ARRAY['game_item_instance_guard','game_item_instance_change_proven',
                           'game_item_ground_removal_proven','game_item_nested_entry_proven'] LOOP
  body:=pg_get_functiondef((n||'()')::regprocedure);
  at_begin:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
  IF at_begin=0 THEN RAISE EXCEPTION 'missing door retirement predecessor %',n; END IF;
  IF n='game_item_instance_guard' THEN
   arm:=$arm$
    IF TG_OP='UPDATE' AND OLD.lifecycle=1 AND NEW.lifecycle=2 AND NEW.quantity=0
       AND (to_jsonb(NEW)-ARRAY['quantity','lifecycle','last_transaction_id'])
           =(to_jsonb(OLD)-ARRAY['quantity','lifecycle','last_transaction_id'])
       AND EXISTS (SELECT 1 FROM game_door_ground_retirements r
          WHERE r.item_instance_id=OLD.item_instance_id AND r.transaction_id=NEW.last_transaction_id
            AND r.state_revision_before=OLD.state_revision AND r.quantity_before=OLD.quantity
            AND r.created_xact_id=pg_current_xact_id()) THEN
        NEW.state_revision:=OLD.state_revision+1;
        RETURN NEW;
    END IF;
   $arm$;
  ELSIF n='game_item_instance_change_proven' THEN
   arm:=$arm$
    IF TG_OP='UPDATE' AND NEW.lifecycle=2 AND NEW.quantity=0
       AND EXISTS (SELECT 1 FROM game_door_ground_retirements r
          WHERE r.item_instance_id=NEW.item_instance_id AND r.transaction_id=NEW.last_transaction_id
            AND r.state_revision_before+1=NEW.state_revision
            AND r.created_xact_id=pg_current_xact_id()) THEN RETURN NULL; END IF;
   $arm$;
  ELSE
   arm:=$arm$
    IF TG_OP='DELETE' AND EXISTS (SELECT 1 FROM game_door_ground_retirements r
        JOIN game_item_instances i USING(item_instance_id)
        WHERE r.item_instance_id=OLD.item_instance_id AND r.location_before=to_jsonb(OLD)
          AND r.transaction_id=i.last_transaction_id AND i.lifecycle=2 AND i.quantity=0
          AND r.created_xact_id=pg_current_xact_id()) THEN RETURN NULL; END IF;
   $arm$;
  END IF;
  EXECUTE overlay(body placing E'BEGIN\n'||arm FROM at_begin FOR 5);
 END LOOP;
 EXECUTE format('ALTER FUNCTION game_retire_door_ground(UUID,UUID,UUID,UUID,UUID,NUMERIC,BYTEA,TEXT,TEXT,BYTEA,TEXT) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $migration$;

REVOKE ALL ON game_door_ground_retirements, game_door_authored_retirements FROM PUBLIC;
GRANT SELECT, INSERT, UPDATE ON game_door_ground_retirements, game_door_authored_retirements TO oteryn_game_runtime;
GRANT SELECT ON game_door_ground_retirements, game_door_authored_retirements TO oteryn_game_control;
REVOKE ALL ON FUNCTION game_door_ground_receipt_admit() FROM PUBLIC;
REVOKE ALL ON FUNCTION game_retire_door_ground(UUID,UUID,UUID,UUID,UUID,NUMERIC,BYTEA,TEXT,TEXT,BYTEA,TEXT) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_retire_door_ground(UUID,UUID,UUID,UUID,UUID,NUMERIC,BYTEA,TEXT,TEXT,BYTEA,TEXT) TO oteryn_game_runtime;
