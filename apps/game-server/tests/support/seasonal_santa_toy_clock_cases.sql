-- Run after shared Cake fixtures using --case-overlay. Immutable source rules
-- and a real same-XID individual movement are required for toy clock creation.
BEGIN;
SET LOCAL ROLE oteryn_game_runtime;
DO $$ BEGIN
 BEGIN
  UPDATE game_santa_toy_lifetimes SET duration_ms=3000 WHERE item_key='oteryn:item.tibia.i6566';
  RAISE EXCEPTION 'runtime changed toy source rules';
 EXCEPTION WHEN insufficient_privilege THEN RAISE NOTICE 'TOY_CLOCK_RULES_IMMUTABLE_PASSED'; END;
 BEGIN
  PERFORM game_santa_toy_movement_start('00000000-0000-7000-8000-000000000099');
  RAISE EXCEPTION 'runtime started clock without actual movement';
 EXCEPTION WHEN check_violation THEN RAISE NOTICE 'TOY_CLOCK_REQUIRES_CURRENT_MOVEMENT_PASSED'; END;
END $$;
ROLLBACK;
BEGIN;
SET LOCAL session_replication_role=replica;
INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id)
 VALUES('00000000-0000-7000-8000-000000000091','00000000-0000-7000-8000-000000000042','Item','oteryn:item.tibia.i6566','definition-r1',1,1,'00000000-0000-7000-8000-000000000092');
SET LOCAL session_replication_role=origin;
SET LOCAL ROLE oteryn_game_runtime;
DO $$ BEGIN
 BEGIN
  INSERT INTO game_spell_item_temporal_schedules(item_instance_id,source_transaction_id,source_ordinal,duration_millis,expires_at_unix_ms,definition_family,definition_production_key,definition_revision,target_family,target_production_key,target_revision,content_digest)
   VALUES('00000000-0000-7000-8000-000000000091','00000000-0000-7000-8000-000000000092',1,3000,1,'Item','oteryn:item.tibia.i6566','definition-r1','Item','oteryn:item.tibia.i5791','definition-r1',decode(repeat('33',32),'hex'));
  SET CONSTRAINTS ALL IMMEDIATE;
  RAISE EXCEPTION 'runtime forged unmoved toy clock';
 EXCEPTION WHEN check_violation OR foreign_key_violation THEN RAISE NOTICE 'TOY_CLOCK_UNMOVED_INSTANCE_REJECTED_PASSED'; END;
END $$;
ROLLBACK;
