-- SRVGAP-C4. Crystal 201189f actions/items/usable_outfit_items.lua:57-102,
-- usable_mount_items.lua:11-34; novelty catalogues retain the other donor citations.
-- The existing one-unit item-use owner retains all custody, fencing and Type 2 audit guards.
DO $$ DECLARE n TEXT; previous TEXT; BEGIN
 FOREACH n IN ARRAY ARRAY['game_item_use_records_cause_kind_check','game_item_use_cause_shape'] LOOP
  SELECT pg_get_expr(conbin,conrelid) INTO STRICT previous FROM pg_constraint
   WHERE conrelid='game_item_use_records'::regclass AND conname=n AND contype='c';
  EXECUTE format('ALTER TABLE game_item_use_records DROP CONSTRAINT %I, ADD CONSTRAINT %I CHECK ((%s) OR (cause_kind=970 AND shape IN(1,3,4)))',n,n,previous);
 END LOOP;
END $$;
CREATE TABLE game_usable_action_receipts (
 transaction_id UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(transaction_id)),
 game_session_id UUID NOT NULL,
 command_id NUMERIC(20,0) NOT NULL CHECK(command_id BETWEEN 1 AND 18446744073709551615),
 character_id UUID NOT NULL REFERENCES game_character_roots,
 world_id UUID NOT NULL,
 channel_id UUID NOT NULL,
 source_item_instance_id UUID NOT NULL REFERENCES game_item_instances,
 intent BYTEA NOT NULL CHECK(octet_length(intent) BETWEEN 1 AND 529),
 action JSONB NOT NULL CHECK((jsonb_typeof(action)='object' AND jsonb_typeof(action->'grants')='array'
   AND jsonb_array_length(action->'grants')<=2 AND jsonb_typeof(action->'consume')='boolean') IS TRUE),
 consumed_transaction_id UUID UNIQUE REFERENCES game_item_use_records DEFERRABLE INITIALLY DEFERRED,
 created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id(),
 CHECK((action->>'consume')::boolean=(consumed_transaction_id IS NOT NULL)),
 CHECK(consumed_transaction_id IS NULL OR consumed_transaction_id=transaction_id),
 UNIQUE(game_session_id,command_id)
);
CREATE INDEX game_usable_action_character ON game_usable_action_receipts(character_id);
CREATE INDEX game_usable_action_source ON game_usable_action_receipts(source_item_instance_id);
CREATE FUNCTION game_usable_action_complete(p_transaction UUID) RETURNS void
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,public,pg_temp AS $$
DECLARE r game_usable_action_receipts%ROWTYPE; u game_item_use_records%ROWTYPE; g JSONB;
BEGIN
 SELECT * INTO r FROM game_usable_action_receipts WHERE transaction_id=p_transaction;
 IF r.transaction_id IS NULL OR r.created_xact_id<>pg_current_xact_id()
   OR NOT EXISTS(SELECT 1 FROM game_item_instances WHERE item_instance_id=r.source_item_instance_id
       AND world_id=r.world_id AND definition_family='Item'
       AND definition_production_key='oteryn:item.tibia.i'||(r.action->>'item_id'))
 THEN RAISE EXCEPTION 'usable action receipt/source missing' USING ERRCODE='23514'; END IF;
 IF r.consumed_transaction_id IS NOT NULL THEN
  SELECT * INTO u FROM game_item_use_records WHERE transaction_id=r.consumed_transaction_id;
  IF u.transaction_id IS NULL OR u.cause_kind<>970 OR u.created_xact_id<>r.created_xact_id
    OR (u.game_session_id,u.command_id,u.character_id,u.world_id,u.channel_id,u.source_item_instance_id)
      IS DISTINCT FROM (r.game_session_id,r.command_id,r.character_id,r.world_id,r.channel_id,r.source_item_instance_id)
    OR (r.action->>'output' IS NULL AND u.shape<>1)
    OR (r.action->>'output' IS NOT NULL AND (u.shape NOT IN(3,4) OR u.flask_definition_family<>'Item'
        OR u.flask_definition_production_key IS DISTINCT FROM 'oteryn:item.tibia.i'||(r.action->>'output')))
  THEN RAISE EXCEPTION 'usable action burn/transform differs' USING ERRCODE='23514'; END IF;
 END IF;
 IF (SELECT count(*) FROM game_account_appearance_unlock_receipts WHERE source_kind='item_action'
       AND source_event_id=uuid_send(r.transaction_id))<>jsonb_array_length(r.action->'grants')
 THEN RAISE EXCEPTION 'usable action grant count differs' USING ERRCODE='23514'; END IF;
 FOR g IN SELECT value FROM jsonb_array_elements(r.action->'grants') LOOP
  IF NOT EXISTS(SELECT 1 FROM game_account_appearance_unlock_receipts p
      JOIN game_account_appearance_unlocks f USING(grant_occurrence_id,account_id,unlock_key)
      JOIN game_character_roots c ON c.account_id=p.account_id
      WHERE c.character_id=r.character_id AND p.granting_character_id=r.character_id
       AND p.unlock_key=g#>>'{}' AND p.source_kind='item_action'
       AND p.source_event_id=uuid_send(r.transaction_id)
       AND p.content_revision='crystal-item-cosmetics-201189f-v1')
  THEN RAISE EXCEPTION 'usable action earned fact missing' USING ERRCODE='23514'; END IF;
 END LOOP;
END $$;
CREATE FUNCTION game_usable_action_consistent() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,public,pg_temp AS $$
BEGIN
 IF TG_TABLE_NAME='game_account_appearance_unlock_receipts' THEN
  -- Main's component/event cosmetic owner keeps validating its own receipt shape.
  IF NOT EXISTS(SELECT 1 FROM game_usable_action_receipts
      WHERE transaction_id=encode(NEW.source_event_id,'hex')::uuid) THEN RETURN NULL; END IF;
  PERFORM game_usable_action_complete(encode(NEW.source_event_id,'hex')::uuid);
 ELSE PERFORM game_usable_action_complete(NEW.transaction_id); END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_usable_action_receipt_complete AFTER INSERT ON game_usable_action_receipts
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_usable_action_consistent();
CREATE CONSTRAINT TRIGGER game_usable_action_burn_complete AFTER INSERT ON game_item_use_records
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW WHEN(NEW.cause_kind=970) EXECUTE FUNCTION game_usable_action_consistent();
CREATE CONSTRAINT TRIGGER game_usable_action_grant_complete AFTER INSERT ON game_account_appearance_unlock_receipts
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW WHEN(NEW.source_kind='item_action') EXECUTE FUNCTION game_usable_action_consistent();
CREATE TRIGGER game_usable_action_immutable BEFORE UPDATE OR DELETE ON game_usable_action_receipts
 FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_usable_action_no_truncate BEFORE TRUNCATE ON game_usable_action_receipts
 FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();
CREATE TRIGGER game_usable_action_stamp BEFORE INSERT ON game_usable_action_receipts
 FOR EACH ROW EXECUTE FUNCTION game_item_stamp_created_xact_id();
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_usable_action_complete(uuid) SET search_path=pg_catalog,%I,pg_temp',current_schema());
 EXECUTE format('ALTER FUNCTION game_usable_action_consistent() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON game_usable_action_receipts FROM PUBLIC;
REVOKE ALL ON FUNCTION game_usable_action_complete(UUID),game_usable_action_consistent() FROM PUBLIC;
GRANT SELECT,INSERT ON game_usable_action_receipts TO oteryn_game_runtime;
GRANT SELECT ON game_usable_action_receipts TO oteryn_game_control;

-- Route the shared source_kind to its immutable receipt owner. The existing trigger
-- still rejects orphan grants; C4's reciprocal deferred proof validates C4 grants.
DO $$ DECLARE body TEXT; at INTEGER; BEGIN
 body:=pg_get_functiondef('game_item_cosmetic_consistent()'::regprocedure);
 at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
 IF at=0 THEN RAISE EXCEPTION 'unexpected cosmetic proof body'; END IF;
 EXECUTE overlay(body placing E'BEGIN\n'||$arm$
 IF TG_TABLE_NAME='game_account_appearance_unlock_receipts' THEN
  IF EXISTS(SELECT 1 FROM game_usable_action_receipts
      WHERE transaction_id=encode(NEW.source_event_id,'hex')::uuid) THEN RETURN NULL; END IF;
 END IF;
$arm$ from at for 5);
END $$;
