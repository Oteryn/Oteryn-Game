-- Accepted Q6a: a committed native Cloak HP callback creates first-stage blood.
-- Extend the existing Encounter/Item cause; retain every old cause arm.
DO $$
DECLARE name TEXT; prior TEXT; arm TEXT;
BEGIN
 arm:=$p$(cause_kind=2 AND spell_family='Encounter'
  AND spell_production_key='oteryn:encounter.soul_war_taint_zones' AND spell_revision='definition-r1'
  AND creature_definition_key='oteryn:creature.cloak_of_terror' AND creature_definition_revision='definition-r1'
  AND ((runtime_scope_kind=1 AND channel_id IS NOT NULL AND instance_id IS NULL)
    OR (runtime_scope_kind=2 AND channel_id IS NULL AND instance_id IS NOT NULL AND game_node_is_uuid_v7(instance_id)))
  AND operation_count BETWEEN 0 AND 1 AND swing_ordinal IS NULL
  AND game_session_id IS NULL AND command_id IS NULL AND character_id IS NULL
  AND cost IS NULL AND cost_binding IS NULL AND caster_lease_generation IS NULL AND caster_placement_digest IS NULL
  AND octet_length(creature_placement)=16 AND creature_placement<>decode(repeat('00',16),'hex')
  AND creature_actor_generation BETWEEN 1 AND 18446744073709551615
  AND creature_occurrence ~ '^cloak-blood:[0-9a-f]{64}$'
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
  IF left(prior,6)<>'CHECK ' THEN RAISE EXCEPTION 'unexpected scoped source cause shape'; END IF;
  EXECUTE format('ALTER TABLE game_spell_item_receipts DROP CONSTRAINT %I',name);
  EXECUTE format('ALTER TABLE game_spell_item_receipts ADD CONSTRAINT %I CHECK ((%s) OR (%s) IS TRUE)',name,substr(prior,7),arm);
 END LOOP;
END $$;

-- The Primal-specific consumer must not consume another closed Encounter's cause.
-- Its original physical proof and every predicate are preserved verbatim.
DO $$
DECLARE prior TEXT; amended TEXT;
BEGIN
 prior:=pg_get_functiondef('game_primal_pod_source_proven()'::regprocedure);
 amended:=replace(prior,'IF NEW.cause_kind<>2 THEN RETURN NULL; END IF;',
  'IF NEW.cause_kind<>2 OR NEW.spell_production_key<>''oteryn:encounter.the_primal_menace'' THEN RETURN NULL; END IF;');
 IF amended=prior THEN RAISE EXCEPTION 'Primal source discriminator changed'; END IF;
 EXECUTE amended;
END $$;

CREATE FUNCTION game_cloak_blood_source_proven() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NEW.cause_kind<>2 OR NEW.spell_production_key<>'oteryn:encounter.soul_war_taint_zones' THEN RETURN NULL; END IF;
 IF NEW.creature_definition_key<>'oteryn:creature.cloak_of_terror'
  OR (SELECT count(*) FROM game_spell_item_lines l WHERE l.transaction_id=NEW.transaction_id)<>NEW.operation_count
  OR EXISTS(SELECT 1 FROM game_spell_item_lines l WHERE l.transaction_id=NEW.transaction_id
    AND (l.operation_kind<>1 OR l.definition_family<>'Item'
      OR l.definition_production_key<>'oteryn:item.tibia.i33854' OR l.definition_revision<>'definition-r1'
      OR l.world_id<>NEW.world_id OR l.runtime_scope_kind<>NEW.runtime_scope_kind
      OR l.channel_id IS DISTINCT FROM NEW.channel_id OR l.instance_id IS DISTINCT FROM NEW.instance_id
      OR l.created_xact_id<>pg_current_xact_id()
      OR l.destination_parent_item_instance_id IS NOT NULL OR l.source_parent_item_instance_id IS NOT NULL
      OR l.quantity_before<>0 OR l.quantity_after<>1 OR l.blocks_movement OR l.blocks_projectile OR l.immovable_block_solid))
 THEN RAISE EXCEPTION 'Cloak native health source requires its exact first-stage Ground Item' USING ERRCODE='23514'; END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_cloak_blood_source_proven AFTER INSERT ON game_spell_item_receipts
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_cloak_blood_source_proven();
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_cloak_blood_source_proven() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_cloak_blood_source_proven() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_cloak_blood_source_proven() TO oteryn_game_runtime,oteryn_game_control;
CREATE UNIQUE INDEX game_cloak_blood_native_occurrence_once ON game_spell_item_receipts
 (world_id,runtime_scope_kind,(coalesce(channel_id,instance_id)),ownership_generation,
  creature_placement,creature_actor_generation,creature_occurrence)
 WHERE cause_kind=2 AND spell_production_key='oteryn:encounter.soul_war_taint_zones';
