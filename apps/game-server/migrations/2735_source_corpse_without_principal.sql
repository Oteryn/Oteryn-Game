-- Source-death completion is not reward damage credit. Preserve the seven
-- closed physical corpse facades without inventing a CharacterId. Every fresh
-- NULL-principal corpse needs its exact immutable same-transaction witness;
-- all existing reservation, MINT, Ground, audit, capacity and decay guards run.
CREATE TABLE game_source_corpse_mints (
 transaction_id UUID PRIMARY KEY REFERENCES game_item_mint_receipts(transaction_id),
 item_instance_id UUID NOT NULL UNIQUE REFERENCES game_item_instances(item_instance_id),
 proof BYTEA NOT NULL CHECK(octet_length(proof) BETWEEN 1 AND 4096),
 binding BYTEA NOT NULL CHECK(binding=sha256(proof)),
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id()
);
CREATE TRIGGER game_source_corpse_mint_stamp BEFORE INSERT ON game_source_corpse_mints
 FOR EACH ROW EXECUTE FUNCTION game_item_stamp_created_xact_id();
CREATE TRIGGER game_source_corpse_mint_immutable BEFORE UPDATE OR DELETE ON game_source_corpse_mints
 FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_source_corpse_mint_no_truncate BEFORE TRUNCATE ON game_source_corpse_mints
 FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();

-- Outburst and Poor Soul are the existing typed empty-corpse ports. The five
-- Rotten bodies are exact Crystal201189f parents. The unloaded declaration
-- RottenBloodBossDeath supplies no fake callback; actual loaded indices stay1.
CREATE FUNCTION game_source_corpse_descriptor_valid(p JSONB, item_key TEXT)
RETURNS BOOLEAN LANGUAGE sql IMMUTABLE AS $$
 SELECT coalesce(
  p->>'schema'='OTERYN_SOURCE_CORPSE_MINT/v1'
  AND p->>'creature_revision'='definition-r1' AND p->'has_reward_damage_credit'='false'::jsonb
  AND p ?& ARRAY['events','death','content','client','map','frame','position','bakragore_loot']
  AND p->>'death' ~ '^([0-9a-f]{104}|02[0-9a-f]{104})$'
  AND p->>'position' ~ '^[0-9a-f]{20}$'
  AND p->>'content' ~ '^[0-9a-f]{64}$' AND p->>'content'<>repeat('0',64)
  AND p->>'client' ~ '^[0-9a-f]{64}$' AND p->>'client'<>repeat('0',64)
  AND p->>'map' ~ '^[0-9a-f]{64}$' AND p->>'map'<>repeat('0',64)
  AND p->>'frame' ~ '^[0-9a-f]{64}$' AND p->>'frame'<>repeat('0',64)
  AND item_key=CASE p->>'creature_key'
   WHEN 'oteryn:creature.outburst' THEN 'oteryn:item.tibia.i23564'
   WHEN 'oteryn:creature.poor_soul' THEN 'oteryn:item.tibia.i33891'
   WHEN 'oteryn:creature.murcion' THEN 'oteryn:item.tibia.i44015'
   WHEN 'oteryn:creature.ichgahal' THEN 'oteryn:item.tibia.i44018'
   WHEN 'oteryn:creature.vemiath' THEN 'oteryn:item.tibia.i44021'
   WHEN 'oteryn:creature.chagorz' THEN 'oteryn:item.tibia.i44024'
   WHEN 'oteryn:creature.bakragore' THEN 'oteryn:item.tibia.i44012' END
  AND p->'events'=CASE p->>'creature_key'
   WHEN 'oteryn:creature.outburst' THEN '[{"registration_index":0,"name":"HeartBossDeath","source_path":"data-otservbr-global/scripts/quests/heart_of_destruction/creaturescripts_heart_boss_death.lua","source_sha256":"37b640f71260ae4b5ca4f50c485a7cf61877a9717149e6a9d6fad6c6a702c3d6"}]'::jsonb
   WHEN 'oteryn:creature.vemiath' THEN '[{"registration_index":1,"name":"VemiathMechanicsDeath","source_path":"data-global/scripts/quests/rotten_blood_quest/vemiath_mechanics_death.lua","source_sha256":"6c777759c51058c12a5725d770c81bcf2475f8bc1308f4f830bd8f311ef915e1"}]'::jsonb
   WHEN 'oteryn:creature.chagorz' THEN '[{"registration_index":1,"name":"ChagorzMechanicsDeath","source_path":"data-global/scripts/quests/rotten_blood_quest/chagorz_mechanics_death.lua","source_sha256":"fbbe03619599a377c08f2def12c382f72211a98a362186114d38d4fa529c1e37"}]'::jsonb
   WHEN 'oteryn:creature.bakragore' THEN '[{"registration_index":1,"name":"BakragoreMechanicsDeath","source_path":"data-global/scripts/quests/rotten_blood_quest/bakragore_mechanics_death.lua","source_sha256":"21ba33eb58d1b5a4a88ca0c950d49a8369b34f3a11fe39a7e549a8b3ab154b46"}]'::jsonb
   ELSE '[]'::jsonb END
  AND CASE WHEN p->>'creature_key'='oteryn:creature.bakragore' THEN
   jsonb_typeof(p->'bakragore_loot')='object'
   AND p->'bakragore_loot' ?& ARRAY['essences','chance_roll','selection_roll','source_item']
   AND p#>>'{bakragore_loot,essences}' ~ '^[0-4]$'
   AND CASE WHEN p#>>'{bakragore_loot,essences}'='0' THEN
    p#>'{bakragore_loot,chance_roll}'='null'::jsonb
    AND p#>'{bakragore_loot,selection_roll}'='null'::jsonb
    AND p#>'{bakragore_loot,source_item}'='null'::jsonb
   ELSE p#>>'{bakragore_loot,chance_roll}' ~ '^([1-9]|[1-9][0-9]|100)$'
    AND CASE WHEN (p#>>'{bakragore_loot,chance_roll}')::integer
      <=(ARRAY[0,5,8,12,15])[(p#>>'{bakragore_loot,essences}')::integer+1] THEN
     p#>>'{bakragore_loot,selection_roll}' IN('1','2')
     AND p#>>'{bakragore_loot,source_item}'=CASE p#>>'{bakragore_loot,selection_roll}' WHEN '1' THEN '43895' WHEN '2' THEN '43860' END
    ELSE p#>'{bakragore_loot,selection_roll}'='null'::jsonb
     AND p#>'{bakragore_loot,source_item}'='null'::jsonb END END
   ELSE p->'bakragore_loot'='null'::jsonb END,false)
$$;

CREATE FUNCTION game_source_corpse_mint_proven() RETURNS TRIGGER
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE w game_source_corpse_mints%ROWTYPE; r game_item_mint_receipts%ROWTYPE;
 v game_item_mint_reservations%ROWTYPE; i game_item_instances%ROWTYPE;
 g game_item_ground_locations%ROWTYPE; p JSONB; selected TEXT; source_channel UUID;
BEGIN
 IF TG_TABLE_NAME='game_item_mint_receipts' THEN
  IF NEW.loot_purpose_key<>'CORPSE_MATERIALIZATION' OR NEW.corpse_top_damage_character_id IS NOT NULL THEN
   IF EXISTS(SELECT 1 FROM game_source_corpse_mints WHERE transaction_id=NEW.transaction_id) THEN
    RAISE EXCEPTION 'source corpse witness cannot relabel a credited or ordinary MINT' USING ERRCODE='23514';
   END IF;
   RETURN NULL;
  END IF;
 END IF;
 SELECT * INTO w FROM game_source_corpse_mints WHERE transaction_id=NEW.transaction_id;
 IF NOT FOUND THEN RAISE EXCEPTION 'no-principal corpse requires its exact source completion witness' USING ERRCODE='23514'; END IF;
 SELECT * INTO r FROM game_item_mint_receipts WHERE transaction_id=w.transaction_id;
 SELECT * INTO v FROM game_item_mint_reservations WHERE transaction_id=w.transaction_id;
 SELECT * INTO i FROM game_item_instances WHERE item_instance_id=w.item_instance_id;
 SELECT * INTO g FROM game_item_ground_locations WHERE item_instance_id=w.item_instance_id;
 p:=convert_from(w.proof,'UTF8')::jsonb;
 IF (r.loot_purpose_key='CORPSE_MATERIALIZATION' AND r.draw_ordinal=0
  AND r.corpse_top_damage_character_id IS NULL AND r.destination_parent_item_instance_id IS NULL
  AND r.item_instance_id=w.item_instance_id AND r.created_xact_id=w.created_xact_id
  AND w.created_xact_id=pg_current_xact_id() AND w.binding=sha256(w.proof)
  AND v.item_instance_id=r.item_instance_id AND v.event_id=r.event_id
  AND v.intent_binding=r.intent_binding AND v.envelope IS NOT NULL
  AND v.death_world_id=r.death_world_id AND v.death_scope_key=r.death_scope_key
  AND v.death_scope_ownership_generation=r.death_scope_ownership_generation
  AND v.death_actor_local_id=r.death_actor_local_id AND v.death_actor_local_generation=r.death_actor_local_generation
  AND v.loot_table_family=r.loot_table_family AND v.loot_table_production_key=r.loot_table_production_key
  AND v.loot_table_revision_ref=r.loot_table_revision_ref AND v.loot_purpose_key=r.loot_purpose_key AND v.draw_ordinal=r.draw_ordinal
  AND i.minted_transaction_id=r.transaction_id AND i.world_id=r.death_world_id AND i.quantity=1 AND i.lifecycle=1
  AND i.definition_family='Item' AND i.definition_revision_ref='definition-r1'
  AND r.loot_table_family=i.definition_family AND r.loot_table_production_key=i.definition_production_key
  AND r.loot_table_revision_ref=i.definition_revision_ref
  AND game_source_corpse_descriptor_valid(p,i.definition_production_key)
  AND g.world_id=r.death_world_id AND g.scope_kind=r.death_scope_kind
  AND g.channel_id IS NOT DISTINCT FROM r.death_channel_id AND g.instance_id IS NOT DISTINCT FROM r.death_instance_id
  AND g.runtime_scope_ownership_generation=r.death_scope_ownership_generation
  AND g.content_revision='sha256:'||(p->>'content') AND g.map_revision='sha256:'||(p->>'map')
  AND g.native_room_placement_context=decode(p->>'frame','hex') AND g.spatial_position=decode(p->>'position','hex')
  AND g.corpse_ref=decode(p->>'death','hex')
  AND g.corpse_ref=(CASE WHEN r.death_scope_kind=2 THEN decode('02','hex') ELSE decode('','hex') END
   ||uuid_send(r.death_world_id)||uuid_send(coalesce(r.death_channel_id,r.death_instance_id))
   ||decode(lpad(to_hex(trunc(r.death_scope_ownership_generation/4294967296)::bigint),8,'0')
      ||lpad(to_hex(mod(r.death_scope_ownership_generation,4294967296)::bigint),8,'0'),'hex')
   ||substring(int8send(r.death_actor_local_id) from 5 for 4)
   ||decode(lpad(to_hex(trunc(r.death_actor_local_generation/4294967296)::bigint),8,'0')
      ||lpad(to_hex(mod(r.death_actor_local_generation,4294967296)::bigint),8,'0'),'hex'))
  AND EXISTS(SELECT 1 FROM game_item_audit_outbox a WHERE a.transaction_id=r.transaction_id
   AND a.item_instance_id=r.item_instance_id AND a.event_id=r.event_id
   AND a.created_xact_id=w.created_xact_id AND a.envelope=v.envelope
   AND a.envelope_sha256=r.envelope_sha256 AND a.occurred_at=r.occurred_at)
 ) IS NOT TRUE THEN RAISE EXCEPTION 'source corpse requires exact same-XID Item/Ground/MINT/audit' USING ERRCODE='23514'; END IF;

 -- The immutable completion is DATA. Current assignment, readiness, node
 -- incarnation and accepted activity/content are independently required.
 IF game_runtime_scope_assignment_history_valid() IS NOT TRUE OR game_node_registration_history_valid() IS NOT TRUE THEN
  RAISE EXCEPTION 'source corpse owner history is invalid' USING ERRCODE='23514'; END IF;
 PERFORM 1 FROM game_runtime_scope_assignments a JOIN game_node_registrations n ON n.node_id=a.holder_node_id
  JOIN game_durability_admission_runtime_guards ready ON ready.scope_key=set_byte(a.scope_key,0,r.death_scope_kind)
  WHERE a.scope_key=r.death_scope_key AND a.world_id=r.death_world_id AND a.state=1 AND a.house_key IS NULL
   AND a.channel_id IS NOT DISTINCT FROM r.death_channel_id AND a.activity_instance_id IS NOT DISTINCT FROM r.death_instance_id
   AND a.ownership_generation=r.death_scope_ownership_generation AND ready.ready AND ready.ownership_generation=a.ownership_generation
   AND a.holder_node_id=v.fence_holder_node_id AND a.holder_registration_revision=v.fence_holder_registration_revision
   AND n.state=1 AND n.registration_revision=a.holder_registration_revision
  FOR SHARE OF a,n,ready;
 IF NOT FOUND THEN RAISE EXCEPTION 'source corpse current owner is missing' USING ERRCODE='23514'; END IF;
 IF r.death_scope_kind=2 THEN
  SELECT origin.origin_channel_id INTO source_channel FROM game_activity_handoff_groups h
   JOIN game_activity_runtime_entries e USING(handoff_id)
   JOIN game_activity_handoff_participants origin ON origin.handoff_id=h.handoff_id
    AND origin.character_id=e.initiating_character_id
   WHERE h.world_id=r.death_world_id AND h.instance_id=r.death_instance_id
    AND h.ownership_generation=r.death_scope_ownership_generation AND h.state=2
    AND e.holder_node_id=v.fence_holder_node_id AND e.holder_registration_revision=v.fence_holder_registration_revision
    AND e.destination_server_artifact_digest=decode(p->>'content','hex')
    AND e.destination_map_revision_digest=decode(p->>'map','hex')
    AND e.destination_coordinate_frame_digest=decode(p->>'frame','hex')
    AND EXISTS(SELECT 1 FROM game_activity_handoff_participants participant
     WHERE participant.handoff_id=h.handoff_id AND NOT EXISTS(SELECT 1 FROM game_activity_runtime_returns returned
      WHERE returned.handoff_id=participant.handoff_id AND returned.character_id=participant.character_id))
    AND (p->>'creature_key'='oteryn:creature.poor_soul' OR EXISTS(SELECT 1 FROM game_boss_room_activities boss
     WHERE boss.admission=h.handoff_id AND boss.boss_creature_key=p->>'creature_key'))
   FOR SHARE OF h,e;
  IF NOT FOUND THEN RAISE EXCEPTION 'source corpse current activity pins are missing' USING ERRCODE='23514'; END IF;
 ELSE source_channel:=r.death_channel_id;
 END IF;
 PERFORM pg_advisory_xact_lock_shared(hashtextextended(
  'oteryn:content-activation:'||r.death_world_id::text||':'||source_channel::text,0));
 PERFORM 1 FROM game_content_activations c WHERE c.world_id=r.death_world_id AND c.channel_id=source_channel
  AND c.server_artifact_digest=decode(p->>'content','hex') AND c.client_artifact_digest=decode(p->>'client','hex')
  AND (r.death_scope_kind=2 OR c.frame_binding_digest=decode(p->>'frame','hex') OR EXISTS(
   -- A World source death retains the actual published bundle frame, while
   -- activation issuance may retain the distinct native-entry frame. Only
   -- the independently current reset map can prove this second representation.
   SELECT 1 FROM game_world_reset_active active JOIN game_world_reset_epochs epoch USING(world_id,reset_epoch)
   WHERE active.world_id=r.death_world_id AND game_world_reset_map_epoch_current(active.world_id,active.reset_epoch)
    AND epoch.server_digest=decode(p->>'content','hex') AND epoch.client_digest=decode(p->>'client','hex')
    AND decode(p->>'map','hex')=sha256(convert_to(epoch.map_revision,'UTF8'))
    AND game_elemental_placement_frame_matches(r.death_world_id,epoch.map_revision,decode(p->>'frame','hex'),c.frame_binding_digest)))
  AND (r.death_scope_kind=2 OR NOT EXISTS(SELECT 1 FROM game_content_activations later WHERE later.world_id=c.world_id AND later.channel_id=c.channel_id
   AND later.activation_sequence>c.activation_sequence))
  ORDER BY c.activation_sequence DESC LIMIT 1 FOR SHARE OF c;
 IF NOT FOUND THEN RAISE EXCEPTION 'source corpse current content pins are missing' USING ERRCODE='23514'; END IF;

 selected:=p#>>'{bakragore_loot,source_item}';
 IF (SELECT count(*) FROM game_item_corpse_container_entries WHERE parent_item_instance_id=w.item_instance_id)
   <>(CASE WHEN selected IS NULL THEN 0 ELSE 1 END)
  OR EXISTS(SELECT 1 FROM game_item_nested_entries WHERE parent_item_instance_id=w.item_instance_id)
  OR (selected IS NOT NULL AND NOT EXISTS(
   SELECT 1 FROM game_item_corpse_container_entries placement
   JOIN game_item_instances child USING(item_instance_id)
   JOIN game_item_mint_receipts child_mint ON child_mint.item_instance_id=child.item_instance_id
   WHERE placement.parent_item_instance_id=w.item_instance_id AND placement.placement_ordinal=1
    AND placement.placed_transaction_id=child.minted_transaction_id AND child_mint.transaction_id=child.minted_transaction_id
    AND child.world_id=r.death_world_id AND child.definition_family='Item' AND child.definition_revision_ref='definition-r1'
    AND child.definition_production_key='oteryn:item.tibia.i'||selected AND child.quantity=1 AND child.lifecycle=1
    AND child_mint.created_xact_id=w.created_xact_id AND child_mint.destination_parent_item_instance_id=w.item_instance_id
    AND child_mint.destination_ordinal=1 AND child_mint.corpse_top_damage_character_id IS NULL
    AND child_mint.death_world_id=r.death_world_id AND child_mint.death_scope_key=r.death_scope_key
    AND child_mint.death_scope_ownership_generation=r.death_scope_ownership_generation
    AND child_mint.death_actor_local_id=r.death_actor_local_id AND child_mint.death_actor_local_generation=r.death_actor_local_generation
    AND child_mint.loot_table_family='Loot' AND child_mint.loot_table_production_key='oteryn:loot.creature.bakragore'
    AND child_mint.loot_table_revision_ref='definition-r1' AND child_mint.loot_purpose_key=child.definition_production_key
    AND child_mint.draw_ordinal=0))
 THEN RAISE EXCEPTION 'source corpse contains a different copied Bakragore draw' USING ERRCODE='23514'; END IF;
 RETURN NULL;
END $$;

ALTER TABLE game_item_mint_receipts DROP CONSTRAINT game_item_mint_receipts_corpse_top_damage_scoped;
ALTER TABLE game_item_mint_receipts ADD CONSTRAINT game_item_mint_receipts_corpse_top_damage_scoped
 CHECK(loot_purpose_key='CORPSE_MATERIALIZATION' OR corpse_top_damage_character_id IS NULL);
CREATE CONSTRAINT TRIGGER game_source_corpse_mint_proven AFTER INSERT ON game_source_corpse_mints
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_source_corpse_mint_proven();
CREATE CONSTRAINT TRIGGER game_source_corpse_mint_proven AFTER INSERT ON game_item_mint_receipts
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_source_corpse_mint_proven();
DO $$ DECLARE name TEXT; BEGIN
 FOREACH name IN ARRAY ARRAY['game_source_corpse_descriptor_valid(jsonb,text)','game_source_corpse_mint_proven()'] LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',name,current_schema());
 END LOOP;
END $$;
REVOKE ALL ON game_source_corpse_mints FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
GRANT SELECT,INSERT ON game_source_corpse_mints TO oteryn_game_runtime;
REVOKE ALL ON FUNCTION game_source_corpse_descriptor_valid(JSONB,TEXT),game_source_corpse_mint_proven()
 FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
