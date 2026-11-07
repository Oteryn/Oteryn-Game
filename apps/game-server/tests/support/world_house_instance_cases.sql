\set ON_ERROR_STOP on
SET search_path=spell_house_privacy_audit,pg_catalog;
-- Actual control-only identity binding, not Instance assignment/admission.
BEGIN;
INSERT INTO game_world_house_instances VALUES('019a0000-0000-7000-8000-000000000002','oteryn:content.house.test','r1','019a0000-0000-7000-8000-000000000081',decode(repeat('11',32),'hex'),decode(repeat('22',32),'hex'),'019a0000-0000-7000-8000-000000000082','1');
DO $$ BEGIN
 IF NOT EXISTS(SELECT 1 FROM game_world_house_instances WHERE created_xact_id=pg_current_xact_id()) THEN RAISE EXCEPTION 'House Instance physical XID substituted'; END IF;
END $$;
COMMIT;
DO $$ BEGIN
 BEGIN UPDATE game_world_house_instances SET instance_id='019a0000-0000-7000-8000-000000000083'; RAISE EXCEPTION 'Instance identity replaced withouthandoff'; EXCEPTION WHEN check_violation THEN NULL; END;
 BEGIN DELETE FROM game_world_house_instances; RAISE EXCEPTION 'Instance bindingdeleted'; EXCEPTION WHEN check_violation THEN NULL; END;
 BEGIN INSERT INTO game_world_house_instances SELECT * FROM game_world_house_instances; RAISE EXCEPTION 'Channel-local Housecopy accepted'; EXCEPTION WHEN unique_violation THEN NULL; END;
 BEGIN INSERT INTO game_world_house_instances SELECT '019a0000-0000-7000-8000-000000000003',house_key,house_revision,instance_id,compatible_content_digest,qualified_placement_digest,'019a0000-0000-7000-8000-000000000085',created_xact_id FROM game_world_house_instances;
 RAISE EXCEPTION 'foreignWorld HouseInstance accepted'; EXCEPTION WHEN foreign_key_violation THEN NULL; END;
END $$;
SET ROLE oteryn_game_runtime;
DO $$ BEGIN
 PERFORM 1 FROM game_world_house_instances LIMIT 1;
 BEGIN INSERT INTO game_world_house_instances SELECT * FROM game_world_house_instances; RAISE EXCEPTION 'runtimeallocated HouseInstance'; EXCEPTION WHEN insufficient_privilege THEN NULL; END;
END $$;
RESET ROLE;
SELECT 'World-House Instance same-XID/immutable/unique/World/no-runtime-allocation guards: PASS' AS result;
