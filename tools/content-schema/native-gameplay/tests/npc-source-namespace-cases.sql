BEGIN;
SET LOCAL ROLE oteryn_game_runtime;
DO $$DECLARE npc_source TEXT;npc_rejected INTEGER:=0;BEGIN
 FOREACH npc_source IN ARRAY ARRAY['quest','npc','item_action','mount_taming','task_board'] LOOP
  BEGIN
   INSERT INTO game_account_appearance_unlock_receipts(grant_occurrence_id,account_id,unlock_key,content_revision,granting_character_id,source_kind,source_event_id)
   VALUES(game_character_uuid_v7(),'01900000-0000-7000-8000-000000000021','oteryn:appearance-grant/npc/norma/citizen/addon2','crystal-201189f-npc-cosmetics-v1','01900000-0000-7000-8000-000000000041',npc_source,decode(repeat('11',16),'hex'));
   RAISE EXCEPTION 'NPC namespace accepted fake % source without parent',npc_source;
  EXCEPTION WHEN check_violation THEN npc_rejected:=npc_rejected+1;END;
 END LOOP;
 IF npc_rejected<>5 THEN RAISE EXCEPTION 'NPC namespace source coverage incomplete';END IF;
 RAISE NOTICE 'PASS NPC namespace rejects all five source-kind disguises';
END $$;
COMMIT;
DO $$BEGIN
 IF EXISTS(SELECT 1 FROM game_account_appearance_unlocks WHERE account_id='01900000-0000-7000-8000-000000000021') THEN RAISE EXCEPTION 'NPC forgery wrote permanent ownership';END IF;
 RAISE NOTICE 'PASS rejected NPC namespace leaves no permanent facts';
END $$;

-- Historical malformed receipts do not authorize a later permanent fact.
BEGIN;
SET LOCAL session_replication_role=replica;
INSERT INTO game_account_appearance_unlock_receipts(grant_occurrence_id,account_id,unlock_key,content_revision,granting_character_id,source_kind,source_event_id)
VALUES('01900000-0000-7000-8000-000000000097','01900000-0000-7000-8000-000000000021','oteryn:appearance-grant/npc/norma/citizen/addon2','crystal-201189f-npc-cosmetics-v1','01900000-0000-7000-8000-000000000041','quest',decode(repeat('11',16),'hex'));
COMMIT;
BEGIN;
SET LOCAL ROLE oteryn_game_runtime;
DO $$BEGIN
 BEGIN
  INSERT INTO game_account_appearance_unlocks(account_id,unlock_key,grant_occurrence_id) VALUES('01900000-0000-7000-8000-000000000021','oteryn:appearance-grant/npc/norma/citizen/addon2','01900000-0000-7000-8000-000000000097');
  RAISE EXCEPTION 'orphan historical NPC receipt authorized permanent fact';
 EXCEPTION WHEN check_violation THEN NULL;END;
 RAISE NOTICE 'PASS historical orphan cannot bypass fact-level NPC reciprocity';
END $$;
COMMIT;
