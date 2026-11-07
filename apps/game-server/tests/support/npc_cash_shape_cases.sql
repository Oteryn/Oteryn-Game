-- Execute the real registered shape trigger with the real outbox CHECKs.
-- These are unarmed adversarial payloads, not forged authoritative NPC fees.
CREATE TEMP TABLE srv7_cash_audit_probe(LIKE game_item_audit_outbox INCLUDING DEFAULTS INCLUDING CONSTRAINTS);
CREATE CONSTRAINT TRIGGER srv7_cash_audit_probe AFTER INSERT ON srv7_cash_audit_probe
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_item_audit_envelope_size_proven();
BEGIN;
DO $shapes$
DECLARE n INTEGER;payload BYTEA;digest BYTEA;registered_type BIGINT;created XID8;rejected BOOLEAN;
BEGIN
 FOR n IN 1..5 LOOP
  payload:=decode(repeat('ab',CASE WHEN n=1 THEN 262145 ELSE 131072 END),'hex');
  digest:=CASE WHEN n=2 THEN decode(repeat('00',32),'hex') ELSE sha256(payload) END;
  registered_type:=CASE WHEN n=3 THEN 3 ELSE 2 END;
  created:=CASE WHEN n=4 THEN ((pg_current_xact_id()::text)::numeric-1)::text::xid8 ELSE pg_current_xact_id() END;
  rejected:=FALSE;
  BEGIN
   INSERT INTO srv7_cash_audit_probe(event_id,transaction_id,transaction_ordinal,transaction_count,event_type_id,schema_revision,retention_profile_id,item_instance_id,occurred_at,expires_at,envelope,envelope_sha256,publication_state,published_at,created_xact_id)
   VALUES('01900000-0000-7000-8000-000000000081','01900000-0000-7000-8000-000000000082',1,1,registered_type,1,'DUR03_ONE_ITEM_DURABLE_AUDIT_RETENTION_V1',NULL,1,7776000001,payload,digest,1,NULL,created);
   SET CONSTRAINTS srv7_cash_audit_probe IMMEDIATE;
  EXCEPTION WHEN check_violation THEN rejected:=TRUE;END;
  IF NOT rejected THEN RAISE EXCEPTION 'unarmed cash audit shape admitted case=%',n;END IF;
  SET CONSTRAINTS srv7_cash_audit_probe DEFERRED;
 END LOOP;
 IF EXISTS(SELECT 1 FROM game_item_audit_outbox) THEN RAISE EXCEPTION 'shape probe manufactured a real audit row';END IF;
 RAISE NOTICE 'SRV7_NPC_CASH_UNARMED_SHAPES cases=5 oversize_wronghash_wrongtype_stalexid_unarmed';
END $shapes$;
ROLLBACK;
DROP TABLE srv7_cash_audit_probe;
