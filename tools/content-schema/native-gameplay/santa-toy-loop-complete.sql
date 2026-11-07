-- A valid individual movement cannot omit its required source clock.
CREATE FUNCTION game_santa_toy_loop_clock_complete() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 IF NEW.from_kind IN(1,2,3) AND NEW.to_kind IN(1,2,3)
  AND NEW.quantity_before=1 AND NEW.quantity_after=1
  AND EXISTS(SELECT 1 FROM game_santa_toy_lifetimes WHERE item_key=NEW.definition_production_key)
  AND NOT EXISTS(SELECT 1 FROM game_spell_item_temporal_schedules WHERE item_instance_id=NEW.item_instance_id)
  AND NOT EXISTS(SELECT 1 FROM game_item_timed_states WHERE item_instance_id=NEW.item_instance_id) THEN
  RAISE EXCEPTION 'individual toy movement requires its original source clock' USING ERRCODE='23514'; END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_santa_toy_loop_clock_complete AFTER INSERT ON game_item_loop_move_lines
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_santa_toy_loop_clock_complete();
DO $$ BEGIN EXECUTE format('ALTER FUNCTION game_santa_toy_loop_clock_complete() SET search_path=pg_catalog,%I,pg_temp',current_schema()); END $$;
REVOKE ALL ON FUNCTION game_santa_toy_loop_clock_complete() FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
