-- ADM-01 §9. No gameplay authority is granted to the outbound admin agent.
DO $$ BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'oteryn_game_admin_agent') THEN
        BEGIN
            CREATE ROLE oteryn_game_admin_agent NOLOGIN;
        EXCEPTION WHEN duplicate_object OR unique_violation THEN
            NULL;
        END;
    END IF;
END $$;

CREATE TABLE IF NOT EXISTS admin_command_ledger (
    ledger_id uuid PRIMARY KEY,
    request_id uuid NOT NULL,
    admin_operation_id uuid NOT NULL,
    operation text NOT NULL,
    operation_version integer NOT NULL CHECK (operation_version > 0),
    params_digest text NOT NULL,
    canonical_request bytea NOT NULL,
    signer_key_id text NOT NULL,
    admin_id text NOT NULL,
    risk_class text NOT NULL CHECK (risk_class IN ('R0','R1','R2','R3','R4')),
    target jsonb NOT NULL,
    status text NOT NULL,
    error_code text,
    effects jsonb NOT NULL CHECK (jsonb_typeof(effects) = 'array'),
    started_at timestamptz NOT NULL,
    finished_at timestamptz NOT NULL,
    agent_id text NOT NULL,
    retain_until timestamptz NOT NULL DEFAULT (CURRENT_TIMESTAMP + INTERVAL '2 years'),
    legal_hold boolean NOT NULL DEFAULT false
);
CREATE INDEX IF NOT EXISTS admin_ledger_operation_idx ON admin_command_ledger(admin_operation_id);
CREATE INDEX IF NOT EXISTS admin_ledger_admin_time_idx ON admin_command_ledger(admin_id, started_at, ledger_id);
CREATE INDEX IF NOT EXISTS admin_ledger_time_idx ON admin_command_ledger(started_at, ledger_id);
CREATE OR REPLACE FUNCTION admin_ledger_append_only() RETURNS trigger
LANGUAGE plpgsql AS $$ BEGIN
    RAISE EXCEPTION 'admin_command_ledger is append-only' USING ERRCODE = '42501';
END $$;
DROP TRIGGER IF EXISTS admin_ledger_append_only ON admin_command_ledger;
CREATE TRIGGER admin_ledger_append_only BEFORE UPDATE OR DELETE OR TRUNCATE
ON admin_command_ledger FOR EACH STATEMENT EXECUTE FUNCTION admin_ledger_append_only();
REVOKE ALL ON FUNCTION admin_ledger_append_only() FROM PUBLIC;

CREATE TABLE IF NOT EXISTS admin_request_nonces (
    nonce text PRIMARY KEY,
    expires_at timestamptz NOT NULL
);
CREATE INDEX IF NOT EXISTS admin_nonce_expiry_idx ON admin_request_nonces(expires_at);
CREATE TABLE IF NOT EXISTS admin_idempotency (
    admin_operation_id uuid PRIMARY KEY,
    params_digest text NOT NULL,
    result jsonb NOT NULL
);
REVOKE ALL ON admin_command_ledger, admin_request_nonces, admin_idempotency FROM PUBLIC;
REVOKE ALL ON admin_command_ledger, admin_request_nonces, admin_idempotency FROM oteryn_game_admin_agent;
GRANT INSERT ON admin_command_ledger TO oteryn_game_admin_agent;
GRANT SELECT, INSERT ON admin_idempotency TO oteryn_game_admin_agent;
GRANT SELECT, INSERT, UPDATE ON admin_request_nonces TO oteryn_game_admin_agent;
-- Control-plane evidence is readable by control, never by gameplay runtime.
GRANT SELECT ON admin_command_ledger, admin_request_nonces, admin_idempotency TO oteryn_game_control;
