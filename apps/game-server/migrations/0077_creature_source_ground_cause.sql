-- Same durable Item owner and existing receipt/line/audit tables; no Player impersonation.
-- DRAFT owning-contract expansion: root must independently qualify before activation.
ALTER TABLE game_spell_item_receipts
 ADD COLUMN cause_kind SMALLINT NOT NULL DEFAULT 0 CHECK(cause_kind IN(0,1)),
 ADD COLUMN creature_placement BYTEA,
 ADD COLUMN creature_actor_generation NUMERIC(20,0),
 ADD COLUMN creature_definition_key TEXT,
 ADD COLUMN creature_definition_revision TEXT,
 ADD COLUMN creature_occurrence TEXT,
 ADD COLUMN creature_map_digest BYTEA,
 ADD COLUMN creature_frame_digest BYTEA,
 ADD COLUMN creature_source_body_digest BYTEA,
 ADD COLUMN creature_cast_digest BYTEA;
ALTER TABLE game_spell_item_receipts ALTER COLUMN game_session_id DROP NOT NULL,
 ALTER COLUMN command_id DROP NOT NULL, ALTER COLUMN character_id DROP NOT NULL,
 ALTER COLUMN cost DROP NOT NULL, ALTER COLUMN cost_binding DROP NOT NULL;
-- Only the obsolete Player-only discriminator/cost predicate is replaced. All existing
-- length/hash/operation-count/scope/line/audit constraints remain in force.
DO $$ DECLARE c RECORD; removed INTEGER:=0; BEGIN
 FOR c IN SELECT conname,pg_get_constraintdef(oid) AS def FROM pg_constraint
 WHERE conrelid='game_spell_item_receipts'::regclass AND contype='c' LOOP
  IF c.def LIKE '%spell_family = %' OR c.def LIKE '%game_spell_cost_binding_valid(cost)%' THEN
   EXECUTE format('ALTER TABLE game_spell_item_receipts DROP CONSTRAINT %I',c.conname);
   removed:=removed+1;
  END IF;
 END LOOP;
 IF removed<>2 THEN RAISE EXCEPTION 'unexpected baseline spell receipt discriminator/cost constraints: %',removed; END IF;
END $$;
ALTER TABLE game_spell_item_receipts ADD CONSTRAINT game_spell_item_typed_cause CHECK ((
 (cause_kind=0 AND spell_family='Spell' AND game_session_id IS NOT NULL AND command_id IS NOT NULL
  AND character_id IS NOT NULL AND cost IS NOT NULL AND cost_binding IS NOT NULL
  AND game_spell_cost_binding_valid(cost)
  AND num_nonnulls(creature_placement,creature_actor_generation,creature_definition_key,
      creature_definition_revision,creature_occurrence,creature_map_digest,creature_frame_digest,
      creature_source_body_digest,creature_cast_digest)=0)
 OR
 (cause_kind=1 AND spell_family='Ability' AND game_session_id IS NULL AND command_id IS NULL
  AND character_id IS NULL AND cost IS NULL AND cost_binding IS NULL
  AND caster_lease_generation IS NULL AND caster_placement_digest IS NULL
  AND octet_length(creature_placement)=16 AND creature_placement<>decode(repeat('00',16),'hex')
  AND creature_actor_generation BETWEEN 1 AND 18446744073709551615
  AND octet_length(creature_definition_key) BETWEEN 1 AND 512
  AND octet_length(creature_definition_revision) BETWEEN 1 AND 512
  AND octet_length(creature_occurrence) BETWEEN 1 AND 1024
  AND octet_length(creature_map_digest)=32 AND octet_length(creature_frame_digest)=32
  AND octet_length(creature_source_body_digest)=32 AND octet_length(creature_cast_digest)=32
  AND num_nonnulls(creature_placement,creature_actor_generation,creature_definition_key,
      creature_definition_revision,creature_occurrence,creature_map_digest,creature_frame_digest,
      creature_source_body_digest,creature_cast_digest)=9)
) IS TRUE);
CREATE UNIQUE INDEX game_spell_item_creature_once ON game_spell_item_receipts
 (world_id,channel_id,ownership_generation,creature_placement,creature_actor_generation,
  spell_production_key,spell_revision,creature_occurrence) WHERE cause_kind=1;
-- Immutable source cause is part of the same intent/audit receipt. Ground-only source
-- mutations cannot smuggle Player inventory/corpse/acquisition operations.
CREATE FUNCTION game_spell_creature_cause_proven() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE p JSONB; BEGIN
 IF NEW.cause_kind<>1 THEN RETURN NULL; END IF;
 p:=convert_from(NEW.intent,'UTF8')::jsonb;
 IF p->>'schema' IS DISTINCT FROM 'OTERYN_CREATURE_GROUND_CAUSE/v1'
  OR p->'player' IS DISTINCT FROM 'null'::jsonb OR p->'cost' IS DISTINCT FROM 'null'::jsonb
  OR p->'caster_origin' IS DISTINCT FROM 'null'::jsonb
  OR p->'creature'->>'key' IS DISTINCT FROM NEW.creature_definition_key
  OR p->'creature'->>'revision' IS DISTINCT FROM NEW.creature_definition_revision
  OR p->'creature'->>'generation' IS DISTINCT FROM NEW.creature_actor_generation::text
  OR p->>'world' IS DISTINCT FROM replace(NEW.world_id::text,'-','')
  OR p->>'channel' IS DISTINCT FROM replace(NEW.channel_id::text,'-','')
  OR p->>'ownership_generation' IS DISTINCT FROM NEW.ownership_generation::text
  OR p->'ability'->>'family' IS DISTINCT FROM NEW.spell_family
  OR p->'ability'->>'key' IS DISTINCT FROM NEW.spell_production_key
  OR p->'ability'->>'revision' IS DISTINCT FROM NEW.spell_revision
  OR p->>'occurrence' IS DISTINCT FROM NEW.creature_occurrence
  OR p->'creature'->>'placement' IS DISTINCT FROM encode(NEW.creature_placement,'hex')
  OR p->>'content' IS DISTINCT FROM encode(NEW.catalog_digest,'hex')
  OR p->>'map' IS DISTINCT FROM encode(NEW.creature_map_digest,'hex')
  OR p->>'frame' IS DISTINCT FROM encode(NEW.creature_frame_digest,'hex')
  OR p->>'source_cast' IS DISTINCT FROM encode(NEW.creature_cast_digest,'hex')
  OR p->>'source_body' IS DISTINCT FROM encode(NEW.creature_source_body_digest,'hex')
  OR EXISTS(SELECT 1 FROM game_spell_item_lines l WHERE l.transaction_id=NEW.transaction_id
   AND (l.operation_kind NOT IN(1,2) OR l.destination_parent_item_instance_id IS NOT NULL))
 THEN RAISE EXCEPTION 'Creature source cause must match same ground-only immutable receipt' USING ERRCODE='23514'; END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_spell_creature_cause_proven AFTER INSERT ON game_spell_item_receipts
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_spell_creature_cause_proven();
