-- P4 Part A: frozen Platform c5780e2c enforcement wire. No network surface is enabled.
CREATE TABLE IF NOT EXISTS game_platform_enforcement_configuration (
 singleton BOOLEAN PRIMARY KEY CHECK(singleton), enabled BOOLEAN NOT NULL DEFAULT false,
 supported_policy_revision BIGINT CHECK(supported_policy_revision>0)
);
INSERT INTO game_platform_enforcement_configuration(singleton) VALUES(true) ON CONFLICT DO NOTHING;
CREATE TABLE IF NOT EXISTS game_platform_enforcement_cursor (
 singleton BOOLEAN PRIMARY KEY CHECK(singleton), stream_cursor TEXT,
 CHECK(stream_cursor IS NULL OR (stream_cursor ~ '^v1:[1-9][0-9]{0,18}$' AND substring(stream_cursor FROM 4)::numeric<=9223372036854775807))
);
INSERT INTO game_platform_enforcement_cursor(singleton) VALUES(true) ON CONFLICT DO NOTHING;
CREATE TABLE IF NOT EXISTS game_platform_enforcement_streams (
 sanction_stream_id UUID PRIMARY KEY, decision_revision BIGINT NOT NULL CHECK(decision_revision>0),
 profile TEXT NOT NULL CHECK(profile IN('MUTE','JAIL')),
 account_id UUID NOT NULL, character_id UUID NOT NULL,
 state TEXT NOT NULL CHECK(state IN('PENDING','ACTIVE','REVOKED','EXPIRED')),
 effective_at_ms BIGINT NOT NULL CHECK(effective_at_ms>0),
 expires_at_ms BIGINT NOT NULL CHECK(expires_at_ms>effective_at_ms),
 reason_code INTEGER NOT NULL CHECK(reason_code BETWEEN 1 AND 65535),
 policy_revision BIGINT NOT NULL CHECK(policy_revision>0)
);
CREATE INDEX IF NOT EXISTS game_platform_enforcement_active_character ON game_platform_enforcement_streams(character_id,expires_at_ms DESC)
 WHERE profile='MUTE' AND state IN('PENDING','ACTIVE');
CREATE TABLE IF NOT EXISTS game_platform_enforcement_receipts (
 operation_id UUID PRIMARY KEY, sanction_stream_id UUID NOT NULL,
 command_bytes BYTEA NOT NULL CHECK(octet_length(command_bytes) BETWEEN 1 AND 1024),
 command_digest TEXT NOT NULL CHECK(command_digest ~ '^[0-9a-f]{64}$'),
 receipt_revision INTEGER NOT NULL CHECK(receipt_revision BETWEEN 1 AND 65535),
 result_bytes BYTEA NOT NULL CHECK(octet_length(result_bytes) BETWEEN 1 AND 1024)
);
CREATE TABLE IF NOT EXISTS game_platform_enforcement_result_outbox (
 operation_id UUID NOT NULL REFERENCES game_platform_enforcement_receipts(operation_id),
 command_digest TEXT NOT NULL CHECK(command_digest ~ '^[0-9a-f]{64}$'),
 receipt_revision INTEGER NOT NULL CHECK(receipt_revision BETWEEN 1 AND 65535),
 result_bytes BYTEA NOT NULL CHECK(octet_length(result_bytes) BETWEEN 1 AND 1024),
 PRIMARY KEY(operation_id,command_digest,receipt_revision)
);
-- The deciding owner samples Game time only after acquiring deciding locks.
CREATE OR REPLACE FUNCTION game_platform_enforcement_now_ms() RETURNS BIGINT LANGUAGE sql VOLATILE
 AS $$ SELECT floor(extract(epoch FROM clock_timestamp())*1000)::bigint $$;
REVOKE ALL ON game_platform_enforcement_configuration,game_platform_enforcement_cursor,
 game_platform_enforcement_streams,game_platform_enforcement_receipts,game_platform_enforcement_result_outbox FROM PUBLIC;
GRANT SELECT ON game_platform_enforcement_configuration TO oteryn_game_runtime;
GRANT SELECT,UPDATE ON game_platform_enforcement_configuration TO oteryn_game_control;
GRANT SELECT,UPDATE ON game_platform_enforcement_cursor TO oteryn_game_runtime,oteryn_game_control;
GRANT SELECT,INSERT,UPDATE ON game_platform_enforcement_streams,game_platform_enforcement_receipts TO oteryn_game_runtime,oteryn_game_control;
GRANT SELECT,INSERT,DELETE ON game_platform_enforcement_result_outbox TO oteryn_game_runtime,oteryn_game_control;
