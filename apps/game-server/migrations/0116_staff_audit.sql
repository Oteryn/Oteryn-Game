-- D1343: staff audit participant schema; STAFF_ACTION event type 5.
-- Audit expiry is exactly 365 elapsed days. Owner provisioning supplies the
-- independently reviewed initial activation; this migration seeds no trust.

CREATE TABLE game_staff_control_links (
 environment text NOT NULL CHECK(length(environment) BETWEEN 1 AND 128),
 control_seq numeric(20,0) NOT NULL CHECK(control_seq BETWEEN 1 AND 18446744073709551615),
 link_sha256 text NOT NULL CHECK(link_sha256 ~ '^[0-9a-f]{64}$'),
 previous_link_sha256 text NOT NULL CHECK(previous_link_sha256 ~ '^[0-9a-f]{64}$'),
 roster_sha256 text NOT NULL CHECK(roster_sha256 ~ '^[0-9a-f]{64}$'),
 link_kind text NOT NULL CHECK(link_kind IN ('REQUEST','RECEIPT','ACTIVATION')),
 signed_bytes bytea NOT NULL CHECK(octet_length(signed_bytes) BETWEEN 1 AND 32768 AND encode(sha256(signed_bytes),'hex')=link_sha256),
 principal text NOT NULL CHECK(length(principal) BETWEEN 1 AND 128),
 key_fingerprint text NOT NULL CHECK(length(key_fingerprint) BETWEEN 1 AND 128),
 signature text NOT NULL CHECK(octet_length(signature) BETWEEN 1 AND 8192),
 created_xact_id bigint NOT NULL CHECK(created_xact_id=txid_current()),
 PRIMARY KEY(environment,control_seq), UNIQUE(environment,link_sha256), UNIQUE(environment,control_seq,link_sha256));
CREATE TABLE game_staff_control_heads (
 environment text PRIMARY KEY, control_seq numeric(20,0) NOT NULL,
 link_sha256 text NOT NULL, activation_sha256 text NOT NULL CHECK(activation_sha256 ~ '^[0-9a-f]{64}$'),
 FOREIGN KEY(environment,control_seq,link_sha256) REFERENCES game_staff_control_links(environment,control_seq,link_sha256));
CREATE TABLE game_staff_operations (
 environment text NOT NULL, operation_id uuid NOT NULL CHECK(game_character_is_uuid_v7(operation_id)),
 request_sha256 text NOT NULL, request_bytes bytea NOT NULL CHECK(octet_length(request_bytes) BETWEEN 1 AND 32768 AND encode(sha256(request_bytes),'hex')=request_sha256),
 outcome_bytes bytea NOT NULL CHECK(octet_length(outcome_bytes) BETWEEN 1 AND 4096),
 outcome_sha256 text NOT NULL CHECK(encode(sha256(outcome_bytes),'hex')=outcome_sha256),
 principal text NOT NULL, key_fingerprint text NOT NULL,
 action_id uuid NOT NULL UNIQUE CHECK(game_character_is_uuid_v7(action_id)), transaction_id uuid NOT NULL UNIQUE CHECK(game_character_is_uuid_v7(transaction_id)),
 control_seq numeric(20,0) NOT NULL, requires_receipt boolean NOT NULL,
 occurred_at bigint NOT NULL CHECK(occurred_at>0),
 created_xact_id bigint NOT NULL CHECK(created_xact_id=txid_current()),
 PRIMARY KEY(environment,operation_id), UNIQUE(environment,request_sha256),
 FOREIGN KEY(environment,control_seq) REFERENCES game_staff_control_links(environment,control_seq));
CREATE TABLE game_staff_action_outbox (
 environment text NOT NULL, operation_id uuid NOT NULL,
 event_id uuid PRIMARY KEY CHECK(game_character_is_uuid_v7(event_id)), transaction_id uuid NOT NULL UNIQUE CHECK(game_character_is_uuid_v7(transaction_id)),
 event_type_id bigint NOT NULL CHECK(event_type_id>0), schema_revision bigint NOT NULL CHECK(schema_revision>0),
 retention_profile_id text NOT NULL CHECK(retention_profile_id='STAFF_ACTION_AUDIT_RETENTION_V1'),
 occurred_at bigint NOT NULL CHECK(occurred_at>0), expires_at bigint NOT NULL CHECK(expires_at=occurred_at+31536000000),
 envelope bytea NOT NULL CHECK(octet_length(envelope) BETWEEN 1 AND 50176),
 envelope_sha256 text NOT NULL CHECK(encode(sha256(envelope),'hex')=envelope_sha256),
 publication_state smallint NOT NULL CHECK(publication_state IN (1,2)),
 created_xact_id bigint NOT NULL,
 FOREIGN KEY(environment,operation_id) REFERENCES game_staff_operations(environment,operation_id),
 FOREIGN KEY(event_id) REFERENCES game_staff_operations(action_id),
 FOREIGN KEY(transaction_id) REFERENCES game_staff_operations(transaction_id));
CREATE TABLE game_staff_action_receipts (
 environment text NOT NULL, operation_id uuid NOT NULL,
 receipt_bytes bytea NOT NULL CHECK(octet_length(receipt_bytes) BETWEEN 1 AND 32768),
 principal text NOT NULL, key_fingerprint text NOT NULL,
 signature text NOT NULL CHECK(octet_length(signature) BETWEEN 1 AND 8192),
 control_seq numeric(20,0) NOT NULL,
 created_xact_id bigint NOT NULL CHECK(created_xact_id=txid_current()),
 PRIMARY KEY(environment,operation_id),
 FOREIGN KEY(environment,operation_id) REFERENCES game_staff_operations(environment,operation_id),
 FOREIGN KEY(environment,control_seq) REFERENCES game_staff_control_links(environment,control_seq));
CREATE FUNCTION game_staff_outbox_insert_stamp() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NEW.created_xact_id<>txid_current() THEN RAISE EXCEPTION 'staff insert transaction mismatch' USING ERRCODE='23514'; END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER game_staff_outbox_insert_stamp BEFORE INSERT ON game_staff_action_outbox
 FOR EACH ROW EXECUTE FUNCTION game_staff_outbox_insert_stamp();
CREATE FUNCTION game_staff_immutable() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN RAISE EXCEPTION 'immutable staff evidence' USING ERRCODE='23514'; END $$;
CREATE FUNCTION game_staff_operation_has_audit() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NOT EXISTS(SELECT 1 FROM game_staff_action_outbox a WHERE a.environment=NEW.environment AND a.operation_id=NEW.operation_id
 AND a.event_id=NEW.action_id AND a.transaction_id=NEW.transaction_id AND a.created_xact_id=NEW.created_xact_id)
 THEN RAISE EXCEPTION 'missing same-transaction staff audit' USING ERRCODE='23514'; END IF;
 IF NOT EXISTS(SELECT 1 FROM game_staff_control_links l WHERE l.environment=NEW.environment AND l.control_seq=NEW.control_seq
 AND l.link_kind='REQUEST' AND l.signed_bytes=NEW.request_bytes AND l.link_sha256=NEW.request_sha256
 AND l.principal=NEW.principal AND l.key_fingerprint=NEW.key_fingerprint AND l.created_xact_id=NEW.created_xact_id)
 THEN RAISE EXCEPTION 'missing same-transaction staff link' USING ERRCODE='23514'; END IF;
 RETURN NULL;
END $$;
CREATE FUNCTION game_staff_link_extends_head() RETURNS trigger LANGUAGE plpgsql SECURITY INVOKER AS $$
DECLARE h game_staff_control_heads%ROWTYPE;
BEGIN
 IF NEW.link_kind='ACTIVATION' THEN
   -- Check authenticated and effective identities, including indirect LOGIN
   -- members. pg_has_role treats superusers as members of every group; retain
   -- their existing owner/bootstrap authority instead of rejecting that path.
   IF EXISTS (
     SELECT 1 FROM pg_catalog.pg_roles r
     WHERE r.rolname IN (session_user,current_user) AND NOT r.rolsuper
       AND (pg_catalog.pg_has_role(r.oid,'oteryn_game_runtime','MEMBER')
         OR pg_catalog.pg_has_role(r.oid,'oteryn_game_control','MEMBER'))
   ) THEN
     RAISE EXCEPTION 'runtime activation forbidden' USING ERRCODE='23514';
   END IF;
 END IF;
 SELECT * INTO h FROM game_staff_control_heads WHERE environment=NEW.environment FOR UPDATE;
 -- Only the initial signed anchor may precede the first head. Later
 -- activations serialize and bind to the same current head as other links.
 IF NOT FOUND AND NEW.link_kind='ACTIVATION' AND NEW.control_seq=1
    AND NEW.previous_link_sha256=repeat('0',64) THEN
   RETURN NEW;
 END IF;
 IF NOT FOUND OR NEW.control_seq<>h.control_seq+1 OR NEW.previous_link_sha256<>h.link_sha256 THEN
   RAISE EXCEPTION 'staff predecessor conflict' USING ERRCODE='23514';
 END IF;
 RETURN NEW;
END $$;
CREATE FUNCTION game_staff_link_has_outcome() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NEW.link_kind='REQUEST' AND NOT EXISTS(SELECT 1 FROM game_staff_operations o WHERE o.environment=NEW.environment
 AND o.control_seq=NEW.control_seq AND o.request_bytes=NEW.signed_bytes AND o.created_xact_id=NEW.created_xact_id)
 THEN RAISE EXCEPTION 'staff link has no outcome' USING ERRCODE='23514'; END IF;
 IF NEW.link_kind='RECEIPT' AND NOT EXISTS(SELECT 1 FROM game_staff_action_receipts r WHERE r.environment=NEW.environment
 AND r.control_seq=NEW.control_seq AND r.receipt_bytes=NEW.signed_bytes AND r.principal=NEW.principal
 AND r.key_fingerprint=NEW.key_fingerprint AND r.signature=NEW.signature AND r.created_xact_id=NEW.created_xact_id)
 THEN RAISE EXCEPTION 'staff receipt link has no receipt' USING ERRCODE='23514'; END IF;
 IF NOT EXISTS(SELECT 1 FROM game_staff_control_heads h WHERE h.environment=NEW.environment AND h.control_seq>=NEW.control_seq)
 THEN RAISE EXCEPTION 'staff head not advanced' USING ERRCODE='23514'; END IF;
 RETURN NULL;
END $$;
CREATE FUNCTION game_staff_receipt_has_link() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NOT EXISTS(SELECT 1 FROM game_staff_control_links l WHERE l.environment=NEW.environment AND l.control_seq=NEW.control_seq
 AND l.link_kind='RECEIPT' AND l.signed_bytes=NEW.receipt_bytes AND l.principal=NEW.principal
 AND l.key_fingerprint=NEW.key_fingerprint AND l.signature=NEW.signature AND l.created_xact_id=NEW.created_xact_id)
 THEN RAISE EXCEPTION 'staff receipt has no same-transaction link' USING ERRCODE='23514'; END IF;
 RETURN NULL;
END $$;
CREATE TRIGGER game_staff_link_predecessor BEFORE INSERT ON game_staff_control_links
 FOR EACH ROW EXECUTE FUNCTION game_staff_link_extends_head();
CREATE CONSTRAINT TRIGGER game_staff_link_outcome AFTER INSERT ON game_staff_control_links
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_staff_link_has_outcome();
CREATE CONSTRAINT TRIGGER game_staff_operation_audit AFTER INSERT ON game_staff_operations
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_staff_operation_has_audit();
CREATE CONSTRAINT TRIGGER game_staff_receipt_link AFTER INSERT ON game_staff_action_receipts
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_staff_receipt_has_link();
CREATE TRIGGER game_staff_link_immutable BEFORE UPDATE OR DELETE ON game_staff_control_links FOR EACH ROW EXECUTE FUNCTION game_staff_immutable();
CREATE TRIGGER game_staff_link_no_truncate BEFORE TRUNCATE ON game_staff_control_links FOR EACH STATEMENT EXECUTE FUNCTION game_staff_immutable();
CREATE TRIGGER game_staff_operation_immutable BEFORE UPDATE OR DELETE ON game_staff_operations FOR EACH ROW EXECUTE FUNCTION game_staff_immutable();
CREATE TRIGGER game_staff_operation_no_truncate BEFORE TRUNCATE ON game_staff_operations FOR EACH STATEMENT EXECUTE FUNCTION game_staff_immutable();
CREATE TRIGGER game_staff_receipt_immutable BEFORE UPDATE OR DELETE ON game_staff_action_receipts FOR EACH ROW EXECUTE FUNCTION game_staff_immutable();
CREATE TRIGGER game_staff_receipt_no_truncate BEFORE TRUNCATE ON game_staff_action_receipts FOR EACH STATEMENT EXECUTE FUNCTION game_staff_immutable();
REVOKE ALL ON game_staff_control_links,game_staff_control_heads,game_staff_operations,game_staff_action_outbox,game_staff_action_receipts FROM PUBLIC;
GRANT SELECT,INSERT ON game_staff_control_links,game_staff_operations,game_staff_action_outbox,game_staff_action_receipts TO oteryn_game_runtime,oteryn_game_control;
GRANT SELECT ON game_staff_control_heads TO oteryn_game_runtime,oteryn_game_control;
GRANT UPDATE(control_seq,link_sha256) ON game_staff_control_heads TO oteryn_game_runtime,oteryn_game_control;
REVOKE ALL ON FUNCTION game_staff_immutable(),game_staff_operation_has_audit(),game_staff_link_extends_head(),game_staff_link_has_outcome() FROM PUBLIC;
REVOKE ALL ON FUNCTION game_staff_receipt_has_link() FROM PUBLIC;
