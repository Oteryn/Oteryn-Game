-- GM-1 / CP D1437: the minimal moderation writers this PR owns (WRITE-MOD-1 is
-- not on main). Sanctions are staff records keyed to their audited staff
-- action, never a Character aggregate write; a text report follows WRITE0 §6
-- (states, dedup, WRITE0-RL-03..06 and the counter lock order). Every row that
-- a staff action writes references that action's audited operation row, which
-- the staff writer inserts later in the same transaction, so those foreign
-- keys are deferred to commit.

-- GM-1 (Codex P1): the signer tier is enforced at the database too, so a
-- runtime or control member LOGIN cannot commit a moderation effect for a
-- genuinely signed request that its signer's role does not allow. The roster
-- the head's activation names is stored here by owner provisioning, with the
-- same identity guard as an ACTIVATION link; no runtime or control member may
-- add, change or remove one. A missing roster refuses every moderation write.
CREATE TABLE game_staff_rosters (
 environment text NOT NULL CHECK(length(environment) BETWEEN 1 AND 128),
 roster_sha256 text NOT NULL CHECK(roster_sha256 ~ '^[0-9a-f]{64}$'),
 roster bytea NOT NULL CHECK(octet_length(roster) BETWEEN 1 AND 65536
  AND encode(sha256(roster),'hex')=roster_sha256),
 PRIMARY KEY(environment,roster_sha256));
CREATE FUNCTION game_staff_roster_owner_only() RETURNS trigger LANGUAGE plpgsql SECURITY INVOKER AS $$
BEGIN
 -- As game_staff_link_extends_head: authenticated and effective identities,
 -- indirect LOGIN members included; superusers keep owner authority.
 IF EXISTS (
   SELECT 1 FROM pg_catalog.pg_roles r
   WHERE r.rolname IN (session_user,current_user) AND NOT r.rolsuper
     AND (pg_catalog.pg_has_role(r.oid,'oteryn_game_runtime','MEMBER')
       OR pg_catalog.pg_has_role(r.oid,'oteryn_game_control','MEMBER'))
 ) THEN
   RAISE EXCEPTION 'runtime roster forbidden' USING ERRCODE='23514';
 END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER game_staff_roster_owner_only BEFORE INSERT ON game_staff_rosters
 FOR EACH ROW EXECUTE FUNCTION game_staff_roster_owner_only();
CREATE TRIGGER game_staff_roster_immutable BEFORE UPDATE OR DELETE ON game_staff_rosters
 FOR EACH ROW EXECUTE FUNCTION game_staff_immutable();
CREATE TRIGGER game_staff_roster_no_truncate BEFORE TRUNCATE ON game_staff_rosters
 FOR EACH STATEMENT EXECUTE FUNCTION game_staff_immutable();
-- Role::allows (staff::signature) for a signed action kind: every role runs
-- the ALPHA moderation actions, clearing report text needs SENIOR_GM, and any
-- other kind needs ADMIN. An unknown role allows nothing.
CREATE FUNCTION game_staff_role_allows(p_role text, p_kind text) RETURNS boolean
 LANGUAGE sql IMMUTABLE AS $$
 SELECT CASE
  WHEN p_role IS NULL OR p_role NOT IN ('GM','SENIOR_GM','ADMIN') OR p_kind IS NULL THEN false
  WHEN p_kind IN ('INSPECT','REPORTS_LIST','KICK','MUTE','UNMUTE','REPORT_CLOSE') THEN true
  WHEN p_kind='REPORT_CLEAR_TEXT' THEN p_role IN ('SENIOR_GM','ADMIN')
  ELSE p_role='ADMIN' END
$$;
-- Whether `p_principal`, signing a request against roster `p_roster`, holds a
-- role allowing `p_kind` under the roster the head's activation names. The
-- request must name that roster, and the principal exactly one role line
-- (`# oteryn-staff-role <principal> <role>`, as VerifierTrust::roles parses).
CREATE FUNCTION game_staff_signer_allows(p_environment text, p_principal text, p_roster text,
 p_kind text) RETURNS boolean LANGUAGE plpgsql STABLE SECURITY INVOKER AS $$
DECLARE
 active text;
 roster bytea;
 line text;
 fields text[];
 signer_role text;
 matched integer:=0;
BEGIN
 SELECT convert_from(l.signed_bytes,'UTF8')::jsonb->'action'->>'roster_sha256' INTO active
  FROM game_staff_control_heads h JOIN game_staff_control_links l
   ON l.environment=h.environment AND l.link_sha256=h.activation_sha256 AND l.link_kind='ACTIVATION'
  WHERE h.environment=p_environment;
 IF active IS NULL OR active IS DISTINCT FROM p_roster THEN
  RETURN false;
 END IF;
 SELECT r.roster INTO roster FROM game_staff_rosters r
  WHERE r.environment=p_environment AND r.roster_sha256=active;
 IF NOT FOUND THEN
  RETURN false;
 END IF;
 FOREACH line IN ARRAY regexp_split_to_array(convert_from(roster,'UTF8'),E'\n') LOOP
  line:=regexp_replace(line,E'^\\s+|\\s+$','','g');
  CONTINUE WHEN left(line,19)<>'# oteryn-staff-role';
  fields:=regexp_split_to_array(regexp_replace(substr(line,20),E'^\\s+','',''),E'\\s+');
  CONTINUE WHEN fields[1] IS DISTINCT FROM p_principal;
  IF array_length(fields,1)<>2 THEN
   RETURN false;
  END IF;
  signer_role:=fields[2];
  matched:=matched+1;
 END LOOP;
 RETURN matched=1 AND game_staff_role_allows(signer_role,p_kind);
END $$;

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
 -- The lift is the unmute's signed execution time on its node's clock, which
 -- may precede the start on another node's clock (as report close does for ended_at).
 CHECK((status='ACTIVE' AND revision=1 AND lifted_at IS NULL AND lift_action_id IS NULL)
  OR (status='LIFTED' AND revision=2 AND lifted_at>0 AND lift_action_id IS NOT NULL)));
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
-- A sanction is the effect of its own signed staff action: a MUTE row binds to
-- a COMMITTED MUTE operation of the same environment, principal, Character,
-- start and duration, and a lift to a COMMITTED UNMUTE of that sanction at the
-- lift time. The operation must be written by the executor in this same
-- transaction, so no row can borrow the action id of another (or an older)
-- operation. Its signer must hold a roster role allowing the action. Deferred:
-- the executor writes the operation row after its effect.
CREATE FUNCTION game_character_sanction_bound() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE
 bound uuid;
 signer text;
 r jsonb;
 o jsonb;
BEGIN
 IF TG_OP='INSERT' AND NEW.status<>'ACTIVE' THEN
  RAISE EXCEPTION 'sanction is not its signed staff action' USING ERRCODE='23514';
 END IF;
 bound:=CASE WHEN TG_OP='INSERT' THEN NEW.mute_action_id ELSE NEW.lift_action_id END;
 SELECT op.principal,convert_from(op.request_bytes,'UTF8')::jsonb,
  convert_from(op.outcome_bytes,'UTF8')::jsonb INTO signer,r,o FROM game_staff_operations op
  WHERE op.action_id=bound AND op.environment=NEW.environment
   AND op.created_xact_id=txid_current();
 IF NOT FOUND OR r->>'environment' IS DISTINCT FROM NEW.environment
  OR o->>'disposition' IS DISTINCT FROM 'COMMITTED' OR o->>'result' IS DISTINCT FROM 'OK'
  OR (TG_OP='INSERT' AND (r->'action'->>'kind' IS DISTINCT FROM 'MUTE'
   OR lower(r->'action'->>'character_id') IS DISTINCT FROM NEW.character_id::text
   OR r->'action'->>'duration_ms' IS DISTINCT FROM (NEW.ends_at-NEW.starts_at)::text
   OR o->>'occurred_at' IS DISTINCT FROM NEW.starts_at::text
   OR signer IS DISTINCT FROM NEW.actor_principal))
  OR (TG_OP='UPDATE' AND (r->'action'->>'kind' IS DISTINCT FROM 'UNMUTE'
   OR lower(r->'action'->>'sanction_id') IS DISTINCT FROM NEW.sanction_id::text
   OR o->>'occurred_at' IS DISTINCT FROM NEW.lifted_at::text))
  OR NOT game_staff_signer_allows(NEW.environment,signer,r->>'roster_sha256',r->'action'->>'kind') THEN
  RAISE EXCEPTION 'sanction is not its signed staff action' USING ERRCODE='23514';
 END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_character_sanction_bound AFTER INSERT OR UPDATE ON game_character_sanctions
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_character_sanction_bound();

-- WRITE0 §6: the restricted moderation store of item-text reports. A report's
-- `entry_seq` is drawn when it enters its state (insert, close or expiry) under
-- its World's counter row lock, which a staff list takes FOR SHARE first: a
-- list keyset on (entry_seq, report_id) therefore never passes a report that
-- enters the listed state later, nor one that entered it uncommitted.
CREATE SEQUENCE game_item_text_report_entry_seq AS bigint;
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
 entry_seq bigint NOT NULL DEFAULT nextval('game_item_text_report_entry_seq'),
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
CREATE INDEX game_item_text_reports_list ON game_item_text_reports(world_id,state,entry_seq,report_id);
CREATE INDEX game_item_text_reports_snapshot_expiry ON game_item_text_reports(expires_at)
 WHERE snapshot_text IS NOT NULL;
-- Sorts before `..._transition`: a new state draws a new entry, under the
-- World row lock; any other update keeps the entry it had.
CREATE FUNCTION game_item_text_report_entry() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF TG_OP='INSERT' OR NEW.state IS DISTINCT FROM OLD.state THEN
  INSERT INTO game_item_text_report_worlds(world_id,open_snapshots) VALUES(NEW.world_id,0)
   ON CONFLICT DO NOTHING;
  PERFORM 1 FROM game_item_text_report_worlds WHERE world_id=NEW.world_id FOR UPDATE;
  NEW.entry_seq:=nextval('game_item_text_report_entry_seq');
 ELSE
  NEW.entry_seq:=OLD.entry_seq;
 END IF;
 RETURN NEW;
END $$;
CREATE FUNCTION game_item_text_report_transition() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF TG_OP<>'UPDATE' OR NEW.report_id<>OLD.report_id OR NEW.world_id<>OLD.world_id
  OR NEW.reporter_character_id<>OLD.reporter_character_id OR NEW.reporter_account_id<>OLD.reporter_account_id
  OR NEW.item_instance_id<>OLD.item_instance_id OR NEW.text_revision<>OLD.text_revision
  OR NEW.writer_character_id IS DISTINCT FROM OLD.writer_character_id OR NEW.written_at<>OLD.written_at
  OR NEW.created_at<>OLD.created_at OR NEW.expires_at<>OLD.expires_at
  OR (NEW.snapshot_text IS NOT NULL AND NEW.snapshot_text IS DISTINCT FROM OLD.snapshot_text)
  -- WRITE0-RL-03: no snapshot is removed before its retention ends on the
  -- database clock, whatever time the writer passes.
  OR (OLD.snapshot_text IS NOT NULL AND NEW.snapshot_text IS NULL
   AND OLD.expires_at>floor(extract(epoch FROM clock_timestamp())*1000)::bigint)
  OR (OLD.cleared_action_id IS NOT NULL AND NEW.cleared_action_id IS DISTINCT FROM OLD.cleared_action_id)
  OR (NEW.state<>OLD.state AND (OLD.state<>'OPEN' OR NEW.revision<>OLD.revision+1))
  OR (NEW.state=OLD.state AND (NEW.revision<>OLD.revision OR NEW.ended_at IS DISTINCT FROM OLD.ended_at
   OR NEW.closed_action_id IS DISTINCT FROM OLD.closed_action_id)) THEN
  RAISE EXCEPTION 'immutable staff evidence' USING ERRCODE='23514';
 END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER game_item_text_report_entry BEFORE INSERT OR UPDATE ON game_item_text_reports
 FOR EACH ROW EXECUTE FUNCTION game_item_text_report_entry();
CREATE TRIGGER game_item_text_report_transition BEFORE UPDATE OR DELETE ON game_item_text_reports
 FOR EACH ROW EXECUTE FUNCTION game_item_text_report_transition();
-- A report is opened by the reporter and ended or cleared only as the effect
-- of its own signed staff action: a close binds to a COMMITTED REPORT_CLOSE of
-- this report and World at its end time, a clear to a COMMITTED
-- REPORT_CLEAR_TEXT of this report, World and revision, each written by the
-- executor in this same transaction, by a signer whose roster role allows it.
-- Deferred like the sanction binding.
CREATE FUNCTION game_item_text_report_bound() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE
 bound uuid;
 op_environment text;
 signer text;
 r jsonb;
 o jsonb;
 kind text;
BEGIN
 IF TG_OP='INSERT' THEN
  IF NEW.state<>'OPEN' OR NEW.closed_action_id IS NOT NULL OR NEW.cleared_action_id IS NOT NULL THEN
   RAISE EXCEPTION 'report transition is not its signed staff action' USING ERRCODE='23514';
  END IF;
  RETURN NULL;
 END IF;
 FOR kind,bound IN SELECT * FROM (VALUES
   ('REPORT_CLOSE',CASE WHEN OLD.closed_action_id IS NULL THEN NEW.closed_action_id END),
   ('REPORT_CLEAR_TEXT',CASE WHEN OLD.cleared_action_id IS NULL THEN NEW.cleared_action_id END)) v
  WHERE v.column2 IS NOT NULL LOOP
  SELECT op.environment,op.principal,convert_from(op.request_bytes,'UTF8')::jsonb,
   convert_from(op.outcome_bytes,'UTF8')::jsonb INTO op_environment,signer,r,o FROM game_staff_operations op
   WHERE op.action_id=bound AND op.created_xact_id=txid_current();
  IF NOT FOUND OR r->>'environment' IS DISTINCT FROM op_environment
   OR o->>'disposition' IS DISTINCT FROM 'COMMITTED' OR o->>'result' IS DISTINCT FROM 'OK'
   OR r->'action'->>'kind' IS DISTINCT FROM kind
   OR lower(r->'action'->>'report_id') IS DISTINCT FROM NEW.report_id::text
   OR lower(r->'scope'->>'world_id') IS DISTINCT FROM NEW.world_id::text
   OR (kind='REPORT_CLOSE' AND (NEW.state IS DISTINCT FROM 'CLOSED'
    OR NEW.ended_at IS DISTINCT FROM GREATEST((o->>'occurred_at')::bigint,NEW.created_at)))
   OR (kind='REPORT_CLEAR_TEXT'
    AND r->'action'->>'text_revision' IS DISTINCT FROM NEW.text_revision::text)
   OR NOT game_staff_signer_allows(op_environment,signer,r->>'roster_sha256',kind) THEN
   RAISE EXCEPTION 'report transition is not its signed staff action' USING ERRCODE='23514';
  END IF;
 END LOOP;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_item_text_report_bound AFTER INSERT OR UPDATE ON game_item_text_reports
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_item_text_report_bound();
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

REVOKE ALL ON game_staff_rosters,game_character_sanctions,game_item_text_reports,
 game_item_text_report_hourly,game_item_text_report_accounts,game_item_text_report_worlds FROM PUBLIC;
-- The bindings read the roster as the writer; nothing but the owner inserts one.
GRANT SELECT ON game_staff_rosters TO oteryn_game_runtime,oteryn_game_control;
-- Only the node's staff executor (the runtime role) writes sanctions.
GRANT SELECT ON game_character_sanctions TO oteryn_game_runtime,oteryn_game_control;
GRANT INSERT,UPDATE(status,revision,lifted_at,lift_action_id) ON game_character_sanctions
 TO oteryn_game_runtime;
GRANT SELECT,INSERT,UPDATE(snapshot_text,state,revision,ended_at,closed_action_id,cleared_action_id)
 ON game_item_text_reports TO oteryn_game_runtime;
GRANT USAGE ON SEQUENCE game_item_text_report_entry_seq TO oteryn_game_runtime;
GRANT SELECT,INSERT,UPDATE(reports),DELETE ON game_item_text_report_hourly TO oteryn_game_runtime;
GRANT SELECT,INSERT,UPDATE(open_reports) ON game_item_text_report_accounts TO oteryn_game_runtime;
GRANT SELECT,INSERT,UPDATE(open_snapshots) ON game_item_text_report_worlds TO oteryn_game_runtime;
REVOKE ALL ON SEQUENCE game_item_text_report_entry_seq FROM PUBLIC;
REVOKE ALL ON FUNCTION game_character_sanction_lift_only(),game_character_sanction_bound(),
 game_item_text_report_entry(),game_item_text_report_transition(),game_item_text_report_bound()
 FROM PUBLIC;
REVOKE ALL ON FUNCTION game_staff_roster_owner_only(),game_staff_role_allows(text,text),
 game_staff_signer_allows(text,text,text,text) FROM PUBLIC;
-- Called from the deferred bindings as the writer.
GRANT EXECUTE ON FUNCTION game_staff_role_allows(text,text),
 game_staff_signer_allows(text,text,text,text) TO oteryn_game_runtime;
