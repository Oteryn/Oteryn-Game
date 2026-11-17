-- Bound the shared current-custody reads; retain every existing proof and lock.
DO $migration$
DECLARE body TEXT;
BEGIN
 body:=pg_get_functiondef('game_item_location_exclusive(UUID)'::regprocedure);
 IF (length(body)-length(replace(body,$old$    v_locations BIGINT;$old$,'')))/length($old$    v_locations BIGINT;$old$)<>1 THEN RAISE EXCEPTION 'current custody owner seam changed'; END IF;
 body:=replace(body,$old$    v_locations BIGINT;$old$,$new$    v_locations BIGINT;
    v_in_house BOOLEAN;
    v_in_inbox BOOLEAN;
    v_in_depot BOOLEAN;$new$);
 IF (length(body)-length(replace(body,$old$    v_locations := (SELECT count(*) FROM game_item_market_escrow_locations WHERE item_instance_id=p_item) +
          (SELECT count(*) FROM game_item_ground_locations WHERE item_instance_id = p_item)
        + (SELECT count(*) FROM game_item_container_slots WHERE item_instance_id = p_item)
        + (SELECT count(*) FROM game_item_container_entries WHERE item_instance_id = p_item)
        + (SELECT count(*) FROM game_item_corpse_container_entries WHERE item_instance_id = p_item)
        + (SELECT count(*) FROM game_item_house_interior_locations WHERE item_instance_id = p_item)
        + (SELECT count(*) FROM game_character_equipment_slots WHERE item_instance_id = p_item)
        + (SELECT count(*) FROM game_item_character_inbox_locations WHERE item_instance_id = p_item)
        + (SELECT count(*) FROM game_item_nested_entries WHERE item_instance_id = p_item)
        + (SELECT count(*) FROM game_item_character_depot_locations WHERE item_instance_id = p_item);
    v_locations := v_locations + (SELECT count(*) FROM game_bazaar_item_locations WHERE item_instance_id=p_item); $old$,'')))/length($old$    v_locations := (SELECT count(*) FROM game_item_market_escrow_locations WHERE item_instance_id=p_item) +
          (SELECT count(*) FROM game_item_ground_locations WHERE item_instance_id = p_item)
        + (SELECT count(*) FROM game_item_container_slots WHERE item_instance_id = p_item)
        + (SELECT count(*) FROM game_item_container_entries WHERE item_instance_id = p_item)
        + (SELECT count(*) FROM game_item_corpse_container_entries WHERE item_instance_id = p_item)
        + (SELECT count(*) FROM game_item_house_interior_locations WHERE item_instance_id = p_item)
        + (SELECT count(*) FROM game_character_equipment_slots WHERE item_instance_id = p_item)
        + (SELECT count(*) FROM game_item_character_inbox_locations WHERE item_instance_id = p_item)
        + (SELECT count(*) FROM game_item_nested_entries WHERE item_instance_id = p_item)
        + (SELECT count(*) FROM game_item_character_depot_locations WHERE item_instance_id = p_item);
    v_locations := v_locations + (SELECT count(*) FROM game_bazaar_item_locations WHERE item_instance_id=p_item); $old$)<>1 THEN RAISE EXCEPTION 'current custody owner seam changed'; END IF;
 body:=replace(body,$old$    v_locations := (SELECT count(*) FROM game_item_market_escrow_locations WHERE item_instance_id=p_item) +
          (SELECT count(*) FROM game_item_ground_locations WHERE item_instance_id = p_item)
        + (SELECT count(*) FROM game_item_container_slots WHERE item_instance_id = p_item)
        + (SELECT count(*) FROM game_item_container_entries WHERE item_instance_id = p_item)
        + (SELECT count(*) FROM game_item_corpse_container_entries WHERE item_instance_id = p_item)
        + (SELECT count(*) FROM game_item_house_interior_locations WHERE item_instance_id = p_item)
        + (SELECT count(*) FROM game_character_equipment_slots WHERE item_instance_id = p_item)
        + (SELECT count(*) FROM game_item_character_inbox_locations WHERE item_instance_id = p_item)
        + (SELECT count(*) FROM game_item_nested_entries WHERE item_instance_id = p_item)
        + (SELECT count(*) FROM game_item_character_depot_locations WHERE item_instance_id = p_item);
    v_locations := v_locations + (SELECT count(*) FROM game_bazaar_item_locations WHERE item_instance_id=p_item); $old$,$new$    SELECT count(*), coalesce(bool_or(kind=5),false),
           coalesce(bool_or(kind=7),false), coalesce(bool_or(kind=9),false)
      INTO v_locations, v_in_house, v_in_inbox, v_in_depot
      FROM (
      SELECT 0 AS kind FROM game_item_market_escrow_locations WHERE item_instance_id=p_item
      UNION ALL SELECT 1 AS kind FROM game_item_ground_locations WHERE item_instance_id=p_item
      UNION ALL SELECT 2 AS kind FROM game_item_container_slots WHERE item_instance_id=p_item
      UNION ALL SELECT 3 AS kind FROM game_item_container_entries WHERE item_instance_id=p_item
      UNION ALL SELECT 4 AS kind FROM game_item_corpse_container_entries WHERE item_instance_id=p_item
      UNION ALL SELECT 5 AS kind FROM game_item_house_interior_locations WHERE item_instance_id=p_item
      UNION ALL SELECT 6 AS kind FROM game_character_equipment_slots WHERE item_instance_id=p_item
      UNION ALL SELECT 7 AS kind FROM game_item_character_inbox_locations WHERE item_instance_id=p_item
      UNION ALL SELECT 8 AS kind FROM game_item_nested_entries WHERE item_instance_id=p_item
      UNION ALL SELECT 9 AS kind FROM game_item_character_depot_locations WHERE item_instance_id=p_item
      UNION ALL SELECT 10 AS kind FROM game_bazaar_item_locations WHERE item_instance_id=p_item
      ) current_locations;
    $new$);
 IF (length(body)-length(replace(body,$old$IF EXISTS (SELECT 1 FROM game_item_house_interior_locations WHERE item_instance_id = p_item)$old$,'')))/length($old$IF EXISTS (SELECT 1 FROM game_item_house_interior_locations WHERE item_instance_id = p_item)$old$)<>1 THEN RAISE EXCEPTION 'current custody owner seam changed'; END IF;
 body:=replace(body,$old$IF EXISTS (SELECT 1 FROM game_item_house_interior_locations WHERE item_instance_id = p_item)$old$,$new$IF v_in_house$new$);
 IF (length(body)-length(replace(body,$old$IF EXISTS (SELECT 1 FROM game_item_character_inbox_locations WHERE item_instance_id = p_item)$old$,'')))/length($old$IF EXISTS (SELECT 1 FROM game_item_character_inbox_locations WHERE item_instance_id = p_item)$old$)<>1 THEN RAISE EXCEPTION 'current custody owner seam changed'; END IF;
 body:=replace(body,$old$IF EXISTS (SELECT 1 FROM game_item_character_inbox_locations WHERE item_instance_id = p_item)$old$,$new$IF v_in_inbox$new$);
 IF (length(body)-length(replace(body,$old$IF EXISTS (SELECT 1 FROM game_item_character_depot_locations WHERE item_instance_id = p_item)$old$,'')))/length($old$IF EXISTS (SELECT 1 FROM game_item_character_depot_locations WHERE item_instance_id = p_item)$old$)<>1 THEN RAISE EXCEPTION 'current custody owner seam changed'; END IF;
 body:=replace(body,$old$IF EXISTS (SELECT 1 FROM game_item_character_depot_locations WHERE item_instance_id = p_item)$old$,$new$IF v_in_depot$new$);
 EXECUTE body;
 body:=pg_get_functiondef('game_house_disposition_native_removal_proven(JSONB)'::regprocedure);
 IF (length(body)-length(replace(body,$old$ LANGUAGE sql
$old$,'')))/length($old$ LANGUAGE sql
$old$)<>1 THEN RAISE EXCEPTION 'native house removal owner seam changed'; END IF;
 body:=replace(body,$old$ LANGUAGE sql
$old$,$new$ LANGUAGE plpgsql
$new$);
 IF (length(body)-length(replace(body,$old$
 SELECT game_house_disposition_native_removal_proven_before_t135(old_location) OR EXISTS(
  SELECT 1 FROM game_house_disposition_wrap_records r
  JOIN game_house_disposition_census c USING(operation_id,item_instance_id)
  JOIN game_house_disposition_steps s USING(operation_id,item_instance_id)
  JOIN game_item_instances actual USING(item_instance_id)
  WHERE c.source_location=old_location AND c.source_item=r.before_item
   AND to_jsonb(actual)=r.after_item AND s.written_xact_id=pg_current_xact_id()
   AND r.created_xact_id=pg_current_xact_id() AND game_house_disposition_delivery_proven(c.operation_id,c.item_instance_id))
$old$,'')))/length($old$
 SELECT game_house_disposition_native_removal_proven_before_t135(old_location) OR EXISTS(
  SELECT 1 FROM game_house_disposition_wrap_records r
  JOIN game_house_disposition_census c USING(operation_id,item_instance_id)
  JOIN game_house_disposition_steps s USING(operation_id,item_instance_id)
  JOIN game_item_instances actual USING(item_instance_id)
  WHERE c.source_location=old_location AND c.source_item=r.before_item
   AND to_jsonb(actual)=r.after_item AND s.written_xact_id=pg_current_xact_id()
   AND r.created_xact_id=pg_current_xact_id() AND game_house_disposition_delivery_proven(c.operation_id,c.item_instance_id))
$old$)<>1 THEN RAISE EXCEPTION 'native house removal owner seam changed'; END IF;
 body:=replace(body,$old$
 SELECT game_house_disposition_native_removal_proven_before_t135(old_location) OR EXISTS(
  SELECT 1 FROM game_house_disposition_wrap_records r
  JOIN game_house_disposition_census c USING(operation_id,item_instance_id)
  JOIN game_house_disposition_steps s USING(operation_id,item_instance_id)
  JOIN game_item_instances actual USING(item_instance_id)
  WHERE c.source_location=old_location AND c.source_item=r.before_item
   AND to_jsonb(actual)=r.after_item AND s.written_xact_id=pg_current_xact_id()
   AND r.created_xact_id=pg_current_xact_id() AND game_house_disposition_delivery_proven(c.operation_id,c.item_instance_id))
$old$,$new$
BEGIN
 -- Both existing physical-proof arms require this exact current census row.
 -- The negative path avoids planning/executing their unrelated receipt joins.
 IF NOT EXISTS(SELECT 1 FROM game_house_disposition_census
               WHERE source_location=old_location) THEN
  RETURN FALSE;
 END IF;
 RETURN game_house_disposition_native_removal_proven_before_t135(old_location) OR EXISTS(
  SELECT 1 FROM game_house_disposition_wrap_records r
  JOIN game_house_disposition_census c USING(operation_id,item_instance_id)
  JOIN game_house_disposition_steps s USING(operation_id,item_instance_id)
  JOIN game_item_instances actual USING(item_instance_id)
  WHERE c.source_location=old_location AND c.source_item=r.before_item
   AND to_jsonb(actual)=r.after_item AND s.written_xact_id=pg_current_xact_id()
   AND r.created_xact_id=pg_current_xact_id() AND game_house_disposition_delivery_proven(c.operation_id,c.item_instance_id));
END
$new$);
 EXECUTE body;
END $migration$;
