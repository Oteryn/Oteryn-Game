-- QUESTCB2. Immutable altar BURN evidence is inseparable from its quest receipt.
ALTER TABLE game_character_quest_receipts DROP CONSTRAINT game_character_quest_receipts_cause_kind_check;
ALTER TABLE game_character_quest_receipts ADD CONSTRAINT game_character_quest_receipts_cause_kind_check
 CHECK(cause_kind IN ('command','use','step','claim_obligation','creature_death'));
-- Existing ItemInstance and Ground guards gain only this exact proof arm.
CREATE TABLE game_quest_callback_consumptions (
 transaction_id UUID NOT NULL CHECK(game_character_is_uuid_v7(transaction_id)),
 item_instance_id UUID NOT NULL REFERENCES game_item_instances,
 character_id UUID NOT NULL,
 cause_id UUID NOT NULL,
 cause_ordinal NUMERIC(20,0) NOT NULL CHECK(cause_ordinal BETWEEN 1 AND 18446744073709551615),
 transition_key TEXT NOT NULL CHECK(octet_length(transition_key) BETWEEN 1 AND 128),
 before_item JSONB NOT NULL,
 before_ground JSONB NOT NULL,
 quantity INT NOT NULL CHECK(quantity BETWEEN 1 AND 100),
 proof BYTEA NOT NULL CHECK(octet_length(proof) BETWEEN 1 AND 16384),
 proof_sha256 BYTEA NOT NULL CHECK(proof_sha256=sha256(proof)),
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
 PRIMARY KEY(transaction_id,item_instance_id),
 UNIQUE(character_id,cause_id,cause_ordinal,transition_key,item_instance_id),
 FOREIGN KEY(character_id,cause_id,cause_ordinal,transition_key)
   REFERENCES game_character_quest_receipts(character_id,cause_id,cause_ordinal,transition_key)
   DEFERRABLE INITIALLY DEFERRED
);

CREATE FUNCTION game_quest_callback_admit() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE i game_item_instances%ROWTYPE; g game_item_ground_locations%ROWTYPE;
BEGIN
 SELECT * INTO i FROM game_item_instances WHERE item_instance_id=NEW.item_instance_id FOR UPDATE;
 SELECT * INTO g FROM game_item_ground_locations WHERE item_instance_id=NEW.item_instance_id;
 IF i.item_instance_id IS NULL OR g.item_instance_id IS NULL
   OR to_jsonb(i) IS DISTINCT FROM NEW.before_item OR to_jsonb(g) IS DISTINCT FROM NEW.before_ground
   OR i.lifecycle<>1 OR i.quantity<NEW.quantity OR i.definition_family<>'Item'
   OR NOT game_item_loop_drop_ref(g.corpse_ref)
   OR EXISTS(SELECT 1 FROM game_item_keys WHERE item_instance_id=i.item_instance_id)
   OR EXISTS(SELECT 1 FROM game_item_timed_states WHERE item_instance_id=i.item_instance_id)
   OR EXISTS(SELECT 1 FROM game_item_texts WHERE item_instance_id=i.item_instance_id)
   OR EXISTS(SELECT 1 FROM game_item_imbuements WHERE item_instance_id=i.item_instance_id)
   OR EXISTS(SELECT 1 FROM game_item_container_entries WHERE parent_item_instance_id=i.item_instance_id)
   OR EXISTS(SELECT 1 FROM game_item_nested_entries WHERE parent_item_instance_id=i.item_instance_id)
 THEN RAISE EXCEPTION 'quest offering must be a plain locked Ground item' USING ERRCODE='23514'; END IF;
 NEW.created_xact_id:=pg_current_xact_id();
 RETURN NEW;
END $$;

CREATE FUNCTION game_quest_callback_complete() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE i game_item_instances%ROWTYPE;
BEGIN
 SELECT * INTO i FROM game_item_instances WHERE item_instance_id=NEW.item_instance_id;
 IF NEW.created_xact_id<>pg_current_xact_id()
   OR i.quantity<>(NEW.before_item->>'quantity')::bigint-NEW.quantity
   OR i.lifecycle<>(CASE WHEN i.quantity=0 THEN 2 ELSE 1 END)
   OR i.last_transaction_id IS DISTINCT FROM NEW.transaction_id
   OR (to_jsonb(i)-ARRAY['quantity','lifecycle','last_transaction_id']) IS DISTINCT FROM
      (NEW.before_item-ARRAY['quantity','lifecycle','last_transaction_id'])
   OR (i.quantity=0 AND EXISTS(SELECT 1 FROM game_item_ground_locations WHERE item_instance_id=i.item_instance_id))
   OR (i.quantity>0 AND NOT EXISTS(SELECT 1 FROM game_item_ground_locations g WHERE g.item_instance_id=i.item_instance_id AND to_jsonb(g)=NEW.before_ground))
   OR NOT EXISTS(SELECT 1 FROM game_character_quest_receipts r WHERE r.character_id=NEW.character_id
      AND r.cause_id=NEW.cause_id AND r.cause_ordinal=NEW.cause_ordinal AND r.transition_key=NEW.transition_key AND r.cause_kind IN ('use','step')
      AND r.created_xact_id=pg_current_xact_id())
 THEN RAISE EXCEPTION 'incomplete atomic quest offering' USING ERRCODE='23514'; END IF;
 RETURN NULL;
END $$;

DO $migration$
DECLARE n TEXT; body TEXT; arm TEXT; begin_at INT;
BEGIN
 FOREACH n IN ARRAY ARRAY['game_item_instance_guard','game_item_instance_change_proven','game_item_ground_removal_proven'] LOOP
  body:=pg_get_functiondef((n||'()')::regprocedure);
  begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
  IF begin_at=0 THEN RAISE EXCEPTION 'unexpected quest proof body: %',n; END IF;
  IF n='game_item_instance_guard' THEN
   arm:=$arm$
    IF TG_OP='UPDATE' AND EXISTS(SELECT 1 FROM game_quest_callback_consumptions r
      WHERE r.item_instance_id=OLD.item_instance_id AND r.transaction_id=NEW.last_transaction_id
        AND r.before_item=to_jsonb(OLD) AND r.created_xact_id=pg_current_xact_id()) THEN
     IF NOT EXISTS(SELECT 1 FROM game_quest_callback_consumptions r WHERE r.item_instance_id=OLD.item_instance_id
       AND r.transaction_id=NEW.last_transaction_id AND NEW.quantity=OLD.quantity-r.quantity
       AND NEW.lifecycle=CASE WHEN NEW.quantity=0 THEN 2 ELSE 1 END)
       OR (to_jsonb(NEW)-ARRAY['quantity','lifecycle','last_transaction_id']) IS DISTINCT FROM
          (to_jsonb(OLD)-ARRAY['quantity','lifecycle','last_transaction_id'])
     THEN RAISE EXCEPTION 'quest offering changes only its declared quantity' USING ERRCODE='23514'; END IF;
     RETURN NEW;
    END IF;
$arm$;
  ELSIF n='game_item_instance_change_proven' THEN
   arm:=$arm$
    IF EXISTS(SELECT 1 FROM game_quest_callback_consumptions r WHERE r.item_instance_id=NEW.item_instance_id
      AND r.transaction_id=NEW.last_transaction_id AND r.created_xact_id=pg_current_xact_id()) THEN RETURN NULL; END IF;
$arm$;
  ELSE
   arm:=$arm$
    IF TG_OP='DELETE' AND EXISTS(SELECT 1 FROM game_quest_callback_consumptions r JOIN game_item_instances i USING(item_instance_id)
      WHERE r.item_instance_id=OLD.item_instance_id AND r.before_ground=to_jsonb(OLD)
        AND i.quantity=0 AND i.lifecycle=2 AND i.last_transaction_id=r.transaction_id
        AND r.created_xact_id=pg_current_xact_id()) THEN RETURN NULL; END IF;
$arm$;
  END IF;
  EXECUTE overlay(body placing E'BEGIN\n'||arm from begin_at for 5);
 END LOOP;
END $migration$;

CREATE TRIGGER game_quest_callback_admit BEFORE INSERT ON game_quest_callback_consumptions FOR EACH ROW EXECUTE FUNCTION game_quest_callback_admit();
CREATE CONSTRAINT TRIGGER game_quest_callback_complete AFTER INSERT ON game_quest_callback_consumptions DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_quest_callback_complete();
CREATE TRIGGER game_quest_callback_immutable BEFORE UPDATE OR DELETE ON game_quest_callback_consumptions FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_quest_callback_no_truncate BEFORE TRUNCATE ON game_quest_callback_consumptions EXECUTE FUNCTION game_item_reject_truncate();
DO $$ DECLARE n TEXT; BEGIN
 FOREACH n IN ARRAY ARRAY['game_quest_callback_admit','game_quest_callback_complete'] LOOP
  EXECUTE format('ALTER FUNCTION %I() SET search_path=pg_catalog,%I,pg_temp',n,current_schema());
 END LOOP;
END $$;
REVOKE ALL ON game_quest_callback_consumptions FROM PUBLIC;
REVOKE ALL ON FUNCTION game_quest_callback_admit(),game_quest_callback_complete() FROM PUBLIC;
GRANT SELECT,INSERT ON game_quest_callback_consumptions TO oteryn_game_runtime;
GRANT SELECT ON game_quest_callback_consumptions TO oteryn_game_control;
