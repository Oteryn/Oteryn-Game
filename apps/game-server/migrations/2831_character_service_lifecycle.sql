CREATE FUNCTION game_character_service_now() RETURNS BIGINT LANGUAGE sql VOLATILE
 SET search_path=pg_catalog AS $$ SELECT CASE WHEN abs(extract(epoch FROM clock_timestamp()-statement_timestamp()))<=1
 THEN floor(extract(epoch FROM clock_timestamp()))::bigint END $$;
REVOKE ALL ON FUNCTION game_character_service_now() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_character_service_now() TO oteryn_game_runtime,oteryn_game_control;
CREATE TABLE game_character_service_deletions (
 character_id UUID PRIMARY KEY REFERENCES game_character_roots,
 account_id UUID NOT NULL, operation_id UUID NOT NULL REFERENCES game_character_service_receipts DEFERRABLE INITIALLY DEFERRED,
 scheduled_at BIGINT NOT NULL CHECK(scheduled_at>0), effective_at BIGINT NOT NULL CHECK(effective_at>scheduled_at),
 finalized_at BIGINT CHECK(finalized_at>=effective_at)
);
CREATE INDEX game_character_service_deletions_due ON game_character_service_deletions(effective_at,character_id) WHERE finalized_at IS NULL;
CREATE INDEX game_character_service_deletions_operation ON game_character_service_deletions(operation_id);
CREATE INDEX game_character_service_deletions_account ON game_character_service_deletions(account_id);
CREATE TABLE game_character_service_changes (
 character_id UUID NOT NULL REFERENCES game_character_roots,
 old_revision NUMERIC(20,0) NOT NULL, new_revision NUMERIC(20,0) NOT NULL CHECK(new_revision=old_revision+1),
 operation_id UUID REFERENCES game_character_service_receipts DEFERRABLE INITIALLY DEFERRED,
 purpose TEXT NOT NULL CHECK(purpose IN('RENAME','SCHEDULE','CANCEL','FINALIZE')),
 old_name TEXT NOT NULL,new_name TEXT NOT NULL,old_lifecycle SMALLINT NOT NULL,new_lifecycle SMALLINT NOT NULL,
 created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id(), occurred_at BIGINT NOT NULL CHECK(occurred_at>0),
 PRIMARY KEY(character_id,new_revision),
 CHECK((purpose='FINALIZE')=(operation_id IS NULL)),
 CHECK((purpose='RENAME' AND old_lifecycle=1 AND new_lifecycle=1 AND game_character_name_key(old_name)<>game_character_name_key(new_name))
 OR (purpose='SCHEDULE' AND old_lifecycle=1 AND new_lifecycle=3 AND old_name=new_name)
 OR (purpose='CANCEL' AND old_lifecycle=3 AND new_lifecycle=1 AND old_name=new_name)
 OR (purpose='FINALIZE' AND old_lifecycle=3 AND new_lifecycle=4 AND old_name=new_name))
);
CREATE INDEX game_character_service_changes_operation ON game_character_service_changes(operation_id);
ALTER TABLE game_character_service_changes
 ADD COLUMN level_before BIGINT NOT NULL,
 ADD COLUMN level_after BIGINT NOT NULL CHECK(level_after=level_before),
 ADD COLUMN experience_before BIGINT NOT NULL,
 ADD COLUMN experience_after BIGINT NOT NULL CHECK(experience_after=experience_before),
 ADD COLUMN profile_revision TEXT NOT NULL,
 ADD COLUMN ruleset_revision TEXT NOT NULL,
 ADD COLUMN content_revision TEXT NOT NULL,
 ADD COLUMN simulation_revision TEXT NOT NULL,
 ADD COLUMN evidence_revision TEXT NOT NULL,
 ADD COLUMN declaration_revision TEXT NOT NULL,
 ADD COLUMN policy_revision TEXT NOT NULL,
 ADD COLUMN reward_revision TEXT NOT NULL,
 ADD CONSTRAINT game_character_service_transition_revision UNIQUE(character_id,new_revision);
CREATE FUNCTION game_character_service_transition_capture() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE s game_character_progression_state;
BEGIN
 SELECT * INTO STRICT s FROM game_character_progression_state WHERE character_id=NEW.character_id FOR UPDATE;
 IF s.character_revision<>NEW.old_revision THEN RAISE EXCEPTION 'Service progression revision mismatch' USING ERRCODE='23514';END IF;
 NEW.level_before:=s.level;NEW.level_after:=s.level;NEW.experience_before:=s.total_experience;NEW.experience_after:=s.total_experience;
 NEW.profile_revision:=s.profile_revision;NEW.ruleset_revision:=s.ruleset_revision;NEW.content_revision:=s.content_revision;
 NEW.simulation_revision:=s.simulation_revision;NEW.evidence_revision:=s.evidence_revision;
 NEW.declaration_revision:=s.declaration_revision;NEW.policy_revision:=s.policy_revision;NEW.reward_revision:=s.reward_revision;
 NEW.created_xact_id:=pg_current_xact_id();

 RETURN NEW;
END $$;
CREATE TRIGGER game_character_service_transition_capture BEFORE INSERT ON game_character_service_changes FOR EACH ROW EXECUTE FUNCTION game_character_service_transition_capture();
CREATE FUNCTION game_character_service_transition_progression() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN
 UPDATE game_character_progression_state SET character_revision=NEW.new_revision WHERE character_id=NEW.character_id AND character_revision=NEW.old_revision;
 IF NOT FOUND THEN RAISE EXCEPTION 'Service progression successor missing' USING ERRCODE='23514';END IF;RETURN NULL;END $$;
CREATE TRIGGER game_character_service_transition_progression AFTER INSERT ON game_character_service_changes FOR EACH ROW EXECUTE FUNCTION game_character_service_transition_progression();
CREATE VIEW game_character_service_revision_chain WITH(security_invoker=true) AS
 SELECT character_id,old_revision AS original_character_revision,new_revision AS committed_character_revision,
 level_before,level_after,experience_before,experience_after,profile_revision,ruleset_revision,content_revision,
 simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision FROM game_character_service_changes;

CREATE OR REPLACE VIEW game_bazaar_xp_revision_chain WITH(security_invoker=true) AS
 SELECT character_id,original_character_revision,committed_character_revision,level_before,level_after,experience_before,experience_after,
 profile_revision,ruleset_revision,content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision
 FROM game_character_xp_receipts UNION ALL SELECT * FROM game_bazaar_character_revision_chain UNION ALL SELECT * FROM game_character_service_revision_chain;
ALTER TABLE game_character_roots DROP CONSTRAINT game_character_roots_lifecycle_check;
ALTER TABLE game_character_roots ADD CONSTRAINT game_character_roots_lifecycle_check CHECK(lifecycle IN(1,2,3,4));
DO $$ DECLARE body TEXT;BEGIN
 SELECT pg_get_functiondef('game_character_root_revision_guard()'::regprocedure) INTO STRICT body;
 body:=replace(body,E'BEGIN\n',E'BEGIN\n IF TG_OP=\'UPDATE\' AND (to_jsonb(NEW)-ARRAY[\'name\',\'name_key\',\'lifecycle\',\'character_revision\'])=(to_jsonb(OLD)-ARRAY[\'name\',\'name_key\',\'lifecycle\',\'character_revision\']) AND EXISTS(SELECT 1 FROM game_character_service_changes t WHERE t.character_id=OLD.character_id AND t.old_revision=OLD.character_revision AND t.new_revision=NEW.character_revision AND t.old_name=OLD.name AND t.new_name=NEW.name AND t.old_lifecycle=OLD.lifecycle AND t.new_lifecycle=NEW.lifecycle AND t.created_xact_id=pg_current_xact_id()) THEN RETURN NEW;END IF;\n IF OLD.lifecycle IN(3,4) THEN RAISE EXCEPTION \'character service lifecycle forbids mutation\' USING ERRCODE=\'23514\';END IF;\n');
 EXECUTE body;
 SELECT pg_get_functiondef('game_character_progression_state_guard()'::regprocedure) INTO STRICT body;
 body:=replace(body,E'BEGIN\n',E'BEGIN\n IF TG_OP=\'UPDATE\' AND (to_jsonb(NEW)-\'character_revision\')=(to_jsonb(OLD)-\'character_revision\') AND EXISTS(SELECT 1 FROM game_character_service_changes t WHERE t.character_id=OLD.character_id AND t.old_revision=OLD.character_revision AND t.new_revision=NEW.character_revision AND t.created_xact_id=pg_current_xact_id()) THEN RETURN NEW;END IF;\n');
 EXECUTE body;
END $$;
CREATE FUNCTION game_character_service_change_proven() RETURNS trigger LANGUAGE plpgsql SET search_path=pg_catalog,public AS $$ BEGIN
 IF NEW.operation_id IS NULL THEN
  IF NOT EXISTS(SELECT 1 FROM game_character_service_deletions d WHERE d.character_id=NEW.character_id AND d.finalized_at=NEW.occurred_at AND d.effective_at<=NEW.occurred_at)
  THEN RAISE EXCEPTION 'service finalization requires its durable deadline';END IF;
 ELSIF NOT EXISTS(SELECT 1 FROM game_character_service_receipts d WHERE d.operation_id=NEW.operation_id AND d.result_code='CHAR_CMD_OK' AND d.created_xact_id=pg_current_xact_id()
  AND (convert_from(d.intent,'UTF8')::jsonb->>'character_id')::uuid=NEW.character_id
  AND (convert_from(d.intent,'UTF8')::jsonb->>'expected_character_revision')::numeric=NEW.old_revision
  AND d.command=CASE NEW.purpose WHEN 'RENAME' THEN 'RenameCharacter' WHEN 'SCHEDULE' THEN 'ScheduleCharacterDeletion' WHEN 'CANCEL' THEN 'CancelCharacterDeletion' END)
 THEN RAISE EXCEPTION 'service change requires its exact committed receipt';END IF;
 IF NOT EXISTS(SELECT 1 FROM game_character_roots r WHERE r.character_id=NEW.character_id AND r.character_revision=NEW.new_revision AND r.name=NEW.new_name AND r.lifecycle=NEW.new_lifecycle)
 THEN RAISE EXCEPTION 'service root does not match its revision proof';END IF;RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_character_service_change_proven AFTER INSERT ON game_character_service_changes DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_character_service_change_proven();
CREATE TRIGGER game_character_service_change_immutable BEFORE UPDATE OR DELETE ON game_character_service_changes FOR EACH ROW EXECUTE FUNCTION game_character_immutable();
CREATE FUNCTION game_character_service_admission_guard() RETURNS trigger LANGUAGE plpgsql SET search_path=pg_catalog,public AS $$ BEGIN
 PERFORM 1 FROM game_character_account_guards WHERE account_id=NEW.account_id FOR UPDATE;
 IF EXISTS(SELECT 1 FROM game_character_roots WHERE character_id=NEW.character_id AND lifecycle IN(3,4)) THEN RAISE EXCEPTION 'character service lifecycle denies admission';END IF;RETURN NEW;
END $$;
CREATE TRIGGER game_character_service_admission_guard BEFORE INSERT ON game_durability_reconnect_sessions FOR EACH ROW EXECUTE FUNCTION game_character_service_admission_guard();
REVOKE ALL ON game_character_service_deletions,game_character_service_changes,game_character_service_revision_chain FROM PUBLIC;
GRANT SELECT,INSERT,UPDATE,DELETE ON game_character_service_deletions TO oteryn_game_runtime;
GRANT SELECT,INSERT ON game_character_service_changes TO oteryn_game_runtime;
GRANT SELECT ON game_character_service_deletions,game_character_service_changes,game_character_service_revision_chain TO oteryn_game_control;
GRANT SELECT ON game_character_service_revision_chain TO oteryn_game_runtime;
