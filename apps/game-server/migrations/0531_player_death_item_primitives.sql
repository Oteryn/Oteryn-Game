-- DEATH-DUR-1 / death-complete0 §4.4: two one-item primitives. Selection is DEATH-3.
-- A container keeps its direct entries by identity when it leaves the main-backpack slot.
-- No child row is rewritten, split, or expanded into an item line. Depth remains one.
DO $$ DECLARE fk TEXT; BEGIN
 SELECT conname INTO STRICT fk FROM pg_constraint
  WHERE conrelid='game_item_container_entries'::regclass
    AND confrelid='game_item_container_slots'::regclass AND contype='f';
 EXECUTE format('ALTER TABLE game_item_container_entries DROP CONSTRAINT %I',fk);
END $$;
ALTER TABLE game_item_container_entries ADD CONSTRAINT game_item_container_entries_parent_identity
 FOREIGN KEY(parent_item_instance_id,world_id) REFERENCES game_item_instances(item_instance_id,world_id);
CREATE INDEX game_item_container_entries_character ON game_item_container_entries(character_id);

CREATE TABLE game_player_death_item_records (
 death_occurrence_id UUID NOT NULL REFERENCES game_character_death_receipts(death_occurrence_id),
 item_instance_id UUID NOT NULL REFERENCES game_item_instances(item_instance_id),
 operation SMALLINT NOT NULL CHECK(operation IN(1,2)), -- TRANSFER, typed amulet BURN
 transaction_id UUID NOT NULL UNIQUE CHECK(game_character_is_uuid_v7(transaction_id)),
 event_id UUID NOT NULL UNIQUE CHECK(game_character_is_uuid_v7(event_id)),
 character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
 world_id UUID NOT NULL CHECK(game_character_is_uuid_v7(world_id)),
 channel_id UUID NOT NULL CHECK(game_character_is_uuid_v7(channel_id)),
 scope_generation NUMERIC(20,0) NOT NULL CHECK(scope_generation BETWEEN 1 AND 18446744073709551615),
 source_slot SMALLINT NOT NULL CHECK(source_slot BETWEEN 0 AND 10),
 source_parent UUID REFERENCES game_item_instances(item_instance_id),
 source_ordinal NUMERIC(20,0) CHECK(source_ordinal BETWEEN 1 AND 18446744073709551615),
 source_placement UUID NOT NULL CHECK(game_character_is_uuid_v7(source_placement)),
 quantity_before BIGINT NOT NULL CHECK(quantity_before BETWEEN 1 AND 100),
 state_revision_before NUMERIC(20,0) NOT NULL CHECK(state_revision_before BETWEEN 1 AND 18446744073709551614),
 equipment_revision_before NUMERIC(20,0) CHECK(equipment_revision_before BETWEEN 1 AND 18446744073709551614),
 request_binding BYTEA NOT NULL CHECK(octet_length(request_binding)=32),
 occurred_at BIGINT NOT NULL CHECK(occurred_at>0),
 envelope_sha256 BYTEA NOT NULL CHECK(octet_length(envelope_sha256)=32),
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
 PRIMARY KEY(death_occurrence_id,item_instance_id),
 CHECK((source_slot=0)=(source_parent IS NOT NULL AND source_ordinal IS NOT NULL)),
 CHECK(source_slot=0 OR (source_parent IS NULL AND source_ordinal IS NULL)),
 CHECK((source_slot NOT IN(0,9))=(equipment_revision_before IS NOT NULL)),
 CHECK(operation=1 OR (source_slot=7 AND quantity_before=1))
);
CREATE INDEX game_player_death_item_records_item ON game_player_death_item_records(item_instance_id);
CREATE INDEX game_player_death_item_records_character ON game_player_death_item_records(character_id);
CREATE INDEX game_player_death_item_records_parent ON game_player_death_item_records(source_parent);
-- Private trigger-captured proof of the actual custody removed, never runtime-authored.
CREATE TABLE game_player_death_item_removals (
 transaction_id UUID PRIMARY KEY REFERENCES game_player_death_item_records(transaction_id),
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id()
);
REVOKE ALL ON game_player_death_item_records,game_player_death_item_removals FROM PUBLIC;
GRANT SELECT,INSERT ON game_player_death_item_records TO oteryn_game_runtime;
GRANT SELECT ON game_player_death_item_removals TO oteryn_game_runtime;
GRANT SELECT ON game_player_death_item_records,game_player_death_item_removals TO oteryn_game_control;

CREATE FUNCTION game_player_death_item_admit() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE i game_item_instances%ROWTYPE;
BEGIN
 SELECT * INTO i FROM game_item_instances WHERE item_instance_id=NEW.item_instance_id FOR UPDATE;
 IF NOT FOUND OR i.lifecycle<>1 OR i.world_id<>NEW.world_id OR i.quantity<>NEW.quantity_before
  OR i.state_revision<>NEW.state_revision_before
  OR NOT EXISTS(SELECT 1 FROM game_character_death_receipts d WHERE d.death_occurrence_id=NEW.death_occurrence_id
    AND d.character_id=NEW.character_id AND d.death_world_id=NEW.world_id AND d.death_channel_id=NEW.channel_id)
  OR (NEW.source_slot=0 AND NOT EXISTS(SELECT 1 FROM game_item_container_entries e
    JOIN game_item_container_slots s ON s.item_instance_id=e.parent_item_instance_id AND s.character_id=e.character_id
    WHERE e.item_instance_id=NEW.item_instance_id AND e.character_id=NEW.character_id AND e.world_id=NEW.world_id
     AND e.parent_item_instance_id=NEW.source_parent AND e.placement_ordinal=NEW.source_ordinal
     AND e.placed_transaction_id=NEW.source_placement))
  OR (NEW.source_slot=9 AND NOT EXISTS(SELECT 1 FROM game_item_container_slots s WHERE s.item_instance_id=NEW.item_instance_id
    AND s.character_id=NEW.character_id AND s.world_id=NEW.world_id AND s.placed_transaction_id=NEW.source_placement))
  OR (NEW.source_slot NOT IN(0,9) AND NOT EXISTS(SELECT 1 FROM game_character_equipment_slots s
    JOIN game_character_equipment_state e USING(character_id) WHERE s.item_instance_id=NEW.item_instance_id
    AND s.character_id=NEW.character_id AND s.world_id=NEW.world_id AND s.slot=NEW.source_slot
    AND s.placed_transaction_id=NEW.source_placement AND e.revision=NEW.equipment_revision_before))
  OR EXISTS(SELECT 1 FROM game_item_nested_entries WHERE parent_item_instance_id=NEW.item_instance_id)
  OR EXISTS(SELECT 1 FROM game_item_container_entries e JOIN game_item_nested_entries n ON n.parent_item_instance_id=e.item_instance_id
    WHERE e.parent_item_instance_id=NEW.item_instance_id)
  OR EXISTS(SELECT 1 FROM game_item_corpse_container_entries WHERE parent_item_instance_id=NEW.item_instance_id)
  OR (NEW.source_slot<>9 AND EXISTS(SELECT 1 FROM game_item_container_entries WHERE parent_item_instance_id=NEW.item_instance_id)) THEN
  RAISE EXCEPTION 'death primitive lacks exact live item, receipt, direct custody or depth-one shape' USING ERRCODE='23514';
 END IF;
 NEW.created_xact_id:=pg_current_xact_id();
 RETURN NEW;
END $$;

CREATE FUNCTION game_player_death_item_complete(p_tx UUID) RETURNS void LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE r game_player_death_item_records%ROWTYPE;
BEGIN
 SELECT * INTO STRICT r FROM game_player_death_item_records WHERE transaction_id=p_tx AND created_xact_id=pg_current_xact_id();
 IF NOT EXISTS(SELECT 1 FROM game_player_death_item_removals WHERE transaction_id=p_tx AND created_xact_id=pg_current_xact_id())
  OR NOT EXISTS(SELECT 1 FROM game_item_instances i JOIN game_item_transfer_quantity_evidence ev USING(item_instance_id)
    WHERE i.item_instance_id=r.item_instance_id AND i.world_id=r.world_id AND i.last_transaction_id=p_tx
     AND i.state_revision=r.state_revision_before+1 AND ev.transaction_id=p_tx AND ev.quantity_before=r.quantity_before
     AND i.quantity=CASE WHEN r.operation=2 THEN 0 ELSE r.quantity_before END
     AND i.lifecycle=CASE WHEN r.operation=2 THEN 2 ELSE 1 END)
  OR NOT EXISTS(SELECT 1 FROM game_item_audit_outbox a WHERE a.transaction_id=p_tx AND a.event_id=r.event_id
    AND a.item_instance_id=r.item_instance_id AND a.event_type_id=2 AND a.transaction_ordinal=1 AND a.transaction_count=1
    AND a.occurred_at=r.occurred_at AND a.envelope_sha256=r.envelope_sha256 AND a.created_xact_id=pg_current_xact_id()
    AND a.publication_state=1 AND a.published_at IS NULL)
  OR (r.source_slot NOT IN(0,9) AND NOT EXISTS(SELECT 1 FROM game_character_equipment_state e
    WHERE e.character_id=r.character_id AND e.revision=r.equipment_revision_before+1 AND e.last_transaction_id=p_tx))
  OR (r.operation=1 AND NOT EXISTS(SELECT 1 FROM game_item_ground_locations g JOIN game_character_death_receipts d
    ON d.death_occurrence_id=r.death_occurrence_id WHERE g.item_instance_id=r.item_instance_id AND g.world_id=r.world_id
    AND g.channel_id=r.channel_id AND g.runtime_scope_ownership_generation=r.scope_generation
    AND g.spatial_position=d.death_spatial_position AND g.map_revision=d.death_map_revision
    AND g.content_revision=d.content_revision AND g.corpse_ref=uuid_send(r.death_occurrence_id)
    AND g.native_room_placement_context=uuid_send(r.death_occurrence_id))) THEN
  RAISE EXCEPTION 'death primitive must commit its item, actual source removal, receipt death cell and audit together' USING ERRCODE='23514';
 END IF;
 PERFORM game_item_location_exclusive(r.item_instance_id);
END $$;
CREATE FUNCTION game_player_death_item_proven() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN PERFORM game_player_death_item_complete(NEW.transaction_id); RETURN NULL; END $$;
CREATE TRIGGER game_player_death_item_admit BEFORE INSERT ON game_player_death_item_records
 FOR EACH ROW EXECUTE FUNCTION game_player_death_item_admit();
CREATE CONSTRAINT TRIGGER game_player_death_item_proven AFTER INSERT ON game_player_death_item_records
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_player_death_item_proven();

-- Extend installed definitions: pg_get_functiondef retains game_world_reset_item_proven
-- and every other statically or dynamically installed proof arm.

DO $migration$
DECLARE body TEXT; begin_at INT;
BEGIN
 body:=pg_get_functiondef('game_item_instance_change_proven()'::regprocedure);
 begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
 IF begin_at=0 THEN RAISE EXCEPTION 'missing guard entry point: game_item_instance_change_proven'; END IF;
 EXECUTE overlay(body placing E'BEGIN\n'||$arm$
 IF EXISTS(SELECT 1 FROM game_player_death_item_records r WHERE r.transaction_id=NEW.last_transaction_id
  AND r.item_instance_id=NEW.item_instance_id AND r.state_revision_before=OLD.state_revision
  AND r.quantity_before=OLD.quantity AND r.created_xact_id=pg_current_xact_id()) THEN
  PERFORM game_player_death_item_complete(NEW.last_transaction_id); RETURN NULL;
 END IF;
$arm$ from begin_at for 5);
END $migration$;

DO $migration$
DECLARE body TEXT; begin_at INT;
BEGIN
 body:=pg_get_functiondef('game_equipment_state_proven()'::regprocedure);
 begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
 IF begin_at=0 THEN RAISE EXCEPTION 'missing guard entry point: game_equipment_state_proven'; END IF;
 EXECUTE overlay(body placing E'BEGIN\n'||$arm$
 IF EXISTS(SELECT 1 FROM game_player_death_item_records r WHERE r.transaction_id=NEW.last_transaction_id
  AND r.character_id=NEW.character_id AND r.source_slot NOT IN(0,9) AND r.equipment_revision_before=OLD.revision
  AND NEW.revision=OLD.revision+1 AND NEW.combat_mode IS NOT DISTINCT FROM OLD.combat_mode
  AND r.created_xact_id=pg_current_xact_id()) THEN RETURN NULL; END IF;
$arm$ from begin_at for 5);
END $migration$;

DO $migration$
DECLARE body TEXT; begin_at INT;
BEGIN
 body:=pg_get_functiondef('game_item_ground_insertion_guard()'::regprocedure);
 begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
 IF begin_at=0 THEN RAISE EXCEPTION 'missing guard entry point: game_item_ground_insertion_guard'; END IF;
 EXECUTE overlay(body placing E'BEGIN\n'||$arm$
 IF EXISTS(SELECT 1 FROM game_player_death_item_records r WHERE r.item_instance_id=NEW.item_instance_id
   AND r.operation=1 AND r.world_id=NEW.world_id AND r.channel_id=NEW.channel_id AND r.created_xact_id=pg_current_xact_id()) THEN
  PERFORM game_player_death_item_complete((SELECT transaction_id FROM game_player_death_item_records
    WHERE item_instance_id=NEW.item_instance_id AND created_xact_id=pg_current_xact_id())); RETURN NULL;
 END IF;
$arm$ from begin_at for 5);
END $migration$;
ALTER FUNCTION game_item_ground_insertion_guard() SECURITY DEFINER;

-- Capture actual OLD custody immediately: deferred consistency checks may run before
-- the existing deferred source-removal guard. Runtime cannot author this evidence.
CREATE FUNCTION game_player_death_item_capture_removal() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 IF TG_TABLE_NAME='game_character_equipment_slots' THEN
  INSERT INTO game_player_death_item_removals(transaction_id)
   SELECT r.transaction_id FROM game_player_death_item_records r
    WHERE r.item_instance_id=OLD.item_instance_id AND r.character_id=OLD.character_id AND r.world_id=OLD.world_id
     AND r.source_slot=OLD.slot AND r.source_placement=OLD.placed_transaction_id
     AND r.created_xact_id=pg_current_xact_id() ON CONFLICT DO NOTHING;
 ELSIF TG_TABLE_NAME='game_item_container_slots' THEN
  INSERT INTO game_player_death_item_removals(transaction_id)
   SELECT r.transaction_id FROM game_player_death_item_records r
    WHERE r.item_instance_id=OLD.item_instance_id AND r.character_id=OLD.character_id AND r.world_id=OLD.world_id
     AND r.source_slot=9 AND r.source_placement=OLD.placed_transaction_id
     AND r.created_xact_id=pg_current_xact_id() ON CONFLICT DO NOTHING;
 ELSIF TG_TABLE_NAME='game_item_container_entries' THEN
  INSERT INTO game_player_death_item_removals(transaction_id)
   SELECT r.transaction_id FROM game_player_death_item_records r
    WHERE r.item_instance_id=OLD.item_instance_id AND r.character_id=OLD.character_id AND r.world_id=OLD.world_id
     AND r.source_slot=0 AND r.source_placement=OLD.placed_transaction_id AND r.source_parent=OLD.parent_item_instance_id AND r.source_ordinal=OLD.placement_ordinal
     AND r.created_xact_id=pg_current_xact_id() ON CONFLICT DO NOTHING;
 END IF;
 RETURN OLD;
END $$;
CREATE TRIGGER game_player_death_item_capture_removal BEFORE DELETE ON game_character_equipment_slots
 FOR EACH ROW EXECUTE FUNCTION game_player_death_item_capture_removal();
CREATE TRIGGER game_player_death_item_capture_removal BEFORE DELETE ON game_item_container_slots
 FOR EACH ROW EXECUTE FUNCTION game_player_death_item_capture_removal();
CREATE TRIGGER game_player_death_item_capture_removal BEFORE DELETE ON game_item_container_entries
 FOR EACH ROW EXECUTE FUNCTION game_player_death_item_capture_removal();

DO $migration$
DECLARE body TEXT; begin_at INT;
BEGIN
 body:=pg_get_functiondef('game_equipment_slot_proven()'::regprocedure);
 begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
 IF begin_at=0 THEN RAISE EXCEPTION 'missing guard entry point: game_equipment_slot_proven'; END IF;
 EXECUTE overlay(body placing E'BEGIN\n'||$arm$
 IF TG_OP='DELETE' THEN
  IF EXISTS(SELECT 1 FROM game_player_death_item_records r JOIN game_player_death_item_removals e USING(transaction_id)
    WHERE r.item_instance_id=OLD.item_instance_id AND r.character_id=OLD.character_id
     AND r.source_slot=OLD.slot AND r.source_placement=OLD.placed_transaction_id
     AND e.created_xact_id=pg_current_xact_id() AND r.created_xact_id=pg_current_xact_id()) THEN RETURN OLD; END IF;
 END IF;
$arm$ from begin_at for 5);
END $migration$;

DO $migration$
DECLARE body TEXT; begin_at INT;
BEGIN
 body:=pg_get_functiondef('game_house_container_slot_removal_proven()'::regprocedure);
 begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
 IF begin_at=0 THEN RAISE EXCEPTION 'missing guard entry point: game_house_container_slot_removal_proven'; END IF;
 EXECUTE overlay(body placing E'BEGIN\n'||$arm$
 IF TG_OP='DELETE' THEN
  IF EXISTS(SELECT 1 FROM game_player_death_item_records r
    WHERE r.item_instance_id=OLD.item_instance_id AND r.character_id=OLD.character_id
     AND r.source_slot=9 AND r.source_placement=OLD.placed_transaction_id
     AND r.created_xact_id=pg_current_xact_id()
     AND (TG_WHEN='BEFORE' OR EXISTS(SELECT 1 FROM game_player_death_item_removals e
       WHERE e.transaction_id=r.transaction_id AND e.created_xact_id=pg_current_xact_id()))) THEN RETURN OLD; END IF;
 END IF;
$arm$ from begin_at for 5);
END $migration$;

DO $migration$
DECLARE body TEXT; begin_at INT;
BEGIN
 body:=pg_get_functiondef('game_item_container_entry_removal_proven()'::regprocedure);
 begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
 IF begin_at=0 THEN RAISE EXCEPTION 'missing guard entry point: game_item_container_entry_removal_proven'; END IF;
 EXECUTE overlay(body placing E'BEGIN\n'||$arm$
 IF TG_OP='DELETE' THEN
  IF EXISTS(SELECT 1 FROM game_player_death_item_records r JOIN game_player_death_item_removals e USING(transaction_id)
    WHERE r.item_instance_id=OLD.item_instance_id AND r.character_id=OLD.character_id
     AND r.source_slot=0 AND r.source_placement=OLD.placed_transaction_id AND r.source_parent=OLD.parent_item_instance_id AND r.source_ordinal=OLD.placement_ordinal
     AND e.created_xact_id=pg_current_xact_id() AND r.created_xact_id=pg_current_xact_id()) THEN RETURN OLD; END IF;
 END IF;
$arm$ from begin_at for 5);
END $migration$;

-- The old FK made character_id an ownership proof. After this storage change, current
-- custody comes from the parent's slot identity; the annotation remains historical metadata.
DO $migration$
DECLARE body TEXT; amended TEXT;
BEGIN
 body:=pg_get_functiondef('game_item_loop_owner(uuid)'::regprocedure);
 amended:=regexp_replace(body,
  'SELECT character_id INTO v_character FROM game_item_container_entries[[:space:]]+WHERE item_instance_id = v_item;',
  'SELECT s.character_id INTO v_character FROM game_item_container_entries e JOIN game_item_container_slots s ON s.item_instance_id=e.parent_item_instance_id WHERE e.item_instance_id=v_item;');
 IF amended=body THEN RAISE EXCEPTION 'missing direct-entry owner query'; END IF;
 EXECUTE amended;
 body:=pg_get_functiondef('game_item_loop_tree_size(uuid)'::regprocedure);
 amended:=replace(body,
  'SELECT item_instance_id, 1 FROM game_item_container_entries WHERE character_id = p_character',
  'SELECT e.item_instance_id, 1 FROM game_item_container_entries e JOIN game_item_container_slots s ON s.item_instance_id=e.parent_item_instance_id WHERE s.character_id=p_character');
 IF amended=body THEN RAISE EXCEPTION 'missing carried tree root query'; END IF;
 EXECUTE amended;
 EXECUTE format('ALTER FUNCTION game_item_loop_owner(UUID) SET search_path = pg_catalog, %I, pg_temp',current_schema());
 EXECUTE format('ALTER FUNCTION game_item_loop_tree_size(UUID) SET search_path = pg_catalog, %I, pg_temp',current_schema());
END $migration$;

CREATE TRIGGER game_player_death_item_records_immutable BEFORE UPDATE OR DELETE ON game_player_death_item_records
 FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_player_death_item_records_no_truncate BEFORE TRUNCATE ON game_player_death_item_records
 FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();

CREATE TRIGGER game_player_death_item_removals_immutable BEFORE UPDATE OR DELETE ON game_player_death_item_removals
 FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_player_death_item_removals_no_truncate BEFORE TRUNCATE ON game_player_death_item_removals
 FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();

CREATE TRIGGER game_player_death_item_removals_stamp BEFORE INSERT ON game_player_death_item_removals
 FOR EACH ROW EXECUTE FUNCTION game_item_stamp_created_xact_id();
-- Re-issued SECURITY DEFINER functions keep the current schema pinned and are not callable
-- as a runtime mutation API. The completion helper is reached only through guarded triggers.
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_player_death_item_capture_removal() SET search_path = pg_catalog, %I, pg_temp',current_schema());
 REVOKE ALL ON FUNCTION game_player_death_item_capture_removal() FROM PUBLIC;
 EXECUTE format('ALTER FUNCTION game_player_death_item_admit() SET search_path = pg_catalog, %I, pg_temp',current_schema());
 REVOKE ALL ON FUNCTION game_player_death_item_admit() FROM PUBLIC;
 EXECUTE format('ALTER FUNCTION game_player_death_item_complete(uuid) SET search_path = pg_catalog, %I, pg_temp',current_schema());
 REVOKE ALL ON FUNCTION game_player_death_item_complete(uuid) FROM PUBLIC;
 EXECUTE format('ALTER FUNCTION game_player_death_item_proven() SET search_path = pg_catalog, %I, pg_temp',current_schema());
 REVOKE ALL ON FUNCTION game_player_death_item_proven() FROM PUBLIC;
 EXECUTE format('ALTER FUNCTION game_item_instance_change_proven() SET search_path = pg_catalog, %I, pg_temp',current_schema());
 REVOKE ALL ON FUNCTION game_item_instance_change_proven() FROM PUBLIC;
 EXECUTE format('ALTER FUNCTION game_equipment_state_proven() SET search_path = pg_catalog, %I, pg_temp',current_schema());
 REVOKE ALL ON FUNCTION game_equipment_state_proven() FROM PUBLIC;
 EXECUTE format('ALTER FUNCTION game_item_ground_insertion_guard() SET search_path = pg_catalog, %I, pg_temp',current_schema());
 REVOKE ALL ON FUNCTION game_item_ground_insertion_guard() FROM PUBLIC;
 EXECUTE format('ALTER FUNCTION game_equipment_slot_proven() SET search_path = pg_catalog, %I, pg_temp',current_schema());
 REVOKE ALL ON FUNCTION game_equipment_slot_proven() FROM PUBLIC;
 EXECUTE format('ALTER FUNCTION game_house_container_slot_removal_proven() SET search_path = pg_catalog, %I, pg_temp',current_schema());
 REVOKE ALL ON FUNCTION game_house_container_slot_removal_proven() FROM PUBLIC;
 EXECUTE format('ALTER FUNCTION game_item_container_entry_removal_proven() SET search_path = pg_catalog, %I, pg_temp',current_schema());
 REVOKE ALL ON FUNCTION game_item_container_entry_removal_proven() FROM PUBLIC;
END $$;
