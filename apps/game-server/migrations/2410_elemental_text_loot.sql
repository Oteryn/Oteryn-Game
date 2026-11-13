-- M49: retire a direct corpse letter without copying private Text into value audit.
-- Current altar/Character/Scope authority remains the parent arrival's responsibility.
-- Item lock -> Text revision lock -> live Text lock matches the Text owner's writer.
-- Mutation operator: archive unchanged; a same-TX revision change fails completion.
ALTER TABLE game_elemental_loot_retire_receipts ADD COLUMN text_state_before JSONB
 CHECK(text_state_before IS NULL OR (jsonb_typeof(text_state_before)='object' AND octet_length(text_state_before::text)<=512));
CREATE FUNCTION game_elemental_loot_text_state(p_item UUID) RETURNS JSONB LANGUAGE plpgsql AS $$
DECLARE revision TEXT; high_water TEXT;
BEGIN
 PERFORM 1 FROM game_item_instances WHERE item_instance_id=p_item FOR UPDATE;
 IF NOT FOUND THEN RAISE EXCEPTION 'elemental text snapshot requires an actual item' USING ERRCODE='23514'; END IF;
 SELECT last_revision::text INTO high_water FROM game_item_text_revisions WHERE item_instance_id=p_item FOR UPDATE;
 SELECT text_revision::text INTO revision FROM game_item_texts WHERE item_instance_id=p_item FOR UPDATE;
 RETURN jsonb_build_object('text_revision',revision,'high_water',high_water);
END $$;
DO $migration$
DECLARE body TEXT; predecessor TEXT; at INTEGER;
BEGIN
 EXECUTE format('ALTER FUNCTION game_elemental_loot_text_state(uuid) SET search_path=pg_catalog,%I,pg_temp',current_schema());
 body:=pg_get_functiondef('game_elemental_loot_retire_admit()'::regprocedure);
 predecessor:='OR EXISTS(SELECT 1 FROM game_item_texts WHERE item_instance_id=i.item_instance_id)';
 IF strpos(body,predecessor)=0 THEN RAISE EXCEPTION 'missing elemental text admission predecessor'; END IF;
 body:=replace(body,predecessor,'');
 at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
 IF at=0 THEN RAISE EXCEPTION 'missing elemental text admission entry'; END IF;
 EXECUTE overlay(body placing E'BEGIN\n'||$arm$
   -- Stamp only actual revision metadata, never a body, writer or fingerprint.
   NEW.text_state_before:=game_elemental_loot_text_state(NEW.item_instance_id);
 $arm$ FROM at FOR 5);
 body:=pg_get_functiondef('game_elemental_loot_retire_complete(uuid)'::regprocedure);
 predecessor:='PERFORM game_elemental_arrival_complete(a.transaction_id);';
 IF strpos(body,predecessor)=0 THEN RAISE EXCEPTION 'missing elemental text completion predecessor'; END IF;
 EXECUTE replace(body,predecessor,$arm$
   IF r.text_state_before IS DISTINCT FROM game_elemental_loot_text_state(p_item) THEN
     RAISE EXCEPTION 'elemental loot retirement must archive its exact Text revisions' USING ERRCODE='23514';
   END IF;
 $arm$||predecessor);
END $migration$;
REVOKE ALL ON FUNCTION game_elemental_loot_text_state(UUID) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_elemental_loot_text_state(UUID) TO oteryn_game_runtime;
