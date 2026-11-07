-- Privileged cache-mechanism fixtures ONLY. These are not authoritative cash
-- records, Node contexts, native fee receipts or physical item MINTs.
DO $indexed_invalidation$
DECLARE relation TEXT;fee UUID:='01900000-0000-7000-8000-000000000091';item UUID:='01900000-0000-7000-8000-000000000092';n INTEGER:=0;
BEGIN
 FOR relation IN SELECT c.relname FROM pg_trigger t JOIN pg_class c ON c.oid=t.tgrelid WHERE t.tgname='game_npc_cash_invalidate' ORDER BY c.relname LOOP
  INSERT INTO game_npc_cash_verified VALUES(pg_backend_pid(),pg_current_xact_id(),fee,statement_timestamp());
  INSERT INTO game_npc_cash_verified_inputs VALUES(pg_backend_pid(),fee,item,pg_current_xact_id(),fee,fee,1,2,0,1,1,'oteryn:item.tibia.i3031',1,'{}'::jsonb,'{}'::jsonb);
  EXECUTE format('DELETE FROM %I WHERE FALSE',relation);
  IF EXISTS(SELECT 1 FROM game_npc_cash_verified_inputs WHERE backend_pid=pg_backend_pid()) THEN
   RAISE EXCEPTION 'indexed custody survived footprint mutation %',relation;END IF;
  n:=n+1;
 END LOOP;
 IF n<>29 THEN RAISE EXCEPTION 'indexed custody invalidation read-set drift %',n;END IF;
 -- Neither a privileged synthetic memo nor its exact-looking tuple can bypass
 -- the helper's mandatory real closed fee/current source proof.
 INSERT INTO game_npc_cash_verified VALUES(pg_backend_pid(),pg_current_xact_id(),fee,statement_timestamp());
 INSERT INTO game_npc_cash_verified_inputs VALUES(pg_backend_pid(),fee,item,pg_current_xact_id(),fee,fee,1,2,0,1,1,'oteryn:item.tibia.i3031',1,'{}'::jsonb,'{}'::jsonb);
 BEGIN PERFORM game_npc_cash_nested_delete_proven(item,fee,fee,1);
 RAISE EXCEPTION 'indexed marker bypassed real fee authority';EXCEPTION WHEN check_violation THEN NULL;END;
 DELETE FROM game_npc_cash_verified WHERE backend_pid=pg_backend_pid();
 RAISE NOTICE 'SRV7_INDEXED_CASH_CUSTODY_INVALIDATION_AND_NO_AUTHORITY cases=30';
END $indexed_invalidation$;
DO $indexed_acl$ DECLARE denied BOOLEAN;BEGIN
 SET LOCAL ROLE oteryn_game_runtime;
 denied:=FALSE;
 BEGIN INSERT INTO game_npc_cash_verified_inputs(backend_pid)VALUES(pg_backend_pid());EXCEPTION WHEN insufficient_privilege THEN denied:=TRUE;END;
 IF NOT denied THEN RAISE EXCEPTION 'runtime injected indexed custody';END IF;
 RESET ROLE;
 SET LOCAL ROLE oteryn_game_control;
 denied:=FALSE;
 BEGIN PERFORM game_npc_cash_nested_delete_proven(NULL,NULL,NULL,NULL);EXCEPTION WHEN insufficient_privilege THEN denied:=TRUE;END;
 IF NOT denied THEN RAISE EXCEPTION 'control executed runtime indexed custody proof';END IF;
 RESET ROLE;
 RAISE NOTICE 'SRV7_INDEXED_CASH_CUSTODY_ACL cases=2';
END $indexed_acl$;
