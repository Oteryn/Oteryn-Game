-- SOULPIT-1: retain exact input custody while outputs use the existing main-backpack proof.
-- Existing historical receipts remain immutable and keep their original validation semantics.
ALTER TABLE game_item_craft_records
 ADD COLUMN source_custody JSONB,
 ADD COLUMN secondary_custody JSONB,
 ADD COLUMN input_channel_id UUID,
 ADD COLUMN equipment_revision_before NUMERIC(20,0) CHECK(equipment_revision_before BETWEEN 1 AND 18446744073709551614),
 ADD CONSTRAINT game_item_craft_custody_shape CHECK (
   (source_custody IS NULL AND secondary_custody IS NULL AND input_channel_id IS NULL)
   OR (source_custody IS NOT NULL AND jsonb_typeof(source_custody)='object' AND input_channel_id IS NOT NULL
       AND ((secondary_item_instance_id IS NULL AND secondary_custody IS NULL)
            OR (secondary_item_instance_id IS NOT NULL AND secondary_custody IS NOT NULL AND jsonb_typeof(secondary_custody)='object'))));
-- Independent input parents are now captured by their exact custody rows.
ALTER TABLE game_item_craft_records DROP CONSTRAINT game_item_craft_input_shape;
ALTER TABLE game_item_craft_records
 ADD CONSTRAINT game_item_craft_input_shape CHECK (
   (recipe NOT IN ('oteryn:soulpit.fuse','oteryn:soulpit.prism','oteryn:soulpit.prism-ominous','oteryn:soulpit.exalted') AND source_units=1
     AND secondary_item_instance_id IS NULL AND secondary_parent_item_instance_id IS NULL
     AND secondary_ordinal IS NULL AND secondary_before IS NULL)
   OR (recipe IN ('oteryn:soulpit.fuse','oteryn:soulpit.prism','oteryn:soulpit.prism-ominous','oteryn:soulpit.exalted')
     AND jsonb_array_length(outputs)=(CASE WHEN recipe='oteryn:soulpit.prism-ominous' THEN 2 ELSE 1 END)
     AND (outputs->0->>'count') IS NOT NULL AND (outputs->0->>'count')::int=1
     AND (recipe<>'oteryn:soulpit.prism-ominous' OR ((outputs->1->>'count') IS NOT NULL AND (outputs->1->>'count')::int=1)) AND (
       (source_units=2 AND recipe='oteryn:soulpit.fuse' AND secondary_item_instance_id IS NULL
         AND secondary_parent_item_instance_id IS NULL AND secondary_ordinal IS NULL AND secondary_before IS NULL)
       OR (source_units=1 AND secondary_item_instance_id IS NOT NULL
         AND secondary_item_instance_id<>source_item_instance_id
         AND secondary_parent_item_instance_id IS NOT NULL
         AND secondary_ordinal IS NOT NULL AND secondary_ordinal>0
         AND secondary_before IS NOT NULL AND jsonb_typeof(secondary_before)='object'))));

CREATE FUNCTION game_item_craft_custody(p_item UUID) RETURNS JSONB LANGUAGE SQL STABLE AS $$
 SELECT (SELECT location FROM (
   SELECT jsonb_build_object('kind','main','row',to_jsonb(e)) AS location FROM game_item_container_entries e WHERE item_instance_id=p_item
   UNION ALL SELECT jsonb_build_object('kind','nested','row',to_jsonb(e)) FROM game_item_nested_entries e WHERE item_instance_id=p_item
   UNION ALL SELECT jsonb_build_object('kind','equipment','row',to_jsonb(e)) FROM game_character_equipment_slots e WHERE item_instance_id=p_item
   UNION ALL SELECT jsonb_build_object('kind','ground','row',to_jsonb(e)) FROM game_item_ground_locations e WHERE item_instance_id=p_item
 ) locations)
$$;

CREATE FUNCTION game_item_craft_custody_admitted(p_item UUID,p_character UUID,p_world UUID,p_channel UUID,p_location JSONB)
RETURNS BOOLEAN LANGUAGE SQL STABLE AS $$
 SELECT coalesce(p_location=game_item_craft_custody(p_item) AND (
   (p_location->>'kind' IN ('main','nested') AND EXISTS(SELECT 1 FROM game_item_loop_owner(p_item) WHERE owner_character=p_character AND depth BETWEEN 1 AND 8))
   OR (p_location->>'kind'='equipment' AND p_location->'row'->>'character_id'=p_character::text AND p_location->'row'->>'world_id'=p_world::text)
   OR (p_location->>'kind'='ground' AND p_location->'row'->>'world_id'=p_world::text AND p_location->'row'->>'channel_id'=p_channel::text)
 ),false)
$$;

CREATE FUNCTION game_item_craft_custody_complete(p_item UUID,p_location JSONB,p_quantity BIGINT) RETURNS BOOLEAN LANGUAGE SQL STABLE AS $$
 SELECT CASE WHEN p_quantity=0 THEN game_item_craft_custody(p_item) IS NULL
             ELSE game_item_craft_custody(p_item) IS NOT DISTINCT FROM p_location END
$$;

CREATE OR REPLACE FUNCTION game_item_craft_admit() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE i game_item_instances%ROWTYPE; o JSONB;
BEGIN
 SELECT * INTO i FROM game_item_instances WHERE item_instance_id=NEW.source_item_instance_id FOR UPDATE;
 IF i.item_instance_id IS NULL OR to_jsonb(i)<>NEW.before_item OR i.lifecycle<>1
    OR i.quantity<NEW.source_units OR i.world_id<>NEW.world_id OR i.definition_family<>'Item'
    OR EXISTS(SELECT 1 FROM game_item_keys WHERE item_instance_id=i.item_instance_id)
    OR EXISTS(SELECT 1 FROM game_item_timed_states WHERE item_instance_id=i.item_instance_id)
    OR EXISTS(SELECT 1 FROM game_item_texts WHERE item_instance_id=i.item_instance_id)
    OR EXISTS(SELECT 1 FROM game_item_imbuements WHERE item_instance_id=i.item_instance_id)
    OR EXISTS(SELECT 1 FROM game_item_container_entries WHERE parent_item_instance_id=i.item_instance_id)
    OR EXISTS(SELECT 1 FROM game_item_nested_entries WHERE parent_item_instance_id=i.item_instance_id)
    OR (CASE WHEN NEW.source_custody IS NULL THEN NOT EXISTS(SELECT 1 FROM game_item_container_entries e JOIN game_item_container_slots b
                    ON b.character_id=e.character_id AND b.item_instance_id=e.parent_item_instance_id
                   WHERE e.item_instance_id=i.item_instance_id AND e.character_id=NEW.character_id
                     AND e.world_id=NEW.world_id AND b.world_id=NEW.world_id
                     AND e.parent_item_instance_id=NEW.parent_item_instance_id AND e.placement_ordinal=NEW.source_ordinal)
       ELSE NOT game_item_craft_custody_admitted(i.item_instance_id,NEW.character_id,NEW.world_id,NEW.input_channel_id,NEW.source_custody)
         OR NOT EXISTS(SELECT 1 FROM game_item_container_slots WHERE item_instance_id=NEW.parent_item_instance_id AND character_id=NEW.character_id AND world_id=NEW.world_id)
       END)
 THEN RAISE EXCEPTION 'craft requires unchanged plain carried input' USING ERRCODE='23514'; END IF;
 IF (SELECT count(DISTINCT value->>'item') FROM jsonb_array_elements(NEW.outputs))<>jsonb_array_length(NEW.outputs)
    OR (SELECT count(DISTINCT value->>'ordinal') FROM jsonb_array_elements(NEW.outputs))<>jsonb_array_length(NEW.outputs)
 THEN RAISE EXCEPTION 'duplicate craft output' USING ERRCODE='23514'; END IF;
 FOR o IN SELECT value FROM jsonb_array_elements(NEW.outputs) LOOP
  IF NOT game_character_is_uuid_v7((o->>'item')::uuid)
     OR EXISTS(SELECT 1 FROM game_item_instances WHERE item_instance_id=(o->>'item')::uuid)
     OR (o->>'count')::int NOT BETWEEN 1 AND 100
     OR octet_length(o->>'key') NOT BETWEEN 1 AND 512
     OR octet_length(o->>'revision') NOT BETWEEN 1 AND 512
     OR ((o->>'ordinal')::numeric<=(SELECT coalesce(max(placement_ordinal),0) FROM game_item_container_entries WHERE parent_item_instance_id=NEW.parent_item_instance_id)
         AND NOT (i.quantity=NEW.source_units AND o=NEW.outputs->0 AND (NEW.source_custody IS NULL OR (NEW.source_custody->>'kind'='main' AND (NEW.source_custody->'row'->>'parent_item_instance_id')::uuid=NEW.parent_item_instance_id)) AND (o->>'ordinal')::numeric=NEW.source_ordinal))
  THEN RAISE EXCEPTION 'invalid fresh craft output' USING ERRCODE='23514'; END IF;
 END LOOP;
 IF NEW.source_custody->>'kind'='equipment' OR NEW.secondary_custody->>'kind'='equipment' THEN
  SELECT revision INTO STRICT NEW.equipment_revision_before FROM game_character_equipment_state WHERE character_id=NEW.character_id FOR UPDATE;
 ELSE
  NEW.equipment_revision_before:=NULL;
 END IF;
 NEW.created_xact_id:=pg_current_xact_id();
 RETURN NEW;
END $$;

CREATE OR REPLACE FUNCTION game_item_craft_secondary_admit() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE i game_item_instances%ROWTYPE;
BEGIN
 IF NEW.secondary_item_instance_id IS NULL THEN RETURN NEW; END IF;
 SELECT * INTO i FROM game_item_instances WHERE item_instance_id=NEW.secondary_item_instance_id FOR UPDATE;
 IF i.item_instance_id IS NULL OR to_jsonb(i)<>NEW.secondary_before OR i.lifecycle<>1
    OR i.quantity<1 OR i.world_id<>NEW.world_id OR i.definition_family<>'Item'
    OR EXISTS(SELECT 1 FROM game_item_keys WHERE item_instance_id=i.item_instance_id)
    OR EXISTS(SELECT 1 FROM game_item_timed_states WHERE item_instance_id=i.item_instance_id)
    OR EXISTS(SELECT 1 FROM game_item_texts WHERE item_instance_id=i.item_instance_id)
    OR EXISTS(SELECT 1 FROM game_item_imbuements WHERE item_instance_id=i.item_instance_id)
    OR EXISTS(SELECT 1 FROM game_item_container_entries WHERE parent_item_instance_id=i.item_instance_id)
    OR EXISTS(SELECT 1 FROM game_item_nested_entries WHERE parent_item_instance_id=i.item_instance_id)
    OR (CASE WHEN NEW.source_custody IS NULL THEN NOT EXISTS(SELECT 1 FROM game_item_container_entries e JOIN game_item_container_slots b
                    ON b.character_id=e.character_id AND b.item_instance_id=e.parent_item_instance_id
                   WHERE e.item_instance_id=i.item_instance_id AND e.character_id=NEW.character_id
                     AND e.world_id=NEW.world_id AND b.world_id=NEW.world_id
                     AND e.parent_item_instance_id=NEW.secondary_parent_item_instance_id
                     AND e.placement_ordinal=NEW.secondary_ordinal)
       ELSE NOT game_item_craft_custody_admitted(i.item_instance_id,NEW.character_id,NEW.world_id,NEW.input_channel_id,NEW.secondary_custody) END)
 THEN RAISE EXCEPTION 'fusion requires unchanged plain carried target' USING ERRCODE='23514'; END IF;
 RETURN NEW;
END $$;


CREATE OR REPLACE FUNCTION game_item_craft_complete(p_transaction UUID) RETURNS void LANGUAGE plpgsql AS $$
DECLARE r game_item_craft_records%ROWTYPE; i game_item_instances%ROWTYPE; o JSONB;
BEGIN
 SELECT * INTO r FROM game_item_craft_records WHERE transaction_id=p_transaction;
 SELECT * INTO i FROM game_item_instances WHERE item_instance_id=r.source_item_instance_id;
 IF r.transaction_id IS NULL OR r.created_xact_id<>pg_current_xact_id()
    OR i.quantity<>(r.before_item->>'quantity')::bigint-r.source_units
    OR i.lifecycle<>(CASE WHEN i.quantity=0 THEN 2 ELSE 1 END)
    OR i.last_transaction_id IS DISTINCT FROM r.transaction_id
    OR (to_jsonb(i)-ARRAY['quantity','lifecycle','last_transaction_id']) IS DISTINCT FROM
       (r.before_item-ARRAY['quantity','lifecycle','last_transaction_id'])
    OR (CASE WHEN r.source_custody IS NULL THEN ((i.quantity=0 AND EXISTS(SELECT 1 FROM game_item_container_entries WHERE item_instance_id=i.item_instance_id))
    OR (i.quantity>0 AND NOT EXISTS(SELECT 1 FROM game_item_container_entries WHERE item_instance_id=i.item_instance_id AND character_id=r.character_id AND parent_item_instance_id=r.parent_item_instance_id AND placement_ordinal=r.source_ordinal)))
       ELSE NOT game_item_craft_custody_complete(i.item_instance_id,r.source_custody,i.quantity) END)
    OR (SELECT count(*) FROM game_item_instances WHERE minted_transaction_id=r.transaction_id)<>jsonb_array_length(r.outputs)
    OR (SELECT count(*) FROM game_item_instances WHERE last_transaction_id=r.transaction_id)<>(CASE WHEN r.secondary_item_instance_id IS NULL THEN 1 ELSE 2 END)
    OR (SELECT count(*) FROM game_item_container_entries WHERE parent_item_instance_id=r.parent_item_instance_id)>20
    OR (r.equipment_revision_before IS NOT NULL AND NOT EXISTS(SELECT 1 FROM game_character_equipment_state WHERE character_id=r.character_id AND revision=r.equipment_revision_before+1 AND last_transaction_id=r.transaction_id))
    OR (SELECT count(*) FROM game_item_audit_outbox WHERE transaction_id=r.transaction_id)<>1
    OR NOT EXISTS(SELECT 1 FROM game_item_audit_outbox WHERE event_id=r.event_id AND transaction_id=r.transaction_id AND item_instance_id=r.source_item_instance_id
                   AND occurred_at=r.occurred_at AND envelope_sha256=r.envelope_sha256 AND created_xact_id=pg_current_xact_id()
                   AND transaction_ordinal=1 AND transaction_count=1 AND publication_state=1 AND published_at IS NULL)
 THEN RAISE EXCEPTION 'incomplete atomic craft' USING ERRCODE='23514'; END IF;
 PERFORM game_item_craft_secondary_complete(p_transaction);
 FOR o IN SELECT value FROM jsonb_array_elements(r.outputs) LOOP
  IF NOT EXISTS(SELECT 1 FROM game_item_instances n JOIN game_item_container_entries e USING(item_instance_id)
     WHERE n.item_instance_id=(o->>'item')::uuid AND n.world_id=r.world_id AND n.lifecycle=1
       AND n.quantity=(o->>'count')::bigint AND n.definition_family='Item' AND n.definition_production_key=o->>'key'
       AND n.definition_revision_ref=o->>'revision' AND n.minted_transaction_id=r.transaction_id AND n.last_transaction_id IS NULL
       AND e.world_id=r.world_id AND e.character_id=r.character_id AND e.parent_item_instance_id=r.parent_item_instance_id
       AND e.placement_ordinal=(o->>'ordinal')::numeric AND e.placed_transaction_id=r.transaction_id)
  THEN RAISE EXCEPTION 'craft output or custody mismatch' USING ERRCODE='23514'; END IF;
 END LOOP;
END $$;

CREATE OR REPLACE FUNCTION game_item_craft_secondary_complete(p_transaction UUID) RETURNS void LANGUAGE plpgsql AS $$
DECLARE r game_item_craft_records%ROWTYPE; i game_item_instances%ROWTYPE;
BEGIN
 SELECT * INTO r FROM game_item_craft_records WHERE transaction_id=p_transaction;
 IF r.secondary_item_instance_id IS NULL THEN RETURN; END IF;
 SELECT * INTO i FROM game_item_instances WHERE item_instance_id=r.secondary_item_instance_id;
 IF i.item_instance_id IS NULL OR r.created_xact_id<>pg_current_xact_id()
    OR i.quantity<>(r.secondary_before->>'quantity')::bigint-1
    OR i.lifecycle<>(CASE WHEN i.quantity=0 THEN 2 ELSE 1 END)
    OR i.last_transaction_id IS DISTINCT FROM r.transaction_id
    OR (to_jsonb(i)-ARRAY['quantity','lifecycle','last_transaction_id']) IS DISTINCT FROM
       (r.secondary_before-ARRAY['quantity','lifecycle','last_transaction_id'])
    OR (CASE WHEN r.source_custody IS NULL THEN ((i.quantity=0 AND EXISTS(SELECT 1 FROM game_item_container_entries WHERE item_instance_id=i.item_instance_id))
    OR (i.quantity>0 AND NOT EXISTS(SELECT 1 FROM game_item_container_entries
         WHERE item_instance_id=i.item_instance_id AND character_id=r.character_id
           AND parent_item_instance_id=r.secondary_parent_item_instance_id AND placement_ordinal=r.secondary_ordinal)))
       ELSE NOT game_item_craft_custody_complete(i.item_instance_id,r.secondary_custody,i.quantity) END)
 THEN RAISE EXCEPTION 'incomplete atomic fusion target' USING ERRCODE='23514'; END IF;
END $$;


-- Exact old row plus a retired input and the same current transaction proves removal.
CREATE FUNCTION game_item_craft_removal_proven(p_item UUID,p_kind TEXT,p_row JSONB) RETURNS BOOLEAN LANGUAGE SQL STABLE AS $$
 SELECT EXISTS(SELECT 1 FROM game_item_craft_records r JOIN game_item_instances i ON i.item_instance_id=p_item
 WHERE r.created_xact_id=pg_current_xact_id() AND i.lifecycle=2 AND i.quantity=0 AND i.last_transaction_id=r.transaction_id
 AND ((r.source_item_instance_id=p_item AND r.source_custody=jsonb_build_object('kind',p_kind,'row',p_row))
   OR (r.secondary_item_instance_id=p_item AND r.secondary_custody=jsonb_build_object('kind',p_kind,'row',p_row))))
$$;
DO $migration$
DECLARE n TEXT; kind TEXT; body TEXT; arm TEXT; begin_at INT;
BEGIN
 FOR n,kind IN VALUES ('game_item_container_entry_removal_proven','main'),('game_item_nested_entry_proven','nested'),('game_equipment_slot_proven','equipment'),('game_item_ground_removal_proven','ground') LOOP
  body:=pg_get_functiondef((n||'()')::regprocedure);
  begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
  IF begin_at=0 THEN RAISE EXCEPTION 'missing custody proof body: %',n; END IF;
  arm:=format($arm$
   IF TG_OP='DELETE' AND game_item_craft_removal_proven(OLD.item_instance_id,%L,to_jsonb(OLD)) THEN RETURN NULL; END IF;
$arm$,kind);
  EXECUTE overlay(body placing E'BEGIN\n'||arm from begin_at for 5);
 END LOOP;
END $migration$;
DO $migration$
DECLARE body TEXT; begin_at INT;
BEGIN
 body:=pg_get_functiondef('game_equipment_state_proven()'::regprocedure);
 begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
 IF begin_at=0 THEN RAISE EXCEPTION 'missing equipment state proof'; END IF;
 EXECUTE overlay(body placing $arm$BEGIN
  IF EXISTS(SELECT 1 FROM game_item_craft_records r WHERE r.character_id=NEW.character_id
     AND r.transaction_id=NEW.last_transaction_id AND r.equipment_revision_before=OLD.revision
     AND NEW.revision=OLD.revision+1 AND NEW.combat_mode IS NOT DISTINCT FROM OLD.combat_mode
     AND r.created_xact_id=pg_current_xact_id()) THEN
   PERFORM game_item_craft_complete(NEW.last_transaction_id); RETURN NULL;
  END IF;
$arm$ from begin_at for 5);
END $migration$;
DO $$ DECLARE signature TEXT; BEGIN
 FOREACH signature IN ARRAY ARRAY['game_item_craft_custody(uuid)','game_item_craft_custody_admitted(uuid,uuid,uuid,uuid,jsonb)','game_item_craft_custody_complete(uuid,jsonb,bigint)','game_item_craft_removal_proven(uuid,text,jsonb)'] LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',signature,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC',signature);
  EXECUTE format('GRANT EXECUTE ON FUNCTION %s TO oteryn_game_runtime',signature);
 END LOOP;
 FOREACH signature IN ARRAY ARRAY['game_item_craft_admit()','game_item_craft_secondary_admit()','game_item_craft_complete(uuid)','game_item_craft_secondary_complete(uuid)'] LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',signature,current_schema());
 END LOOP;
END $$;
