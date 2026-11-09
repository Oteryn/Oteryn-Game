-- M13: bind the carried instance in a World USE proof to the actual atomic input.
CREATE FUNCTION game_quest_selected_world_tool_admit() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE proof JSONB; source_bytes JSONB;
BEGIN
 proof:=convert_from(NEW.proof,'UTF8')::JSONB;
 IF NEW.location_kind='ground' OR proof #>> '{callback,item_on_target,selected_instance}' IS DISTINCT FROM 'true' THEN
  RETURN NEW;
 END IF;
 SELECT jsonb_agg(get_byte(uuid_send(NEW.item_instance_id),n) ORDER BY n) INTO source_bytes FROM generate_series(0,15) bytes(n);
 IF proof #> '{context,selected_tool,source}' IS DISTINCT FROM source_bytes
  OR jsonb_typeof(proof #> '{context,selected_tool,intent}') IS DISTINCT FROM 'array'
  OR jsonb_array_length(proof #> '{context,selected_tool,intent}') NOT BETWEEN 1 AND 4096
  OR proof #>> '{callback,trigger}' IS DISTINCT FROM 'use'
  OR proof #>> '{callback,requires_world_admission}' IS DISTINCT FROM 'true'
  OR proof #>> '{callback,item_on_target,reusable}'='true'
 THEN RAISE EXCEPTION 'selected World tool differs from its carried consumption' USING ERRCODE='23514'; END IF;
 IF EXISTS(SELECT 1 FROM jsonb_array_elements(proof #> '{context,selected_tool,intent}') part(value)
   WHERE CASE WHEN part.value::TEXT ~ '^[0-9]{1,3}$' THEN part.value::TEXT::INTEGER NOT BETWEEN 0 AND 255 ELSE true END)
 THEN RAISE EXCEPTION 'selected World tool lacks bounded ingress bytes' USING ERRCODE='23514'; END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER game_quest_selected_world_tool_admit BEFORE INSERT ON game_quest_callback_consumptions
 FOR EACH ROW EXECUTE FUNCTION game_quest_selected_world_tool_admit();
REVOKE ALL ON FUNCTION game_quest_selected_world_tool_admit() FROM PUBLIC;
