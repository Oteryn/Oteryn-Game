-- P4 Part A: frozen Platform c5780e2c enforcement wire. No network surface is enabled.
CREATE TABLE IF NOT EXISTS game_platform_enforcement_configuration (
 singleton BOOLEAN PRIMARY KEY CHECK(singleton), enabled BOOLEAN NOT NULL DEFAULT false,
 supported_policy_revision BIGINT CHECK(supported_policy_revision>0)
);
INSERT INTO game_platform_enforcement_configuration(singleton) VALUES(true) ON CONFLICT DO NOTHING;
CREATE TABLE IF NOT EXISTS game_platform_enforcement_cursor (
 singleton BOOLEAN PRIMARY KEY CHECK(singleton), stream_cursor TEXT, reconcile_cursor TEXT,
 stream_stopped BOOLEAN NOT NULL DEFAULT false,reconcile_stopped BOOLEAN NOT NULL DEFAULT false,
 CHECK(reconcile_cursor IS NULL OR (reconcile_cursor ~ '^v1:[1-9][0-9]{0,18}$' AND substring(reconcile_cursor FROM 4)::numeric<=9223372036854775807)),
 CHECK(stream_cursor IS NULL OR (stream_cursor ~ '^v1:[1-9][0-9]{0,18}$' AND substring(stream_cursor FROM 4)::numeric<=9223372036854775807))
);
INSERT INTO game_platform_enforcement_cursor(singleton) VALUES(true) ON CONFLICT DO NOTHING;
CREATE TABLE IF NOT EXISTS game_platform_enforcement_streams (
 sanction_stream_id UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(sanction_stream_id)), decision_revision BIGINT NOT NULL CHECK(decision_revision>0),
 profile TEXT NOT NULL CHECK(profile IN('MUTE','JAIL')),
 account_id UUID NOT NULL CHECK(game_character_is_uuid_v7(account_id)), character_id UUID NOT NULL CHECK(game_character_is_uuid_v7(character_id)),
 state TEXT NOT NULL CHECK(state IN('PENDING','ACTIVE','REVOKED','EXPIRED')),
 effective_at_ms BIGINT NOT NULL CHECK(effective_at_ms>0),
 expires_at_ms BIGINT NOT NULL CHECK(expires_at_ms>effective_at_ms),
 reason_code INTEGER NOT NULL CHECK(reason_code BETWEEN 1 AND 65535),
 policy_revision BIGINT NOT NULL CHECK(policy_revision>0)
);
CREATE INDEX IF NOT EXISTS game_platform_enforcement_active_character ON game_platform_enforcement_streams(character_id,expires_at_ms DESC)
 WHERE profile='MUTE' AND state IN('PENDING','ACTIVE');
CREATE TABLE IF NOT EXISTS game_platform_enforcement_receipts (
 operation_id UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(operation_id)), sanction_stream_id UUID NOT NULL CHECK(game_character_is_uuid_v7(sanction_stream_id)),
 command_bytes BYTEA NOT NULL CHECK(octet_length(command_bytes) BETWEEN 1 AND 1024),
 command_digest TEXT NOT NULL CHECK(command_digest ~ '^[0-9a-f]{64}$'),
 receipt_revision INTEGER NOT NULL CHECK(receipt_revision BETWEEN 1 AND 65535),
 result_bytes BYTEA NOT NULL CHECK(octet_length(result_bytes) BETWEEN 1 AND 1024),
 publication_stopped BOOLEAN NOT NULL DEFAULT false,
 overdue_alarmed BOOLEAN NOT NULL DEFAULT false
);
CREATE TABLE IF NOT EXISTS game_platform_enforcement_result_outbox (
 delivery_position BIGSERIAL UNIQUE,
 acknowledged BOOLEAN NOT NULL DEFAULT false,held BOOLEAN NOT NULL DEFAULT false,reconcile_required BOOLEAN NOT NULL DEFAULT false,
 operation_id UUID NOT NULL CHECK(game_character_is_uuid_v7(operation_id)) REFERENCES game_platform_enforcement_receipts(operation_id),
 command_digest TEXT NOT NULL CHECK(command_digest ~ '^[0-9a-f]{64}$'),
 receipt_revision INTEGER NOT NULL CHECK(receipt_revision BETWEEN 1 AND 65535),
 result_bytes BYTEA NOT NULL CHECK(octet_length(result_bytes) BETWEEN 1 AND 1024),
 PRIMARY KEY(operation_id,receipt_revision)
);
-- Contradictory input is durable local evidence, never another A3 result for
-- the original receipt key (§4.3 rejects that publication; §6.2 calls it conflict).
CREATE TABLE IF NOT EXISTS game_platform_enforcement_conflicts (
 operation_id UUID NOT NULL REFERENCES game_platform_enforcement_receipts(operation_id) CHECK(game_character_is_uuid_v7(operation_id)),
 command_digest TEXT NOT NULL CHECK(command_digest ~ '^[0-9a-f]{64}$'),
 command_bytes BYTEA NOT NULL CHECK(octet_length(command_bytes) BETWEEN 1 AND 1024),
 result_code TEXT NOT NULL CHECK(result_code='OPERATION_CONFLICT'),
 PRIMARY KEY(operation_id,command_digest)
);
CREATE OR REPLACE FUNCTION game_platform_enforcement_receipt_immutable() RETURNS trigger
 LANGUAGE plpgsql SET search_path=pg_catalog,public AS $$ BEGIN
 IF (NEW.operation_id,NEW.sanction_stream_id,NEW.command_bytes,NEW.command_digest)
  IS DISTINCT FROM (OLD.operation_id,OLD.sanction_stream_id,OLD.command_bytes,OLD.command_digest) THEN
  RAISE EXCEPTION 'enforcement command binding is immutable' USING ERRCODE='23514';
 END IF;
 RETURN NEW;
END $$;
CREATE OR REPLACE TRIGGER game_platform_enforcement_receipt_immutable
 BEFORE UPDATE ON game_platform_enforcement_receipts FOR EACH ROW
 EXECUTE FUNCTION game_platform_enforcement_receipt_immutable();
CREATE OR REPLACE FUNCTION game_platform_enforcement_result_immutable() RETURNS trigger
 LANGUAGE plpgsql SET search_path=pg_catalog,public AS $$ BEGIN
 IF (NEW.operation_id,NEW.command_digest,NEW.receipt_revision,NEW.result_bytes,NEW.delivery_position)
  IS DISTINCT FROM (OLD.operation_id,OLD.command_digest,OLD.receipt_revision,OLD.result_bytes,OLD.delivery_position) THEN
  RAISE EXCEPTION 'enforcement result binding is immutable' USING ERRCODE='23514';
 END IF;
 RETURN NEW;
END $$;
CREATE OR REPLACE TRIGGER game_platform_enforcement_result_immutable
 BEFORE UPDATE ON game_platform_enforcement_result_outbox FOR EACH ROW
 EXECUTE FUNCTION game_platform_enforcement_result_immutable();
REVOKE ALL ON FUNCTION game_platform_enforcement_result_immutable() FROM PUBLIC;
-- The deciding owner samples Game time only after acquiring deciding locks.
CREATE OR REPLACE FUNCTION game_platform_enforcement_now_ms() RETURNS BIGINT LANGUAGE sql VOLATILE
 SET search_path=pg_catalog,public
 AS $$ SELECT floor(extract(epoch FROM clock_timestamp())*1000)::bigint $$;
REVOKE ALL ON FUNCTION game_platform_enforcement_now_ms(),game_platform_enforcement_receipt_immutable() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_platform_enforcement_now_ms() TO oteryn_game_runtime;
REVOKE ALL ON game_platform_enforcement_configuration,game_platform_enforcement_cursor,
 game_platform_enforcement_streams,game_platform_enforcement_receipts,game_platform_enforcement_result_outbox,
 game_platform_enforcement_conflicts FROM PUBLIC,oteryn_game_control;
GRANT SELECT ON game_platform_enforcement_configuration TO oteryn_game_runtime;
GRANT SELECT,UPDATE ON game_platform_enforcement_cursor TO oteryn_game_runtime;
GRANT SELECT,INSERT,UPDATE ON game_platform_enforcement_streams,game_platform_enforcement_receipts TO oteryn_game_runtime;
GRANT SELECT,INSERT,UPDATE ON game_platform_enforcement_result_outbox TO oteryn_game_runtime;
GRANT SELECT,INSERT ON game_platform_enforcement_conflicts TO oteryn_game_runtime;
GRANT SELECT ON game_platform_enforcement_configuration,game_platform_enforcement_cursor,
 game_platform_enforcement_streams,game_platform_enforcement_receipts,game_platform_enforcement_result_outbox,
 game_platform_enforcement_conflicts TO oteryn_game_control;

GRANT USAGE,SELECT ON SEQUENCE game_platform_enforcement_result_outbox_delivery_position_seq TO oteryn_game_runtime;
