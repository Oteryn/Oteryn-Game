-- SRVGAP-C1. Crystal 201189f8 send_first_items.lua:106-125, vocation_door.lua:26-36,
-- dawnport.lua:145-171. Preserve the original backpack proof and extend its exact item set.
ALTER TABLE game_character_starter_grants
 ADD COLUMN loadout JSONB NOT NULL DEFAULT '[]' CHECK(jsonb_typeof(loadout)='array' AND jsonb_array_length(loadout)<=20),
 ADD COLUMN equipment_revision_before NUMERIC(20,0) CHECK(equipment_revision_before BETWEEN 1 AND 18446744073709551614);

CREATE TABLE game_dawnport_departures (
 character_id UUID PRIMARY KEY REFERENCES game_character_roots(character_id),
 transaction_id UUID UNIQUE NOT NULL CHECK(game_character_is_uuid_v7(transaction_id)),
 event_id UUID UNIQUE NOT NULL CHECK(game_character_is_uuid_v7(event_id)),
 world_id UUID NOT NULL,
 game_session_id UUID NOT NULL,
 command_id NUMERIC(20,0) NOT NULL CHECK(command_id BETWEEN 1 AND 18446744073709551615),
 vocation TEXT NOT NULL CHECK(vocation IN('sorcerer','druid','paladin','knight')),
 destination INTEGER[] NOT NULL,
 character_lease_generation NUMERIC(20,0) NOT NULL CHECK(character_lease_generation>0),
 write_sequence NUMERIC(20,0) NOT NULL CHECK(write_sequence>0),
 equipment_revision_before NUMERIC(20,0),
 removed JSONB NOT NULL CHECK(jsonb_typeof(removed)='array' AND jsonb_array_length(removed)<=2048),
 content_digest BYTEA NOT NULL CHECK(octet_length(content_digest)=32),
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
 CHECK(destination=CASE vocation WHEN 'sorcerer' THEN ARRAY[32054,31883,6]
  WHEN 'druid' THEN ARRAY[32073,31883,6] WHEN 'paladin' THEN ARRAY[32059,31883,6]
  WHEN 'knight' THEN ARRAY[32068,31883,6] END)
);
REVOKE ALL ON game_dawnport_departures FROM PUBLIC;
GRANT SELECT,INSERT ON game_dawnport_departures TO oteryn_game_runtime;
GRANT SELECT ON game_dawnport_departures TO oteryn_game_control;
CREATE TRIGGER game_dawnport_departures_immutable BEFORE UPDATE OR DELETE ON game_dawnport_departures
 FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_dawnport_departures_no_truncate BEFORE TRUNCATE ON game_dawnport_departures
 FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();

-- A narrow same-transaction proof arm. The receipt's deferred trigger proves EVERY member;
-- no existing custody, location-exclusivity or legacy receipt arm is removed.
CREATE FUNCTION game_starting_item_current(p_item UUID) RETURNS BOOLEAN
LANGUAGE sql STABLE SECURITY DEFINER AS $$
 SELECT EXISTS(SELECT 1 FROM game_character_starter_grants r,
  LATERAL jsonb_array_elements(r.loadout) l WHERE r.created_xact_id=pg_current_xact_id()
   AND (l->>'item')::uuid=p_item AND r.outcome=1)
 OR EXISTS(SELECT 1 FROM game_dawnport_departures r,
  LATERAL jsonb_array_elements(r.removed) l WHERE r.created_xact_id=pg_current_xact_id()
   AND (l->'before'->>'item_instance_id')::uuid=p_item)
$$;

CREATE FUNCTION game_starting_loadout_proven() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE l JSONB; i game_item_instances%ROWTYPE; n BIGINT;
BEGIN
 IF NEW.loadout='[]'::jsonb THEN RETURN NULL; END IF;
 IF NEW.outcome<>1 OR NEW.template_key<>'oteryn:starter-template.main'
  OR NEW.created_xact_id<>pg_current_xact_id()
  OR (convert_from(NEW.grant_envelope,'UTF8')::jsonb->'loadout') IS DISTINCT FROM NEW.loadout
  OR (SELECT count(DISTINCT (v->>'item')::uuid) FROM jsonb_array_elements(NEW.loadout) v)<>jsonb_array_length(NEW.loadout)
 THEN RAISE EXCEPTION 'invalid starter loadout receipt' USING ERRCODE='23514'; END IF;
 FOR l IN SELECT * FROM jsonb_array_elements(NEW.loadout) LOOP
  SELECT * INTO i FROM game_item_instances WHERE item_instance_id=(l->>'item')::uuid;
  IF NOT FOUND OR i.world_id<>NEW.world_id OR i.lifecycle<>1 OR i.quantity<>(l->>'quantity')::bigint
   OR i.definition_family IS DISTINCT FROM l->>'family'
   OR i.definition_production_key IS DISTINCT FROM l->>'key'
   OR i.definition_revision_ref IS DISTINCT FROM l->>'revision'
   OR i.state_revision<>1 OR i.last_transaction_id IS NOT NULL OR i.minted_transaction_id<>NEW.transaction_id
  THEN RAISE EXCEPTION 'starter loadout item differs' USING ERRCODE='23514'; END IF;
  IF (l->>'slot')::int=0 THEN
   IF NOT EXISTS(SELECT 1 FROM game_item_container_entries e WHERE e.item_instance_id=i.item_instance_id
    AND e.character_id=NEW.character_id AND e.world_id=NEW.world_id
    AND e.parent_item_instance_id=NEW.item_instance_id AND e.placement_ordinal=(l->>'ordinal')::numeric
    AND e.placed_transaction_id=NEW.transaction_id)
   THEN RAISE EXCEPTION 'starter loadout backpack entry differs' USING ERRCODE='23514'; END IF;
  ELSIF NOT EXISTS(SELECT 1 FROM game_character_equipment_slots e WHERE e.item_instance_id=i.item_instance_id
    AND e.character_id=NEW.character_id AND e.world_id=NEW.world_id AND e.slot=(l->>'slot')::smallint
    AND e.placed_transaction_id=NEW.transaction_id) THEN
   RAISE EXCEPTION 'starter loadout equipment differs' USING ERRCODE='23514';
  END IF;
 END LOOP;
 SELECT count(*) INTO n FROM game_item_instances WHERE minted_transaction_id=NEW.transaction_id;
 IF n<>jsonb_array_length(NEW.loadout)+1 OR
  (EXISTS(SELECT 1 FROM jsonb_array_elements(NEW.loadout) member WHERE (member->>'slot')::int<>0)
    IS DISTINCT FROM (NEW.equipment_revision_before IS NOT NULL))
  OR (NEW.equipment_revision_before IS NOT NULL AND NOT EXISTS(
   SELECT 1 FROM game_character_equipment_state e WHERE e.character_id=NEW.character_id
    AND e.revision=NEW.equipment_revision_before+1 AND e.last_transaction_id=NEW.transaction_id))
 THEN RAISE EXCEPTION 'starter loadout incomplete' USING ERRCODE='23514'; END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_starting_loadout_proven AFTER INSERT ON game_character_starter_grants
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_starting_loadout_proven();

-- The set of prohibited ids is the donor's complete list, not an appearance or name match.
CREATE FUNCTION game_dawnport_prohibited(p_key TEXT) RETURNS BOOLEAN
LANGUAGE sql IMMUTABLE AS $$
 SELECT p_key=ANY(ARRAY['oteryn:item.tibia.i3355','oteryn:item.tibia.i3562','oteryn:item.tibia.i3559',
  'oteryn:item.tibia.i3552','oteryn:item.tibia.i21348','oteryn:item.tibia.i21350','oteryn:item.tibia.i21400',
  'oteryn:item.tibia.i3350','oteryn:item.tibia.i3267','oteryn:item.tibia.i3412','oteryn:item.tibia.i35562',
  'oteryn:item.tibia.i21470','oteryn:item.tibia.i266','oteryn:item.tibia.i268','oteryn:item.tibia.i7876',
  'oteryn:item.tibia.i21352','oteryn:item.tibia.i21351'])
$$;

-- Removing a container removes its contents too (Crystal src/game/game.cpp:2587-2636).
-- Start from actual equipped roots; a dropped backpack's old child character_id is not custody.
CREATE FUNCTION game_dawnport_carried_items(p_character UUID) RETURNS TABLE(item UUID,cleanup BOOLEAN)
LANGUAGE sql STABLE SECURITY DEFINER AS $$
 WITH RECURSIVE roots AS (
  SELECT item_instance_id FROM game_item_container_slots WHERE character_id=p_character
  UNION ALL SELECT item_instance_id FROM game_character_equipment_slots WHERE character_id=p_character
 ), carried(item,cleanup,path) AS (
  SELECT i.item_instance_id,game_dawnport_prohibited(i.definition_production_key),ARRAY[i.item_instance_id]
   FROM roots r JOIN game_item_instances i USING(item_instance_id) WHERE i.lifecycle=1
  UNION ALL
  SELECT i.item_instance_id,c.cleanup OR game_dawnport_prohibited(i.definition_production_key),c.path||i.item_instance_id
   FROM carried c JOIN LATERAL (
    SELECT item_instance_id FROM game_item_container_entries WHERE parent_item_instance_id=c.item
    UNION ALL SELECT item_instance_id FROM game_item_nested_entries WHERE parent_item_instance_id=c.item
   ) e ON true JOIN game_item_instances i USING(item_instance_id)
   WHERE i.lifecycle=1 AND cardinality(c.path)<17 AND NOT i.item_instance_id=ANY(c.path)
 ) SELECT item,cleanup FROM carried
$$;

CREATE FUNCTION game_dawnport_departure_admit() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE l JSONB; i game_item_instances%ROWTYPE; actual JSONB;
BEGIN
 IF NEW.created_xact_id<>pg_current_xact_id()
  OR (EXISTS(SELECT 1 FROM jsonb_array_elements(NEW.removed) member WHERE member->>'kind'='equipment')
      IS DISTINCT FROM (NEW.equipment_revision_before IS NOT NULL))
  OR (NEW.equipment_revision_before IS NOT NULL AND NOT EXISTS(SELECT 1 FROM game_character_equipment_state e
      WHERE e.character_id=NEW.character_id AND e.revision=NEW.equipment_revision_before))
  OR NOT EXISTS(SELECT 1 FROM game_character_build_receipts b WHERE b.character_id=NEW.character_id
   AND b.cause='vocation_choice' AND b.vocation_after=NEW.vocation)
  OR NOT EXISTS(SELECT 1 FROM game_character_build_state b WHERE b.character_id=NEW.character_id AND b.vocation=NEW.vocation)
  OR (SELECT count(DISTINCT v->'before'->>'item_instance_id') FROM jsonb_array_elements(NEW.removed) v)<>jsonb_array_length(NEW.removed)
 THEN RAISE EXCEPTION 'Dawnport requires the confirmed vocation' USING ERRCODE='23514'; END IF;
 FOR l IN SELECT * FROM jsonb_array_elements(NEW.removed) LOOP
  SELECT * INTO i FROM game_item_instances WHERE item_instance_id=(l->'before'->>'item_instance_id')::uuid FOR UPDATE;
  IF NOT FOUND OR to_jsonb(i) IS DISTINCT FROM l->'before' OR i.lifecycle<>1 OR i.world_id<>NEW.world_id
   OR NOT EXISTS(SELECT 1 FROM game_dawnport_carried_items(NEW.character_id) c WHERE c.item=i.item_instance_id AND c.cleanup)
  THEN RAISE EXCEPTION 'Dawnport cleanup predecessor differs' USING ERRCODE='23514'; END IF;
  IF l->>'kind'='equipment' THEN
   SELECT to_jsonb(e) INTO actual FROM game_character_equipment_slots e
    WHERE e.item_instance_id=i.item_instance_id AND e.character_id=NEW.character_id;
  ELSIF l->>'kind'='entry' THEN
   SELECT to_jsonb(e) INTO actual FROM game_item_container_entries e
    WHERE e.item_instance_id=i.item_instance_id AND e.character_id=NEW.character_id;
  ELSIF l->>'kind'='nested' THEN
   SELECT to_jsonb(e) INTO actual FROM game_item_nested_entries e
    WHERE e.item_instance_id=i.item_instance_id;
  ELSE RAISE EXCEPTION 'invalid Dawnport cleanup custody' USING ERRCODE='23514'; END IF;
  IF actual IS NULL OR actual IS DISTINCT FROM l->'location' THEN
   RAISE EXCEPTION 'Dawnport cleanup source differs' USING ERRCODE='23514'; END IF;
 END LOOP;
 RETURN NEW;
END $$;
CREATE TRIGGER game_dawnport_departure_admit BEFORE INSERT ON game_dawnport_departures
 FOR EACH ROW EXECUTE FUNCTION game_dawnport_departure_admit();

CREATE FUNCTION game_dawnport_departure_proven() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE l JSONB; i game_item_instances%ROWTYPE;
BEGIN
 FOR l IN SELECT * FROM jsonb_array_elements(NEW.removed) LOOP
  SELECT * INTO i FROM game_item_instances WHERE item_instance_id=(l->'before'->>'item_instance_id')::uuid;
  IF NOT FOUND OR i.lifecycle<>2 OR i.quantity<>0
   OR i.state_revision<>(l->'before'->>'state_revision')::numeric+1 OR i.last_transaction_id IS DISTINCT FROM NEW.transaction_id
   OR (to_jsonb(i)-'quantity'-'lifecycle'-'state_revision'-'last_transaction_id') IS DISTINCT FROM
      ((l->'before')-'quantity'-'lifecycle'-'state_revision'-'last_transaction_id')
   OR EXISTS(SELECT 1 FROM game_character_equipment_slots e WHERE e.item_instance_id=i.item_instance_id)
   OR EXISTS(SELECT 1 FROM game_item_container_entries e WHERE e.item_instance_id=i.item_instance_id)
   OR EXISTS(SELECT 1 FROM game_item_nested_entries e WHERE e.item_instance_id=i.item_instance_id)
  THEN RAISE EXCEPTION 'Dawnport cleanup successor differs' USING ERRCODE='23514'; END IF;
 END LOOP;
 IF NOT EXISTS(SELECT 1 FROM game_character_last_positions p WHERE p.character_id=NEW.character_id
   AND p.world_id=NEW.world_id AND ARRAY[p.tile_x,p.tile_y,p.tile_floor::integer]=NEW.destination
   AND p.character_lease_generation=NEW.character_lease_generation AND p.write_sequence=NEW.write_sequence)
  OR (NEW.equipment_revision_before IS NOT NULL AND NOT EXISTS(
   SELECT 1 FROM game_character_equipment_state e WHERE e.character_id=NEW.character_id
    AND e.revision=NEW.equipment_revision_before+1 AND e.last_transaction_id=NEW.transaction_id))
  OR EXISTS(SELECT 1 FROM game_dawnport_carried_items(NEW.character_id) c WHERE c.cleanup)
 THEN RAISE EXCEPTION 'Dawnport departure incomplete' USING ERRCODE='23514'; END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_dawnport_departure_proven AFTER INSERT ON game_dawnport_departures
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_dawnport_departure_proven();

-- Equipment placements now have two exact proof owners; the existing deferred slot proof
-- continues to enforce the old receipt FK semantics for every non-starter insertion.
ALTER TABLE game_character_equipment_slots DROP CONSTRAINT game_character_equipment_slots_placed_transaction_id_fkey;

DO $migration$
DECLARE name TEXT; body TEXT; arm TEXT; start_at INTEGER;
BEGIN
 FOREACH name IN ARRAY ARRAY['game_item_instance_guard','game_item_instance_change_proven',
  'game_item_mint_consistency_guard','game_item_placement_proven','game_equipment_slot_proven',
  'game_item_container_entry_removal_proven','game_item_nested_entry_proven'] LOOP
  body:=pg_get_functiondef((name||'()')::regprocedure);
  start_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
  IF start_at=0 THEN RAISE EXCEPTION 'starting progression guard entry missing: %',name; END IF;
  arm:=$arm$
 IF TG_OP='DELETE' THEN
  IF game_starting_item_current(OLD.item_instance_id) THEN RETURN OLD; END IF;
 ELSE
  IF game_starting_item_current(NEW.item_instance_id) THEN RETURN NEW; END IF;
 END IF;
$arm$;
  EXECUTE overlay(body placing E'BEGIN\n'||arm from start_at for 5);
 END LOOP;
 body:=pg_get_functiondef('game_equipment_state_proven()'::regprocedure);
 start_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
 IF start_at=0 THEN RAISE EXCEPTION 'starting equipment proof entry missing'; END IF;
 arm:=$arm$
 IF NEW.revision=OLD.revision+1 AND NEW.combat_mode IS NOT DISTINCT FROM OLD.combat_mode AND (
  EXISTS(SELECT 1 FROM game_character_starter_grants r WHERE r.character_id=NEW.character_id
   AND r.transaction_id=NEW.last_transaction_id AND r.equipment_revision_before=OLD.revision AND r.created_xact_id=pg_current_xact_id())
  OR EXISTS(SELECT 1 FROM game_dawnport_departures r WHERE r.character_id=NEW.character_id
   AND r.transaction_id=NEW.last_transaction_id AND r.equipment_revision_before=OLD.revision AND r.created_xact_id=pg_current_xact_id()))
 THEN RETURN NULL; END IF;
$arm$;
 EXECUTE overlay(body placing E'BEGIN\n'||arm from start_at for 5);
 FOREACH name IN ARRAY ARRAY['game_starting_item_current(uuid)','game_starting_loadout_proven()',
  'game_dawnport_prohibited(text)','game_dawnport_carried_items(uuid)','game_dawnport_departure_admit()','game_dawnport_departure_proven()'] LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',name,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC',name);
 END LOOP;
END $migration$;
GRANT EXECUTE ON FUNCTION game_starting_item_current(UUID),game_dawnport_prohibited(TEXT),game_dawnport_carried_items(UUID) TO oteryn_game_runtime;
