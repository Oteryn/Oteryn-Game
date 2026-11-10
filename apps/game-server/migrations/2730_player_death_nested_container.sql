-- BAGS-1 contents retain their parent identities when DEATH-3 transfers a root.
-- Keep the installed Channel/Instance receipt binding and all terminal proofs.
DO $migration$
DECLARE body TEXT; old_guard TEXT; new_guard TEXT;
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
 body:=replace(body,'direct custody or depth-one shape','direct custody or supported container shape');
 EXECUTE body;
END $migration$;
