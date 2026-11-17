-- A terminal Item UPDATE keeps the same immutable ItemInstanceId. The existing
-- callback checks its NEW and OLD UUID consecutively; both calls read the same
-- current, locked row, rather than the historical OLD state. Keep one complete
-- location proof at the first call site. Custody callbacks and every other Item
-- event retain the installed guard body and all deferred authority proofs.
DO $migration$
DECLARE
 body TEXT;
 anchor TEXT := 'IF TG_OP <> ''DELETE'' THEN';
 terminal_arm TEXT := $arm$
 IF TG_OP='UPDATE' AND TG_TABLE_NAME='game_item_instances' THEN
  -- Other tables sharing this callback have no lifecycle/quantity fields.
  -- Resolve those record fields only inside the Item-specific branch.
  IF OLD.item_instance_id=NEW.item_instance_id
   AND OLD.lifecycle=1 AND NEW.lifecycle=2 AND NEW.quantity=0 THEN
   PERFORM game_item_location_exclusive(NEW.item_instance_id);
   RETURN NULL;
  END IF;
 END IF;
 $arm$;
BEGIN
 body := pg_get_functiondef('game_item_location_exclusivity_guard()'::regprocedure);
 IF (length(body)-length(replace(body,anchor,'')))/length(anchor)<>1
  OR (length(body)-length(replace(body,'PERFORM game_item_location_exclusive(NEW.item_instance_id);','')))
     /length('PERFORM game_item_location_exclusive(NEW.item_instance_id);')<>1
  OR (length(body)-length(replace(body,'PERFORM game_item_location_exclusive(OLD.item_instance_id);','')))
     /length('PERFORM game_item_location_exclusive(OLD.item_instance_id);')<>1 THEN
  RAISE EXCEPTION 'unexpected installed Item location guard';
 END IF;
 -- pg_get_functiondef retains the current security mode and pinned search_path;
 -- CREATE OR REPLACE preserves the function identity, ownership and grants.
 EXECUTE replace(body,anchor,terminal_arm||anchor);
END $migration$;
