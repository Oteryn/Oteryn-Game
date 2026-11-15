-- Compose the QMECH retirement admission arms into game_item_instance_change_proven()
-- after the accepted MAX501 proofs (2837), which pin its accepted source.
-- Each arm is the exact one its owner migration (2401, 2405, 2605, 2707) used to
-- insert; they are applied in that order, so the latest owner is checked first.
DO $migration$
DECLARE body TEXT; arm TEXT; at INTEGER;
BEGIN
 -- owner migration 2401.
 body:=pg_get_functiondef('game_item_instance_change_proven()'::regprocedure);
 at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
 IF at=0 THEN RAISE EXCEPTION 'missing Item change proof predecessor for 2401'; END IF;
     arm:=$arm$
     IF EXISTS(SELECT 1 FROM game_elemental_arrival_receipts r WHERE r.transaction_id=NEW.last_transaction_id
       AND r.created_xact_id=pg_current_xact_id() AND r.corpse_before->'item'=to_jsonb(OLD)) THEN
       PERFORM game_elemental_arrival_complete(NEW.last_transaction_id); RETURN NULL;
     END IF;
     $arm$;
 EXECUTE overlay(body placing E'BEGIN\n'||arm FROM at FOR 5);
 -- owner migration 2405.
 body:=pg_get_functiondef('game_item_instance_change_proven()'::regprocedure);
 at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
 IF at=0 THEN RAISE EXCEPTION 'missing Item change proof predecessor for 2405'; END IF;
     arm:=$arm$
     IF EXISTS(SELECT 1 FROM game_elemental_loot_retire_receipts r WHERE r.transaction_id=NEW.last_transaction_id
       AND r.created_xact_id=pg_current_xact_id() AND r.item_before=to_jsonb(OLD)) THEN
       PERFORM game_elemental_loot_retire_complete(NEW.item_instance_id); RETURN NULL;
     END IF;
     $arm$;
 EXECUTE overlay(body placing E'BEGIN\n'||arm FROM at FOR 5);
 -- owner migration 2605.
 body:=pg_get_functiondef('game_item_instance_change_proven()'::regprocedure);
 at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
 IF at=0 THEN RAISE EXCEPTION 'missing Item change proof predecessor for 2605'; END IF;
     arm:=$arm$
     IF EXISTS(SELECT 1 FROM game_elemental_altar_receipts r,LATERAL jsonb_array_elements(r.inputs) v
       WHERE r.transaction_id=NEW.last_transaction_id AND r.created_xact_id=pg_current_xact_id() AND v->'item'=to_jsonb(OLD)) THEN
       PERFORM game_elemental_altar_complete(NEW.last_transaction_id); RETURN NULL;
     END IF;
     $arm$;
 EXECUTE overlay(body placing E'BEGIN\n'||arm FROM at FOR 5);
 -- owner migration 2707.
 body:=pg_get_functiondef('game_item_instance_change_proven()'::regprocedure);
 at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
 IF at=0 THEN RAISE EXCEPTION 'missing Item change proof predecessor for 2707'; END IF;
   arm:=$arm$
    IF EXISTS(SELECT 1 FROM game_dreamer_ring_offerings r WHERE r.transaction_id=NEW.last_transaction_id
     AND r.item_instance_id=OLD.item_instance_id AND r.created_xact_id=pg_current_xact_id() AND r.before_item=to_jsonb(OLD)) THEN
     PERFORM game_dreamer_ring_complete(NEW.last_transaction_id); RETURN NULL;
    END IF;
$arm$;
 EXECUTE overlay(body placing E'BEGIN\n'||arm FROM at FOR 5);
END $migration$;
