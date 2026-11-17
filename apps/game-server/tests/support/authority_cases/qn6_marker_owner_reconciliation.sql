\set ON_ERROR_STOP on
\timing on
BEGIN;
CREATE SCHEMA qn6_marker_reconciliation;
SET LOCAL search_path=qn6_marker_reconciliation,public;
SELECT 'QN6_POSTGRES_READY marker_owner_reconciliation',current_setting('server_version');
CREATE TABLE game_character_roots(character_id uuid PRIMARY KEY);
CREATE TABLE game_quest_reward_sources(character_id uuid,cause_id uuid,cause_ordinal numeric,transition_key text,plan jsonb,created_xact_id xid8 DEFAULT pg_current_xact_id(),PRIMARY KEY(character_id,cause_id,cause_ordinal,transition_key));
\ir ../../../migrations/2501_personal_quest_map_markers.sql
-- Model the canonical owner's installed schema objects; its own migration remains unchanged.
CREATE TABLE game_character_quest_map_marks(id integer);
CREATE FUNCTION canonical_marker_guard() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RETURN NULL; END $$;
CREATE CONSTRAINT TRIGGER game_quest_source_map_mark_complete AFTER INSERT ON game_quest_reward_sources
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION canonical_marker_guard();
\ir ../../../migrations/2502_canonical_quest_marker_owner.sql
DO $$ BEGIN
 IF EXISTS(SELECT 1 FROM pg_trigger WHERE tgrelid='game_quest_reward_sources'::regclass AND tgname='game_quest_marker_reward_complete') THEN RAISE EXCEPTION 'parallel marker producer guard still active'; END IF;
 IF NOT EXISTS(SELECT 1 FROM pg_trigger WHERE tgrelid='game_quest_reward_sources'::regclass AND tgname='game_quest_source_map_mark_complete') THEN RAISE EXCEPTION 'canonical marker guard removed'; END IF;
 IF has_table_privilege('oteryn_game_runtime','game_character_quest_markers','INSERT') THEN RAISE EXCEPTION 'legacy marker table still writable'; END IF;
 IF NOT has_table_privilege('oteryn_game_control','game_character_quest_markers','SELECT') THEN RAISE EXCEPTION 'audit history lost'; END IF;
 RAISE NOTICE 'QN6_GREEN canonical marker owner retained, legacy audit preserved';
END $$;
ROLLBACK;
