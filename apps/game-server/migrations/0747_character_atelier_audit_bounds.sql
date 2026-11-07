-- P07 registered ATELIER-RL-AUDIT-BYTES: the new carried/stash fee and split
-- dismantle forms retain one type-2 outbox event under their own 262144-byte ceiling.
-- Preserve the existing one-item, fee, NPC trade and reward-inbox ceilings.
ALTER TABLE game_item_audit_outbox DROP CONSTRAINT IF EXISTS game_item_audit_outbox_envelope_check;
ALTER TABLE game_item_audit_outbox ADD CONSTRAINT game_item_audit_outbox_envelope_check
 CHECK(octet_length(envelope) BETWEEN 1 AND 262144);

DO $migration$
DECLARE body TEXT; at_begin INT;
BEGIN
 body:=pg_get_functiondef('game_item_audit_envelope_size_proven()'::regprocedure);
 IF position('Atelier registered audit ceiling' IN body)=0 THEN
  at_begin:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
  IF at_begin=0 THEN RAISE EXCEPTION 'missing existing audit size guard BEGIN'; END IF;
  EXECUTE overlay(body placing $arm$BEGIN
 -- Atelier registered audit ceiling: only immutable evidence from this physical transaction.
 IF octet_length(NEW.envelope)<=262144 AND NEW.created_xact_id=pg_current_xact_id()
  AND NEW.event_type_id=2 AND NEW.transaction_ordinal=1 AND NEW.transaction_count=1
  AND NEW.publication_state=1 AND NEW.published_at IS NULL
  AND (EXISTS(SELECT 1 FROM game_atelier_item_records a
    WHERE a.transaction_id=NEW.transaction_id AND a.event_id=NEW.event_id AND a.cause='DISMANTLE'
     AND a.created_xact_id=pg_current_xact_id() AND a.occurred_at=NEW.occurred_at
     AND a.envelope_sha256=NEW.envelope_sha256
     AND (a.outputs->0->>'item')::uuid=NEW.item_instance_id)
   OR EXISTS(SELECT 1 FROM game_item_fee_burns f JOIN game_atelier_fee_coin_records c USING(transaction_id)
    WHERE f.transaction_id=NEW.transaction_id AND f.event_id=NEW.event_id AND f.cause_kind IN (9,10,11)
     AND f.created_xact_id=pg_current_xact_id() AND c.created_xact_id=pg_current_xact_id()
     AND c.character_id=f.character_id AND c.world_id=f.world_id
     AND f.occurred_at=NEW.occurred_at AND f.envelope_sha256=NEW.envelope_sha256)) THEN
  RETURN NULL;
 END IF;
$arm$ from at_begin for 5);
 END IF;
 EXECUTE format('ALTER FUNCTION game_item_audit_envelope_size_proven() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $migration$;
REVOKE ALL ON FUNCTION game_item_audit_envelope_size_proven() FROM PUBLIC;
