-- W01 copied CorruptedSoul deaths run in both Channel and Instance scopes.
-- 1423 added op7 only to the Instance operation shape; retain every existing
-- arm and admit the two Channel successor definitions. The deferred W01 line,
-- receipt and Item guards still prove the current owner and exact source edge.
DO $$
DECLARE prior TEXT; arm TEXT;
BEGIN
 SELECT pg_get_constraintdef(oid) INTO STRICT prior FROM pg_constraint
  WHERE conrelid='game_spell_item_lines'::regclass AND conname='spell_item_operation_shape';
 IF left(prior,6)<>'CHECK ' THEN
  RAISE EXCEPTION 'unexpected W01 transform line constraint';
 END IF;
 arm:=$p$(operation_kind=7 AND runtime_scope_kind=1 AND channel_id IS NOT NULL
  AND game_node_is_uuid_v7(channel_id) AND instance_id IS NULL
  AND quantity_before=1 AND quantity_after=1 AND state_revision_before>0 AND source_stack_ordinal>0
  AND destination_parent_item_instance_id IS NULL AND destination_ordinal IS NULL
  AND source_parent_item_instance_id IS NULL AND source_placement_ordinal IS NULL
  AND source_equipment_slot IS NULL AND source_equipment_revision IS NULL
  AND source_corpse_root IS NULL AND source_entry_is_corpse IS NULL AND expires_at_unix_ms IS NULL
  AND definition_family='Item' AND definition_revision='definition-r1'
  AND definition_production_key IN('oteryn:item.tibia.i23727','oteryn:item.tibia.i23728'))$p$;
 ALTER TABLE game_spell_item_lines DROP CONSTRAINT spell_item_operation_shape;
 EXECUTE format('ALTER TABLE game_spell_item_lines ADD CONSTRAINT spell_item_operation_shape CHECK ((%s OR %s) IS TRUE)',substr(prior,7),arm);
END $$;
