-- Crystal 201189f8's Targuna createFunction attaches HeraldDeath but omits
-- BossLeverOnDeath. Victory opens the portal and leaves the original fight
-- timeout in force. Keep the existing terminal policy for every other room.
CREATE OR REPLACE FUNCTION game_boss_room_runtime_guard() RETURNS trigger
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
           AND (CASE
               WHEN OLD.boss_key='oteryn:boss.herald_of_fire'
                   AND OLD.boss_creature_key='oteryn:creature.herald_of_fire'
               THEN NEW.exit_deadline=OLD.deadline
               ELSE (OLD.after_kill_ms=0 AND NEW.exit_deadline IS NULL)
                    OR (OLD.after_kill_ms>0 AND NEW.exit_deadline=NEW.terminal_at+OLD.after_kill_ms)
           END) THEN RETURN NEW; END IF;
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
REVOKE ALL ON FUNCTION game_boss_room_runtime_guard() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_boss_room_runtime_guard() TO oteryn_game_runtime;

-- Crystal 201189f8 heraldoffirefields. Reuse the exact Instance CreatureEvent
-- custody and audit chain with a separately checked, closed source rule.
CREATE FUNCTION game_herald_field_source_shape(r game_spell_item_receipts) RETURNS BOOLEAN
LANGUAGE sql IMMUTABLE SET search_path = pg_catalog, public, pg_temp AS $$
 SELECT coalesce(r.cause_kind=3 AND r.spell_family='Creature'
  AND r.spell_production_key='oteryn:creature.herald_of_fire' AND r.spell_revision='definition-r1'
  AND r.creature_definition_key=r.spell_production_key AND r.creature_definition_revision=r.spell_revision
  AND r.runtime_scope_kind=2 AND r.channel_id IS NULL AND r.instance_id IS NOT NULL
  AND game_node_is_uuid_v7(r.instance_id)
  AND r.swing_ordinal IS NULL AND r.game_session_id IS NULL AND r.command_id IS NULL AND r.character_id IS NULL
  AND r.cost IS NULL AND r.cost_binding IS NULL AND r.caster_lease_generation IS NULL AND r.caster_placement_digest IS NULL
  AND octet_length(r.creature_placement)=16 AND r.creature_placement<>decode(repeat('00',16),'hex')
  AND r.creature_actor_generation BETWEEN 1 AND 18446744073709551615
  AND octet_length(r.creature_occurrence) BETWEEN 1 AND 1024
  AND octet_length(r.creature_map_digest)=32 AND octet_length(r.creature_frame_digest)=32 AND octet_length(r.creature_cast_digest)=32
  AND convert_from(r.intent,'UTF8')::jsonb->>'schema'='OTERYN_CREATURE_EVENT_GROUND_CAUSE/v1'
  AND convert_from(r.intent,'UTF8')::jsonb->>'runtime_scope_kind'='2'
  AND convert_from(r.intent,'UTF8')::jsonb->'channel'='null'::jsonb
  AND convert_from(r.intent,'UTF8')::jsonb->>'instance'=replace(r.instance_id::text,'-','')
  AND CASE convert_from(r.intent,'UTF8')::jsonb->'source_event'->>'rule'
   WHEN 'targuna.herald.fire-fields' THEN r.operation_count BETWEEN 25 AND 100
       AND r.creature_source_body_digest=decode('8aece116ed66f01cf040743d7579cdc1f32e572a018bd4beb55c1be066da9fab','hex')
   WHEN 'targuna.herald.red-floors' THEN r.operation_count BETWEEN 0 AND 500
       AND r.creature_source_body_digest=decode('e096ce49eff310b5dc377f22f198c658391bccbca574c3c30fd71f59f48ffbfd','hex')
   ELSE false END
  AND convert_from(r.intent,'UTF8')::jsonb->'source_event'->'action_id'='null'::jsonb,false);
$$;

DO $$
DECLARE n TEXT; prior TEXT; body TEXT; amended TEXT;
BEGIN
 FOREACH n IN ARRAY ARRAY['game_spell_item_typed_cause','game_spell_item_exact_runtime_scope'] LOOP
  SELECT pg_get_constraintdef(oid) INTO STRICT prior FROM pg_constraint
   WHERE conrelid='game_spell_item_receipts'::regclass AND conname=n;
  IF left(prior,6)<>'CHECK ' THEN RAISE EXCEPTION 'Herald scoped item receipt guard changed'; END IF;
  EXECUTE format('ALTER TABLE game_spell_item_receipts DROP CONSTRAINT %I',n);
  EXECUTE format('ALTER TABLE game_spell_item_receipts ADD CONSTRAINT %I CHECK ((%s OR game_herald_field_source_shape(game_spell_item_receipts)) IS TRUE)',n,substr(prior,7));
 END LOOP;
 body:=pg_get_functiondef('game_rotten_portal_source_proven()'::regprocedure);
 amended:=replace(body,E'BEGIN\n',E'BEGIN\n IF game_herald_field_source_shape(NEW) THEN RETURN NULL; END IF;\n');
 IF amended=body THEN RAISE EXCEPTION 'Herald independent source guard changed'; END IF;
 EXECUTE amended;
END $$;

CREATE FUNCTION game_herald_field_source_proven() RETURNS trigger
LANGUAGE plpgsql SET search_path = pg_catalog, public, pg_temp AS $$
DECLARE refresh BOOLEAN;
BEGIN
 IF NOT game_herald_field_source_shape(NEW) THEN RETURN NULL; END IF;
 refresh:=convert_from(NEW.intent,'UTF8')::jsonb->'source_event'->>'rule'='targuna.herald.fire-fields';
 IF NOT EXISTS (
  SELECT 1 FROM game_activity_handoff_groups g
  JOIN game_activity_runtime_entries e USING(handoff_id)
  JOIN game_boss_room_activities a ON a.admission=g.handoff_id
  WHERE g.world_id=NEW.world_id AND g.instance_id=NEW.instance_id AND g.state=2
   AND g.ownership_generation=NEW.ownership_generation
   AND e.destination_server_artifact_digest=NEW.catalog_digest
   AND e.destination_map_revision_digest=NEW.creature_map_digest
   AND e.destination_coordinate_frame_digest=NEW.creature_frame_digest
   AND a.boss_key='oteryn:boss.herald_of_fire' AND a.boss_creature_key=NEW.creature_definition_key
 ) OR NEW.created_xact_id<>pg_current_xact_id()
 OR NEW.operation_count<>(SELECT count(*) FROM game_spell_item_lines l WHERE l.transaction_id=NEW.transaction_id)
 OR (CASE WHEN refresh THEN 25 ELSE 0 END)<>(SELECT count(*) FROM game_spell_item_lines l WHERE l.transaction_id=NEW.transaction_id AND l.operation_kind=1)
 OR EXISTS (
  SELECT 1 FROM game_spell_item_lines l WHERE l.transaction_id=NEW.transaction_id
   AND (l.world_id<>NEW.world_id OR l.runtime_scope_kind<>2 OR l.channel_id IS NOT NULL
    OR l.instance_id IS DISTINCT FROM NEW.instance_id OR l.created_xact_id<>pg_current_xact_id()
    OR l.definition_family<>'Item' OR l.definition_revision<>'definition-r1'
    OR l.map_revision<>'sha256:'||encode(NEW.creature_map_digest,'hex')
    OR l.content_revision<>'sha256:'||encode(NEW.catalog_digest,'hex')
    OR l.content_generation_digest<>NEW.catalog_digest OR l.placement_context<>NEW.creature_frame_digest
    OR (refresh AND NOT EXISTS (SELECT 1 FROM generate_series(32492,32500,2) x CROSS JOIN generate_series(32652,32660,2) y
     WHERE l.spatial_position=int4send(x)||int4send(y)||int2send(15::smallint)))
    OR (NOT refresh AND NOT (
      octet_length(l.spatial_position)=10 AND substring(l.spatial_position from 9)=int2send(15::smallint)
      AND substring(l.spatial_position from 1 for 4) BETWEEN int4send(32487) AND int4send(32504)
      AND substring(l.spatial_position from 5 for 4) BETWEEN int4send(32649) AND int4send(32664)))
    OR NOT (
     (l.operation_kind=1 AND l.definition_production_key='oteryn:item.tibia.i2118'
       AND l.quantity_before=0 AND l.quantity_after=1 AND l.destination_parent_item_instance_id IS NULL
       AND NOT l.blocks_movement AND NOT l.blocks_projectile AND l.expires_at_unix_ms IS NULL
       AND EXISTS (SELECT 1 FROM game_spell_field_temporal_schedules s
        WHERE s.item_instance_id=l.item_instance_id AND s.source_transaction_id=l.transaction_id
         AND s.source_ordinal=l.ordinal AND s.source_stage=1 AND s.created_xact_id=l.created_xact_id))
     OR (l.operation_kind=2 AND l.definition_production_key IN
       ('oteryn:item.tibia.i2118','oteryn:item.tibia.i2119','oteryn:item.tibia.i2120')
       AND l.quantity_before=1 AND l.quantity_after=0 AND l.source_parent_item_instance_id IS NULL)
    ) IS TRUE)
 ) OR (refresh AND EXISTS (
  SELECT 1 FROM game_spell_item_lines l WHERE l.transaction_id=NEW.transaction_id
   GROUP BY l.spatial_position HAVING count(*) FILTER(WHERE l.operation_kind=1)<>1 OR count(*)>4
 )) OR EXISTS (
  SELECT 1 FROM game_spell_item_lines l WHERE l.transaction_id=NEW.transaction_id AND l.operation_kind=2
   GROUP BY l.spatial_position,l.definition_production_key HAVING count(*)>1
 )
 THEN RAISE EXCEPTION 'Herald field callback requires its exact current scoped Ground batch' USING ERRCODE='23514'; END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_herald_field_source_proven AFTER INSERT ON game_spell_item_receipts
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_herald_field_source_proven();

REVOKE ALL ON FUNCTION game_herald_field_source_shape(game_spell_item_receipts),game_herald_field_source_proven() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_herald_field_source_shape(game_spell_item_receipts),game_herald_field_source_proven() TO oteryn_game_runtime,oteryn_game_control;

-- The existing Instance deadline owner also transforms the exact Herald field chain.
-- The same source proof is used by a player STEP when it decodes the current stage.
CREATE FUNCTION game_herald_field_ground_source(
 p_item UUID,p_world UUID,p_instance UUID,p_generation NUMERIC,p_stack BIGINT
) RETURNS BOOLEAN LANGUAGE sql STABLE STRICT SET search_path=pg_catalog,public,pg_temp AS $$
 SELECT EXISTS (
  SELECT 1 FROM game_item_ground_locations g
  JOIN game_item_instances i USING(item_instance_id)
  JOIN game_spell_item_lines l ON l.item_instance_id=i.item_instance_id
   AND l.operation_kind=1 AND l.transaction_id=i.minted_transaction_id
  JOIN game_spell_item_receipts r ON r.transaction_id=l.transaction_id
  JOIN game_spell_item_audit_outbox a ON a.transaction_id=r.transaction_id AND a.event_id=r.event_id
   AND a.envelope=r.intent AND a.envelope_sha256=r.binding AND a.created_xact_id=r.created_xact_id
  JOIN game_runtime_scope_assignments s ON s.scope_key='\x03'::bytea||uuid_send(p_world)||uuid_send(p_instance)
   AND s.state=1 AND s.ownership_generation=p_generation
  WHERE g.item_instance_id=p_item AND g.scope_kind=2 AND g.channel_id IS NULL
   AND (g.world_id,g.instance_id,g.runtime_scope_ownership_generation,g.stack_ordinal)=(p_world,p_instance,p_generation,p_stack)
   AND (r.world_id,r.instance_id,r.ownership_generation)=(p_world,p_instance,p_generation)
   AND (l.world_id,l.instance_id,l.spatial_position,l.map_revision,l.content_revision,l.placement_context)=
       (g.world_id,g.instance_id,g.spatial_position,g.map_revision,g.content_revision,g.native_room_placement_context)
   AND l.runtime_scope_kind=2 AND l.channel_id IS NULL AND l.created_xact_id=r.created_xact_id
   AND l.definition_production_key='oteryn:item.tibia.i2118' AND l.content_generation_digest=r.catalog_digest
   AND game_herald_field_source_shape(r)
 );
$$;
REVOKE ALL ON FUNCTION game_herald_field_ground_source(UUID,UUID,UUID,NUMERIC,BIGINT) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_herald_field_ground_source(UUID,UUID,UUID,NUMERIC,BIGINT) TO oteryn_game_runtime,oteryn_game_control;

DO $$ DECLARE body TEXT; f TEXT; row_name TEXT; current_guard TEXT; herald_guard TEXT; BEGIN
 FOREACH f IN ARRAY ARRAY['game_spell_field_temporal_capture','game_spell_field_temporal_receipt_proven'] LOOP
  row_name:=CASE WHEN f='game_spell_field_temporal_capture' THEN 'r' ELSE 'NEW' END;
  body:=pg_get_functiondef((f||'()')::regprocedure);
  current_guard:=format('game_instance_corpse_stage_current(g.item_instance_id,%1$s.world_id,%1$s.instance_id,%1$s.ownership_generation,%1$s.source_stack_ordinal)',row_name);
  herald_guard:=replace(current_guard,'game_instance_corpse_stage_current','game_herald_field_ground_source');
  IF position(current_guard IN body)=0 THEN RAISE EXCEPTION 'Herald field custody predicate changed: %',f; END IF;
  EXECUTE replace(body,current_guard,format('(%s OR %s)',current_guard,herald_guard));
 END LOOP;
END $$;
