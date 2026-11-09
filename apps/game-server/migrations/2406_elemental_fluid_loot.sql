-- M49 follows corpse decay: retire the physical item and retain fluid metadata
-- as archived component history, binding that exact history to the own receipt.
ALTER TABLE game_elemental_loot_retire_receipts ADD COLUMN fluid_before JSONB
 CHECK(fluid_before IS NULL OR (jsonb_typeof(fluid_before)='object' AND octet_length(fluid_before::text)<=1024));
DO $migration$
DECLARE body TEXT; predecessor TEXT; at INTEGER;
BEGIN
 body:=pg_get_functiondef('game_elemental_loot_retire_admit()'::regprocedure);
 predecessor:='OR EXISTS(SELECT 1 FROM game_item_fluid_states WHERE item_instance_id=i.item_instance_id)';
 IF strpos(body,predecessor)=0 THEN RAISE EXCEPTION 'missing elemental fluid admission predecessor'; END IF;
 body:=replace(body,predecessor,'');
 body:=replace(body,'elemental loot retirement requires an exact plain direct corpse entry',
                    'elemental loot retirement requires an exact direct corpse entry');
 at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
 IF at=0 THEN RAISE EXCEPTION 'missing elemental fluid admission entry'; END IF;
 EXECUTE overlay(body placing E'BEGIN\n'||$arm$
   -- Stamp actual locked component state; caller-supplied history is not proof.
   SELECT to_jsonb(f) INTO NEW.fluid_before FROM game_item_fluid_states f
     WHERE f.item_instance_id=NEW.item_instance_id FOR UPDATE;
 $arm$ FROM at FOR 5);
 body:=pg_get_functiondef('game_elemental_loot_retire_complete(uuid)'::regprocedure);
 predecessor:='PERFORM game_elemental_arrival_complete(a.transaction_id);';
 IF strpos(body,predecessor)=0 THEN RAISE EXCEPTION 'missing elemental fluid completion predecessor'; END IF;
 EXECUTE replace(body,predecessor,$arm$
   IF r.fluid_before IS DISTINCT FROM (SELECT to_jsonb(f) FROM game_item_fluid_states f WHERE f.item_instance_id=p_item) THEN
     RAISE EXCEPTION 'elemental loot retirement must archive its exact fluid component' USING ERRCODE='23514';
   END IF;
 $arm$||predecessor);
END $migration$;
