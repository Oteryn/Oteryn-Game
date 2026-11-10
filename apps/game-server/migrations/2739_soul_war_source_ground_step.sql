-- Literal Item-id MoveEvents share the existing physical Ground owner. Class5
-- remains Feroxa; class6 has no invented spell, command, cost or Creature minter.
DO $$ DECLARE prior TEXT; actual INTEGER[]; name TEXT; arm TEXT; BEGIN
 SELECT pg_get_constraintdef(oid) INTO STRICT prior FROM pg_constraint
  WHERE conrelid='game_spell_item_receipts'::regclass AND conname='game_spell_item_receipts_cause_kind_check';
 SELECT array_agg(n ORDER BY n) INTO actual FROM (
  SELECT DISTINCT trim(part)::INTEGER n FROM regexp_matches(prior,
   'cause_kind[[:space:]]*=[[:space:]]*ANY[[:space:]]*\([[:space:]]*ARRAY\[([^]]+)\]','g') AS found(value)
  CROSS JOIN LATERAL regexp_split_to_table(value[1],',') part
 ) values_of_cause;
 IF actual IS DISTINCT FROM ARRAY[0,1,2,3,4,5] THEN RAISE EXCEPTION 'source Ground STEP cause registry changed: %',actual; END IF;
 ALTER TABLE game_spell_item_receipts DROP CONSTRAINT game_spell_item_receipts_cause_kind_check;
 ALTER TABLE game_spell_item_receipts ADD CONSTRAINT game_spell_item_receipts_cause_kind_check CHECK(cause_kind IN(0,1,2,3,4,5,6));
 arm:=$arm$(cause_kind=6 AND spell_family='Encounter'
  AND spell_production_key='oteryn:encounter.soul_war_taint_zones' AND spell_revision='definition-r1'
  AND ((runtime_scope_kind=1 AND channel_id IS NOT NULL AND instance_id IS NULL)
    OR (runtime_scope_kind=2 AND channel_id IS NULL AND instance_id IS NOT NULL AND game_node_is_uuid_v7(instance_id)))
  AND operation_count BETWEEN 0 AND 1 AND swing_ordinal IS NULL
  AND num_nonnulls(game_session_id,command_id,character_id,cost,cost_binding,caster_lease_generation,caster_placement_digest,
    creature_placement,creature_actor_generation,creature_definition_key,creature_definition_revision,
    feroxa_firing_id,feroxa_phase,feroxa_reset_epoch)=0
  AND creature_occurrence='source-ground-step:'||encode(creature_cast_digest,'hex')
  AND octet_length(creature_map_digest)=32 AND creature_map_digest<>decode(repeat('00',32),'hex')
  AND octet_length(creature_frame_digest)=32 AND creature_frame_digest<>decode(repeat('00',32),'hex')
  AND creature_source_body_digest=decode('9e1107dc41abc16b4f56e795180c1c17c917d8e73754547c443f070ed3167916','hex')
  AND octet_length(creature_cast_digest)=32 AND creature_cast_digest<>decode(repeat('00',32),'hex'))$arm$;
 FOREACH name IN ARRAY ARRAY['game_spell_item_typed_cause','game_spell_item_exact_runtime_scope'] LOOP
  SELECT pg_get_constraintdef(oid) INTO STRICT prior FROM pg_constraint
   WHERE conrelid='game_spell_item_receipts'::regclass AND conname=name;
  IF left(prior,6)<>'CHECK ' THEN RAISE EXCEPTION 'source Ground STEP predecessor constraint changed'; END IF;
  EXECUTE format('ALTER TABLE game_spell_item_receipts DROP CONSTRAINT %I',name);
  EXECUTE format('ALTER TABLE game_spell_item_receipts ADD CONSTRAINT %I CHECK ((%s OR %s) IS TRUE)',name,substr(prior,7),arm);
 END LOOP;
END $$;
CREATE UNIQUE INDEX game_spell_source_ground_step_once ON game_spell_item_receipts
 (world_id,runtime_scope_kind,coalesce(channel_id,instance_id),ownership_generation,creature_occurrence)
 WHERE cause_kind=6;

CREATE FUNCTION game_spell_source_ground_step_proven() RETURNS TRIGGER
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE p JSONB; step JSONB; session game_durability_reconnect_sessions%ROWTYPE;
BEGIN
 IF NEW.cause_kind<>6 THEN RETURN NULL; END IF;
 p:=convert_from(NEW.intent,'UTF8')::jsonb; step:=p->'source_step';
 IF (p->>'schema'='OTERYN_SOURCE_GROUND_STEP/v1'
  AND p->'player'='null'::jsonb AND p->'cost'='null'::jsonb AND p->'caster_origin'='null'::jsonb AND p->'creature'='null'::jsonb
  AND p->>'world'=replace(NEW.world_id::text,'-','')
  AND p->>'runtime_scope_kind'=NEW.runtime_scope_kind::text
  AND p->>'channel' IS NOT DISTINCT FROM replace(NEW.channel_id::text,'-','')
  AND p->>'instance' IS NOT DISTINCT FROM replace(NEW.instance_id::text,'-','')
  AND p->>'ownership_generation'=NEW.ownership_generation::text
  AND p->'ability'=jsonb_build_object('family',NEW.spell_family,'key',NEW.spell_production_key,'revision',NEW.spell_revision)
  AND p->>'occurrence'=NEW.creature_occurrence AND p->>'source_cast'=encode(NEW.creature_cast_digest,'hex')
  AND p->>'content'=encode(NEW.catalog_digest,'hex') AND p->>'map'=encode(NEW.creature_map_digest,'hex')
  AND p->>'frame'=encode(NEW.creature_frame_digest,'hex') AND p->>'source_body'=encode(NEW.creature_source_body_digest,'hex')
  AND p->>'client_content' ~ '^[0-9a-f]{64}$' AND p->>'client_content'<>repeat('0',64)
  AND p->>'activation_sequence' ~ '^[1-9][0-9]{0,19}$'
  AND step ?& ARRAY['actor_kind','actor','actor_generation','character','session','connection','lease','movement_revision','translation_revision','movement_binding','previous_revision',
   'node','node_registration_revision','assignment_revision','item','definition','epoch','position','map_revision','content_revision','placement_context']
  AND step->>'actor_kind'='player' AND step->>'actor' ~ '^[0-9a-f]{32}$' AND step->>'actor'<>repeat('0',32)
  AND step->>'actor_generation' ~ '^[1-9][0-9]{0,19}$'
  AND step->>'character' ~ '^[0-9a-f]{32}$' AND step->>'session' ~ '^[0-9a-f]{32}$'
  AND step->>'connection' ~ '^[1-9][0-9]{0,19}$' AND step->>'lease' ~ '^[1-9][0-9]{0,19}$'
  AND step->>'movement_revision' ~ '^[1-9][0-9]{0,19}$' AND step->>'translation_revision'=step->>'movement_revision'
  AND step->>'movement_binding' ~ '^[0-9a-f]{64}$' AND step->>'movement_binding'<>repeat('0',64)
  AND step->>'previous_revision' ~ '^(0|[1-9][0-9]{0,19})$'
  AND step->>'node' ~ '^[0-9a-f]{32}$' AND step->>'node_registration_revision' ~ '^[1-9][0-9]{0,19}$'
  AND step->>'assignment_revision' ~ '^[1-9][0-9]{0,19}$' AND step->>'item' ~ '^[0-9a-f]{32}$'
  AND step->>'epoch' ~ '^(0|[1-9][0-9]{0,19})$'
  AND step->'definition'->>'family'='Item' AND step->'definition'->>'revision'='definition-r1'
  AND step->'definition'->>'key' IN('oteryn:item.tibia.i33876','oteryn:item.tibia.i33949','oteryn:item.tibia.i33984')
  AND step->>'map_revision'='sha256:'||encode(NEW.creature_map_digest,'hex')
  AND step->>'content_revision'='sha256:'||encode(NEW.catalog_digest,'hex')
  AND step->'placement_context'=to_jsonb(ARRAY(SELECT get_byte(NEW.creature_frame_digest,n) FROM generate_series(0,31)n))
  AND jsonb_typeof(step->'position')='array' AND jsonb_array_length(step->'position')=10
  AND jsonb_typeof(p->'operations')='array' AND jsonb_array_length(p->'operations')=NEW.operation_count
 ) IS NOT TRUE THEN RAISE EXCEPTION 'source Ground STEP requires its exact original movement and Item proof' USING ERRCODE='23514'; END IF;
 IF NOT game_character_is_uuid_v7((step->>'item')::uuid) OR NOT game_character_is_uuid_v7((step->>'session')::uuid)
  OR NOT game_character_is_uuid_v7((step->>'character')::uuid) OR NOT game_node_is_uuid_v7((step->>'node')::uuid)
  OR (step->>'movement_revision')::numeric>18446744073709551615
  OR (step->>'previous_revision')::numeric>=(step->>'movement_revision')::numeric
  OR (step->>'actor_generation')::numeric>18446744073709551615
  OR NOT game_world_reset_map_epoch_current(NEW.world_id,(step->>'epoch')::numeric)
  OR game_runtime_scope_assignment_history_valid() IS NOT TRUE OR game_node_registration_history_valid() IS NOT TRUE
 THEN RAISE EXCEPTION 'source Ground STEP current owner/reset proof' USING ERRCODE='23514'; END IF;
 SELECT * INTO session FROM game_durability_reconnect_sessions s
 WHERE s.game_session_id=(step->>'session')::uuid AND s.character_id=(step->>'character')::uuid
  AND s.world_id=NEW.world_id AND s.runtime_scope_world_id=NEW.world_id AND s.runtime_scope_house_key IS NULL
  AND s.session_state IN(1,2) AND s.current_generation=(step->>'connection')::numeric
  AND s.character_lease_generation=(step->>'lease')::numeric AND s.scope_ownership_generation=NEW.ownership_generation
  AND s.runtime_scope_kind=NEW.runtime_scope_kind AND s.runtime_scope_channel_id IS NOT DISTINCT FROM NEW.channel_id
  AND s.runtime_scope_instance_id IS NOT DISTINCT FROM NEW.instance_id;
 IF NOT FOUND
  OR (NEW.runtime_scope_kind=2 AND game_activity_session_origin_valid(session.game_session_id,session.character_id) IS NOT TRUE)
  OR NOT EXISTS(SELECT 1 FROM game_durability_admission_character_guards c JOIN game_durability_admission_account_guards a USING(account_id)
   WHERE c.character_id=session.character_id AND c.account_id=session.account_id AND c.world_id=NEW.world_id
    AND c.eligible AND c.lease_generation=session.character_lease_generation AND c.holder_game_session_id=session.game_session_id
    AND a.presence_character_id=c.character_id AND a.holder_game_session_id=c.holder_game_session_id)
  OR NOT EXISTS(SELECT 1 FROM game_runtime_scope_assignments a JOIN game_node_registrations n ON n.node_id=a.holder_node_id
   JOIN game_durability_admission_runtime_guards ready ON ready.scope_key=set_byte(a.scope_key,0,NEW.runtime_scope_kind)
   WHERE a.world_id=NEW.world_id AND a.state=1 AND a.house_key IS NULL AND a.ownership_generation=NEW.ownership_generation
    AND a.holder_node_id=(step->>'node')::uuid AND a.holder_registration_revision=(step->>'node_registration_revision')::numeric
    AND a.source_revision=(step->>'assignment_revision')::numeric AND n.state=1 AND n.registration_revision=a.holder_registration_revision
    AND ready.ready AND ready.ownership_generation=NEW.ownership_generation
    AND ((NEW.runtime_scope_kind=1 AND a.scope_kind=1 AND a.channel_id=NEW.channel_id AND a.activity_instance_id IS NULL)
      OR (NEW.runtime_scope_kind=2 AND a.scope_kind=3 AND a.channel_id IS NULL AND a.activity_instance_id=NEW.instance_id)))
  OR (NEW.runtime_scope_kind=1 AND NOT EXISTS(SELECT 1 FROM game_content_activations c WHERE c.world_id=NEW.world_id
   AND c.activation_sequence=(p->>'activation_sequence')::numeric AND c.server_artifact_digest=NEW.catalog_digest
   AND c.client_artifact_digest=decode(p->>'client_content','hex')
   AND c.channel_id=NEW.channel_id AND
    (c.frame_binding_digest=NEW.creature_frame_digest OR EXISTS(
     SELECT 1 FROM game_world_reset_active active JOIN game_world_reset_epochs epoch USING(world_id,reset_epoch)
     WHERE active.world_id=NEW.world_id AND active.reset_epoch=(step->>'epoch')::numeric
      AND epoch.server_digest=NEW.catalog_digest AND epoch.client_digest=c.client_artifact_digest
      AND NEW.creature_map_digest=sha256(convert_to(epoch.map_revision,'UTF8'))
      AND game_elemental_placement_frame_matches(NEW.world_id,epoch.map_revision,NEW.creature_frame_digest,c.frame_binding_digest)))
   AND NOT EXISTS(SELECT 1 FROM game_content_activations later WHERE later.world_id=c.world_id AND later.channel_id=c.channel_id
    AND later.activation_sequence>c.activation_sequence)))
  -- An accepted Instance retains its entry pins after the source Channel
  -- activates new content. Current destination authority is the exact live
  -- runtime entry; its initiating participant identifies the historical
  -- source Channel activation that supplied the retained client/sequence.
  OR (NEW.runtime_scope_kind=2 AND NOT EXISTS(
   SELECT 1 FROM game_activity_handoff_groups h
   JOIN game_activity_runtime_entries e USING(handoff_id)
   JOIN game_activity_handoff_participants origin ON origin.handoff_id=h.handoff_id
    AND origin.character_id=e.initiating_character_id
   JOIN game_activity_handoff_participants participant ON participant.handoff_id=h.handoff_id
    AND participant.character_id=session.character_id AND participant.destination_game_session_id=session.game_session_id
   JOIN game_content_activations c ON c.world_id=h.world_id AND c.channel_id=origin.origin_channel_id
   WHERE h.world_id=NEW.world_id AND h.instance_id=NEW.instance_id
    AND h.ownership_generation=NEW.ownership_generation AND h.state=2
    AND e.holder_node_id=(step->>'node')::uuid AND e.holder_registration_revision=(step->>'node_registration_revision')::numeric
    AND e.destination_server_artifact_digest=NEW.catalog_digest
    AND e.destination_map_revision_digest=NEW.creature_map_digest
    AND e.destination_coordinate_frame_digest=NEW.creature_frame_digest
    AND origin.server_artifact_digest=NEW.catalog_digest
    AND c.activation_sequence=(p->>'activation_sequence')::numeric
    AND c.server_artifact_digest=NEW.catalog_digest AND c.client_artifact_digest=decode(p->>'client_content','hex')
    AND NOT EXISTS(SELECT 1 FROM game_activity_runtime_returns returned
     WHERE returned.handoff_id=participant.handoff_id AND returned.character_id=participant.character_id)))
 THEN RAISE EXCEPTION 'source Ground STEP requires its independently current session/scope/content' USING ERRCODE='23514'; END IF;

 IF (SELECT count(*) FROM game_spell_item_lines WHERE transaction_id=NEW.transaction_id)<>NEW.operation_count
  OR EXISTS(SELECT 1 FROM game_spell_item_lines l WHERE l.transaction_id=NEW.transaction_id AND (
   l.created_xact_id<>NEW.created_xact_id OR l.created_xact_id<>pg_current_xact_id() OR l.ordinal<>1 OR l.operation_kind<>2
   OR (l.world_id,l.runtime_scope_kind,l.channel_id,l.instance_id) IS DISTINCT FROM (NEW.world_id,NEW.runtime_scope_kind,NEW.channel_id,NEW.instance_id)
   OR l.item_instance_id<>(step->>'item')::uuid OR l.definition_family<>'Item'
   OR l.definition_production_key<>step->'definition'->>'key' OR l.definition_revision<>'definition-r1'
   OR l.quantity_before<>1 OR l.quantity_after<>0 OR l.state_revision_before<1 OR l.source_stack_ordinal<1
   OR to_jsonb(ARRAY(SELECT get_byte(l.spatial_position,n) FROM generate_series(0,octet_length(l.spatial_position)-1)n))<>step->'position'
   OR l.map_revision<>step->>'map_revision' OR l.content_revision<>step->>'content_revision'
   OR l.placement_context<>NEW.creature_frame_digest OR l.content_generation_digest<>NEW.catalog_digest
   OR l.blocks_movement OR l.blocks_projectile OR l.immovable_block_solid OR l.expires_at_unix_ms IS NOT NULL
   OR num_nonnulls(l.destination_parent_item_instance_id,l.source_parent_item_instance_id,l.source_equipment_slot,l.source_corpse_root)<>0
   OR p->'operations'->0->>'kind' IS DISTINCT FROM 'remove_ground'
   OR p->'operations'->0->'id' IS DISTINCT FROM to_jsonb(ARRAY(SELECT get_byte(uuid_send(l.item_instance_id),n) FROM generate_series(0,15)n))
   OR p->'operations'->0->'definition' IS DISTINCT FROM step->'definition'
   OR p->'operations'->0->>'quantity' IS DISTINCT FROM '1'
   OR p->'operations'->0->>'revision' IS DISTINCT FROM l.state_revision_before::text
   OR p->'operations'->0->>'ordinal' IS DISTINCT FROM l.source_stack_ordinal::text
   OR p->'operations'->0->'placement'->'position' IS DISTINCT FROM step->'position'
   OR p->'operations'->0->'placement'->>'map_revision' IS DISTINCT FROM l.map_revision
   OR p->'operations'->0->'placement'->>'content_revision' IS DISTINCT FROM l.content_revision
   OR p->'operations'->0->'placement'->'context' IS DISTINCT FROM step->'placement_context'))
  OR (NEW.operation_count=0 AND EXISTS(SELECT 1 FROM game_item_instances i JOIN game_item_ground_locations g USING(item_instance_id)
   WHERE i.item_instance_id=(step->>'item')::uuid AND i.lifecycle=1 AND g.world_id=NEW.world_id
    AND g.scope_kind=NEW.runtime_scope_kind AND g.channel_id IS NOT DISTINCT FROM NEW.channel_id AND g.instance_id IS NOT DISTINCT FROM NEW.instance_id
    AND to_jsonb(ARRAY(SELECT get_byte(g.spatial_position,n) FROM generate_series(0,octet_length(g.spatial_position)-1)n))=step->'position'
    AND g.map_revision=step->>'map_revision' AND g.content_revision=step->>'content_revision' AND g.native_room_placement_context=NEW.creature_frame_digest))
  OR NOT EXISTS(SELECT 1 FROM game_spell_item_audit_outbox a WHERE a.transaction_id=NEW.transaction_id
   AND a.created_xact_id=NEW.created_xact_id AND a.event_id=NEW.event_id AND a.envelope=NEW.intent
   AND a.envelope_sha256=NEW.binding AND a.occurred_at_unix_ms=NEW.occurred_at_unix_ms)
 THEN RAISE EXCEPTION 'source Ground STEP requires its exact same-xact original Item removal/audit' USING ERRCODE='23514'; END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_spell_source_ground_step_proven AFTER INSERT ON game_spell_item_receipts
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_spell_source_ground_step_proven();
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_spell_source_ground_step_proven() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_spell_source_ground_step_proven() FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
