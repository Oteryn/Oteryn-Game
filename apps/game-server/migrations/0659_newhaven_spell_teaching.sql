-- Viola's paid teaching extends the existing NPC purchase/coin BURN owner.
-- Free spells ride quest transitions, without fabricating a zero-value gold fee.
DO $$ DECLARE body TEXT; name TEXT; BEGIN
 SELECT pg_get_constraintdef(oid) INTO STRICT body FROM pg_constraint
 WHERE conrelid='game_item_fee_burns'::regclass AND conname='game_item_fee_burns_cause_kind_check';
 ALTER TABLE game_item_fee_burns DROP CONSTRAINT game_item_fee_burns_cause_kind_check;
 EXECUTE 'ALTER TABLE game_item_fee_burns ADD CONSTRAINT game_item_fee_burns_cause_kind_check CHECK (('||substr(body,7)||') OR cause_kind=13)';
 SELECT regexp_replace(pg_get_constraintdef(oid),' NOT VALID$','') INTO STRICT body FROM pg_constraint
 WHERE conrelid='game_item_fee_burns'::regclass AND conname='game_item_fee_burns_purchase_cause';
 ALTER TABLE game_item_fee_burns DROP CONSTRAINT game_item_fee_burns_purchase_cause;
 EXECUTE 'ALTER TABLE game_item_fee_burns ADD CONSTRAINT game_item_fee_burns_purchase_cause CHECK (('||substr(body,7)||') OR (cause_kind=13 AND charm_key IS NULL AND ((purchase_service_key=''oteryn:spell.cure_poison'' AND fee_gold_units=150) OR (purchase_service_key=''oteryn:spell.magic_rope'' AND fee_gold_units=200)))) NOT VALID';
 SELECT conname,pg_get_constraintdef(oid) INTO STRICT name,body FROM pg_constraint
 WHERE conrelid='game_character_npc_purchase_receipts'::regclass AND contype='c'
   AND pg_get_constraintdef(oid) LIKE '%cause_kind%' AND pg_get_constraintdef(oid) NOT LIKE '%service_key%';
 EXECUTE format('ALTER TABLE game_character_npc_purchase_receipts DROP CONSTRAINT %I',name);
 EXECUTE 'ALTER TABLE game_character_npc_purchase_receipts ADD CONSTRAINT game_npc_purchase_kind CHECK (('||substr(body,7)||') OR cause_kind=13)';
 SELECT pg_get_constraintdef(oid) INTO STRICT body FROM pg_constraint
 WHERE conrelid='game_character_npc_purchase_receipts'::regclass AND conname='game_npc_purchase_service_shape';
 ALTER TABLE game_character_npc_purchase_receipts DROP CONSTRAINT game_npc_purchase_service_shape;
 EXECUTE 'ALTER TABLE game_character_npc_purchase_receipts ADD CONSTRAINT game_npc_purchase_service_shape CHECK (('||substr(body,7)||') OR (cause_kind=13 AND npc_key=''oteryn:npc.viola'' AND blessing_key IS NULL AND blessing_keys IS NULL AND ((service_key=''oteryn:spell.cure_poison'' AND fee_gold_units=150) OR (service_key=''oteryn:spell.magic_rope'' AND fee_gold_units=200))))';
 -- Extend the current Character fee proof without replacing its earlier branches.
 SELECT pg_get_functiondef('game_item_fee_burn_consistency_guard()'::regprocedure) INTO STRICT body;
 IF position('(NEW.cause_kind=5 AND EXISTS' IN body)=0 THEN RAISE EXCEPTION 'missing fee Character proof'; END IF;
 EXECUTE replace(body,'(NEW.cause_kind=5 AND EXISTS','(NEW.cause_kind=13 AND EXISTS(SELECT 1 FROM game_character_npc_purchase_receipts p WHERE p.character_id=NEW.character_id AND p.purchase_occurrence_id=NEW.cause_occurrence_id AND p.cause_kind=13 AND p.committed_character_revision=NEW.committed_character_revision AND p.fee_gold_units=NEW.fee_gold_units AND p.created_xact_id=pg_current_xact_id())) OR (NEW.cause_kind=5 AND EXISTS');
 SELECT pg_get_functiondef('game_npc_purchase_fee_source_guard()'::regprocedure) INTO STRICT body;
 IF position('NEW.cause_kind IN (2, 3)' IN body)>0 THEN
  EXECUTE replace(body,'NEW.cause_kind IN (2, 3)','NEW.cause_kind IN (2, 3, 13)');
 ELSIF position('NEW.cause_kind IN (2,3)' IN body)>0 THEN
  EXECUTE replace(body,'NEW.cause_kind IN (2,3)','NEW.cause_kind IN (2,3,13)');
 ELSE RAISE EXCEPTION 'missing NPC fee proof'; END IF;
END $$;

CREATE FUNCTION game_newhaven_spell_purchase_guard() RETURNS trigger
LANGUAGE plpgsql SET search_path=pg_catalog,public,pg_temp AS $$
BEGIN
 IF NEW.cause_kind=13 AND NOT EXISTS (
  SELECT 1 FROM game_character_known_spells s WHERE s.character_id=NEW.character_id
   AND s.spell_key=replace(NEW.service_key,'oteryn:spell.','candidate:spell/')) THEN
  RAISE EXCEPTION 'teaching requires atomic known spell grant' USING ERRCODE='23514';
 END IF;
 RETURN NULL;
END $$;
REVOKE ALL ON FUNCTION game_newhaven_spell_purchase_guard() FROM PUBLIC;
CREATE CONSTRAINT TRIGGER game_newhaven_spell_purchase_guard
AFTER INSERT ON game_character_npc_purchase_receipts DEFERRABLE INITIALLY DEFERRED
FOR EACH ROW EXECUTE FUNCTION game_newhaven_spell_purchase_guard();

DO $$ DECLARE name TEXT; BEGIN
 FOREACH name IN ARRAY ARRAY['game_newhaven_spell_purchase_guard()', 'game_item_fee_burn_consistency_guard()', 'game_npc_purchase_fee_source_guard()'] LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',name,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC',name);
 END LOOP;
END $$;
