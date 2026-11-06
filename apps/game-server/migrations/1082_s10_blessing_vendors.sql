-- S10: source-bound nonpremium blessing vendors. Historical receipts remain immutable.
DO $$
DECLARE body TEXT;
BEGIN
 SELECT pg_get_constraintdef(oid) INTO STRICT body FROM pg_constraint
 WHERE conrelid='game_character_npc_purchase_receipts'::regclass AND conname='game_npc_purchase_service_shape';
 ALTER TABLE game_character_npc_purchase_receipts DROP CONSTRAINT game_npc_purchase_service_shape;
 EXECUTE 'ALTER TABLE game_character_npc_purchase_receipts ADD CONSTRAINT game_npc_purchase_service_shape CHECK (('
 ||substr(body,7)||') OR (cause_kind=3 AND blessing_keys IS NULL AND blessing_key IS NOT NULL
 AND blessing_key=''oteryn:blessing.''||substr(service_key,25) AND ((service_key=''oteryn:service.blessing.the_spiritual_shielding'' AND npc_key IN (''oteryn:npc.norf'')) OR (service_key=''oteryn:service.blessing.the_embrace_of_tibia'' AND npc_key IN (''oteryn:npc.humphrey'')) OR (service_key=''oteryn:service.blessing.the_fire_of_the_suns'' AND npc_key IN (''oteryn:npc.edala'')) OR (service_key=''oteryn:service.blessing.the_spark_of_the_phoenix'' AND npc_key IN (''oteryn:npc.pydar'')) OR (service_key=''oteryn:service.blessing.twist_of_fate'' AND npc_key IN (''oteryn:npc.alia'',''oteryn:npc.amanda'',''oteryn:npc.azalea'',''oteryn:npc.brewster'',''oteryn:npc.cedrik'',''oteryn:npc.ferus'',''oteryn:npc.iptar_sin'',''oteryn:npc.kasmir'',''oteryn:npc.kjesse'',''oteryn:npc.lorietta'',''oteryn:npc.maealil'',''oteryn:npc.ninev'',''oteryn:npc.prezil'',''oteryn:npc.quentin'',''oteryn:npc.yberius'',''oteryn:npc.zedrulon_the_fallen'')))))';
 SELECT pg_get_constraintdef(oid) INTO STRICT body FROM pg_constraint
 WHERE conrelid='game_item_fee_burns'::regclass AND conname='game_item_fee_burns_purchase_cause';
 body:=regexp_replace(body,' NOT VALID$','');
 ALTER TABLE game_item_fee_burns DROP CONSTRAINT game_item_fee_burns_purchase_cause;
 EXECUTE 'ALTER TABLE game_item_fee_burns ADD CONSTRAINT game_item_fee_burns_purchase_cause CHECK (('
 ||substr(body,7)||') OR (cause_kind=3 AND charm_key IS NULL AND purchase_service_key IN (''oteryn:service.blessing.the_spiritual_shielding'',''oteryn:service.blessing.the_embrace_of_tibia'',''oteryn:service.blessing.the_fire_of_the_suns'',''oteryn:service.blessing.the_spark_of_the_phoenix'',''oteryn:service.blessing.twist_of_fate''))) NOT VALID';
END;
$$;
-- Donor StdModule.bless uses getItemById(3057,true): carried, including nested bags.
-- Reuse BAGS-1's existing depth-eight carried tree and physical equipment roots.
CREATE FUNCTION game_s10_has_carried_amulet(p_character UUID) RETURNS BOOLEAN
LANGUAGE sql STABLE AS $$
 WITH RECURSIVE carried(item,depth) AS (
   (SELECT item_instance_id,0 FROM game_item_container_slots WHERE character_id=p_character
    UNION SELECT item_instance_id,0 FROM game_character_equipment_slots WHERE character_id=p_character)
   UNION ALL
   SELECT e.item_instance_id,c.depth+1 FROM carried c
   JOIN LATERAL (
     SELECT item_instance_id FROM game_item_container_entries
      WHERE parent_item_instance_id=c.item AND character_id=p_character
     UNION ALL SELECT item_instance_id FROM game_item_nested_entries WHERE parent_item_instance_id=c.item
   ) e ON true WHERE c.depth<8
 )
 SELECT EXISTS(SELECT 1 FROM carried c JOIN game_item_instances i ON i.item_instance_id=c.item
   WHERE i.definition_family='Item' AND i.definition_production_key='oteryn:item.tibia.i3057'
     AND i.lifecycle=1 AND i.quantity>0)
$$;
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_s10_has_carried_amulet(uuid) SET search_path=%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_s10_has_carried_amulet(uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_s10_has_carried_amulet(uuid) TO oteryn_game_runtime;

-- Quote at the receipt's locked level, using the accepted 2026-10-18 owner price ruling.
CREATE FUNCTION game_s10_blessing_quote_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NEW.service_key IN ('oteryn:service.blessing.the_spiritual_shielding','oteryn:service.blessing.the_embrace_of_tibia','oteryn:service.blessing.the_fire_of_the_suns','oteryn:service.blessing.the_spark_of_the_phoenix','oteryn:service.blessing.twist_of_fate') AND NEW.fee_gold_units <>
    (CASE WHEN NEW.service_key='oteryn:service.blessing.twist_of_fate' THEN
      CASE WHEN NEW.level_before<=30 THEN 2000 WHEN NEW.level_before<270 THEN 2000+200*(NEW.level_before-30) ELSE 50000 END
    ELSE CASE WHEN NEW.level_before<=30 THEN 2000 WHEN NEW.level_before<120 THEN 2000+200*(NEW.level_before-30) ELSE 20000+75*(NEW.level_before-120) END END) THEN
  RAISE EXCEPTION 'S10 blessing requires exact locked-level quote' USING ERRCODE='23514';
 END IF;
 IF NEW.service_key='oteryn:service.blessing.twist_of_fate' AND NOT (
   EXISTS(SELECT 1 FROM game_character_blessings WHERE character_id=NEW.character_id AND blessing_key<>'oteryn:blessing.twist_of_fate') OR
   game_s10_has_carried_amulet(NEW.character_id)) THEN
  RAISE EXCEPTION 'Twist requires another blessing or carried amulet of loss' USING ERRCODE='23514';
 END IF;
 RETURN NULL;
END;
$$;
CREATE CONSTRAINT TRIGGER game_s10_blessing_quote AFTER INSERT ON game_character_npc_purchase_receipts
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_s10_blessing_quote_guard();
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_s10_blessing_quote_guard() SET search_path=%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_s10_blessing_quote_guard() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_s10_blessing_quote_guard() TO oteryn_game_runtime;
