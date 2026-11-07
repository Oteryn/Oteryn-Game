-- Large atomic recipes retain the exact carried-consumption guards from 0268.
-- Resolve their current-item proof before unrelated historical guard branches.
CREATE INDEX game_quest_consumption_item_xact_idx
 ON game_quest_callback_consumptions(item_instance_id,created_xact_id);
DO $migration$
DECLARE n TEXT; body TEXT; arm TEXT; begin_at INT;
BEGIN
 FOREACH n IN ARRAY ARRAY['game_item_instance_guard','game_item_instance_change_proven',
  'game_item_container_entry_removal_proven','game_item_nested_entry_proven'] LOOP
  body:=pg_get_functiondef((n||'()')::regprocedure);
  begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
  IF begin_at=0 THEN RAISE EXCEPTION 'unexpected carried proof body: %',n; END IF;
  IF n='game_item_instance_guard' THEN
   arm:=$arm$
    IF TG_OP='UPDATE' AND EXISTS(SELECT 1 FROM game_quest_callback_consumptions r
      WHERE r.location_kind<>'ground' AND r.item_instance_id=OLD.item_instance_id
        AND r.before_item=to_jsonb(OLD) AND r.created_xact_id=pg_current_xact_id()
        AND game_quest_callback_after(r,NEW)) THEN RETURN NEW; END IF;
$arm$;
  ELSIF n='game_item_instance_change_proven' THEN
   arm:=$arm$
    IF EXISTS(SELECT 1 FROM game_quest_callback_consumptions r
      WHERE r.location_kind<>'ground' AND r.item_instance_id=NEW.item_instance_id
        AND r.created_xact_id=pg_current_xact_id() AND game_quest_callback_after(r,NEW)) THEN RETURN NULL; END IF;
$arm$;
  ELSE
   arm:=format($arm$
    IF TG_OP='DELETE' AND TG_TABLE_NAME=%L THEN
    IF EXISTS(SELECT 1 FROM game_quest_callback_consumptions r
      JOIN game_item_instances i USING(item_instance_id)
      WHERE r.location_kind=%L AND r.before_location=to_jsonb(OLD)
        AND r.item_instance_id=OLD.item_instance_id AND r.created_xact_id=pg_current_xact_id()
        AND i.quantity=0 AND i.lifecycle=2 AND game_quest_callback_after(r,i)) THEN
      IF TG_WHEN='BEFORE' THEN RETURN OLD; END IF;
      RETURN NULL;
    END IF;
    END IF;
$arm$,CASE n WHEN 'game_item_container_entry_removal_proven' THEN 'game_item_container_entries'
 ELSE 'game_item_nested_entries' END,
 CASE n WHEN 'game_item_container_entry_removal_proven' THEN 'container' ELSE 'nested' END);
  END IF;
  EXECUTE overlay(body placing E'BEGIN\n'||arm from begin_at for 5);
 END LOOP;
END $migration$;
