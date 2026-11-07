-- Carried costs include worn items; invalidate their durable equipment snapshot.
ALTER TABLE game_srv7_item_costs ADD COLUMN equipment_revision_before NUMERIC(20,0)
 CHECK(equipment_revision_before BETWEEN 1 AND 18446744073709551614);
DO $migration$ DECLARE body TEXT; needle TEXT; BEGIN
 body:=pg_get_functiondef('game_srv7_cost_admit()'::regprocedure);
 needle:=' NEW.created_xact_id:=pg_current_xact_id();';
 IF strpos(body,needle)=0 THEN RAISE EXCEPTION 'missing closed cost admission stamp'; END IF;
 EXECUTE replace(body,needle,$new$ IF EXISTS(SELECT 1 FROM jsonb_array_elements(NEW.inputs) AS cost_input(value) WHERE cost_input.value->>'kind'='equipment') THEN
  SELECT revision INTO STRICT NEW.equipment_revision_before FROM game_character_equipment_state
   WHERE character_id=NEW.character_id FOR UPDATE;
 ELSE NEW.equipment_revision_before:=NULL; END IF;
 NEW.created_xact_id:=pg_current_xact_id();$new$);
 body:=pg_get_functiondef('game_srv7_cost_complete()'::regprocedure);
 needle:=' IF NEW.cause_kind=''npc_cosmetic'' THEN';
 IF strpos(body,needle)=0 THEN RAISE EXCEPTION 'missing closed cost domain receipt check'; END IF;
 EXECUTE replace(body,needle,$new$ IF NEW.equipment_revision_before IS NOT NULL AND NOT EXISTS(
  SELECT 1 FROM game_character_equipment_state s WHERE s.character_id=NEW.character_id
   AND s.revision=NEW.equipment_revision_before+1 AND s.last_transaction_id=NEW.transaction_id) THEN
  RAISE EXCEPTION 'worn item cost lacks exact equipment successor' USING ERRCODE='23514'; END IF;
 IF NEW.cause_kind='npc_cosmetic' THEN$new$);
 body:=pg_get_functiondef('game_equipment_state_proven()'::regprocedure);
 needle:=E'BEGIN\n';
 IF strpos(body,needle)=0 THEN RAISE EXCEPTION 'unexpected equipment proof body'; END IF;
 EXECUTE replace(body,needle,$new$BEGIN
 IF EXISTS(SELECT 1 FROM game_srv7_item_costs r WHERE r.transaction_id=NEW.last_transaction_id
  AND r.character_id=NEW.character_id AND r.created_xact_id=pg_current_xact_id()
  AND r.equipment_revision_before=OLD.revision AND NEW.revision=OLD.revision+1
  AND (to_jsonb(NEW)-ARRAY['revision','last_transaction_id'])=(to_jsonb(OLD)-ARRAY['revision','last_transaction_id'])) THEN RETURN NULL; END IF;
$new$);
END $migration$;
