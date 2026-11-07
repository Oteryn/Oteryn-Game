-- Reuse the current complete constructor writer. Only native Wrap13 grows;
-- legacy45 and ordinary nativeUSE keep their original admission/replay gates.
DO $writer$ DECLARE body TEXT; needle TEXT; replacement TEXT; source_at INTEGER; receipt_at INTEGER; mutation_at INTEGER; update_at INTEGER; branch TEXT; BEGIN
 body:=pg_get_functiondef('game_house_furniture_construct(uuid,uuid,numeric,numeric,numeric,uuid,numeric,uuid,text,numeric,uuid,numeric,numeric,smallint,bytea,bytea,text,integer,text,bytea)'::regprocedure);
 needle:='IF p_action<>2 OR p_use_intent IS NULL OR octet_length(p_use_intent) NOT BETWEEN 1 AND 529 THEN';
 IF position(needle IN body)=0 THEN RAISE EXCEPTION 'missing native constructor purpose guard'; END IF;
 body:=replace(body,needle,'IF p_use_intent IS NULL OR NOT ((p_action=2 AND octet_length(p_use_intent) BETWEEN 1 AND 529) OR (p_action=1 AND octet_length(p_use_intent)=13 AND get_byte(p_use_intent,0) IN(3,4))) THEN');
 needle:='a RECORD; guild UUID; parent UUID; ordinal NUMERIC; tx UUID;';
 IF position(needle IN body)=0 THEN RAISE EXCEPTION 'missing native writer local declarations'; END IF;
 body:=replace(body,needle,needle||' carried BOOLEAN; carried_nested BOOLEAN:=FALSE; n game_item_nested_entries%ROWTYPE;');
 needle:='tx:=game_character_uuid_v7();';
 IF position(needle IN body)=0 THEN RAISE EXCEPTION 'missing native writer transaction creation'; END IF;
 -- Set the explicit namespace before quantity admission, not by custody fallback.
 body:=replace(body,'IF i.quantity<>1 OR',
  'IF (i.quantity<>1 AND NOT (p_action=1 AND get_byte(p_use_intent,0)=4)) OR (p_action=1 AND get_byte(p_use_intent,0)=4 AND NOT EXISTS(SELECT 1 FROM game_house_furniture_portable_wrap_sources wrap_policy WHERE i.definition_production_key=''oteryn:item.tibia.i''||wrap_policy.item_id::text AND i.quantity BETWEEN 1 AND wrap_policy.stack_max)) OR');
 IF position('IF i.quantity<>1 OR' IN body)>0 OR position('wrap_policy.stack_max' IN body)=0 THEN RAISE EXCEPTION 'missing exact native Wrap quantity guard'; END IF;
 body:=replace(body,needle,needle||E'\n carried:=p_action=1 AND get_byte(p_use_intent,0)=4;');
 source_at:=position('IF p_action=1 THEN' IN body);
 receipt_at:=position('INSERT INTO game_house_furniture_wrap_receipts' IN body);
 IF source_at=0 OR receipt_at<=source_at THEN RAISE EXCEPTION 'missing mature native custody preparation'; END IF;
 branch:=substring(body FROM source_at FOR receipt_at-source_at);
 replacement:=$carried$IF carried THEN
  SELECT entry.* INTO e FROM game_item_container_entries entry
   WHERE entry.item_instance_id=p_item AND entry.world_id=p_world
    AND entry.character_id=p_character AND entry.parent_item_instance_id=parent FOR UPDATE;
  IF NOT FOUND THEN
   SELECT entry.* INTO n FROM game_item_nested_entries entry,
    LATERAL game_item_loop_owner(entry.parent_item_instance_id) owner
    WHERE entry.item_instance_id=p_item AND entry.world_id=p_world
     AND owner.owner_character=p_character AND owner.depth BETWEEN 0 AND 7 FOR UPDATE OF entry;
   IF NOT FOUND THEN RAISE EXCEPTION 'WRAP_NOT_WRAPABLE'; END IF;
   carried_nested:=TRUE;
  END IF;
 ELSE
$carried$||branch||E' END IF;\n ';
 body:=overlay(body placing replacement FROM source_at FOR receipt_at-source_at);
 needle:='p_after,to_jsonb(h),to_jsonb(e),';
 IF position(needle IN body)=0 THEN RAISE EXCEPTION 'missing exact native custody receipt values'; END IF;
 body:=replace(body,needle,'p_after,CASE WHEN carried THEN jsonb_build_object(''custody_kind'',''carried'',''actor_position'',encode(p_position,''hex'')) ELSE to_jsonb(h) END,CASE WHEN carried_nested THEN jsonb_build_object(''custody_kind'',''nested'',''custody'',to_jsonb(n)) ELSE to_jsonb(e) END,');
 -- Main/nested carried Wrap transforms the same instance and leaves its exact
 -- physical custody row and ordinal untouched. Placed Wrap follows old custody.
 receipt_at:=position('INSERT INTO game_house_furniture_wrap_receipts' IN body);
 mutation_at:=receipt_at+position('IF p_action=1 THEN' IN substring(body FROM receipt_at))-1;
 update_at:=position('UPDATE game_item_instances SET definition_production_key=p_after' IN body);
 IF mutation_at<receipt_at OR update_at<=mutation_at THEN RAISE EXCEPTION 'missing mature native custody mutation block'; END IF;
 branch:=substring(body FROM mutation_at FOR update_at-mutation_at);
 body:=overlay(body placing E'IF NOT carried THEN\n '||branch||E'END IF;\n ' FROM mutation_at FOR update_at-mutation_at);
 needle:='SET definition_production_key=p_after,last_transaction_id=tx,state_revision=state_revision+1';
 IF position(needle IN body)=0 THEN RAISE EXCEPTION 'missing native instance successor update'; END IF;
 body:=replace(body,needle,needle||',quantity=game_house_furniture_wrap_quantity(tx)');
 EXECUTE body;
 -- A legitimate new hidden-amount wrapper may later be opened by the existing
 -- legacy owner. Derive only its quantity from that same immutable receipt;
 -- every old45 admission, authority, pair and custody check is retained.
 body:=pg_get_functiondef('game_house_furniture_wrap_before_srv7_timers(uuid,uuid,numeric,numeric,numeric,uuid,numeric,uuid,text,numeric,uuid,numeric,numeric,smallint,bytea,bytea,text,integer,text)'::regprocedure);
 IF position(needle IN body)=0 THEN RAISE EXCEPTION 'missing legacy instance successor update'; END IF;
 EXECUTE replace(body,needle,needle||',quantity=game_house_furniture_wrap_quantity(tx)');
END $writer$;

-- Only the new marker4 nested shape extends completion. Other paths still
-- require the original exact main entry/House/reclaim proof unchanged.
DO $nested$ DECLARE body TEXT; needle TEXT; replacement TEXT; BEGIN
 body:=pg_get_functiondef('game_house_furniture_wrap_complete(uuid)'::regprocedure);
 needle:='OR NOT EXISTS(SELECT 1 FROM game_item_container_entries e WHERE to_jsonb(e)=r.inventory_location)';
 IF position(needle IN body)=0 THEN RAISE EXCEPTION 'missing mature Wrap carried-custody completion'; END IF;
 replacement:=$new$OR NOT (EXISTS(SELECT 1 FROM game_item_container_entries e WHERE to_jsonb(e)=r.inventory_location)
    OR (octet_length(r.use_intent)=13 AND get_byte(r.use_intent,0)=4
     AND r.inventory_location->>'custody_kind'='nested'
     AND EXISTS(SELECT 1 FROM game_item_nested_entries nested_entry,
      LATERAL game_item_loop_owner(nested_entry.parent_item_instance_id) owner
      WHERE to_jsonb(nested_entry)=r.inventory_location->'custody' AND nested_entry.item_instance_id=r.item_instance_id
       AND nested_entry.world_id=r.world_id AND owner.owner_character=r.character_id AND owner.depth BETWEEN 0 AND 7)))$new$;
 EXECUTE replace(body,needle,replacement);
END $nested$;
