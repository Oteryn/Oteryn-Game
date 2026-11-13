-- A real committed Player STEP consumes source-created Cloak blood through
-- the existing scoped Item owner. The original MINT cause remains unchanged.
DO $$
DECLARE name TEXT; prior TEXT; arm TEXT;
BEGIN
 arm:=$p$(cause_kind=2 AND spell_family='Encounter'
 AND spell_production_key='oteryn:encounter.soul_war_taint_zones' AND spell_revision='definition-r1'
 AND creature_definition_key='oteryn:creature.cloak_of_terror' AND creature_definition_revision='definition-r1'
 AND ((runtime_scope_kind=1 AND channel_id IS NOT NULL AND instance_id IS NULL)
   OR (runtime_scope_kind=2 AND channel_id IS NULL AND instance_id IS NOT NULL AND game_node_is_uuid_v7(instance_id)))
 AND operation_count=1 AND swing_ordinal IS NULL
 AND game_session_id IS NULL AND command_id IS NULL AND character_id IS NULL
 AND cost IS NULL AND cost_binding IS NULL AND caster_lease_generation IS NULL AND caster_placement_digest IS NULL
 AND octet_length(creature_placement)=16 AND creature_placement<>decode(repeat('00',16),'hex')
 AND creature_actor_generation BETWEEN 1 AND 18446744073709551615
 AND creature_occurrence ~ '^cloak-blood-step:[0-9a-f]{64}:[0-9a-f]{32}$'
 AND octet_length(creature_map_digest)=32 AND creature_map_digest<>decode(repeat('00',32),'hex')
 AND octet_length(creature_frame_digest)=32 AND creature_frame_digest<>decode(repeat('00',32),'hex')
 AND creature_source_body_digest=decode('9e1107dc41abc16b4f56e795180c1c17c917d8e73754547c443f070ed3167916','hex')
 AND octet_length(creature_cast_digest)=32 AND creature_cast_digest<>decode(repeat('00',32),'hex')
 AND convert_from(intent,'UTF8')::jsonb->>'schema'='OTERYN_ENCOUNTER_GROUND_CAUSE/v2'
 AND convert_from(intent,'UTF8')::jsonb->'player'='null'::jsonb
 AND convert_from(intent,'UTF8')::jsonb->'cost'='null'::jsonb
 AND convert_from(intent,'UTF8')::jsonb->'caster_origin'='null'::jsonb)$p$;
 FOREACH name IN ARRAY ARRAY['game_spell_item_typed_cause','game_spell_item_exact_runtime_scope'] LOOP
  SELECT pg_get_constraintdef(oid) INTO STRICT prior FROM pg_constraint
   WHERE conrelid='game_spell_item_receipts'::regclass AND conname=name;
  IF left(prior,6)<>'CHECK ' THEN RAISE EXCEPTION 'unexpected Cloak cause shape'; END IF;
  EXECUTE format('ALTER TABLE game_spell_item_receipts DROP CONSTRAINT %I',name);
  EXECUTE format('ALTER TABLE game_spell_item_receipts ADD CONSTRAINT %I CHECK ((%s) OR (%s) IS TRUE)',name,substr(prior,7),arm);
 END LOOP;
END $$;
DO $$
DECLARE prior TEXT; amended TEXT;
BEGIN
 prior:=pg_get_functiondef('game_cloak_blood_source_proven()'::regprocedure);
 amended:=replace(prior,
  'IF NEW.cause_kind<>2 OR NEW.spell_production_key<>''oteryn:encounter.soul_war_taint_zones'' THEN RETURN NULL; END IF;',
  'IF NEW.cause_kind<>2 OR NEW.spell_production_key<>''oteryn:encounter.soul_war_taint_zones'' OR NEW.creature_occurrence LIKE ''cloak-blood-step:%'' THEN RETURN NULL; END IF;');
 IF amended=prior THEN RAISE EXCEPTION 'Cloak MINT source discriminator changed'; END IF;
 EXECUTE amended;
END $$;
CREATE FUNCTION game_cloak_blood_step_source_proven() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NEW.cause_kind<>2 OR NEW.spell_production_key<>'oteryn:encounter.soul_war_taint_zones'
   OR NEW.creature_occurrence NOT LIKE 'cloak-blood-step:%' THEN RETURN NULL; END IF;
 IF (SELECT count(*) FROM game_spell_item_lines WHERE transaction_id=NEW.transaction_id)<>1
 OR NOT EXISTS(SELECT 1 FROM game_spell_item_lines l
   JOIN game_item_instances i USING(item_instance_id)
   JOIN game_spell_item_receipts r ON r.transaction_id=i.minted_transaction_id
   JOIN game_spell_item_lines original ON original.transaction_id=r.transaction_id AND original.item_instance_id=i.item_instance_id
   WHERE l.transaction_id=NEW.transaction_id AND l.operation_kind=2
    AND l.definition_family='Item' AND l.definition_revision='definition-r1'
    AND l.definition_production_key IN('oteryn:item.tibia.i33854','oteryn:item.tibia.i34006','oteryn:item.tibia.i34007')
    AND i.definition_production_key=l.definition_production_key AND i.definition_revision_ref=l.definition_revision
    AND l.quantity_before=1 AND l.quantity_after=0 AND l.created_xact_id=pg_current_xact_id()
    AND l.source_parent_item_instance_id IS NULL AND l.destination_parent_item_instance_id IS NULL
    AND NOT l.blocks_movement AND NOT l.blocks_projectile AND NOT l.immovable_block_solid
    AND l.world_id=NEW.world_id AND l.runtime_scope_kind=NEW.runtime_scope_kind
    AND l.channel_id IS NOT DISTINCT FROM NEW.channel_id AND l.instance_id IS NOT DISTINCT FROM NEW.instance_id
    AND r.cause_kind=2 AND r.spell_production_key=NEW.spell_production_key
    AND r.creature_definition_key=NEW.creature_definition_key AND r.creature_definition_revision=NEW.creature_definition_revision
    AND r.creature_occurrence ~ '^cloak-blood:[0-9a-f]{64}$'
    AND r.creature_placement=NEW.creature_placement AND r.creature_actor_generation=NEW.creature_actor_generation
    AND r.creature_source_body_digest=NEW.creature_source_body_digest
    AND r.world_id=NEW.world_id AND r.runtime_scope_kind=NEW.runtime_scope_kind
    AND r.channel_id IS NOT DISTINCT FROM NEW.channel_id AND r.instance_id IS NOT DISTINCT FROM NEW.instance_id
    AND r.ownership_generation=NEW.ownership_generation AND r.catalog_digest=NEW.catalog_digest
    AND r.creature_map_digest=NEW.creature_map_digest AND r.creature_frame_digest=NEW.creature_frame_digest
    AND original.operation_kind=1 AND original.definition_production_key='oteryn:item.tibia.i33854'
    AND l.spatial_position=original.spatial_position AND l.map_revision=original.map_revision
    AND l.content_revision=original.content_revision AND l.placement_context=original.placement_context)
 THEN RAISE EXCEPTION 'Cloak blood STEP requires its exact original native pool' USING ERRCODE='23514'; END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_cloak_blood_step_source_proven AFTER INSERT ON game_spell_item_receipts
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_cloak_blood_step_source_proven();
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_cloak_blood_step_source_proven() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_cloak_blood_step_source_proven() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_cloak_blood_step_source_proven() TO oteryn_game_runtime,oteryn_game_control;
