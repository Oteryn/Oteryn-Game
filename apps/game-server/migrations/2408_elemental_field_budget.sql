-- The existing RL-06 channel budget is shared by spell and script field writers.
-- Its callers hold the physical channel Item lock while admitting a new field.
CREATE FUNCTION game_live_channel_field_count(p_world UUID,p_channel UUID,p_removed BYTEA[])
RETURNS BIGINT LANGUAGE sql STABLE AS $$
 SELECT count(*) FROM (
   SELECT 1 FROM game_item_ground_locations g JOIN game_item_instances i USING(item_instance_id)
   LEFT JOIN game_spell_item_lines l ON l.item_instance_id=i.item_instance_id AND l.transaction_id=i.minted_transaction_id AND l.operation_kind=1
   WHERE g.world_id=p_world AND g.channel_id=p_channel AND i.lifecycle=1
     AND NOT (uuid_send(i.item_instance_id)=ANY(coalesce(p_removed,ARRAY[]::bytea[])))
     AND ((l.transaction_id IS NOT NULL AND (l.expires_at_unix_ms IS NOT NULL OR EXISTS(
       SELECT 1 FROM game_spell_field_temporal_schedules p WHERE p.item_instance_id=i.item_instance_id AND p.source_transaction_id=l.transaction_id)))
       OR EXISTS(SELECT 1 FROM game_elemental_energy_field_receipts e WHERE e.item_instance_id=i.item_instance_id AND e.transaction_id=i.minted_transaction_id))
   LIMIT 20001
 ) fields
$$;
DO $migration$
DECLARE body TEXT;
BEGIN
 body:=pg_get_functiondef('game_elemental_energy_field_admit()'::regprocedure);
 IF position(' RETURN NEW;' IN body)=0 THEN RAISE EXCEPTION 'missing energy field admission predecessor'; END IF;
 body:=replace(body,' RETURN NEW;',$arm$
 IF game_live_channel_field_count(NEW.world_id,NEW.channel_id,ARRAY[]::bytea[])>=20000
 THEN RAISE EXCEPTION 'channel field count bound' USING ERRCODE='23514'; END IF;
 RETURN NEW;
 $arm$);
 EXECUTE body;
 EXECUTE format('ALTER FUNCTION game_live_channel_field_count(uuid,uuid,bytea[]) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $migration$;
REVOKE ALL ON FUNCTION game_live_channel_field_count(UUID,UUID,BYTEA[]) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_live_channel_field_count(UUID,UUID,BYTEA[]) TO oteryn_game_runtime;
