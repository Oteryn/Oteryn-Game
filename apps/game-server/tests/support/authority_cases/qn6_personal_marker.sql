\set ON_ERROR_STOP on
\timing on
BEGIN;
CREATE SCHEMA qn6_marker;
SET LOCAL search_path=qn6_marker,public;
SELECT 'QN6_POSTGRES_READY personal_marker',current_setting('server_version');
CREATE TABLE game_character_roots(character_id uuid PRIMARY KEY);
CREATE TABLE game_quest_reward_sources(character_id uuid,cause_id uuid,cause_ordinal numeric,transition_key text,plan jsonb,created_xact_id xid8 DEFAULT pg_current_xact_id(),PRIMARY KEY(character_id,cause_id,cause_ordinal,transition_key));
\ir ../../../migrations/2501_personal_quest_map_markers.sql
INSERT INTO game_character_roots VALUES('00000000-0000-0000-0000-000000000001');
-- A progress/reward participant cannot commit without its marker.
DO $$ BEGIN
 BEGIN
 INSERT INTO game_quest_reward_sources VALUES('00000000-0000-0000-0000-000000000001','00000000-0000-0000-0000-000000000002',1,'marker','{"map_mark":{"position":{"x":31948,"y":31924,"floor":-7},"icon":"cross","text":"Treasure"}}',pg_current_xact_id());
 SET CONSTRAINTS ALL IMMEDIATE;
 RAISE EXCEPTION 'unpaired reward accepted';
 EXCEPTION WHEN check_violation THEN RAISE NOTICE 'QN6_NEGATIVE missing marker refused'; END;
END $$;
INSERT INTO game_quest_reward_sources VALUES('00000000-0000-0000-0000-000000000001','00000000-0000-0000-0000-000000000002',1,'marker','{"map_mark":{"position":{"x":31948,"y":31924,"floor":-7},"icon":"cross","text":"Treasure"}}',pg_current_xact_id());
INSERT INTO game_character_quest_markers SELECT character_id,cause_id,cause_ordinal,transition_key,plan->'map_mark',pg_current_xact_id() FROM game_quest_reward_sources;
SET CONSTRAINTS ALL IMMEDIATE;
DO $$ BEGIN
 IF (SELECT count(*) FROM game_character_quest_markers)<>1 THEN RAISE EXCEPTION 'missing personal marker'; END IF;
 BEGIN
 INSERT INTO game_character_quest_markers VALUES('00000000-0000-0000-0000-000000000001','00000000-0000-0000-0000-000000000002',2,'forged','{"position":{"x":31949,"y":31924,"floor":-7},"icon":"cross","text":"Treasure"}',pg_current_xact_id());
 SET CONSTRAINTS ALL IMMEDIATE;
 RAISE EXCEPTION 'forged marker accepted';
 EXCEPTION WHEN check_violation OR foreign_key_violation THEN RAISE NOTICE 'QN6_NEGATIVE unbound marker refused'; END;
 BEGIN
 SET CONSTRAINTS ALL DEFERRED;
 INSERT INTO game_quest_reward_sources VALUES('00000000-0000-0000-0000-000000000001','00000000-0000-0000-0000-000000000002',2,'mismatch','{"map_mark":{"position":{"x":31948,"y":31924,"floor":-7},"icon":"cross","text":"Treasure"}}',pg_current_xact_id());
 INSERT INTO game_character_quest_markers VALUES('00000000-0000-0000-0000-000000000001','00000000-0000-0000-0000-000000000002',2,'mismatch','{"position":{"x":31949,"y":31924,"floor":-7},"icon":"cross","text":"Treasure"}',pg_current_xact_id());
 SET CONSTRAINTS ALL IMMEDIATE;
 RAISE EXCEPTION 'mismatched marker accepted';
 EXCEPTION WHEN check_violation THEN RAISE NOTICE 'QN6_NEGATIVE substituted marker refused'; END;
 BEGIN
 SET CONSTRAINTS ALL DEFERRED;
 INSERT INTO game_quest_reward_sources SELECT character_id,cause_id,3,'old_source',plan,'1'::xid8 FROM game_quest_reward_sources WHERE transition_key='marker';
 INSERT INTO game_character_quest_markers SELECT character_id,cause_id,3,'old_source',plan->'map_mark',pg_current_xact_id() FROM game_quest_reward_sources WHERE transition_key='old_source';
 SET CONSTRAINTS ALL IMMEDIATE;
 RAISE EXCEPTION 'old source provenance accepted';
 EXCEPTION WHEN check_violation THEN RAISE NOTICE 'QN6_NEGATIVE old source transaction refused'; END;
 BEGIN
 SET CONSTRAINTS ALL DEFERRED;
 INSERT INTO game_quest_reward_sources SELECT character_id,cause_id,4,'old_marker',plan,pg_current_xact_id() FROM game_quest_reward_sources WHERE transition_key='marker';
 INSERT INTO game_character_quest_markers SELECT character_id,cause_id,4,'old_marker',plan->'map_mark','1'::xid8 FROM game_quest_reward_sources WHERE transition_key='old_marker';
 SET CONSTRAINTS ALL IMMEDIATE;
 RAISE EXCEPTION 'old marker provenance accepted';
 EXCEPTION WHEN check_violation THEN RAISE NOTICE 'QN6_NEGATIVE old marker transaction refused'; END;
 BEGIN
 INSERT INTO game_character_quest_markers SELECT * FROM game_character_quest_markers;
 RAISE EXCEPTION 'duplicate accepted';
 EXCEPTION WHEN unique_violation THEN RAISE NOTICE 'QN6_NEGATIVE repeated marker refused'; END;
 BEGIN
 INSERT INTO game_character_quest_markers VALUES('00000000-0000-0000-0000-000000000001','00000000-0000-0000-0000-000000000002',3,'null','{"position":{"x":null,"y":31924,"floor":-7},"icon":"cross","text":"Treasure"}',pg_current_xact_id());
 RAISE EXCEPTION 'null coordinate accepted';
 EXCEPTION WHEN check_violation THEN RAISE NOTICE 'QN6_NEGATIVE null coordinate refused'; END;
 RAISE NOTICE 'QN6_GREEN personal marker and progress participant are atomic';
END $$;
ROLLBACK;
