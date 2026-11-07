-- Native floor constructor: same mature writer/fence/pair/lineage, no new
-- placement or inventory custody. Existing inventory-first translation stays.
DO $floor$ DECLARE body TEXT; needle TEXT; source_at INTEGER; receipt_at INTEGER; branch TEXT; BEGIN
 body:=pg_get_functiondef('game_house_furniture_construct(uuid,uuid,numeric,numeric,numeric,uuid,numeric,uuid,text,numeric,uuid,numeric,numeric,smallint,bytea,bytea,text,integer,text,bytea)'::regprocedure);
 needle:='a RECORD; guild UUID; parent UUID; ordinal NUMERIC; tx UUID;';
 IF position(needle IN body)=0 THEN RAISE EXCEPTION 'missing constructor local declaration'; END IF;
 body:=replace(body,needle,needle||' floor_constructor BOOLEAN; floor_provenance game_item_house_reclaim_provenance%ROWTYPE;');
 needle:='SELECT b.item_instance_id INTO parent FROM game_item_container_slots b';
 IF position(needle IN body)=0 THEN RAISE EXCEPTION 'missing constructor backpack read'; END IF;
 body:=replace(body,needle,$new$floor_constructor:=p_action=2 AND EXISTS(SELECT 1 FROM game_item_house_interior_locations floor_source
  WHERE floor_source.item_instance_id=p_item AND floor_source.world_id=p_world AND floor_source.house_key=p_house);
 IF NOT floor_constructor THEN
 SELECT b.item_instance_id INTO parent FROM game_item_container_slots b$new$);
 needle:=$old$IF parent IS NULL THEN RAISE EXCEPTION 'WRAP_INVENTORY_FULL'; END IF;$old$;
 IF position(needle IN body)=0 THEN RAISE EXCEPTION 'missing constructor backpack requirement'; END IF;
 body:=replace(body,needle,needle||E'\n END IF;');
 source_at:=position('IF carried THEN' IN body);
 receipt_at:=position('INSERT INTO game_house_furniture_wrap_receipts' IN body);
 IF source_at=0 OR receipt_at<=source_at THEN RAISE EXCEPTION 'missing current constructor custody preparation'; END IF;
 branch:=substring(body FROM source_at FOR receipt_at-source_at);
 body:=overlay(body placing $new$IF floor_constructor THEN
  SELECT floor_source.* INTO h FROM game_item_house_interior_locations floor_source
   WHERE floor_source.item_instance_id=p_item AND floor_source.world_id=p_world
    AND floor_source.house_key=p_house AND floor_source.spatial_position=p_position FOR UPDATE;
  IF NOT FOUND OR game_house_access_tile(p_world,p_house,h.spatial_position) IS DISTINCT FROM TRUE
   THEN RAISE EXCEPTION 'WRAP_NOT_WRAPABLE'; END IF;
  SELECT floor_proof.* INTO floor_provenance FROM game_item_house_reclaim_provenance floor_proof
   WHERE floor_proof.item_instance_id=p_item AND floor_proof.world_id=p_world AND floor_proof.house_key=p_house
    AND floor_proof.placement_transaction_id=h.placed_transaction_id FOR UPDATE;
  IF NOT FOUND THEN RAISE EXCEPTION 'WRAP_NOT_WRAPABLE'; END IF;
  IF NOT EXISTS(SELECT 1 FROM game_house_furniture_rules construction_rule WHERE construction_rule.action=2
    AND i.definition_production_key='oteryn:item.tibia.i'||construction_rule.first_item::text
    AND p_after='oteryn:item.tibia.i'||construction_rule.result_item::text)
   AND NOT(i.definition_production_key='oteryn:item.tibia.i23398' AND
    (EXISTS(SELECT 1 FROM game_house_furniture_wrap_receipts original_wrap WHERE original_wrap.item_instance_id=p_item
      AND original_wrap.action=1 AND original_wrap.after_key=i.definition_production_key
      AND original_wrap.before_item->>'definition_production_key'=p_after)
     OR EXISTS(SELECT 1 FROM game_house_furniture_kit_origins kit_origin WHERE kit_origin.item_instance_id=p_item
      AND kit_origin.target_key=p_after AND kit_origin.target_revision=i.definition_revision_ref)))
   THEN RAISE EXCEPTION 'WRAP_NOT_WRAPABLE'; END IF;
 ELSE
$new$||branch||E' END IF;\n ' FROM source_at FOR receipt_at-source_at);
 needle:='CASE WHEN carried_nested THEN jsonb_build_object(''custody_kind'',''nested'',''custody'',to_jsonb(n)) ELSE to_jsonb(e) END';
 IF position(needle IN body)=0 THEN RAISE EXCEPTION 'missing current constructor custody receipt'; END IF;
 body:=replace(body,needle,'CASE WHEN floor_constructor THEN jsonb_build_object(''custody_kind'',''house_floor_constructor'',''provenance'',to_jsonb(floor_provenance)) ELSE '||needle||' END');
 needle:='IF NOT carried THEN';
 IF position(needle IN body)=0 THEN RAISE EXCEPTION 'missing current constructor custody mutation boundary'; END IF;
 body:=replace(body,needle,'IF NOT carried AND NOT floor_constructor THEN');
 EXECUTE body;
END $floor$;

DO $complete$ DECLARE body TEXT; needle TEXT; replacement TEXT; BEGIN
 body:=pg_get_functiondef('game_house_furniture_wrap_complete(uuid)'::regprocedure);
 needle:=$old$ELSE
  IF EXISTS(SELECT 1 FROM game_item_container_entries WHERE item_instance_id=r.item_instance_id)$old$;
 IF position(needle IN body)=0 THEN RAISE EXCEPTION 'missing original constructor completion branch'; END IF;
 replacement:=$new$ELSIF r.inventory_location->>'custody_kind'='house_floor_constructor' THEN
  IF r.use_intent IS NULL OR octet_length(r.use_intent) NOT BETWEEN 1 AND 529
   OR EXISTS(SELECT 1 FROM game_item_container_entries WHERE item_instance_id=r.item_instance_id)
   OR EXISTS(SELECT 1 FROM game_item_nested_entries WHERE item_instance_id=r.item_instance_id)
   OR NOT EXISTS(SELECT 1 FROM game_item_house_interior_locations floor_source
    WHERE to_jsonb(floor_source)=r.house_location AND floor_source.item_instance_id=r.item_instance_id
     AND floor_source.world_id=r.world_id AND floor_source.house_key=r.house_key)
   OR NOT EXISTS(SELECT 1 FROM game_item_house_reclaim_provenance floor_proof
    WHERE to_jsonb(floor_proof)=r.inventory_location->'provenance'
     AND floor_proof.item_instance_id=r.item_instance_id AND floor_proof.world_id=r.world_id
     AND floor_proof.house_key=r.house_key)
   THEN RAISE EXCEPTION 'incomplete floor constructor custody' USING ERRCODE='23514'; END IF;
 ELSE
  IF EXISTS(SELECT 1 FROM game_item_container_entries WHERE item_instance_id=r.item_instance_id)$new$;
 EXECUTE replace(body,needle,replacement);
END $complete$;
