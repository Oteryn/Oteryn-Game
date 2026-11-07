-- Incremental 0993 amendment; the full NPC cash template also owns these bytes.
-- The old generic, Gem, NPC trade and all other audit-shape ceilings are intact.
DO $npc_cash_audit_shape$
DECLARE definition TEXT; at INTEGER; arm TEXT;
BEGIN
 definition:=pg_get_functiondef('game_item_audit_envelope_size_proven()'::regprocedure);
 IF strpos(definition,'SRV7_CLOSED_NPC_CASH_AUDIT')>0 THEN RAISE EXCEPTION 'NPC cash audit shape already installed';END IF;
 arm:=$arm$
 -- SRV7_CLOSED_NPC_CASH_AUDIT: source-qualified 508-leaf fee proof only.
 IF NEW.created_xact_id=pg_current_xact_id()
  AND octet_length(NEW.envelope)<=262144
  AND EXISTS(SELECT 1 FROM game_item_fee_burns fee
   JOIN game_npc_fee_cash_records cash USING(transaction_id)
   WHERE fee.transaction_id=NEW.transaction_id AND fee.event_id=NEW.event_id
    AND fee.cause_kind IN(473,474) AND fee.created_xact_id=pg_current_xact_id()
    AND cash.created_xact_id=pg_current_xact_id() AND cash.character_id=fee.character_id
    AND cash.world_id=fee.world_id AND fee.envelope_sha256=NEW.envelope_sha256
    AND NEW.envelope_sha256=sha256(NEW.envelope)) THEN
  PERFORM game_npc_cash_complete(NEW.transaction_id);
  RETURN NULL;
 END IF;
$arm$;
 at:=regexp_instr(definition,E'\\mBEGIN\\M',1,1,0,'i');
 IF at=0 THEN RAISE EXCEPTION 'NPC cash audit shape boundary changed';END IF;
 EXECUTE overlay(definition placing E'BEGIN\n'||arm from at for 5);
END $npc_cash_audit_shape$;
