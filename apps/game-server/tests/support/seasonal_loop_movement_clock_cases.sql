-- Assert the physical receipt timestamp is server-chosen even when runtime
-- explicitly supplies one. This rollback probes INSERT authority, not completion.
BEGIN;
SET LOCAL ROLE oteryn_game_runtime;
INSERT INTO game_item_loop_move_receipts(transaction_id,event_id,character_id,game_session_id,command_id,connection_generation,lease_generation,world_id,channel_id,scope_generation,content_digest,operation,line_count,intent,intent_sha256,created_at)
 VALUES('00000000-0000-7000-8000-000000000086','00000000-0000-7000-8000-000000000087','00000000-0000-7000-8000-000000000041','00000000-0000-7000-8000-000000000040',701,1,1,'00000000-0000-7000-8000-000000000042','00000000-0000-7000-8000-000000000043',1,decode(repeat('33',32),'hex'),4,1,'\x01',sha256('\x01'::bytea),'2100-01-01T00:00:00Z');
DO $$ BEGIN
 IF NOT EXISTS(SELECT 1 FROM game_item_loop_move_receipts WHERE command_id=701 AND created_at>=transaction_timestamp() AND created_at<=clock_timestamp()) THEN
  RAISE EXCEPTION 'LOOP_MOVEMENT_CLOCK_RUNTIME_TIMESTAMP_FORGERY' USING ERRCODE='23514'; END IF;
 RAISE NOTICE 'LOOP_MOVEMENT_CLOCK_PROOF_PASSED explicit_runtime_timestamp_ignored=true';
END $$;
ROLLBACK;
