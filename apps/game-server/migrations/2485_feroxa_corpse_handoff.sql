-- M52: placement generation records history, not the current clear owner.
-- Only the independently fenced Feroxa World-event cause may retire an older
-- Channel corpse placement. Player/Creature consumption still requires equality.
CREATE OR REPLACE FUNCTION game_spell_item_capture_custody() RETURNS trigger
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
  ELSIF NOT EXISTS(SELECT 1 FROM game_item_container_entries e WHERE e.item_instance_id=OLD.item_instance_id AND e.character_id=r.character_id AND e.world_id=r.world_id AND e.parent_item_instance_id=l.source_parent_item_instance_id AND e.placement_ordinal=l.source_placement_ordinal) AND NOT game_quiver_entry(decode(replace(r.character_id::text,'-',''),'hex'),l.source_parent_item_instance_id,OLD.item_instance_id,l.source_placement_ordinal,r.world_id) THEN
   RAISE EXCEPTION 'spell consumption source inventory custody changed' USING ERRCODE='23514';
  END IF;
 ELSE
  IF NOT ((l.source_entry_is_corpse AND EXISTS(SELECT 1 FROM game_item_corpse_container_entries e WHERE e.item_instance_id=OLD.item_instance_id AND e.world_id=r.world_id AND e.parent_item_instance_id=l.source_parent_item_instance_id AND e.placement_ordinal=l.source_placement_ordinal))
     OR (NOT l.source_entry_is_corpse AND EXISTS(SELECT 1 FROM game_item_container_entries e WHERE e.item_instance_id=OLD.item_instance_id AND e.world_id=r.world_id AND e.parent_item_instance_id=l.source_parent_item_instance_id AND e.placement_ordinal=l.source_placement_ordinal))) THEN
   RAISE EXCEPTION 'contained spell retirement source custody changed' USING ERRCODE='23514';
  END IF;
  IF NOT EXISTS(SELECT 1 FROM game_item_ground_locations g JOIN game_item_instances i USING(item_instance_id) JOIN game_item_mint_receipts m ON m.item_instance_id=i.item_instance_id
      WHERE g.item_instance_id=l.source_corpse_root AND i.lifecycle=1 AND m.loot_purpose_key='CORPSE_MATERIALIZATION'
       AND (g.world_id,g.channel_id)=(r.world_id,r.channel_id)
       AND (g.runtime_scope_ownership_generation=r.ownership_generation
         OR (r.cause_kind=5 AND g.scope_kind=1 AND g.instance_id IS NULL
           AND g.runtime_scope_ownership_generation<r.ownership_generation))
       AND (g.spatial_position,g.map_revision,g.content_revision,g.native_room_placement_context)=(l.spatial_position,l.map_revision,l.content_revision,l.placement_context)) THEN
   RAISE EXCEPTION 'contained retirement root lacks actual current corpse placement' USING ERRCODE='23514';
  END IF;
 END IF;
 INSERT INTO game_spell_item_source_custody(transaction_id,item_instance_id,quantity_before,state_revision_before,source_parent,source_ordinal,source_slot)
 VALUES(l.transaction_id,OLD.item_instance_id,OLD.quantity,OLD.state_revision,l.source_parent_item_instance_id,l.source_placement_ordinal,l.source_equipment_slot);
 RETURN NEW;
END $$;

DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_spell_item_capture_custody() SET search_path = pg_catalog, %I, pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_spell_item_capture_custody() FROM PUBLIC;
