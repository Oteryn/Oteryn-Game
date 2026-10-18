-- GM-1 / CP D1437: the minimal moderation writers this PR owns (WRITE-MOD-1 is
-- not on main). Sanctions are staff records keyed to their audited staff
-- action, never a Character aggregate write; a text report follows WRITE0 §6
-- (states, dedup, WRITE0-RL-03..06 and the counter lock order). Every row that
-- a staff action writes references that action's audited operation row, which
-- the staff writer inserts later in the same transaction, so those foreign
-- keys are deferred to commit.

CREATE TABLE game_character_sanctions (
 sanction_id uuid PRIMARY KEY CHECK(game_character_is_uuid_v7(sanction_id)),
 environment text NOT NULL CHECK(length(environment) BETWEEN 1 AND 128),
 character_id uuid NOT NULL REFERENCES game_character_roots(character_id) ON DELETE RESTRICT,
 kind text NOT NULL CHECK(kind='MUTE'),
 starts_at bigint NOT NULL CHECK(starts_at>0),
 ends_at bigint NOT NULL,
 actor_principal text NOT NULL CHECK(length(actor_principal) BETWEEN 1 AND 128),
 mute_action_id uuid NOT NULL UNIQUE
  REFERENCES game_staff_operations(action_id) DEFERRABLE INITIALLY DEFERRED,
 status text NOT NULL CHECK(status IN ('ACTIVE','LIFTED')),
 revision smallint NOT NULL CHECK(revision IN (1,2)),
 lifted_at bigint,
 lift_action_id uuid UNIQUE
  REFERENCES game_staff_operations(action_id) DEFERRABLE INITIALLY DEFERRED,
 CHECK(ends_at>starts_at AND ends_at-starts_at<=31536000000),
 CHECK((status='ACTIVE' AND revision=1 AND lifted_at IS NULL AND lift_action_id IS NULL)
  OR (status='LIFTED' AND revision=2 AND lifted_at>=starts_at AND lift_action_id IS NOT NULL)));
CREATE INDEX game_character_sanctions_active ON game_character_sanctions(character_id,ends_at)
 WHERE status='ACTIVE';
CREATE FUNCTION game_character_sanction_lift_only() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF TG_OP='UPDATE' AND OLD.status='ACTIVE' AND NEW.status='LIFTED' AND NEW.revision=OLD.revision+1
  AND NEW.sanction_id=OLD.sanction_id AND NEW.environment=OLD.environment
  AND NEW.character_id=OLD.character_id AND NEW.kind=OLD.kind AND NEW.starts_at=OLD.starts_at
  AND NEW.ends_at=OLD.ends_at AND NEW.actor_principal=OLD.actor_principal
  AND NEW.mute_action_id=OLD.mute_action_id THEN
  RETURN NEW;
 END IF;
 RAISE EXCEPTION 'immutable staff evidence' USING ERRCODE='23514';
END $$;
CREATE TRIGGER game_character_sanction_lift_only BEFORE UPDATE OR DELETE ON game_character_sanctions
 FOR EACH ROW EXECUTE FUNCTION game_character_sanction_lift_only();
CREATE TRIGGER game_character_sanction_no_truncate BEFORE TRUNCATE ON game_character_sanctions
 FOR EACH STATEMENT EXECUTE FUNCTION game_staff_immutable();

-- WRITE0 §6: the restricted moderation store of item-text reports.
CREATE TABLE game_item_text_reports (
 report_id uuid PRIMARY KEY CHECK(game_character_is_uuid_v7(report_id)),
 world_id uuid NOT NULL,
 reporter_character_id uuid NOT NULL REFERENCES game_character_roots(character_id) ON DELETE RESTRICT,
 reporter_account_id uuid NOT NULL,
 item_instance_id uuid NOT NULL REFERENCES game_item_instances(item_instance_id) ON DELETE RESTRICT,
 text_revision numeric(20,0) NOT NULL CHECK(text_revision BETWEEN 1 AND 18446744073709551615),
 snapshot_text text CHECK(octet_length(snapshot_text) BETWEEN 1 AND 7996),
 writer_character_id uuid,
 written_at timestamptz NOT NULL,
 created_at bigint NOT NULL CHECK(created_at>0),
 expires_at bigint NOT NULL,
 state text NOT NULL CHECK(state IN ('OPEN','CLOSED','EXPIRED')),
 revision smallint NOT NULL CHECK(revision IN (1,2)),
 ended_at bigint,
 closed_action_id uuid UNIQUE
  REFERENCES game_staff_operations(action_id) DEFERRABLE INITIALLY DEFERRED,
 cleared_action_id uuid UNIQUE
  REFERENCES game_staff_operations(action_id) DEFERRABLE INITIALLY DEFERRED,
 CHECK(expires_at=created_at+15552000000),
 CHECK((state='OPEN' AND revision=1 AND ended_at IS NULL AND closed_action_id IS NULL AND snapshot_text IS NOT NULL)
  OR (state='CLOSED' AND revision=2 AND ended_at>=created_at AND closed_action_id IS NOT NULL)
  OR (state='EXPIRED' AND revision=2 AND ended_at>=expires_at AND closed_action_id IS NULL AND snapshot_text IS NULL)));
CREATE UNIQUE INDEX game_item_text_reports_open_dedup
 ON game_item_text_reports(reporter_character_id,item_instance_id,text_revision) WHERE state='OPEN';
CREATE INDEX game_item_text_reports_list ON game_item_text_reports(state,created_at,report_id);
CREATE INDEX game_item_text_reports_snapshot_expiry ON game_item_text_reports(expires_at)
 WHERE snapshot_text IS NOT NULL;
CREATE FUNCTION game_item_text_report_transition() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF TG_OP<>'UPDATE' OR NEW.report_id<>OLD.report_id OR NEW.world_id<>OLD.world_id
  OR NEW.reporter_character_id<>OLD.reporter_character_id OR NEW.reporter_account_id<>OLD.reporter_account_id
  OR NEW.item_instance_id<>OLD.item_instance_id OR NEW.text_revision<>OLD.text_revision
  OR NEW.writer_character_id IS DISTINCT FROM OLD.writer_character_id OR NEW.written_at<>OLD.written_at
  OR NEW.created_at<>OLD.created_at OR NEW.expires_at<>OLD.expires_at
  OR (NEW.snapshot_text IS NOT NULL AND NEW.snapshot_text IS DISTINCT FROM OLD.snapshot_text)
  OR (OLD.cleared_action_id IS NOT NULL AND NEW.cleared_action_id IS DISTINCT FROM OLD.cleared_action_id)
  OR (NEW.state<>OLD.state AND (OLD.state<>'OPEN' OR NEW.revision<>OLD.revision+1))
  OR (NEW.state=OLD.state AND (NEW.revision<>OLD.revision OR NEW.ended_at IS DISTINCT FROM OLD.ended_at
   OR NEW.closed_action_id IS DISTINCT FROM OLD.closed_action_id)) THEN
  RAISE EXCEPTION 'immutable staff evidence' USING ERRCODE='23514';
 END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER game_item_text_report_transition BEFORE UPDATE OR DELETE ON game_item_text_reports
 FOR EACH ROW EXECUTE FUNCTION game_item_text_report_transition();
CREATE TRIGGER game_item_text_report_no_truncate BEFORE TRUNCATE ON game_item_text_reports
 FOR EACH STATEMENT EXECUTE FUNCTION game_staff_immutable();

-- WRITE0-RL-04..06 counters, upserted at zero then locked in this order.
CREATE TABLE game_item_text_report_hourly (
 reporter_character_id uuid NOT NULL REFERENCES game_character_roots(character_id) ON DELETE RESTRICT,
 hour bigint NOT NULL CHECK(hour>=0),
 reports smallint NOT NULL CHECK(reports BETWEEN 0 AND 10),
 PRIMARY KEY(reporter_character_id,hour));
CREATE TABLE game_item_text_report_accounts (
 account_id uuid PRIMARY KEY,
 open_reports smallint NOT NULL CHECK(open_reports BETWEEN 0 AND 20));
CREATE TABLE game_item_text_report_worlds (
 world_id uuid PRIMARY KEY,
 open_snapshots integer NOT NULL CHECK(open_snapshots BETWEEN 0 AND 100000));

REVOKE ALL ON game_character_sanctions,game_item_text_reports,game_item_text_report_hourly,
 game_item_text_report_accounts,game_item_text_report_worlds FROM PUBLIC;
GRANT SELECT,INSERT,UPDATE(status,revision,lifted_at,lift_action_id) ON game_character_sanctions
 TO oteryn_game_runtime,oteryn_game_control;
GRANT SELECT,INSERT,UPDATE(snapshot_text,state,revision,ended_at,closed_action_id,cleared_action_id)
 ON game_item_text_reports TO oteryn_game_runtime;
GRANT SELECT,INSERT,UPDATE(reports),DELETE ON game_item_text_report_hourly TO oteryn_game_runtime;
GRANT SELECT,INSERT,UPDATE(open_reports) ON game_item_text_report_accounts TO oteryn_game_runtime;
GRANT SELECT,INSERT,UPDATE(open_snapshots) ON game_item_text_report_worlds TO oteryn_game_runtime;
REVOKE ALL ON FUNCTION game_character_sanction_lift_only(),game_item_text_report_transition() FROM PUBLIC;
