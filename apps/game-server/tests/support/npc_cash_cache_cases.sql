-- Memo-mechanism tests use a privileged synthetic cached marker; they do not
-- claim to execute a genuine paid NPC transaction (covered by native418 PG).
DO $cache$ DECLARE relation TEXT;fee UUID:='01900000-0000-7000-8000-000000000073';BEGIN
 FOR relation IN SELECT c.relname FROM pg_trigger t JOIN pg_class c ON c.oid=t.tgrelid
 WHERE t.tgname='game_npc_cash_invalidate' ORDER BY c.relname LOOP
  INSERT INTO game_npc_cash_verified VALUES(pg_backend_pid(),pg_current_xact_id(),fee,statement_timestamp());
  EXECUTE format('DELETE FROM %I WHERE FALSE',relation);
  IF EXISTS(SELECT 1 FROM game_npc_cash_verified WHERE backend_pid=pg_backend_pid()) THEN
   RAISE EXCEPTION 'cache survived a footprint write statement: %',relation;END IF;
 END LOOP;
 IF (SELECT count(*) FROM pg_trigger WHERE tgname='game_npc_cash_invalidate')<>29 THEN
  RAISE EXCEPTION 'cache invalidation read-set changed without qualification';END IF;
 INSERT INTO game_npc_cash_verified VALUES(pg_backend_pid(),pg_current_xact_id(),fee,statement_timestamp());
 BEGIN
  PERFORM game_npc_cash_complete(fee);
  RAISE EXCEPTION 'cached marker bypassed current source authority';
 EXCEPTION WHEN check_violation THEN NULL;END;
 DELETE FROM game_npc_cash_verified WHERE backend_pid=pg_backend_pid();
 RAISE NOTICE 'SRV7_CASH_MEMO_MUTATION_AND_AUTHORITY cases=30';
END $cache$;
BEGIN;
SET LOCAL ROLE oteryn_game_runtime;
DO $private$ BEGIN
 BEGIN
  INSERT INTO game_npc_cash_verified VALUES(pg_backend_pid(),pg_current_xact_id(),'01900000-0000-7000-8000-000000000073',statement_timestamp());
  RAISE EXCEPTION 'runtime injected a successful proof';
 EXCEPTION WHEN insufficient_privilege THEN NULL;END;
 RAISE NOTICE 'SRV7_CASH_MEMO_RUNTIME_INJECTION_DENIED';
END $private$;
ROLLBACK;
-- Deferred constraint sequencing of the actual memo invalidation helper. This
-- deliberately small validator isolates invalidation from unrelated NPC setup.
CREATE TABLE srv7_cash_memo_probe(value INTEGER);
CREATE FUNCTION srv7_cash_memo_probe_check() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE fee UUID:='01900000-0000-7000-8000-000000000074';BEGIN
 IF EXISTS(SELECT 1 FROM game_npc_cash_verified WHERE backend_pid=pg_backend_pid()
  AND transaction_xid=pg_current_xact_id() AND fee_transaction_id=fee AND statement_started=statement_timestamp()) THEN RETURN NULL;END IF;
 IF (SELECT value FROM srv7_cash_memo_probe)<>1 THEN RAISE EXCEPTION 'mutated physical proof' USING ERRCODE='23514';END IF;
 INSERT INTO game_npc_cash_verified VALUES(pg_backend_pid(),pg_current_xact_id(),fee,statement_timestamp())
 ON CONFLICT(backend_pid,fee_transaction_id) DO UPDATE SET transaction_xid=EXCLUDED.transaction_xid,statement_started=EXCLUDED.statement_started;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER srv7_cash_memo_probe_consistent AFTER INSERT OR UPDATE ON srv7_cash_memo_probe
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION srv7_cash_memo_probe_check();
CREATE TRIGGER srv7_cash_memo_probe_invalidate AFTER INSERT OR UPDATE OR DELETE ON srv7_cash_memo_probe
 FOR EACH STATEMENT EXECUTE FUNCTION game_npc_cash_invalidate();
BEGIN;
INSERT INTO srv7_cash_memo_probe VALUES(1);
SET CONSTRAINTS srv7_cash_memo_probe_consistent IMMEDIATE;
SET CONSTRAINTS srv7_cash_memo_probe_consistent DEFERRED;
DO $changed$ BEGIN
 BEGIN
  UPDATE srv7_cash_memo_probe SET value=2;
  SET CONSTRAINTS srv7_cash_memo_probe_consistent IMMEDIATE;
  RAISE EXCEPTION 'later mutation escaped a previously successful deferred proof';
 EXCEPTION WHEN check_violation THEN NULL;END;
 IF (SELECT value FROM srv7_cash_memo_probe)<>1 THEN RAISE EXCEPTION 'negative mutation did not roll back';END IF;
 RAISE NOTICE 'SRV7_CASH_MEMO_DEFERRED_THEN_MUTATE_REJECTED';
END $changed$;
ROLLBACK;
DROP TABLE srv7_cash_memo_probe;
DROP FUNCTION srv7_cash_memo_probe_check();

-- SECURITY DEFINER completion is callable only by the runtime composition.
DO $cash_acl$ BEGIN
 IF has_function_privilege('oteryn_game_control','game_npc_cash_complete(uuid)','EXECUTE')
 OR NOT has_function_privilege('oteryn_game_runtime','game_npc_cash_complete(uuid)','EXECUTE') THEN
 RAISE EXCEPTION 'cash complete role surface differs from runtime-only contract';END IF;
END $cash_acl$;
BEGIN;
SET LOCAL ROLE oteryn_game_control;
DO $cash_acl_denied$ BEGIN
 BEGIN
  PERFORM game_npc_cash_complete('01900000-0000-7000-8000-000000000073');
  RAISE EXCEPTION 'unauthorized control executed private cash proof';
 EXCEPTION WHEN insufficient_privilege THEN NULL;END;
 RAISE NOTICE 'SRV7_CASH_COMPLETE_UNAUTHORIZED_EXECUTE_DENIED';
END $cash_acl_denied$;
ROLLBACK;
