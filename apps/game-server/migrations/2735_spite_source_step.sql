-- Crystal 201189f soul_war_mechanics.lua:434-480. A sealed committed Player
-- translation consumes exactly one CURRENT i33877 at a registered anchor and
-- writes its existing Quest clock in the same transaction. SourceStep is the
-- Rust owner's deterministic UUIDv8; existing command/death causes stay UUIDv7.
-- committed_at remains statement time in milliseconds. The Quest owner's
-- sampled clock in seconds is bound through the immutable consumption below.
CREATE FUNCTION game_spite_source_step_receipt_shape(r game_character_quest_receipts)
RETURNS BOOLEAN LANGUAGE sql IMMUTABLE AS $$
 SELECT coalesce(r.cause_kind='source_step' AND r.cause_ordinal=0
  AND get_byte(uuid_send(r.cause_id),6)>>4=8
  AND (get_byte(uuid_send(r.cause_id),8)&192)=128
  AND r.quest_key='oteryn:quest/soul-war/source-items'
  AND r.transition_key='oteryn:quest-transition/soul-war/source-items/spite-fire'
  AND NOT r.completes AND octet_length(r.request_binding)=33 AND get_byte(r.request_binding,0)=1
  AND r.track_keys=ARRAY['oteryn:quest-progress/soul-war/goshnar-spite-fire']
  AND cardinality(r.values_before)=1 AND cardinality(r.values_after)=1
  AND r.values_before[1]>=0 AND r.values_after[1]::numeric>=r.values_before[1]::numeric+56
  AND r.level_after=r.level_before AND r.experience_after=r.experience_before,false)
$$;

DO $migration$
DECLARE name TEXT; prior TEXT; arm TEXT;
BEGIN
 arm:=$predicate$(cause_kind='source_step' AND cause_ordinal=0
  AND get_byte(uuid_send(cause_id),6)>>4=8 AND (get_byte(uuid_send(cause_id),8)&192)=128
  AND quest_key='oteryn:quest/soul-war/source-items'
  AND transition_key='oteryn:quest-transition/soul-war/source-items/spite-fire'
  AND NOT completes AND octet_length(request_binding)=33 AND get_byte(request_binding,0)=1
  AND track_keys=ARRAY['oteryn:quest-progress/soul-war/goshnar-spite-fire']
  AND cardinality(values_before)=1 AND cardinality(values_after)=1
  AND values_before[1]>=0 AND values_after[1]::numeric>=values_before[1]::numeric+56
  AND level_after=level_before AND experience_after=experience_before)$predicate$;
 FOREACH name IN ARRAY ARRAY['game_character_quest_receipts_cause_kind_check',
   'game_character_quest_receipt_cause_shape','game_character_quest_receipts_cause_id_check'] LOOP
  SELECT pg_get_constraintdef(oid) INTO STRICT prior FROM pg_constraint
   WHERE conrelid='game_character_quest_receipts'::regclass AND conname=name;
  IF left(prior,6)<>'CHECK ' OR strpos(prior,'source_step')<>0 THEN
   RAISE EXCEPTION 'unexpected prior Quest source-step constraint: %',name;
  END IF;
  EXECUTE format('ALTER TABLE game_character_quest_receipts DROP CONSTRAINT %I',name);
  EXECUTE format('ALTER TABLE game_character_quest_receipts ADD CONSTRAINT %I CHECK (((cause_kind<>''source_step'' AND %s) OR %s) IS TRUE)',name,substr(prior,7),arm);
 END LOOP;
END $migration$;

ALTER TABLE game_quest_callback_consumptions
 DROP CONSTRAINT game_quest_callback_consumptions_cause_ordinal_check,
 ADD CONSTRAINT game_quest_callback_consumptions_cause_ordinal_check CHECK (
  cause_ordinal BETWEEN 1 AND 18446744073709551615 OR
  (cause_ordinal=0 AND transition_key='oteryn:quest-transition/soul-war/source-items/spite-fire'
   AND get_byte(uuid_send(cause_id),6)>>4=8 AND (get_byte(uuid_send(cause_id),8)&192)=128
   AND location_kind='ground' AND replacement IS NULL AND quantity=1));

-- These are current authority reads, independent of the immutable proof's
-- expected identities. The Rust transaction has already taken the same scope,
-- recovery, admission and Character locks in canonical order.
CREATE FUNCTION game_spite_source_step_input_matches(
 r game_quest_callback_consumptions, i game_item_instances, g game_item_ground_locations)
RETURNS BOOLEAN LANGUAGE plpgsql VOLATILE AS $$
DECLARE p JSONB; session_row game_durability_reconnect_sessions%ROWTYPE;
BEGIN
 IF r.cause_ordinal<>0 OR r.transition_key<>'oteryn:quest-transition/soul-war/source-items/spite-fire' THEN RETURN false; END IF;
 p:=convert_from(r.proof,'UTF8')::jsonb;
 IF NOT coalesce(r.cause_ordinal=0
   AND r.transition_key='oteryn:quest-transition/soul-war/source-items/spite-fire'
   AND get_byte(uuid_send(r.cause_id),6)>>4=8 AND (get_byte(uuid_send(r.cause_id),8)&192)=128
   AND r.location_kind='ground' AND r.before_location IS NULL AND r.replacement IS NULL
   AND r.quantity=1 AND i.quantity=1 AND i.lifecycle=1
   AND i.item_instance_id=r.item_instance_id AND g.item_instance_id=i.item_instance_id
   AND to_jsonb(i)=r.before_item AND to_jsonb(g)=r.before_ground
   AND i.world_id=g.world_id AND i.definition_family='Item'
   AND i.definition_production_key='oteryn:item.tibia.i33877' AND i.definition_revision_ref='definition-r1'
   AND p->>'schema'='OTERYN_SPITE_SOURCE_STEP/v1'
   AND jsonb_typeof(p->'quest_clock_seconds')='number'
   AND p->>'source_body'='9e1107dc41abc16b4f56e795180c1c17c917d8e73754547c443f070ed3167916'
   AND p->>'character'=r.character_id::text AND p->>'item'=i.item_instance_id::text
   AND p->>'cause'=r.cause_id::text AND p->>'world'=i.world_id::text
   AND p->>'scope_kind'=g.scope_kind::text
   AND p->>'scope'=coalesce(g.channel_id,g.instance_id)::text
   AND p->>'scope_generation'=g.runtime_scope_ownership_generation::text
   AND p->>'server' ~ '^[0-9a-f]{64}$' AND p->>'client' ~ '^[0-9a-f]{64}$'
   AND p->>'frame' ~ '^[0-9a-f]{64}$' AND p->>'map' ~ '^[0-9a-f]{64}$'
   AND p->>'actor' ~ '^[0-9a-f]{32}$' AND p->>'request_binding' ~ '^01[0-9a-f]{64}$'
   AND p->>'map_revision'='sha256:'||(p->>'map') AND p->>'map_revision'=g.map_revision
   AND p->>'content_revision'='sha256:'||(p->>'server') AND p->>'content_revision'=g.content_revision
   AND p->>'placement_context'=p->>'frame'
   AND p->>'placement_context'=encode(g.native_room_placement_context,'hex')
   AND p->>'spatial'=encode(g.spatial_position,'hex')
   AND p->'position' IN ('[33743,31628,14]'::jsonb,'[33736,31632,14]'::jsonb,
                       '[33750,31632,14]'::jsonb,'[33742,31637,14]'::jsonb)
   AND g.spatial_position=int4send((p->'position'->>0)::int)||int4send((p->'position'->>1)::int)||int2send(14::smallint)
   AND (p->>'movement_revision')::numeric BETWEEN 1 AND 18446744073709551615
   AND p->>'translation_revision'=p->>'movement_revision'
   AND (p->>'actor_generation')::numeric BETWEEN 1 AND 18446744073709551615
   AND (p->>'reset_epoch')::numeric=coalesce((SELECT reset_epoch FROM game_world_reset_active WHERE world_id=i.world_id),0)
   AND NOT EXISTS(SELECT 1 FROM game_item_keys WHERE item_instance_id=i.item_instance_id)
   AND NOT EXISTS(SELECT 1 FROM game_item_timed_states WHERE item_instance_id=i.item_instance_id)
   AND NOT EXISTS(SELECT 1 FROM game_item_texts WHERE item_instance_id=i.item_instance_id)
   AND NOT EXISTS(SELECT 1 FROM game_item_imbuements WHERE item_instance_id=i.item_instance_id)
   AND NOT EXISTS(SELECT 1 FROM game_item_container_entries WHERE parent_item_instance_id=i.item_instance_id)
   AND NOT EXISTS(SELECT 1 FROM game_item_nested_entries WHERE parent_item_instance_id=i.item_instance_id)
   AND NOT EXISTS(SELECT 1 FROM game_item_corpse_container_entries WHERE parent_item_instance_id=i.item_instance_id)
   -- The source getItemById chooses the first current stack member, not a
   -- retained GlobalEvent's old UUID and not every matching field on the tile.
   AND NOT EXISTS(SELECT 1 FROM game_item_ground_locations other JOIN game_item_instances item USING(item_instance_id)
     WHERE other.world_id=g.world_id AND other.scope_kind=g.scope_kind
      AND other.channel_id IS NOT DISTINCT FROM g.channel_id AND other.instance_id IS NOT DISTINCT FROM g.instance_id
      AND other.spatial_position=g.spatial_position AND other.map_revision=g.map_revision
      AND other.content_revision=g.content_revision AND other.native_room_placement_context=g.native_room_placement_context
      AND other.runtime_scope_ownership_generation=g.runtime_scope_ownership_generation
      AND other.stack_ordinal>g.stack_ordinal AND item.lifecycle=1
      AND item.definition_family='Item' AND item.definition_production_key='oteryn:item.tibia.i33877'
      AND item.definition_revision_ref='definition-r1'),false) THEN RETURN false; END IF;

 SELECT * INTO session_row FROM game_durability_reconnect_sessions s
 WHERE s.game_session_id=(p->>'session')::uuid AND s.character_id=r.character_id
  AND s.world_id=i.world_id AND s.runtime_scope_world_id=i.world_id
  AND s.runtime_scope_kind=g.scope_kind
  AND s.runtime_scope_channel_id IS NOT DISTINCT FROM g.channel_id
  AND s.runtime_scope_instance_id IS NOT DISTINCT FROM g.instance_id
  AND s.runtime_scope_house_key IS NULL AND s.session_state IN(1,2)
  AND s.current_generation=(p->>'connection')::numeric
  AND s.character_lease_generation=(p->>'lease')::numeric
  AND s.scope_ownership_generation=g.runtime_scope_ownership_generation;
 IF NOT FOUND THEN RETURN false; END IF;
 RETURN coalesce(
  (g.scope_kind=1 OR game_activity_session_origin_valid(session_row.game_session_id,r.character_id))
  AND game_runtime_scope_assignment_history_valid() AND game_node_registration_history_valid()
  AND EXISTS(SELECT 1 FROM game_durability_admission_character_guards c
   JOIN game_durability_admission_account_guards a ON a.account_id=c.account_id
   WHERE c.character_id=r.character_id AND c.account_id=session_row.account_id AND c.world_id=i.world_id
    AND c.eligible AND c.lease_generation=session_row.character_lease_generation
    AND c.holder_game_session_id=session_row.game_session_id
    AND a.presence_character_id=c.character_id AND a.holder_game_session_id=c.holder_game_session_id)
  AND EXISTS(SELECT 1 FROM game_runtime_scope_assignments a
   JOIN game_node_registrations n ON n.node_id=a.holder_node_id
   JOIN game_durability_admission_runtime_guards ready
    ON ready.scope_key=set_byte(a.scope_key,0,g.scope_kind)
   WHERE a.world_id=i.world_id AND a.state=1 AND a.house_key IS NULL
    AND a.ownership_generation=g.runtime_scope_ownership_generation
    AND a.holder_node_id=(p->>'node')::uuid AND a.holder_registration_revision=(p->>'node_registration')::numeric
    AND a.source_revision=(p->>'assignment_revision')::numeric
    AND n.state=1 AND n.registration_revision=a.holder_registration_revision
    AND ready.ready AND ready.ownership_generation=a.ownership_generation
    AND ((g.scope_kind=1 AND a.scope_kind=1 AND a.channel_id=g.channel_id AND a.activity_instance_id IS NULL)
      OR (g.scope_kind=2 AND a.scope_kind=3 AND a.channel_id IS NULL AND a.activity_instance_id=g.instance_id)))
  AND EXISTS(SELECT 1 FROM game_content_activations c WHERE c.world_id=i.world_id
   AND c.activation_sequence=(p->>'activation')::numeric AND c.server_artifact_digest=decode(p->>'server','hex')
   AND c.client_artifact_digest=decode(p->>'client','hex')
   AND (g.scope_kind=2 OR (c.channel_id=g.channel_id AND c.frame_binding_digest=decode(p->>'frame','hex')))
   AND NOT EXISTS(SELECT 1 FROM game_content_activations later WHERE later.world_id=c.world_id
    AND later.channel_id=c.channel_id AND later.activation_sequence>c.activation_sequence)),false);
END $$;

-- Extend only the Ground admit predicate. All prior carried/transform/altar
-- branches and every canonical Item/ground mutation guard remain intact.
DO $migration$
DECLARE body TEXT; needle TEXT:='OR NOT game_item_loop_drop_ref(g.corpse_ref)';
BEGIN
 body:=pg_get_functiondef('game_quest_callback_admit()'::regprocedure);
 IF (length(body)-length(replace(body,needle,'')))/length(needle)<>1 THEN
  RAISE EXCEPTION 'unexpected current Quest Ground admit predicate';
 END IF;
 body:=replace(body,needle,'OR NOT ((NEW.cause_ordinal>0 AND game_item_loop_drop_ref(g.corpse_ref)) OR game_spite_source_step_input_matches(NEW,i,g))');
 EXECUTE body;
END $migration$;

CREATE FUNCTION game_spite_source_step_consumption_matches(
 c game_quest_callback_consumptions,r game_character_quest_receipts)
RETURNS BOOLEAN LANGUAGE sql IMMUTABLE AS $$
 SELECT coalesce(game_spite_source_step_receipt_shape(r)
  AND c.character_id=r.character_id AND c.cause_id=r.cause_id AND c.cause_ordinal=0
  AND c.transition_key=r.transition_key AND c.location_kind='ground' AND c.quantity=1
  AND c.replacement IS NULL AND c.before_location IS NULL
  AND c.before_item->>'definition_family'='Item'
  AND c.before_item->>'definition_production_key'='oteryn:item.tibia.i33877'
  AND c.before_item->>'definition_revision_ref'='definition-r1'
  AND c.before_item->>'quantity'='1' AND c.before_item->>'lifecycle'='1'
  AND c.created_xact_id=r.created_xact_id
  AND convert_from(c.proof,'UTF8')::jsonb->>'schema'='OTERYN_SPITE_SOURCE_STEP/v1'
  AND convert_from(c.proof,'UTF8')::jsonb->'quest_clock_seconds'=to_jsonb(r.values_after[1])
  AND convert_from(c.proof,'UTF8')::jsonb->>'request_binding'=encode(r.request_binding,'hex'),false)
$$;

-- Preserve the complete existing quantity/lifecycle/custody completion proof;
-- add precisely the new receipt kind to its Ground-only receipt predicate.
DO $migration$
DECLARE body TEXT; needle TEXT:='r.cause_kind IN (''use'',''step'')';
BEGIN
 body:=pg_get_functiondef('game_quest_callback_complete()'::regprocedure);
 IF (length(body)-length(replace(body,needle,'')))/length(needle)<>1 THEN
  RAISE EXCEPTION 'unexpected current Quest Ground completion predicate';
 END IF;
 body:=replace(body,needle,'(r.cause_kind IN (''use'',''step'') OR game_spite_source_step_consumption_matches(NEW,r))');
 EXECUTE body;
END $migration$;

-- The reverse edge is mandatory too: neither a naked clock receipt nor two
-- consumed fields can commit for this one original source occurrence.
CREATE FUNCTION game_spite_source_step_atomic() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NEW.cause_kind<>'source_step'
  AND NEW.transition_key<>'oteryn:quest-transition/soul-war/source-items/spite-fire' THEN RETURN NULL; END IF;
 IF NOT game_spite_source_step_receipt_shape(NEW)
  OR NEW.created_xact_id<>pg_current_xact_id()
  OR (SELECT count(*) FROM game_quest_callback_consumptions c
    WHERE c.character_id=NEW.character_id AND c.cause_id=NEW.cause_id
     AND c.cause_ordinal=NEW.cause_ordinal AND c.transition_key=NEW.transition_key)<>1
  OR NOT EXISTS(SELECT 1 FROM game_quest_callback_consumptions c
    WHERE game_spite_source_step_consumption_matches(c,NEW))
 THEN RAISE EXCEPTION 'Spite source STEP requires one atomic current Ground consumption' USING ERRCODE='23514'; END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_spite_source_step_atomic AFTER INSERT ON game_character_quest_receipts
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_spite_source_step_atomic();

DO $migration$
DECLARE name TEXT;
BEGIN
 FOREACH name IN ARRAY ARRAY[
  'game_spite_source_step_receipt_shape(game_character_quest_receipts)',
  'game_spite_source_step_input_matches(game_quest_callback_consumptions,game_item_instances,game_item_ground_locations)',
  'game_spite_source_step_consumption_matches(game_quest_callback_consumptions,game_character_quest_receipts)',
  'game_spite_source_step_atomic()'] LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',name,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC',name);
  EXECUTE format('GRANT EXECUTE ON FUNCTION %s TO oteryn_game_runtime,oteryn_game_control',name);
 END LOOP;
END $migration$;
