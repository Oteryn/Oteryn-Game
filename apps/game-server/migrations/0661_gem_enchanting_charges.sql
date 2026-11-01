-- A fresh craft output installs its declared 1000 charges in the same transaction.
-- Later checkpoint/expiry changes still require the existing timed write owner.
DO $migration$
DECLARE body TEXT; name TEXT; arm TEXT; at INTEGER;
BEGIN
 FOREACH name IN ARRAY ARRAY['game_item_timed_state_row_guard()','game_item_timed_state_row_proven()'] LOOP
  body:=pg_get_functiondef(name::regprocedure);
  at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
  IF at=0 THEN RAISE EXCEPTION 'missing timed proof body %',name; END IF;
  arm:=$arm$IF TG_OP='INSERT' AND NEW.charges=1000 AND NEW.remaining_ms IS NULL AND NEW.deadline_at IS NULL AND NEW.state_revision=1
   AND EXISTS(SELECT 1 FROM game_item_instances i JOIN game_item_craft_records r ON r.transaction_id=i.minted_transaction_id WHERE i.item_instance_id=NEW.item_instance_id AND r.recipe='oteryn:enchanting.weapon' AND r.enchanting IS NOT NULL
    AND r.created_xact_id=pg_current_xact_id() AND (r.outputs->0->>'item')::uuid=NEW.item_instance_id AND (r.outputs->0->>'charges')::integer=1000)
   THEN $arm$;
  IF name='game_item_timed_state_row_guard()' THEN arm:=arm||'RETURN NEW; END IF;';
  ELSE arm:=arm||'PERFORM game_item_craft_complete((SELECT minted_transaction_id FROM game_item_instances WHERE item_instance_id=NEW.item_instance_id)); RETURN NULL; END IF;'; END IF;
  EXECUTE overlay(body placing E'BEGIN\n'||arm from at for 5);
 END LOOP;
END $migration$;
