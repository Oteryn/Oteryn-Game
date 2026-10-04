\set ON_ERROR_STOP on
SET search_path=spell_house_privacy_audit,pg_catalog;
-- Isolated persistence fixture only. It grants no session/runtime authority.
SET session_replication_role=replica;
INSERT INTO game_character_account_guards(account_id) VALUES('019a0000-0000-7000-8000-000000000001');
INSERT INTO game_character_roots(character_id,account_id,world_id,lifecycle,character_revision,profile_revision,ruleset_revision,content_revision,starter_template_revision,name) VALUES
('019a0000-0000-7000-8000-000000000011','019a0000-0000-7000-8000-000000000001','019a0000-0000-7000-8000-000000000002',1,1,'p1','r1','c1','s1','Owner'),
('019a0000-0000-7000-8000-000000000012','019a0000-0000-7000-8000-000000000001','019a0000-0000-7000-8000-000000000002',1,1,'p1','r1','c1','s1','Guest'),
('019a0000-0000-7000-8000-000000000013','019a0000-0000-7000-8000-000000000001','019a0000-0000-7000-8000-000000000003',1,1,'p1','r1','c1','s1','Foreign');
SET session_replication_role=origin;
INSERT INTO game_house_ownership VALUES('019a0000-0000-7000-8000-000000000002','oteryn:content.house.test','019a0000-0000-7000-8000-000000000011',1,'019a0000-0000-7000-8000-000000000099');
BEGIN;
INSERT INTO game_character_spell_privacy_receipts(receipt_id,character_id,game_session_id,command_id,revision_before,revision_after,preferences,created_xact_id) VALUES
('019a0000-0000-7000-8000-000000000021','019a0000-0000-7000-8000-000000000011','019a0000-0000-7000-8000-000000000030',1,0,1,'{"allow_all":false}','1');
INSERT INTO game_character_spell_privacy(character_id,revision,preferences,last_receipt) VALUES('019a0000-0000-7000-8000-000000000011',1,'{"allow_all":false}','019a0000-0000-7000-8000-000000000021');
SET CONSTRAINTS ALL IMMEDIATE;
DO $$ BEGIN IF NOT EXISTS(SELECT 1 FROM game_character_spell_privacy_receipts WHERE receipt_id='019a0000-0000-7000-8000-000000000021' AND created_xact_id=pg_current_xact_id()) THEN RAISE EXCEPTION 'receipt failed physical transaction stamp'; END IF; END $$;
COMMIT;
DO $$ BEGIN
 BEGIN UPDATE game_character_spell_privacy SET revision=3 WHERE character_id='019a0000-0000-7000-8000-000000000011'; RAISE EXCEPTION 'revision skip accepted'; EXCEPTION WHEN check_violation THEN NULL; END;
 BEGIN UPDATE game_character_spell_privacy_receipts SET preferences='{}' WHERE receipt_id='019a0000-0000-7000-8000-000000000021'; RAISE EXCEPTION 'receipt mutation accepted'; EXCEPTION WHEN check_violation THEN NULL; END;
END $$;
-- A structurally matching historical receipt cannot authorize today's state.
INSERT INTO game_character_spell_privacy_receipts(receipt_id,character_id,game_session_id,command_id,revision_before,revision_after,preferences) VALUES('019a0000-0000-7000-8000-000000000022','019a0000-0000-7000-8000-000000000011','019a0000-0000-7000-8000-000000000030',2,1,2,'{"allow_all":true}');
DO $$ BEGIN
 BEGIN
 UPDATE game_character_spell_privacy SET revision=2,preferences='{"allow_all":true}',last_receipt='019a0000-0000-7000-8000-000000000022' WHERE character_id='019a0000-0000-7000-8000-000000000011';
 SET CONSTRAINTS ALL IMMEDIATE;
 RAISE EXCEPTION 'historical receipt accepted';
 EXCEPTION WHEN check_violation THEN NULL; END;
END $$;
BEGIN;
INSERT INTO game_house_acl_receipts(receipt_id,world_id,house_key,list_id,character_id,game_session_id,command_id,ownership_revision,revision_before,revision_after,allow_everyone,members) VALUES
('019a0000-0000-7000-8000-000000000041','019a0000-0000-7000-8000-000000000002','oteryn:content.house.test',-1,'019a0000-0000-7000-8000-000000000011','019a0000-0000-7000-8000-000000000030',3,1,0,1,false,ARRAY['019a0000-0000-7000-8000-000000000012'::uuid]);
INSERT INTO game_house_acl VALUES('019a0000-0000-7000-8000-000000000002','oteryn:content.house.test',-1,1,false,ARRAY['019a0000-0000-7000-8000-000000000012'::uuid],'019a0000-0000-7000-8000-000000000041');
SET CONSTRAINTS ALL IMMEDIATE;
COMMIT;
DO $$ BEGIN
 BEGIN
 INSERT INTO game_house_acl_receipts(receipt_id,world_id,house_key,list_id,character_id,game_session_id,command_id,ownership_revision,revision_before,revision_after,allow_everyone,members) VALUES
('019a0000-0000-7000-8000-000000000042','019a0000-0000-7000-8000-000000000002','oteryn:content.house.test',-1,'019a0000-0000-7000-8000-000000000011','019a0000-0000-7000-8000-000000000030',4,1,1,2,false,ARRAY['019a0000-0000-7000-8000-000000000013'::uuid]);
 UPDATE game_house_acl SET revision=2,members=ARRAY['019a0000-0000-7000-8000-000000000013'::uuid],last_receipt='019a0000-0000-7000-8000-000000000042';
 SET CONSTRAINTS ALL IMMEDIATE;
 RAISE EXCEPTION 'foreign World ACL member accepted';
 EXCEPTION WHEN check_violation THEN NULL; END;
END $$;
INSERT INTO game_house_editors(editor_id,world_id,house_key,list_id,character_id,game_session_id,opening_command_id,ownership_revision,acl_revision,scope_generation,content_digest,expires_at,consumed) VALUES('019a0000-0000-7000-8000-000000000051','019a0000-0000-7000-8000-000000000002','oteryn:content.house.test',-1,'019a0000-0000-7000-8000-000000000011','019a0000-0000-7000-8000-000000000030',5,1,1,1,decode(repeat('11',32),'hex'),statement_timestamp()+interval '10 minutes',false);
DO $$ BEGIN
 BEGIN UPDATE game_house_editors SET game_session_id='019a0000-0000-7000-8000-000000000031',consumed=true; RAISE EXCEPTION 'editor rebinding accepted'; EXCEPTION WHEN check_violation THEN NULL; END;
END $$;
UPDATE game_house_editors SET consumed=true;
DO $$ BEGIN
 BEGIN UPDATE game_house_editors SET consumed=false; RAISE EXCEPTION 'editor reopen accepted'; EXCEPTION WHEN check_violation THEN NULL; END;
 IF EXISTS(SELECT 1 FROM game_character_spell_access) OR EXISTS(SELECT 1 FROM game_character_house_privileges) THEN RAISE EXCEPTION 'missing role projection became a default'; END IF;
END $$;
SET ROLE oteryn_game_runtime;
DO $$ BEGIN
 PERFORM 1 FROM game_house_ownership LIMIT 1;
 BEGIN INSERT INTO game_house_ownership VALUES('019a0000-0000-7000-8000-000000000002','oteryn:content.house.selfgrant','019a0000-0000-7000-8000-000000000011',1,'019a0000-0000-7000-8000-000000000099'); RAISE EXCEPTION 'runtime selfgrant accepted'; EXCEPTION WHEN insufficient_privilege THEN NULL; END;
END $$;
RESET ROLE;
SELECT 'privacy receipt/successor/history guards; House foreign-World/one-use/rebind/no-selfgrant guards: PASS' AS result;
