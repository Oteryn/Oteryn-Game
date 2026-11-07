DO $$ BEGIN
 IF EXISTS(SELECT 1 FROM pg_constraint WHERE conrelid='game_house_furniture_kit_origins'::regclass AND contype='f' AND confrelid='game_npc_trades'::regclass) THEN RAISE EXCEPTION 'shop-only transaction FK survived';END IF;
 IF (SELECT count(*) FROM pg_constraint WHERE conrelid='game_house_furniture_kit_origins'::regclass AND contype='f' AND confrelid IN('game_item_instances'::regclass,'game_house_furniture_kit_catalogue'::regclass))<>2 THEN RAISE EXCEPTION 'item/catalogue FKs weakened';END IF;
 IF NOT EXISTS(SELECT 1 FROM pg_constraint c JOIN pg_attribute a ON a.attrelid=c.conrelid AND a.attname='mint_transaction_id' WHERE c.conrelid='game_house_furniture_kit_origins'::regclass AND c.contype='u' AND c.conkey=ARRAY[a.attnum]::smallint[]) THEN RAISE EXCEPTION 'mint uniqueness weakened';END IF;
 RAISE NOTICE 'PASS kit union preserves item catalogue and unique transaction';
 IF game_npc_royal_kit_proven('01900000-0000-7000-8000-000000000001','01900000-0000-7000-8000-000000000002') THEN RAISE EXCEPTION 'unarmed Royal proof accepted';END IF;
 RAISE NOTICE 'PASS unarmed Royal kit proof refused';
 IF game_npc_royal_inbox_admit('01900000-0000-7000-8000-000000000001','oteryn:npc-cosmetic/king_tibianus/golden-buy-armor') THEN RAISE EXCEPTION 'unarmed Inbox admission accepted';END IF;
 RAISE NOTICE 'PASS unarmed Royal Inbox admission refused';
END $$;
BEGIN;
SET LOCAL session_replication_role=replica;
INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id)
 VALUES('01900000-0000-7000-8000-000000000002','01900000-0000-7000-8000-000000000042','Item','oteryn:item.tibia.i23398','definition-r1',1,1,'01900000-0000-7000-8000-000000000003');
SET LOCAL session_replication_role=origin;
DO $$ BEGIN
 BEGIN
 INSERT INTO game_house_furniture_kit_origins(item_instance_id,mint_transaction_id,source_row,target_key,target_revision)VALUES('01900000-0000-7000-8000-000000000002','01900000-0000-7000-8000-000000000003',2000000,'oteryn:item.tibia.i31510','definition-r1');
 SET CONSTRAINTS ALL IMMEDIATE;
 RAISE EXCEPTION 'orphan paid Royal kit accepted';
 EXCEPTION WHEN check_violation THEN IF SQLERRM<>'royal origin requires its same-transaction paid NPC source' THEN RAISE;END IF;END;
 RAISE NOTICE 'PASS actual Royal orphan insert rejected by paid source proof';
 BEGIN
 INSERT INTO game_house_furniture_kit_origins(item_instance_id,mint_transaction_id,source_row,target_key,target_revision)
 SELECT '01900000-0000-7000-8000-000000000002','01900000-0000-7000-8000-000000000003',source_row,target_key,'definition-r1' FROM game_house_furniture_kit_catalogue WHERE source_row<1000000 ORDER BY source_row LIMIT 1;
 SET CONSTRAINTS ALL IMMEDIATE;
 RAISE EXCEPTION 'orphan legacy NPCBUY accepted after FK change';
 EXCEPTION WHEN check_violation THEN IF SQLERRM<>'furniture kit requires its exact same-transaction native NPCBUY mint' THEN RAISE;END IF;END;
 RAISE NOTICE 'PASS actual legacy orphan still requires native NPCBUY';
END $$;
SET CONSTRAINTS ALL IMMEDIATE;
DO $$ BEGIN
 BEGIN
 INSERT INTO game_house_furniture_kit_origins(item_instance_id,mint_transaction_id,source_row,target_key,target_revision)VALUES('01900000-0000-7000-8000-000000000002','01900000-0000-7000-8000-000000000003',2000002,'oteryn:item.tibia.i31510','definition-r1');
 RAISE EXCEPTION 'late wrong-source origin escaped previously forced constraints';
 EXCEPTION WHEN check_violation THEN IF SQLERRM<>'royal origin requires its same-transaction paid NPC source' THEN RAISE;END IF;END;
 RAISE NOTICE 'PASS late wrong-source forgery rejected after SET CONSTRAINTS';
END $$;
ROLLBACK;
