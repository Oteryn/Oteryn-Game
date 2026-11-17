-- Crystal201189f soul_war_mechanics.lua579-598,862-911,966-1012:
-- the three literal source USEs use the existing Item Type-2 writer. Sanity's
-- wrong-target branch commits its cooldown but does not consume the source.
DO $$ DECLARE body TEXT; actual INTEGER[]; constraint_name TEXT; BEGIN
 FOREACH constraint_name IN ARRAY ARRAY['game_item_use_records_cause_kind_check','game_item_use_cause_shape'] LOOP
  SELECT pg_get_constraintdef(oid) INTO STRICT body FROM pg_constraint
   WHERE conrelid='game_item_use_records'::regclass AND conname=constraint_name;
  SELECT array_agg(n ORDER BY n) INTO actual FROM (
   SELECT DISTINCT n FROM (
    SELECT trim(part)::INTEGER n FROM regexp_matches(body,
     'cause_kind[[:space:]]*=[[:space:]]*ANY[[:space:]]*\([[:space:]]*ARRAY\[([^]]+)\]','g') AS found(value)
     CROSS JOIN LATERAL regexp_split_to_table(value[1],',') part
    UNION ALL SELECT (value[1])::INTEGER FROM regexp_matches(body,
     'cause_kind[[:space:]]*=[[:space:]]*([0-9]+)','g') AS found(value)
   ) values_of_cause
  ) distinct_causes;
  -- Support the exact fresh predecessor and accepted-main Temple Scroll extension.
  IF (actual IS DISTINCT FROM ARRAY[1,2,3,4,328,470,471,494,495,527,578]
      AND actual IS DISTINCT FROM ARRAY[1,2,3,4,328,470,471,481,494,495,527,578]) OR left(body,6)<>'CHECK ' THEN
   RAISE EXCEPTION 'source USE Item cause registry differs: %',actual;
  END IF;
  EXECUTE format('ALTER TABLE game_item_use_records DROP CONSTRAINT %I',constraint_name);
  EXECUTE format('ALTER TABLE game_item_use_records ADD CONSTRAINT %I CHECK (%s OR (cause_kind=5 AND shape=1))',constraint_name,substr(body,7));
 END LOOP;
END $$;
ALTER TABLE game_item_use_records DROP CONSTRAINT game_rotten_essence_use_scope;
ALTER TABLE game_item_use_records ADD CONSTRAINT game_rotten_essence_use_scope CHECK(
 (activity_instance_id IS NULL AND channel_id IS NOT NULL)
 OR (activity_instance_id IS NOT NULL AND channel_id IS NULL AND source_house_key IS NULL AND cause_kind IN(4,5)));
ALTER TABLE game_item_use_records ADD CONSTRAINT game_soul_war_use_exact_source CHECK(cause_kind<>5 OR (
 source_definition_family='Item' AND source_definition_revision_ref='definition-r1'
 AND source_definition_production_key IN('oteryn:item.tibia.i33793','oteryn:item.tibia.i33891','oteryn:item.tibia.i33950')
 AND shape=1 AND source_kind=1 AND source_parent_item_instance_id IS NOT NULL
 AND source_house_key IS NULL AND flask_item_instance_id IS NULL));

-- Immutable provenance of the existing receipt, not a second progress owner.
-- Target acceptance is derived from the actual typed selection. A caller cannot
-- label a burning USE as a false Sanity target to bypass its reciprocal guard.
CREATE TABLE game_soul_war_source_use_proofs (
 transaction_id UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(transaction_id)),
 event_id UUID NOT NULL UNIQUE CHECK(game_character_is_uuid_v7(event_id) AND event_id<>transaction_id),
 character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
 game_session_id UUID NOT NULL CHECK(game_character_is_uuid_v7(game_session_id)),
 command_id NUMERIC(20,0) NOT NULL CHECK(command_id BETWEEN 1 AND 18446744073709551615),
 source_item_instance_id UUID NOT NULL REFERENCES game_item_instances(item_instance_id),
 source_key TEXT NOT NULL CHECK(source_key IN('oteryn:item.tibia.i33793','oteryn:item.tibia.i33891','oteryn:item.tibia.i33950')),
 source_parent_item_instance_id UUID NOT NULL CHECK(game_character_is_uuid_v7(source_parent_item_instance_id)),
 source_placement_ordinal NUMERIC(20,0) NOT NULL CHECK(source_placement_ordinal BETWEEN 1 AND 18446744073709551615),
 source_quantity_before BIGINT NOT NULL CHECK(source_quantity_before BETWEEN 1 AND 100),
 item_request_binding BYTEA NOT NULL CHECK(octet_length(item_request_binding)=32),
 quest_request_binding BYTEA CHECK(quest_request_binding IS NULL OR (octet_length(quest_request_binding)=33 AND get_byte(quest_request_binding,0)=1)),
 content_revision TEXT NOT NULL CHECK(octet_length(content_revision) BETWEEN 1 AND 512),
 ruleset_revision TEXT NOT NULL CHECK(octet_length(ruleset_revision) BETWEEN 1 AND 512),
 sim_revision TEXT NOT NULL CHECK(octet_length(sim_revision) BETWEEN 1 AND 512),
 world_id UUID NOT NULL CHECK(game_character_is_uuid_v7(world_id)),
 channel_id UUID CHECK(channel_id IS NULL OR game_character_is_uuid_v7(channel_id)),
 activity_instance_id UUID CHECK(activity_instance_id IS NULL OR game_character_is_uuid_v7(activity_instance_id)),
 scope_generation NUMERIC(20,0) NOT NULL CHECK(scope_generation BETWEEN 1 AND 18446744073709551615),
 connection_generation NUMERIC(20,0) NOT NULL CHECK(connection_generation BETWEEN 1 AND 18446744073709551615),
 character_lease_generation NUMERIC(20,0) NOT NULL CHECK(character_lease_generation BETWEEN 1 AND 18446744073709551615),
 node_id UUID NOT NULL CHECK(game_character_is_uuid_v7(node_id)),
 node_registration NUMERIC(20,0) NOT NULL CHECK(node_registration BETWEEN 1 AND 18446744073709551615),
 assignment_revision NUMERIC(20,0) NOT NULL CHECK(assignment_revision BETWEEN 1 AND 18446744073709551615),
 activation_sequence NUMERIC(20,0) NOT NULL CHECK(activation_sequence BETWEEN 1 AND 18446744073709551615),
 server_digest BYTEA NOT NULL CHECK(octet_length(server_digest)=32),
 client_digest BYTEA NOT NULL CHECK(octet_length(client_digest)=32),
 frame_digest BYTEA NOT NULL CHECK(octet_length(frame_digest)=32),
 map_digest BYTEA NOT NULL CHECK(octet_length(map_digest)=32),
 target_kind SMALLINT NOT NULL CHECK(target_kind IN(1,2,3)),
 target_family TEXT,
 target_key TEXT,
 target_revision TEXT,
 target_actor BYTEA,
 target_generation NUMERIC(20,0),
 target_position INTEGER[] NOT NULL CHECK(cardinality(target_position)=3 AND array_ndims(target_position)=1
  AND array_lower(target_position,1)=1 AND array_position(target_position,NULL) IS NULL
  AND target_position[3] BETWEEN -32768 AND 32767),
 accepted BOOLEAN GENERATED ALWAYS AS(coalesce(CASE
  WHEN source_key='oteryn:item.tibia.i33793' THEN target_kind=2 AND target_family='Creature'
   AND target_revision='definition-r1' AND target_key IN('oteryn:creature.ashes_of_burning_hatred',
    'oteryn:creature.spark_of_burning_hatred','oteryn:creature.flame_of_burning_hatred','oteryn:creature.blaze_of_burning_hatred')
  ELSE target_kind=1 AND target_family='Item' AND target_key='oteryn:item.tibia.i33890' AND target_revision='definition-r1'
 END,false)) STORED,
 transition_key TEXT GENERATED ALWAYS AS(CASE source_key
  WHEN 'oteryn:item.tibia.i33891' THEN 'oteryn:quest-transition/soul-war/source-items/greedy-maw'
  WHEN 'oteryn:item.tibia.i33950' THEN 'oteryn:quest-transition/soul-war/source-items/cleansed-sanity' END) STORED,
 occurred_at BIGINT NOT NULL CHECK(occurred_at>0),
 created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id(),
 UNIQUE(game_session_id,command_id),
 UNIQUE(character_id,game_session_id,command_id,transition_key),
 CHECK((channel_id IS NULL)<>(activity_instance_id IS NULL)),
 CHECK((source_key='oteryn:item.tibia.i33793')=(quest_request_binding IS NULL)),
 CHECK(accepted OR source_key='oteryn:item.tibia.i33950'),
 CHECK((target_kind=3 AND target_family IS NULL AND target_key IS NULL AND target_revision IS NULL
   AND target_actor IS NULL AND target_generation IS NULL)
  OR (target_kind IN(1,2) AND target_family IS NOT NULL AND target_family=CASE target_kind WHEN 1 THEN 'Item' ELSE 'Creature' END
   AND target_key IS NOT NULL AND target_key ~ '^oteryn:[A-Za-z0-9._:/-]+$' AND octet_length(target_key)<=512
   AND target_revision IS NOT NULL AND octet_length(target_revision) BETWEEN 1 AND 512
   AND ((target_kind=1 AND target_actor IS NULL AND target_generation IS NULL)
    OR (target_kind=2 AND target_actor IS NOT NULL AND octet_length(target_actor)=16
     AND target_generation IS NOT NULL AND target_generation BETWEEN 1 AND 18446744073709551615)))),
 FOREIGN KEY(character_id,game_session_id,command_id,transition_key)
  REFERENCES game_character_quest_receipts(character_id,cause_id,cause_ordinal,transition_key)
  DEFERRABLE INITIALLY DEFERRED
);
CREATE INDEX game_soul_war_source_use_item ON game_soul_war_source_use_proofs(source_item_instance_id);

CREATE FUNCTION game_soul_war_source_use_proven(p_transaction UUID) RETURNS VOID
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE p game_soul_war_source_use_proofs%ROWTYPE; u game_item_use_records%ROWTYPE;
 s game_durability_reconnect_sessions%ROWTYPE; track TEXT;
BEGIN
 SELECT * INTO p FROM game_soul_war_source_use_proofs WHERE transaction_id=p_transaction;
 IF NOT FOUND OR p.created_xact_id<>pg_current_xact_id() THEN
  RAISE EXCEPTION 'source USE lacks exact same-xact selected-target proof' USING ERRCODE='23514';
 END IF;
 SELECT * INTO s FROM game_durability_reconnect_sessions
 WHERE game_session_id=p.game_session_id AND character_id=p.character_id AND world_id=p.world_id
  AND runtime_scope_world_id=p.world_id AND runtime_scope_house_key IS NULL AND session_state IN(1,2)
  AND current_generation=p.connection_generation AND character_lease_generation=p.character_lease_generation
  AND scope_ownership_generation=p.scope_generation
  AND runtime_scope_kind=CASE WHEN p.activity_instance_id IS NULL THEN 1 ELSE 2 END
  AND runtime_scope_channel_id IS NOT DISTINCT FROM p.channel_id
  AND runtime_scope_instance_id IS NOT DISTINCT FROM p.activity_instance_id;
 IF NOT FOUND OR game_runtime_scope_assignment_history_valid() IS NOT TRUE OR game_node_registration_history_valid() IS NOT TRUE
  OR (p.activity_instance_id IS NOT NULL AND game_activity_session_origin_valid(p.game_session_id,p.character_id) IS NOT TRUE)
  OR NOT EXISTS(SELECT 1 FROM game_durability_admission_character_guards c
   JOIN game_durability_admission_account_guards a ON a.account_id=c.account_id
   WHERE c.character_id=p.character_id AND c.account_id=s.account_id AND c.world_id=p.world_id
    AND c.eligible AND c.lease_generation=p.character_lease_generation AND c.holder_game_session_id=p.game_session_id
    AND a.presence_character_id=c.character_id AND a.holder_game_session_id=c.holder_game_session_id)
  OR NOT EXISTS(SELECT 1 FROM game_runtime_scope_assignments a
   JOIN game_node_registrations n ON n.node_id=a.holder_node_id
   JOIN game_durability_admission_runtime_guards ready
    ON ready.scope_key=set_byte(a.scope_key,0,CASE WHEN p.activity_instance_id IS NULL THEN 1 ELSE 2 END)
   WHERE a.world_id=p.world_id AND a.state=1 AND a.house_key IS NULL AND a.ownership_generation=p.scope_generation
    AND a.holder_node_id=p.node_id AND a.holder_registration_revision=p.node_registration
    AND a.source_revision=p.assignment_revision AND n.state=1 AND n.registration_revision=p.node_registration
    AND ready.ready AND ready.ownership_generation=p.scope_generation
    AND ((p.activity_instance_id IS NULL AND a.scope_kind=1 AND a.channel_id=p.channel_id AND a.activity_instance_id IS NULL)
     OR (p.activity_instance_id IS NOT NULL AND a.scope_kind=3 AND a.channel_id IS NULL AND a.activity_instance_id=p.activity_instance_id)))
  OR (p.activity_instance_id IS NULL AND NOT EXISTS(SELECT 1 FROM game_content_activations c WHERE c.world_id=p.world_id
   AND c.activation_sequence=p.activation_sequence AND c.server_artifact_digest=p.server_digest AND c.client_artifact_digest=p.client_digest
   AND c.channel_id=p.channel_id AND
    (c.frame_binding_digest=p.frame_digest OR EXISTS(
     -- The node's native-entry issuance and its full World runtime have
     -- distinct frames. Only the independently current reset map can qualify
     -- the bundle form; a frame/map copied from this proof is not authority.
     SELECT 1 FROM game_world_reset_active active JOIN game_world_reset_epochs epoch USING(world_id,reset_epoch)
     WHERE active.world_id=p.world_id AND game_world_reset_map_epoch_current(active.world_id,active.reset_epoch)
      AND epoch.server_digest=p.server_digest AND epoch.client_digest=p.client_digest
      AND p.map_digest=sha256(convert_to(epoch.map_revision,'UTF8'))
      AND game_elemental_placement_frame_matches(p.world_id,epoch.map_revision,p.frame_digest,c.frame_binding_digest)))
   AND NOT EXISTS(SELECT 1 FROM game_content_activations later WHERE later.world_id=c.world_id
    AND later.channel_id=c.channel_id AND later.activation_sequence>c.activation_sequence)))
  -- A running Instance keeps the exact accepted destination pins. A later
  -- Channel activation cannot invalidate this separate current scope owner.
  OR (p.activity_instance_id IS NOT NULL AND NOT EXISTS(
   SELECT 1 FROM game_activity_handoff_groups h
   JOIN game_activity_runtime_entries e USING(handoff_id)
   JOIN game_activity_handoff_participants origin ON origin.handoff_id=h.handoff_id
    AND origin.character_id=e.initiating_character_id
   JOIN game_activity_handoff_participants participant ON participant.handoff_id=h.handoff_id
    AND participant.character_id=p.character_id AND participant.destination_game_session_id=p.game_session_id
   JOIN game_content_activations c ON c.world_id=h.world_id AND c.channel_id=origin.origin_channel_id
   WHERE h.world_id=p.world_id AND h.instance_id=p.activity_instance_id
    AND h.ownership_generation=p.scope_generation AND h.state=2
    AND e.holder_node_id=p.node_id AND e.holder_registration_revision=p.node_registration
    AND e.destination_server_artifact_digest=p.server_digest
    AND e.destination_map_revision_digest=p.map_digest
    AND e.destination_coordinate_frame_digest=p.frame_digest
    AND origin.server_artifact_digest=p.server_digest
    AND c.activation_sequence=p.activation_sequence
    AND c.server_artifact_digest=p.server_digest AND c.client_artifact_digest=p.client_digest
    AND NOT EXISTS(SELECT 1 FROM game_activity_runtime_returns returned
     WHERE returned.handoff_id=participant.handoff_id AND returned.character_id=participant.character_id))) THEN
  RAISE EXCEPTION 'source USE requires its exact current session and qualified source scope' USING ERRCODE='23514';
 END IF;

 SELECT * INTO u FROM game_item_use_records WHERE transaction_id=p.transaction_id;
 IF p.accepted THEN
  IF u.transaction_id IS NULL OR u.cause_kind<>5 OR u.shape<>1 OR u.created_xact_id<>pg_current_xact_id()
   OR u.event_id<>p.event_id OR u.character_id<>p.character_id OR u.game_session_id<>p.game_session_id OR u.command_id<>p.command_id
   OR u.request_binding<>p.item_request_binding OR u.occurred_at<>p.occurred_at
   OR u.content_revision<>p.content_revision OR u.ruleset_revision<>p.ruleset_revision OR u.sim_revision<>p.sim_revision
   OR u.world_id<>p.world_id OR u.channel_id IS DISTINCT FROM p.channel_id OR u.activity_instance_id IS DISTINCT FROM p.activity_instance_id
   OR u.runtime_scope_ownership_generation<>p.scope_generation OR u.expected_session_generation<>p.connection_generation
   OR u.source_item_instance_id<>p.source_item_instance_id OR u.source_definition_production_key<>p.source_key
   OR u.source_definition_family<>'Item' OR u.source_definition_revision_ref<>'definition-r1'
   OR u.source_parent_item_instance_id<>p.source_parent_item_instance_id OR u.source_placement_ordinal<>p.source_placement_ordinal
   OR u.source_quantity_before<>p.source_quantity_before OR u.source_quantity_after<>p.source_quantity_before-1 THEN
   RAISE EXCEPTION 'accepted source USE requires its exact same-xact one-unit burn' USING ERRCODE='23514';
  END IF;
 ELSE
  -- Sanity records the cooldown before testing its target. Its live carried
  -- source must remain unchanged; no other burn may hide behind this proof.
  IF p.source_key<>'oteryn:item.tibia.i33950' OR u.transaction_id IS NOT NULL
   OR EXISTS(SELECT 1 FROM game_item_use_records burn
    WHERE (burn.game_session_id=p.game_session_id AND burn.command_id=p.command_id)
     OR burn.event_id=p.event_id
     OR (burn.source_item_instance_id=p.source_item_instance_id AND burn.created_xact_id=pg_current_xact_id()))
   OR NOT EXISTS(SELECT 1 FROM game_item_instances i
    JOIN (SELECT item_instance_id,parent_item_instance_id,placement_ordinal FROM game_item_container_entries WHERE character_id=p.character_id
     UNION ALL SELECT item_instance_id,parent_item_instance_id,placement_ordinal FROM game_item_nested_entries) e USING(item_instance_id)
    CROSS JOIN LATERAL game_item_loop_owner(e.parent_item_instance_id) owner
    WHERE i.item_instance_id=p.source_item_instance_id AND i.world_id=p.world_id AND i.lifecycle=1 AND i.quantity=p.source_quantity_before
     AND i.definition_family='Item' AND i.definition_production_key=p.source_key AND i.definition_revision_ref='definition-r1'
     AND e.parent_item_instance_id=p.source_parent_item_instance_id AND e.placement_ordinal=p.source_placement_ordinal
     AND owner.owner_character=p.character_id AND owner.depth BETWEEN 0 AND 7)
   OR EXISTS(SELECT 1 FROM game_item_transfer_quantity_evidence ev
    WHERE ev.item_instance_id=p.source_item_instance_id AND ev.transaction_id=p.transaction_id) THEN
   RAISE EXCEPTION 'wrong-target Sanity must retain its exact current carried Item without a burn' USING ERRCODE='23514';
  END IF;
 END IF;
 IF p.transition_key IS NOT NULL THEN
  track:=CASE p.source_key WHEN 'oteryn:item.tibia.i33891' THEN 'oteryn:quest-progress/soul-war/furious-crater/greedy-maw-action'
   ELSE 'oteryn:quest-progress/soul-war/furious-crater/cleansed-sanity-action' END;
  IF NOT EXISTS(SELECT 1 FROM game_character_quest_receipts q
   WHERE q.character_id=p.character_id AND q.cause_kind='use' AND q.cause_id=p.game_session_id AND q.cause_ordinal=p.command_id
    AND q.transition_key=p.transition_key AND q.quest_key='oteryn:quest/soul-war/source-items' AND NOT q.completes
    AND q.request_binding=p.quest_request_binding AND q.track_keys=ARRAY[track]
    AND cardinality(q.values_before)=1 AND q.values_before[1]>=0 AND q.values_after[1]::NUMERIC>=q.values_before[1]::NUMERIC+30
    AND q.level_before=q.level_after AND q.experience_before=q.experience_after
    AND q.content_revision=p.content_revision AND q.ruleset_revision=p.ruleset_revision AND q.created_xact_id=pg_current_xact_id())
   OR EXISTS(SELECT 1 FROM game_character_quest_xp_obligations x WHERE x.character_id=p.character_id
    AND x.cause_id=p.game_session_id AND x.cause_ordinal=p.command_id AND x.transition_key=p.transition_key) THEN
   RAISE EXCEPTION 'source USE requires its exact same-xact Quest cooldown' USING ERRCODE='23514';
  END IF;
 ELSE
  IF EXISTS(SELECT 1 FROM game_character_quest_receipts q WHERE q.character_id=p.character_id
   AND q.cause_id=p.game_session_id AND q.cause_ordinal=p.command_id AND q.created_xact_id=pg_current_xact_id()) THEN
   RAISE EXCEPTION 'Sorrow is Item-only and has no Quest transition' USING ERRCODE='23514';
  END IF;
 END IF;
END $$;

CREATE FUNCTION game_soul_war_source_use_pair_guard() RETURNS TRIGGER
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE transaction UUID;
BEGIN
 IF TG_TABLE_NAME='game_character_quest_receipts' THEN
  IF NEW.transition_key NOT IN('oteryn:quest-transition/soul-war/source-items/greedy-maw','oteryn:quest-transition/soul-war/source-items/cleansed-sanity') THEN RETURN NULL; END IF;
  SELECT transaction_id INTO transaction FROM game_soul_war_source_use_proofs p
   WHERE p.character_id=NEW.character_id AND p.game_session_id=NEW.cause_id AND p.command_id=NEW.cause_ordinal AND p.transition_key=NEW.transition_key;
 ELSE
  IF TG_TABLE_NAME='game_item_use_records' THEN
   IF NEW.cause_kind<>5 THEN RETURN NULL; END IF;
  END IF;
  transaction:=NEW.transaction_id;
 END IF;
 PERFORM game_soul_war_source_use_proven(transaction);
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_soul_war_source_use_proof_pair AFTER INSERT ON game_soul_war_source_use_proofs
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_soul_war_source_use_pair_guard();
CREATE CONSTRAINT TRIGGER game_soul_war_source_use_item_pair AFTER INSERT ON game_item_use_records
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_soul_war_source_use_pair_guard();
CREATE CONSTRAINT TRIGGER game_soul_war_source_use_quest_pair AFTER INSERT ON game_character_quest_receipts
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_soul_war_source_use_pair_guard();
CREATE TRIGGER game_soul_war_source_use_immutable BEFORE UPDATE OR DELETE ON game_soul_war_source_use_proofs
 FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_soul_war_source_use_no_truncate BEFORE TRUNCATE ON game_soul_war_source_use_proofs
 FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();
CREATE TRIGGER game_soul_war_source_use_stamp BEFORE INSERT ON game_soul_war_source_use_proofs
 FOR EACH ROW EXECUTE FUNCTION game_item_stamp_created_xact_id();
DO $$ DECLARE f TEXT; BEGIN
 FOREACH f IN ARRAY ARRAY['game_soul_war_source_use_proven(uuid)','game_soul_war_source_use_pair_guard()'] LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',f,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC,oteryn_game_runtime,oteryn_game_control',f);
 END LOOP;
END $$;
REVOKE ALL ON game_soul_war_source_use_proofs FROM PUBLIC;
GRANT SELECT,INSERT ON game_soul_war_source_use_proofs TO oteryn_game_runtime;
GRANT SELECT ON game_soul_war_source_use_proofs TO oteryn_game_control;
