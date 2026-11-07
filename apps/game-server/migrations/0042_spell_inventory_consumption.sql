-- A spell grants more than one distinct ItemInstance in one transaction;
-- per-item custody remains unique, while legacy one-item TRANSFER retains its
-- own receipt/source guards. Migration0031 already removed the older
-- transaction-only constraint; add the exact per-item source uniqueness.
ALTER TABLE game_item_container_entries ADD CONSTRAINT game_item_container_entry_source_unique UNIQUE(placed_transaction_id,item_instance_id);
-- LOCAL CANDIDATE SPELL-INVENTORY-1. Extends the actual Item owner, not a
-- second inventory. Every consumption/contained retirement shares the original
-- caster's source receipt, current physical XID and quantity/state successor.
DO $$ DECLARE c RECORD; BEGIN
 FOR c IN SELECT conname FROM pg_constraint WHERE conrelid='game_spell_item_lines'::regclass AND contype='c'
  AND pg_get_constraintdef(oid) LIKE '%operation_kind%' AND pg_get_constraintdef(oid) NOT LIKE '%expires_at_unix_ms%'
 LOOP EXECUTE format('ALTER TABLE game_spell_item_lines DROP CONSTRAINT %I',c.conname); END LOOP;
END $$;
ALTER TABLE game_spell_item_lines
 ADD COLUMN source_parent_item_instance_id UUID REFERENCES game_item_instances(item_instance_id),
 ADD COLUMN source_placement_ordinal NUMERIC(20,0),
 ADD COLUMN source_equipment_slot SMALLINT,
 ADD COLUMN source_equipment_revision NUMERIC(20,0),
 ADD COLUMN source_corpse_root UUID REFERENCES game_item_instances(item_instance_id),
 ADD COLUMN source_entry_is_corpse BOOLEAN,
 ADD CONSTRAINT spell_item_operation_kind CHECK(operation_kind IN(1,2,3,4,5,6)),
 ADD CONSTRAINT spell_item_destination_ordinal CHECK(destination_ordinal IS NULL OR (operation_kind IN(1,4) AND destination_ordinal BETWEEN 1 AND 18446744073709551615)),
 ADD CONSTRAINT spell_item_operation_shape CHECK(
   (operation_kind=1 AND quantity_before=0 AND quantity_after>0 AND state_revision_before=0 AND source_stack_ordinal IS NULL)
   OR (operation_kind IN(2,3) AND quantity_before>0 AND quantity_after=0 AND state_revision_before>0 AND source_stack_ordinal>0)
   OR (operation_kind=4 AND quantity_before>0 AND quantity_after>quantity_before AND state_revision_before>0 AND source_stack_ordinal IS NULL AND destination_ordinal IS NOT NULL)
   OR (operation_kind=5 AND quantity_before>quantity_after AND state_revision_before>0 AND source_stack_ordinal IS NULL)
   OR (operation_kind=6 AND quantity_before>0 AND quantity_after=0 AND state_revision_before>0 AND source_stack_ordinal IS NULL)),
 ADD CONSTRAINT spell_item_consumption_custody CHECK((
   (operation_kind NOT IN(5,6) AND num_nonnulls(source_parent_item_instance_id,source_placement_ordinal,source_equipment_slot,source_equipment_revision,source_corpse_root,source_entry_is_corpse)=0)
   OR (operation_kind=5 AND source_corpse_root IS NULL AND source_entry_is_corpse IS NULL AND destination_parent_item_instance_id IS NULL
     AND ((source_parent_item_instance_id IS NOT NULL AND source_placement_ordinal BETWEEN 1 AND 18446744073709551615 AND source_equipment_slot IS NULL AND source_equipment_revision IS NULL)
       OR (source_parent_item_instance_id IS NULL AND source_placement_ordinal IS NULL AND source_equipment_slot BETWEEN 1 AND 10 AND source_equipment_slot<>9 AND source_equipment_revision BETWEEN 1 AND 18446744073709551614)))
   OR (operation_kind=6 AND source_parent_item_instance_id IS NOT NULL AND source_placement_ordinal BETWEEN 1 AND 18446744073709551615
       AND source_equipment_slot IS NULL AND source_equipment_revision IS NULL AND source_corpse_root IS NOT NULL AND source_entry_is_corpse IS NOT NULL
       AND source_corpse_root<>item_instance_id AND destination_parent_item_instance_id IS NULL)) IS TRUE);
CREATE TABLE game_spell_equipment_consumptions (
 transaction_id UUID PRIMARY KEY REFERENCES game_spell_item_receipts(transaction_id),
 character_id UUID NOT NULL REFERENCES game_character_equipment_state(character_id),
 revision_before NUMERIC(20,0) NOT NULL CHECK(revision_before>0),
 revision_after NUMERIC(20,0) NOT NULL CHECK(revision_after=revision_before+1 AND revision_after<=18446744073709551615),
 combat_mode SMALLINT CHECK(combat_mode IN(1,2,3)),
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id()
);
CREATE TABLE game_spell_item_source_custody (
 transaction_id UUID NOT NULL REFERENCES game_spell_item_receipts(transaction_id),
 item_instance_id UUID NOT NULL REFERENCES game_item_instances(item_instance_id),
 quantity_before BIGINT NOT NULL CHECK(quantity_before>0),
 state_revision_before NUMERIC(20,0) NOT NULL CHECK(state_revision_before>0),
 source_parent UUID, source_ordinal NUMERIC(20,0), source_slot SMALLINT,
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
 PRIMARY KEY(transaction_id,item_instance_id)
);
CREATE FUNCTION game_spell_item_capture_custody() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE l game_spell_item_lines%ROWTYPE; r game_spell_item_receipts%ROWTYPE;
BEGIN
 SELECT * INTO l FROM game_spell_item_lines WHERE transaction_id=NEW.last_transaction_id AND item_instance_id=NEW.item_instance_id AND operation_kind IN(5,6) AND created_xact_id=pg_current_xact_id();
 IF NOT FOUND THEN RETURN NEW; END IF;
 SELECT * INTO r FROM game_spell_item_receipts WHERE transaction_id=l.transaction_id AND created_xact_id=pg_current_xact_id();
 IF NOT FOUND OR OLD.lifecycle<>1 OR OLD.world_id<>r.world_id OR OLD.state_revision<>l.state_revision_before
   OR OLD.quantity<>l.quantity_before OR NEW.quantity<>l.quantity_after OR (NEW.definition_family,NEW.definition_production_key,NEW.definition_revision_ref)<>(l.definition_family,l.definition_production_key,l.definition_revision) THEN
  RAISE EXCEPTION 'spell consumption captures exact actual OLD item state only' USING ERRCODE='23514';
 END IF;
 IF l.operation_kind=5 THEN
  IF l.source_equipment_slot IS NOT NULL THEN
   IF NOT EXISTS(SELECT 1 FROM game_character_equipment_slots s JOIN game_character_equipment_state es USING(character_id)
     WHERE s.item_instance_id=OLD.item_instance_id AND s.character_id=r.character_id AND s.world_id=r.world_id
       AND s.slot=l.source_equipment_slot AND es.revision=l.source_equipment_revision) THEN
    RAISE EXCEPTION 'spell consumption source equipment custody changed' USING ERRCODE='23514';
   END IF;
  ELSIF NOT EXISTS(SELECT 1 FROM game_item_container_entries e WHERE e.item_instance_id=OLD.item_instance_id AND e.character_id=r.character_id AND e.world_id=r.world_id AND e.parent_item_instance_id=l.source_parent_item_instance_id AND e.placement_ordinal=l.source_placement_ordinal) THEN
   RAISE EXCEPTION 'spell consumption source inventory custody changed' USING ERRCODE='23514';
  END IF;
 ELSE
  IF NOT ((l.source_entry_is_corpse AND EXISTS(SELECT 1 FROM game_item_corpse_container_entries e WHERE e.item_instance_id=OLD.item_instance_id AND e.world_id=r.world_id AND e.parent_item_instance_id=l.source_parent_item_instance_id AND e.placement_ordinal=l.source_placement_ordinal))
     OR (NOT l.source_entry_is_corpse AND EXISTS(SELECT 1 FROM game_item_container_entries e WHERE e.item_instance_id=OLD.item_instance_id AND e.world_id=r.world_id AND e.parent_item_instance_id=l.source_parent_item_instance_id AND e.placement_ordinal=l.source_placement_ordinal))) THEN
   RAISE EXCEPTION 'contained spell retirement source custody changed' USING ERRCODE='23514';
  END IF;
  IF NOT EXISTS(SELECT 1 FROM game_item_ground_locations g JOIN game_item_instances i USING(item_instance_id) JOIN game_item_mint_receipts m ON m.item_instance_id=i.item_instance_id
      WHERE g.item_instance_id=l.source_corpse_root AND i.lifecycle=1 AND m.loot_purpose_key='CORPSE_MATERIALIZATION'
       AND (g.world_id,g.channel_id,g.runtime_scope_ownership_generation)=(r.world_id,r.channel_id,r.ownership_generation)
       AND (g.spatial_position,g.map_revision,g.content_revision,g.native_room_placement_context)=(l.spatial_position,l.map_revision,l.content_revision,l.placement_context)) THEN
   RAISE EXCEPTION 'contained retirement root lacks actual current corpse placement' USING ERRCODE='23514';
  END IF;
 END IF;
 INSERT INTO game_spell_item_source_custody(transaction_id,item_instance_id,quantity_before,state_revision_before,source_parent,source_ordinal,source_slot)
 VALUES(l.transaction_id,OLD.item_instance_id,OLD.quantity,OLD.state_revision,l.source_parent_item_instance_id,l.source_placement_ordinal,l.source_equipment_slot);
 RETURN NEW;
END $$;
CREATE TRIGGER game_spell_item_capture_custody BEFORE UPDATE ON game_item_instances FOR EACH ROW EXECUTE FUNCTION game_spell_item_capture_custody();
CREATE TRIGGER game_spell_item_source_custody_immutable BEFORE UPDATE OR DELETE ON game_spell_item_source_custody FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_spell_item_source_custody_no_truncate BEFORE TRUNCATE ON game_spell_item_source_custody FOR EACH STATEMENT EXECUTE FUNCTION game_item_immutable();
REVOKE ALL ON game_spell_item_source_custody FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
GRANT SELECT ON game_spell_item_source_custody TO oteryn_game_runtime,oteryn_game_control;
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_spell_item_capture_custody() SET search_path = pg_catalog, %I, pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_spell_item_capture_custody() FROM PUBLIC;
CREATE FUNCTION game_spell_equipment_consumption_proven() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NEW.created_xact_id<>pg_current_xact_id() OR NOT EXISTS(
   SELECT 1 FROM game_spell_item_receipts r JOIN game_character_equipment_state s ON s.character_id=r.character_id
    WHERE r.transaction_id=NEW.transaction_id AND r.character_id=NEW.character_id AND r.created_xact_id=pg_current_xact_id()
     AND s.revision=NEW.revision_after AND s.last_transaction_id=NEW.transaction_id AND s.combat_mode IS NOT DISTINCT FROM NEW.combat_mode
     AND EXISTS(SELECT 1 FROM game_spell_item_lines l WHERE l.transaction_id=r.transaction_id AND l.operation_kind=5
       AND l.source_equipment_slot IS NOT NULL AND l.source_equipment_revision=NEW.revision_before AND l.created_xact_id=pg_current_xact_id())) THEN
  RAISE EXCEPTION 'equipped spell consumption requires actual source lines and exact state epoch' USING ERRCODE='23514';
 END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_spell_equipment_consumption_proven AFTER INSERT ON game_spell_equipment_consumptions
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_spell_equipment_consumption_proven();
CREATE TRIGGER game_spell_equipment_consumption_immutable BEFORE UPDATE OR DELETE ON game_spell_equipment_consumptions
 FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_spell_equipment_consumption_no_truncate BEFORE TRUNCATE ON game_spell_equipment_consumptions
 FOR EACH STATEMENT EXECUTE FUNCTION game_item_immutable();
CREATE OR REPLACE FUNCTION game_spell_item_line_proven() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.operation_kind IN(5,6) THEN
      IF NOT EXISTS(SELECT 1 FROM game_spell_item_source_custody c WHERE c.transaction_id=NEW.transaction_id AND c.item_instance_id=NEW.item_instance_id
        AND c.quantity_before=NEW.quantity_before AND c.state_revision_before=NEW.state_revision_before
        AND c.source_parent IS NOT DISTINCT FROM NEW.source_parent_item_instance_id AND c.source_ordinal IS NOT DISTINCT FROM NEW.source_placement_ordinal
        AND c.source_slot IS NOT DISTINCT FROM NEW.source_equipment_slot AND c.created_xact_id=pg_current_xact_id()) THEN
        RAISE EXCEPTION 'spell source line lacks actual OLD custody capture' USING ERRCODE='23514';
      END IF;
      IF NOT EXISTS(SELECT 1 FROM game_spell_item_receipts r JOIN game_item_instances i ON i.item_instance_id=NEW.item_instance_id
        WHERE r.transaction_id=NEW.transaction_id AND r.created_xact_id=pg_current_xact_id() AND NEW.created_xact_id=pg_current_xact_id()
         AND NEW.ordinal<=r.operation_count AND (NEW.world_id,NEW.channel_id)=(r.world_id,r.channel_id)
         AND (i.world_id,i.definition_family,i.definition_production_key,i.definition_revision_ref)=(NEW.world_id,NEW.definition_family,NEW.definition_production_key,NEW.definition_revision)
         AND i.last_transaction_id=NEW.transaction_id AND i.state_revision=NEW.state_revision_before+1 AND i.quantity=NEW.quantity_after
         AND i.lifecycle=CASE WHEN NEW.quantity_after=0 THEN 2 ELSE 1 END
         AND EXISTS(SELECT 1 FROM game_item_transfer_quantity_evidence q WHERE q.item_instance_id=i.item_instance_id AND q.transaction_id=NEW.transaction_id AND q.quantity_before=NEW.quantity_before)
         AND NOT EXISTS(SELECT 1 FROM game_item_ground_locations g WHERE g.item_instance_id=i.item_instance_id)
         AND ((NEW.quantity_after=0 AND NOT EXISTS(SELECT 1 FROM game_item_container_entries e WHERE e.item_instance_id=i.item_instance_id OR e.parent_item_instance_id=i.item_instance_id)
             AND NOT EXISTS(SELECT 1 FROM game_item_corpse_container_entries e WHERE e.item_instance_id=i.item_instance_id OR e.parent_item_instance_id=i.item_instance_id)
             AND NOT EXISTS(SELECT 1 FROM game_character_equipment_slots e WHERE e.item_instance_id=i.item_instance_id))
           OR (NEW.quantity_after>0 AND ((NEW.source_equipment_slot IS NOT NULL AND EXISTS(SELECT 1 FROM game_character_equipment_slots e WHERE e.character_id=r.character_id AND e.slot=NEW.source_equipment_slot AND e.item_instance_id=i.item_instance_id AND e.world_id=r.world_id))
             OR (NEW.source_equipment_slot IS NULL AND EXISTS(SELECT 1 FROM game_item_container_entries e WHERE e.character_id=r.character_id AND e.item_instance_id=i.item_instance_id AND e.parent_item_instance_id=NEW.source_parent_item_instance_id AND e.placement_ordinal=NEW.source_placement_ordinal AND e.world_id=r.world_id)))))) THEN
        RAISE EXCEPTION 'inventory spell source line lacks exact current item successor/custody' USING ERRCODE='23514';
      END IF;
      IF NEW.operation_kind=5 AND NEW.source_equipment_slot IS NOT NULL AND NOT EXISTS(
         SELECT 1 FROM game_spell_equipment_consumptions e WHERE e.transaction_id=NEW.transaction_id AND e.revision_before=NEW.source_equipment_revision AND e.created_xact_id=pg_current_xact_id()) THEN
        RAISE EXCEPTION 'equipped consumption requires receipt-linked equipment epoch' USING ERRCODE='23514';
      END IF;
      IF NEW.operation_kind=6 AND NOT EXISTS(
         WITH RECURSIVE chain(item,parent,path) AS (
           SELECT NEW.item_instance_id,NEW.source_parent_item_instance_id,ARRAY[NEW.item_instance_id]
           UNION ALL SELECT p.item_instance_id,p.source_parent_item_instance_id,c.path||p.item_instance_id
             FROM chain c JOIN game_spell_item_lines p ON p.transaction_id=NEW.transaction_id AND p.item_instance_id=c.parent AND p.operation_kind=6
             WHERE NOT p.item_instance_id=ANY(c.path) AND cardinality(c.path)<500 AND p.created_xact_id=pg_current_xact_id()
         ) SELECT 1 FROM chain WHERE parent=NEW.source_corpse_root) THEN
        RAISE EXCEPTION 'contained retirement must reach the actual root without cycles' USING ERRCODE='23514';
      END IF;
      IF NEW.operation_kind=6 AND NOT EXISTS(
         SELECT 1 FROM game_spell_item_lines root JOIN game_item_mint_receipts m ON m.item_instance_id=root.item_instance_id
          WHERE root.transaction_id=NEW.transaction_id AND root.item_instance_id=NEW.source_corpse_root AND root.operation_kind IN(2,3)
           AND root.created_xact_id=pg_current_xact_id() AND m.loot_purpose_key='CORPSE_MATERIALIZATION'
           AND (root.world_id,root.channel_id,root.spatial_position,root.map_revision,root.content_revision,root.placement_context)
             =(NEW.world_id,NEW.channel_id,NEW.spatial_position,NEW.map_revision,NEW.content_revision,NEW.placement_context)
           AND (NEW.source_parent_item_instance_id=root.item_instance_id OR EXISTS(SELECT 1 FROM game_spell_item_lines parent WHERE parent.transaction_id=NEW.transaction_id AND parent.operation_kind=6 AND parent.item_instance_id=NEW.source_parent_item_instance_id AND parent.source_corpse_root=NEW.source_corpse_root AND parent.created_xact_id=pg_current_xact_id()))) THEN
        RAISE EXCEPTION 'contained retirement requires its same-cast real corpse root/ancestor chain' USING ERRCODE='23514';
      END IF;
      RETURN NULL;
    END IF;

    IF NOT EXISTS(SELECT 1 FROM game_spell_item_receipts r JOIN game_item_instances i ON i.item_instance_id=NEW.item_instance_id
        WHERE r.transaction_id=NEW.transaction_id AND r.created_xact_id=pg_current_xact_id()
          AND NEW.created_xact_id=pg_current_xact_id() AND NEW.ordinal<=r.operation_count
          AND (NEW.world_id,NEW.channel_id)=(r.world_id,r.channel_id)
          AND (i.world_id,i.definition_family,i.definition_production_key,i.definition_revision_ref)=(NEW.world_id,NEW.definition_family,NEW.definition_production_key,NEW.definition_revision)
          AND ((NEW.operation_kind=1 AND i.lifecycle=1 AND i.quantity=NEW.quantity_after
                AND i.minted_transaction_id=NEW.transaction_id AND i.state_revision=1
                AND ((NEW.destination_parent_item_instance_id IS NULL AND EXISTS(SELECT 1 FROM game_item_ground_locations g WHERE g.item_instance_id=i.item_instance_id
                    AND (g.world_id,g.channel_id,g.spatial_position,g.map_revision,g.content_revision,g.native_room_placement_context)
                      =(NEW.world_id,NEW.channel_id,NEW.spatial_position,NEW.map_revision,NEW.content_revision,NEW.placement_context)
                    AND g.runtime_scope_ownership_generation=r.ownership_generation AND g.stack_ordinal IS NOT NULL))
                  OR (NEW.destination_parent_item_instance_id IS NOT NULL AND EXISTS(SELECT 1 FROM game_item_container_entries e WHERE e.item_instance_id=i.item_instance_id AND e.character_id=r.character_id AND e.parent_item_instance_id=NEW.destination_parent_item_instance_id AND e.placement_ordinal=NEW.destination_ordinal AND e.placed_transaction_id=NEW.transaction_id)))
                AND NOT (EXISTS(SELECT 1 FROM game_item_ground_locations g WHERE g.item_instance_id=i.item_instance_id) AND EXISTS(SELECT 1 FROM game_item_container_entries e WHERE e.item_instance_id=i.item_instance_id)))
            OR (NEW.operation_kind=4 AND i.lifecycle=1 AND i.last_transaction_id=NEW.transaction_id
                AND i.quantity=NEW.quantity_after AND i.state_revision=NEW.state_revision_before+1
                AND EXISTS(SELECT 1 FROM game_item_transfer_quantity_evidence q WHERE q.item_instance_id=i.item_instance_id AND q.transaction_id=NEW.transaction_id AND q.quantity_before=NEW.quantity_before)
                AND EXISTS(SELECT 1 FROM game_item_container_entries e WHERE e.item_instance_id=i.item_instance_id AND e.character_id=r.character_id AND e.parent_item_instance_id=NEW.destination_parent_item_instance_id AND e.placement_ordinal=NEW.destination_ordinal)
                AND NOT EXISTS(SELECT 1 FROM game_item_ground_locations g WHERE g.item_instance_id=i.item_instance_id))
            OR (NEW.operation_kind IN(2,3) AND i.lifecycle=2 AND i.last_transaction_id=NEW.transaction_id
                AND i.quantity=0 AND i.state_revision=NEW.state_revision_before+1
                AND EXISTS(SELECT 1 FROM game_item_transfer_quantity_evidence q WHERE q.item_instance_id=i.item_instance_id AND q.transaction_id=NEW.transaction_id AND q.quantity_before=NEW.quantity_before)
                AND NOT EXISTS(SELECT 1 FROM game_item_ground_locations g WHERE g.item_instance_id=i.item_instance_id)
                AND NOT EXISTS(SELECT 1 FROM game_item_corpse_container_entries e WHERE e.parent_item_instance_id=i.item_instance_id)))) THEN
        RAISE EXCEPTION 'spell item line does not prove its actual item/custody successor' USING ERRCODE='23514';
    END IF;
    IF NEW.operation_kind=3 AND NOT EXISTS(SELECT 1 FROM game_item_mint_receipts m
       WHERE m.item_instance_id=NEW.item_instance_id AND m.loot_purpose_key='CORPSE_MATERIALIZATION') THEN
        RAISE EXCEPTION 'Animate Dead consumes only an actual materialized corpse' USING ERRCODE='23514';
    END IF;
    IF NEW.operation_kind=3 AND NOT EXISTS(SELECT 1 FROM game_spell_companion_acquisition_receipts c WHERE c.transaction_id=NEW.transaction_id AND c.corpse_item_instance_id=NEW.item_instance_id AND c.corpse_state_revision=NEW.state_revision_before AND c.created_xact_id=pg_current_xact_id()) THEN RAISE EXCEPTION 'corpse consumption requires its prepared actual companion acquisition receipt' USING ERRCODE='23514'; END IF;
    RETURN NULL;
END $$;
CREATE OR REPLACE FUNCTION game_item_container_entry_removal_proven() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF EXISTS(SELECT 1 FROM game_spell_item_lines l JOIN game_spell_item_receipts r USING(transaction_id) JOIN game_item_instances i ON i.item_instance_id=l.item_instance_id
      WHERE l.operation_kind IN(5,6) AND l.item_instance_id=OLD.item_instance_id AND l.quantity_after=0
       AND l.source_parent_item_instance_id=OLD.parent_item_instance_id AND l.source_placement_ordinal=OLD.placement_ordinal
       AND i.last_transaction_id=l.transaction_id AND i.lifecycle=2 AND i.quantity=0 AND i.state_revision=l.state_revision_before+1
       AND (l.operation_kind=6 OR r.character_id=OLD.character_id)
       AND l.created_xact_id=pg_current_xact_id() AND r.created_xact_id=pg_current_xact_id()) THEN RETURN NULL; END IF;

    IF EXISTS(SELECT 1 FROM game_character_equipment_receipts r JOIN game_item_instances i USING(item_instance_id)
       WHERE r.operation=1 AND r.to_slot IS NOT NULL AND r.item_instance_id=OLD.item_instance_id
         AND r.character_id=OLD.character_id AND r.backpack_item_instance_id=OLD.parent_item_instance_id
         AND r.backpack_ordinal=OLD.placement_ordinal AND i.last_transaction_id=r.transaction_id
         AND i.state_revision=r.state_revision_before+1 AND r.created_xact_id=pg_current_xact_id()) THEN
        RETURN NULL;
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM game_item_fee_burn_lines l
          JOIN game_item_fee_burns f ON f.transaction_id = l.transaction_id
          JOIN game_item_instances i ON i.item_instance_id = l.item_instance_id
         WHERE l.item_instance_id = OLD.item_instance_id
           AND l.quantity_after = 0
           AND l.placement_ordinal = OLD.placement_ordinal
           AND f.character_id = OLD.character_id
           AND f.backpack_item_instance_id = OLD.parent_item_instance_id
           AND i.last_transaction_id = l.transaction_id
           AND l.created_xact_id = pg_current_xact_id()
           AND f.created_xact_id = pg_current_xact_id()) THEN
        RAISE EXCEPTION 'backpack entry removal must commit with its fee BURN line'
            USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END;
$$;
CREATE OR REPLACE FUNCTION game_item_corpse_entry_removal_proven() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF EXISTS(SELECT 1 FROM game_spell_item_lines l JOIN game_spell_item_receipts r USING(transaction_id) JOIN game_item_instances i ON i.item_instance_id=l.item_instance_id
      WHERE l.operation_kind IN(5,6) AND l.item_instance_id=OLD.item_instance_id AND l.quantity_after=0
       AND l.source_parent_item_instance_id=OLD.parent_item_instance_id AND l.source_placement_ordinal=OLD.placement_ordinal
       AND i.last_transaction_id=l.transaction_id AND i.lifecycle=2 AND i.quantity=0 AND i.state_revision=l.state_revision_before+1
       AND l.operation_kind=6 AND l.source_entry_is_corpse
       AND l.created_xact_id=pg_current_xact_id() AND r.created_xact_id=pg_current_xact_id()) THEN RETURN NULL; END IF;

    IF NOT EXISTS (
        SELECT 1 FROM game_item_transfer_receipts r
          JOIN game_item_instances i ON i.item_instance_id = r.source_item_instance_id
         WHERE r.source_item_instance_id = OLD.item_instance_id
           AND i.last_transaction_id = r.transaction_id
           AND r.created_xact_id = pg_current_xact_id())
       AND NOT EXISTS (
        SELECT 1 FROM game_item_decay_retire_receipts d
          JOIN game_item_instances i ON i.item_instance_id = d.item_instance_id
         WHERE d.item_instance_id = OLD.item_instance_id
           AND d.corpse_item_instance_id = OLD.parent_item_instance_id
           AND d.item_instance_id <> d.corpse_item_instance_id
           AND i.last_transaction_id = d.transaction_id
           AND d.created_xact_id = pg_current_xact_id()) THEN
        RAISE EXCEPTION 'corpse container entry removal must commit with its TRANSFER receipt'
            USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END;
$$;
CREATE OR REPLACE FUNCTION game_equipment_state_proven() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF EXISTS(SELECT 1 FROM game_spell_equipment_consumptions r WHERE r.transaction_id=NEW.last_transaction_id
      AND r.character_id=NEW.character_id AND r.revision_before=OLD.revision AND r.revision_after=NEW.revision
      AND OLD.combat_mode IS NOT DISTINCT FROM NEW.combat_mode AND r.combat_mode IS NOT DISTINCT FROM NEW.combat_mode
      AND r.created_xact_id=pg_current_xact_id()) THEN RETURN NULL; END IF;

    IF NOT EXISTS(SELECT 1 FROM game_character_equipment_receipts r
        WHERE r.transaction_id=NEW.last_transaction_id AND r.character_id=NEW.character_id
          AND r.revision_after=NEW.revision AND r.revision_before=OLD.revision
          AND r.mode_before IS NOT DISTINCT FROM OLD.combat_mode
          AND r.mode_after IS NOT DISTINCT FROM NEW.combat_mode
          AND r.created_xact_id=pg_current_xact_id()) THEN
        RAISE EXCEPTION 'equipment successor lacks same-transaction receipt' USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END $$;
CREATE OR REPLACE FUNCTION game_equipment_slot_proven() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP='DELETE' AND EXISTS(SELECT 1 FROM game_spell_item_lines l JOIN game_spell_item_receipts r USING(transaction_id)
      JOIN game_spell_equipment_consumptions c USING(transaction_id) JOIN game_item_instances i ON i.item_instance_id=l.item_instance_id
      WHERE l.operation_kind=5 AND l.quantity_after=0 AND l.item_instance_id=OLD.item_instance_id AND l.source_equipment_slot=OLD.slot
       AND l.source_equipment_revision=c.revision_before AND c.character_id=OLD.character_id AND r.character_id=OLD.character_id
       AND r.world_id=OLD.world_id AND i.lifecycle=2 AND i.quantity=0 AND i.last_transaction_id=l.transaction_id
       AND i.state_revision=l.state_revision_before+1 AND l.created_xact_id=pg_current_xact_id()
       AND r.created_xact_id=pg_current_xact_id() AND c.created_xact_id=pg_current_xact_id()) THEN RETURN NULL; END IF;

    IF TG_OP='UPDATE' THEN RAISE EXCEPTION 'equipment slot updates require delete/insert' USING ERRCODE='23514'; END IF;
    IF TG_OP='INSERT' THEN
        IF NOT EXISTS(SELECT 1 FROM game_character_equipment_receipts r JOIN game_item_instances i USING(item_instance_id)
            WHERE r.transaction_id=NEW.placed_transaction_id AND r.character_id=NEW.character_id
              AND r.item_instance_id=NEW.item_instance_id AND r.to_slot=NEW.slot AND r.operation=1
              AND r.world_id=NEW.world_id AND i.world_id=NEW.world_id AND i.lifecycle=1
              AND i.last_transaction_id=r.transaction_id AND i.state_revision=r.state_revision_before+1
              AND r.created_xact_id=pg_current_xact_id()) THEN
            RAISE EXCEPTION 'equipment insertion lacks actual movement receipt' USING ERRCODE='23514';
        END IF;
    ELSIF NOT EXISTS(SELECT 1 FROM game_character_equipment_receipts r
        WHERE r.character_id=OLD.character_id AND r.item_instance_id=OLD.item_instance_id
          AND r.from_slot=OLD.slot AND r.operation=1 AND r.created_xact_id=pg_current_xact_id()) THEN
        RAISE EXCEPTION 'equipment removal lacks actual movement receipt' USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END $$;
DO $$ DECLARE n TEXT; BEGIN
 FOREACH n IN ARRAY ARRAY['game_spell_equipment_consumption_proven','game_spell_item_line_proven','game_item_container_entry_removal_proven','game_item_corpse_entry_removal_proven','game_equipment_state_proven','game_equipment_slot_proven'] LOOP
  EXECUTE format('ALTER FUNCTION %I() SET search_path = pg_catalog, %I, pg_temp',n,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %I() FROM PUBLIC',n);
 END LOOP;
END $$;
REVOKE ALL ON game_spell_equipment_consumptions FROM PUBLIC;
GRANT SELECT,INSERT ON game_spell_equipment_consumptions TO oteryn_game_runtime;
GRANT SELECT ON game_spell_equipment_consumptions TO oteryn_game_control;

-- All existing corpse-entry writers share the physical root's Channel owner
-- lock. Locking only present children cannot protect against an absent-row
-- insertion; the source root scope lock closes that phantom race.
CREATE FUNCTION game_spell_corpse_entry_owner_lock() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE p UUID; w UUID; c UUID;
BEGIN
 IF TG_OP='UPDATE' THEN RAISE EXCEPTION 'corpse entry relink requires source-bound delete/insert' USING ERRCODE='23514'; END IF;
 IF TG_OP='INSERT' THEN p:=NEW.parent_item_instance_id; ELSE p:=OLD.parent_item_instance_id; END IF;
 WITH RECURSIVE ancestors(id,path) AS (
   SELECT p,ARRAY[p]
   UNION ALL SELECT e.parent_item_instance_id,a.path||e.parent_item_instance_id FROM ancestors a
    JOIN game_item_corpse_container_entries e ON e.item_instance_id=a.id
    WHERE NOT e.parent_item_instance_id=ANY(a.path) AND cardinality(a.path)<500
 ) SELECT g.world_id,g.channel_id INTO w,c FROM ancestors a JOIN game_item_ground_locations g ON g.item_instance_id=a.id;
 IF w IS NULL OR c IS NULL THEN RAISE EXCEPTION 'corpse custody mutation lacks a qualified physical Ground root' USING ERRCODE='23514'; END IF;
 PERFORM pg_advisory_xact_lock(hashtextextended(encode(uuid_send(w),'hex')||encode(uuid_send(c),'hex'),33));
 IF TG_OP='DELETE' THEN RETURN OLD; END IF; RETURN NEW;
END $$;
CREATE TRIGGER game_spell_corpse_entry_owner_lock BEFORE INSERT OR DELETE OR UPDATE ON game_item_corpse_container_entries
 FOR EACH ROW EXECUTE FUNCTION game_spell_corpse_entry_owner_lock();
DO $$ BEGIN EXECUTE format('ALTER FUNCTION game_spell_corpse_entry_owner_lock() SET search_path = pg_catalog, %I, pg_temp',current_schema()); END $$;
REVOKE ALL ON FUNCTION game_spell_corpse_entry_owner_lock() FROM PUBLIC;

-- Preserve0034 branches, resolve table-specific NEW fields only for entries.
CREATE OR REPLACE FUNCTION game_item_placement_proven() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, public, pg_temp AS $$
BEGIN
    IF TG_TABLE_NAME='game_item_container_entries' THEN
    IF EXISTS(
       SELECT 1 FROM game_character_equipment_receipts r JOIN game_item_instances i USING(item_instance_id)
       WHERE r.operation=1 AND r.from_slot IS NOT NULL AND r.item_instance_id=NEW.item_instance_id
         AND r.transaction_id=NEW.placed_transaction_id AND r.character_id=NEW.character_id
         AND r.backpack_item_instance_id=NEW.parent_item_instance_id AND r.backpack_ordinal=NEW.placement_ordinal
         AND i.last_transaction_id=r.transaction_id AND i.state_revision=r.state_revision_before+1
         AND i.lifecycle=1 AND r.created_xact_id=pg_current_xact_id()) THEN
       PERFORM 1 FROM game_item_container_slots WHERE item_instance_id=NEW.parent_item_instance_id FOR UPDATE;
       IF (SELECT count(*) FROM game_item_container_entries e WHERE e.parent_item_instance_id=NEW.parent_item_instance_id)>20 THEN
           RAISE EXCEPTION 'equipment backpack entry ceiling exceeded' USING ERRCODE='23514';
       END IF;
       RETURN NULL;
    END IF;
    IF EXISTS(SELECT 1 FROM game_spell_item_lines l JOIN game_spell_item_receipts r USING(transaction_id)
       WHERE l.operation_kind=1 AND l.item_instance_id=NEW.item_instance_id AND l.transaction_id=NEW.placed_transaction_id
        AND l.destination_parent_item_instance_id=NEW.parent_item_instance_id AND l.destination_ordinal=NEW.placement_ordinal
        AND r.character_id=NEW.character_id AND r.created_xact_id=pg_current_xact_id() AND l.created_xact_id=pg_current_xact_id()) THEN
       PERFORM 1 FROM game_item_container_slots WHERE item_instance_id=NEW.parent_item_instance_id FOR UPDATE;
       IF (SELECT count(*) FROM game_item_container_entries e WHERE e.parent_item_instance_id=NEW.parent_item_instance_id)>20 THEN RAISE EXCEPTION 'container entry ceiling exceeded' USING ERRCODE='23514'; END IF;
       RETURN NULL;
    END IF;
    END IF;
    IF TG_TABLE_NAME = 'game_item_container_slots' THEN
        IF NOT EXISTS (
            SELECT 1 FROM game_item_transfer_receipts r
             WHERE r.transaction_id = NEW.placed_transaction_id
               AND r.character_id = NEW.character_id
               AND r.source_item_instance_id = NEW.item_instance_id
               AND r.shape = 1
               AND r.created_xact_id = pg_current_xact_id()
               AND EXISTS (SELECT 1 FROM game_item_instances i
                            WHERE i.item_instance_id = NEW.item_instance_id AND i.lifecycle = 1)) THEN
            RAISE EXCEPTION 'item placement must commit with its TRANSFER receipt'
                USING ERRCODE = '23514';
        END IF;
    ELSIF TG_TABLE_NAME = 'game_item_container_entries' THEN
        IF NOT EXISTS (
            SELECT 1 FROM game_item_transfer_receipts r
             WHERE r.transaction_id = NEW.placed_transaction_id
               AND r.character_id = NEW.character_id
               AND r.source_item_instance_id = NEW.item_instance_id
               AND r.shape IN (2,4)
               AND r.destination_parent_item_instance_id = NEW.parent_item_instance_id
               AND r.destination_ordinal = NEW.placement_ordinal
               AND r.created_xact_id = pg_current_xact_id()
               AND EXISTS (SELECT 1 FROM game_item_instances i
                            WHERE i.item_instance_id = NEW.item_instance_id AND i.lifecycle = 1))
           AND NOT EXISTS (
            SELECT 1 FROM game_reward_claim_mint_receipts r
             WHERE r.transaction_id = NEW.placed_transaction_id
               AND r.character_id = NEW.character_id
               AND r.item_instance_id = NEW.item_instance_id
               AND r.destination_parent_item_instance_id = NEW.parent_item_instance_id
               AND r.destination_ordinal = NEW.placement_ordinal
               AND r.created_xact_id = pg_current_xact_id()
               AND EXISTS (SELECT 1 FROM game_item_instances i
                            WHERE i.item_instance_id = NEW.item_instance_id AND i.lifecycle = 1))
           AND NOT EXISTS (
            SELECT 1 FROM game_item_fee_burns f
             WHERE f.transaction_id = NEW.placed_transaction_id
               AND f.character_id = NEW.character_id
               AND f.backpack_item_instance_id = NEW.parent_item_instance_id
               AND NEW.item_instance_id IN (f.change_platinum_item_instance_id,
                                            f.change_gold_item_instance_id)
               AND f.created_xact_id = pg_current_xact_id()
               AND EXISTS (SELECT 1 FROM game_item_instances i
                            WHERE i.item_instance_id = NEW.item_instance_id AND i.lifecycle = 1)) THEN
            RAISE EXCEPTION 'item placement must commit with its TRANSFER, reward-claim MINT or fee change receipt'
                USING ERRCODE = '23514';
        END IF;
        PERFORM 1 FROM game_item_container_slots
         WHERE item_instance_id = NEW.parent_item_instance_id FOR UPDATE;
        IF (SELECT count(*) FROM game_item_container_entries e
             WHERE e.parent_item_instance_id = NEW.parent_item_instance_id) > 20 THEN
            RAISE EXCEPTION 'container entry ceiling exceeded' USING ERRCODE = '23514';
        END IF;
    END IF;
    RETURN NULL;
END;
$$;
DO $$ BEGIN EXECUTE format('ALTER FUNCTION game_item_placement_proven() SET search_path = pg_catalog, %I, pg_temp',current_schema()); END $$;
REVOKE ALL ON FUNCTION game_item_placement_proven() FROM PUBLIC;
