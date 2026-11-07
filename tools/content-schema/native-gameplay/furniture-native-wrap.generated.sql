-- Policy population only. No per-Item state or clock is introduced.
CREATE TABLE game_house_furniture_portable_wrap_sources (
 item_id INTEGER PRIMARY KEY CHECK(item_id>0),
 stack_max INTEGER NOT NULL CHECK(stack_max BETWEEN 1 AND 100)
);
INSERT INTO game_house_furniture_portable_wrap_sources VALUES
(9046,1),
(14687,1),
(14688,1),
(22737,100),
(23473,1),
(23536,100),
(23722,1),
(25903,1),
(25904,1),
(25905,1),
(25906,1),
(25907,1),
(25908,1),
(25909,1),
(25910,1),
(25911,1),
(25912,1),
(25913,1),
(25914,1),
(28525,1),
(36838,100),
(36951,100),
(36952,100),
(36953,100),
(36954,100),
(36955,100),
(36956,100),
(37354,100),
(37357,100),
(37358,100),
(37360,100),
(37362,100),
(37364,100),
(37366,100),
(37374,100),
(37375,100),
(37376,100),
(37377,100),
(37378,100),
(37379,100),
(37380,100),
(37382,100),
(37390,100),
(37391,100),
(37392,100),
(37393,100),
(37394,100),
(37395,100),
(37396,100),
(37471,100),
(37472,100),
(37491,100),
(37496,100),
(37497,100),
(37498,100),
(37499,100),
(37500,100),
(37501,100),
(37502,100),
(37503,100),
(37504,100),
(37505,100),
(37506,100),
(37507,100),
(37508,100),
(37509,100),
(37510,100),
(37511,100),
(37512,100),
(37513,100),
(37514,100),
(37515,100),
(37516,100),
(37517,100),
(37518,100),
(39669,100),
(39670,100),
(39671,100),
(39672,100),
(39673,100),
(39674,100),
(39675,100),
(39676,100),
(39677,100),
(39678,100),
(39679,100),
(39680,100),
(52589,100);
REVOKE ALL ON game_house_furniture_portable_wrap_sources FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
GRANT SELECT ON game_house_furniture_portable_wrap_sources TO oteryn_game_runtime;

-- Generic House action source stays exactly quantity1. Only source-qualified
-- portable Wrap stacks receive this fallback, under the mature lock/fence order.
CREATE FUNCTION game_house_furniture_wrap_source(p_session UUID,p_character UUID,p_connection NUMERIC,
 p_lease NUMERIC,p_scope NUMERIC,p_node UUID,p_node_revision NUMERIC,p_world UUID,p_house TEXT,p_item UUID)
RETURNS TABLE(before_item JSONB,location JSONB,provenance JSONB,spatial_position BYTEA,stack_ordinal TEXT)
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE i game_item_instances%ROWTYPE; e game_item_container_entries%ROWTYPE;
 n game_item_nested_entries%ROWTYPE;
BEGIN
 RETURN QUERY SELECT * FROM game_house_construction_source(p_session,p_character,p_connection,p_lease,p_scope,
  p_node,p_node_revision,p_world,p_house,p_item);
 IF FOUND THEN RETURN; END IF;
 SELECT candidate.* INTO i FROM game_item_instances candidate
 JOIN game_house_furniture_portable_wrap_sources policy
  ON candidate.definition_production_key='oteryn:item.tibia.i'||policy.item_id::text
 WHERE candidate.item_instance_id=p_item AND candidate.world_id=p_world
  AND candidate.lifecycle=1 AND candidate.definition_family='Item' AND candidate.definition_revision_ref='definition-r1'
  AND candidate.quantity BETWEEN 2 AND policy.stack_max FOR UPDATE OF candidate;
 IF NOT FOUND THEN RETURN; END IF;
 SELECT entry.* INTO e FROM game_item_container_entries entry
 JOIN game_item_container_slots bag ON bag.character_id=entry.character_id
  AND bag.item_instance_id=entry.parent_item_instance_id AND bag.world_id=entry.world_id
 WHERE entry.item_instance_id=p_item AND entry.world_id=p_world AND entry.character_id=p_character FOR UPDATE OF entry;
 IF FOUND THEN
  RETURN QUERY SELECT to_jsonb(i),jsonb_build_object('custody_kind','container','custody',to_jsonb(e)),
   'null'::jsonb,NULL::bytea,e.placement_ordinal::text;
  RETURN;
 END IF;
 SELECT entry.* INTO n FROM game_item_nested_entries entry,
 LATERAL game_item_loop_owner(entry.parent_item_instance_id) owner
 WHERE entry.item_instance_id=p_item AND entry.world_id=p_world
  AND owner.owner_character=p_character AND owner.depth BETWEEN 0 AND 7 FOR UPDATE OF entry;
 IF FOUND THEN
  RETURN QUERY SELECT to_jsonb(i),jsonb_build_object('custody_kind','nested','custody',to_jsonb(n)),
   'null'::jsonb,NULL::bytea,n.placement_ordinal::text;
 END IF;
END $$;
DO $path$ BEGIN
 EXECUTE format('ALTER FUNCTION game_house_furniture_wrap_source(uuid,uuid,numeric,numeric,numeric,uuid,numeric,uuid,text,uuid) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $path$;
REVOKE ALL ON FUNCTION game_house_furniture_wrap_source(UUID,UUID,NUMERIC,NUMERIC,NUMERIC,UUID,NUMERIC,UUID,TEXT,UUID)
 FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
GRANT EXECUTE ON FUNCTION game_house_furniture_wrap_source(UUID,UUID,NUMERIC,NUMERIC,NUMERIC,UUID,NUMERIC,UUID,TEXT,UUID)
 TO oteryn_game_runtime;

-- Hidden amount is the immutable original source quantity, not mutable attributes.
CREATE FUNCTION game_house_furniture_wrap_quantity(p_transaction UUID) RETURNS NUMERIC
LANGUAGE plpgsql STABLE SECURITY DEFINER AS $$
DECLARE wrap_receipt game_house_furniture_wrap_receipts%ROWTYPE;
 source_wrap game_house_furniture_wrap_receipts%ROWTYPE; source_amount NUMERIC;
BEGIN
 SELECT * INTO wrap_receipt FROM game_house_furniture_wrap_receipts WHERE transaction_id=p_transaction;
 IF NOT FOUND THEN RAISE EXCEPTION 'WRAP_NOT_WRAPABLE'; END IF;
 source_amount:=(wrap_receipt.before_item->>'quantity')::numeric;
 IF source_amount IS NULL OR source_amount<1 OR source_amount>100 OR source_amount<>trunc(source_amount) THEN RAISE EXCEPTION 'WRAP_NOT_WRAPABLE'; END IF;
 IF wrap_receipt.action=1 THEN
  IF source_amount<>1 AND NOT coalesce((octet_length(wrap_receipt.use_intent)=13 AND get_byte(wrap_receipt.use_intent,0)=4
   AND wrap_receipt.before_item->>'definition_family'='Item'
   AND wrap_receipt.before_item->>'definition_revision_ref'='definition-r1'
   AND EXISTS(SELECT 1 FROM game_house_furniture_portable_wrap_sources wrap_policy
    WHERE wrap_receipt.before_item->>'definition_production_key'='oteryn:item.tibia.i'||wrap_policy.item_id::text
     AND source_amount BETWEEN 2 AND wrap_policy.stack_max AND source_amount=trunc(source_amount))),FALSE)
   THEN RAISE EXCEPTION 'WRAP_NOT_WRAPABLE'; END IF;
  RETURN 1;
 END IF;
 IF source_amount<>1 THEN RAISE EXCEPTION 'WRAP_NOT_WRAPABLE'; END IF;
 IF wrap_receipt.before_item->>'definition_production_key'<>'oteryn:item.tibia.i23398' THEN RETURN 1; END IF;
 SELECT prior_wrap.* INTO source_wrap FROM game_house_furniture_wrap_receipts prior_wrap
 WHERE prior_wrap.item_instance_id=wrap_receipt.item_instance_id AND prior_wrap.world_id=wrap_receipt.world_id
  AND prior_wrap.action=1 AND prior_wrap.after_key=wrap_receipt.before_item->>'definition_production_key'
  AND (prior_wrap.before_item->>'state_revision')::numeric<(wrap_receipt.before_item->>'state_revision')::numeric
 ORDER BY (prior_wrap.before_item->>'state_revision')::numeric DESC LIMIT 1;
 IF NOT FOUND OR (source_wrap.before_item->>'quantity')::numeric=1 THEN RETURN 1; END IF;
 source_amount:=(source_wrap.before_item->>'quantity')::numeric;
 IF octet_length(source_wrap.use_intent) IS DISTINCT FROM 13 OR get_byte(source_wrap.use_intent,0) IS DISTINCT FROM 4
  OR source_wrap.before_item->>'definition_production_key' IS DISTINCT FROM wrap_receipt.after_key
  OR NOT EXISTS(SELECT 1 FROM game_house_furniture_portable_wrap_sources wrap_policy
   WHERE wrap_receipt.after_key='oteryn:item.tibia.i'||wrap_policy.item_id::text
    AND source_amount BETWEEN 2 AND wrap_policy.stack_max AND source_amount=trunc(source_amount))
 THEN RAISE EXCEPTION 'WRAP_NOT_WRAPABLE'; END IF;
 RETURN source_amount;
END $$;
DO $path$ BEGIN
 EXECUTE format('ALTER FUNCTION game_house_furniture_wrap_quantity(uuid) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $path$;
REVOKE ALL ON FUNCTION game_house_furniture_wrap_quantity(UUID) FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;

-- Change only the exact immutable Wrap arm of the existing Item guard.
DO $guard$ DECLARE body TEXT; old_arm TEXT; new_arm TEXT; BEGIN
 body:=pg_get_functiondef('game_item_instance_guard()'::regprocedure);
 old_arm:=$old$IF TG_OP='UPDATE' AND EXISTS(SELECT 1 FROM game_house_furniture_wrap_receipts r WHERE r.item_instance_id=OLD.item_instance_id
     AND r.transaction_id=NEW.last_transaction_id AND r.before_item=to_jsonb(OLD) AND r.created_xact_id=pg_current_xact_id()) THEN
     IF NEW.state_revision<>OLD.state_revision+1 OR NEW.lifecycle<>1 OR NEW.quantity<>OLD.quantity
      OR (to_jsonb(NEW)-ARRAY['definition_production_key','last_transaction_id','state_revision']) IS DISTINCT FROM
       (to_jsonb(OLD)-ARRAY['definition_production_key','last_transaction_id','state_revision'])
      THEN RAISE EXCEPTION 'wrap preserves item lineage' USING ERRCODE='23514'; END IF;
     RETURN NEW;
    END IF;$old$;
 IF position(old_arm IN body)=0 THEN RAISE EXCEPTION 'missing exact immutable Wrap Item arm'; END IF;
 new_arm:=replace(old_arm,'NEW.quantity<>OLD.quantity','NEW.quantity IS DISTINCT FROM game_house_furniture_wrap_quantity(NEW.last_transaction_id)');
 new_arm:=replace(new_arm,'''definition_production_key'',''last_transaction_id'',''state_revision''','''definition_production_key'',''last_transaction_id'',''state_revision'',''quantity''');
 EXECUTE replace(body,old_arm,new_arm);
 body:=pg_get_functiondef('game_house_furniture_wrap_complete(uuid)'::regprocedure);
 old_arm:=$old$OR i.definition_production_key IS DISTINCT FROM r.after_key$old$;
 IF position(old_arm IN body)=0 THEN RAISE EXCEPTION 'missing exact Wrap complete definition proof'; END IF;
 body:=replace(body,old_arm,old_arm||' OR i.quantity IS DISTINCT FROM game_house_furniture_wrap_quantity(p_tx)');
 body:=replace(body,'''definition_production_key'',''last_transaction_id'',''state_revision''','''definition_production_key'',''last_transaction_id'',''state_revision'',''quantity''');
 EXECUTE body;
END $guard$;

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
