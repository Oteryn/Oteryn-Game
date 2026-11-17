-- BAGS-1 contents retain their parent identities when DEATH-3 transfers a root.
-- Keep the installed Channel/Instance receipt binding and all terminal proofs.
DO $migration$
DECLARE body TEXT; old_guard TEXT; new_guard TEXT; old_direct_guard TEXT;
BEGIN
 body:=pg_get_functiondef('game_player_death_item_admit()'::regprocedure);
 old_guard:=$old$  OR EXISTS(SELECT 1 FROM game_item_nested_entries WHERE parent_item_instance_id=NEW.item_instance_id)
  OR EXISTS(SELECT 1 FROM game_item_container_entries e JOIN game_item_nested_entries n ON n.parent_item_instance_id=e.item_instance_id
    WHERE e.parent_item_instance_id=NEW.item_instance_id)$old$;
 new_guard:=$new$  OR (NEW.operation=2 AND (
    EXISTS(SELECT 1 FROM game_item_nested_entries WHERE parent_item_instance_id=NEW.item_instance_id)
    OR EXISTS(SELECT 1 FROM game_item_container_entries WHERE parent_item_instance_id=NEW.item_instance_id)))$new$;
 IF strpos(body,old_guard)=0 THEN
  RAISE EXCEPTION 'missing installed death item container guard';
 END IF;
 body:=replace(body,old_guard,new_guard);
 -- A historically picked main may be a current direct bag while retaining its
 -- original direct child edges. Keep equipment rejection; both supported bag
 -- sources still require their exact current custody and terminal root proof.
 old_direct_guard:='NEW.source_slot<>9 AND EXISTS(SELECT 1 FROM game_item_container_entries WHERE parent_item_instance_id=NEW.item_instance_id)';
 IF (length(body)-length(replace(body,old_direct_guard,'')))/length(old_direct_guard)<>1 THEN
  RAISE EXCEPTION 'missing exact installed death direct-container guard';
 END IF;
 body:=replace(body,old_direct_guard,'NEW.source_slot NOT IN(0,9) AND EXISTS(SELECT 1 FROM game_item_container_entries WHERE parent_item_instance_id=NEW.item_instance_id)');
 body:=replace(body,'direct custody or depth-one shape','direct custody or supported container shape');
 EXECUTE body;
END $migration$;
