-- S04c. Control decision PR #2780 comment 6547489522: a current role projection
-- beside immutable source-group evidence; no role is accepted from a game client.
CREATE TABLE game_chat_role_receipts (
    audit_receipt UUID PRIMARY KEY,
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    revision BIGINT NOT NULL CHECK (revision > 0),
    group_id SMALLINT NOT NULL CHECK (group_id BETWEEN 1 AND 7),
    account_type SMALLINT NOT NULL CHECK (account_type BETWEEN 1 AND 6),
    group_access BOOLEAN NOT NULL,
    decision_identity TEXT NOT NULL CHECK (octet_length(decision_identity) BETWEEN 1 AND 256),
    issued_by NAME NOT NULL,
    created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
    UNIQUE (character_id, revision, audit_receipt)
);
CREATE TABLE game_chat_roles (
    character_id UUID PRIMARY KEY REFERENCES game_character_roots(character_id),
    revision BIGINT NOT NULL CHECK (revision > 0),
    group_id SMALLINT NOT NULL CHECK (group_id BETWEEN 1 AND 7),
    account_type SMALLINT NOT NULL CHECK (account_type BETWEEN 1 AND 6),
    group_access BOOLEAN NOT NULL,
    audit_receipt UUID NOT NULL UNIQUE,
    FOREIGN KEY (character_id, revision, audit_receipt)
        REFERENCES game_chat_role_receipts(character_id, revision, audit_receipt)
);
REVOKE ALL ON game_chat_roles, game_chat_role_receipts FROM PUBLIC;
GRANT SELECT ON game_chat_roles TO oteryn_game_runtime;
-- The trusted Control role owns its issuing function and the data it issues.
ALTER TABLE game_chat_roles OWNER TO oteryn_game_control;
ALTER TABLE game_chat_role_receipts OWNER TO oteryn_game_control;

CREATE FUNCTION game_chat_role_issue(
    p_character UUID, p_expected_revision BIGINT, p_group SMALLINT,
    p_account SMALLINT, p_access BOOLEAN, p_receipt UUID, p_decision TEXT
) RETURNS BIGINT LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE v_revision BIGINT; v_receipt game_chat_role_receipts%ROWTYPE;
BEGIN
    IF p_character IS NULL OR p_expected_revision IS NULL OR p_expected_revision < 0
       OR p_group IS NULL OR p_group NOT BETWEEN 1 AND 7
       OR p_account IS NULL OR p_account NOT BETWEEN 1 AND 6
       OR p_access IS NULL OR p_receipt IS NULL
       OR p_decision IS NULL OR octet_length(p_decision) NOT BETWEEN 1 AND 256 THEN
        RAISE EXCEPTION 'invalid chat role issue' USING ERRCODE='23514';
    END IF;
    PERFORM pg_advisory_xact_lock(hashtextextended(p_character::text, 717));
    SELECT * INTO v_receipt FROM game_chat_role_receipts WHERE audit_receipt=p_receipt;
    IF FOUND THEN
        IF v_receipt.character_id IS DISTINCT FROM p_character
           OR v_receipt.revision IS DISTINCT FROM p_expected_revision+1
           OR v_receipt.group_id IS DISTINCT FROM p_group
           OR v_receipt.account_type IS DISTINCT FROM p_account
           OR v_receipt.group_access IS DISTINCT FROM p_access
           OR v_receipt.decision_identity IS DISTINCT FROM p_decision THEN
            RAISE EXCEPTION 'chat role receipt reused with different request' USING ERRCODE='23514';
        END IF;
        RETURN v_receipt.revision;
    END IF;
    SELECT revision INTO v_revision FROM game_chat_roles WHERE character_id=p_character FOR UPDATE;
    IF COALESCE(v_revision,0) IS DISTINCT FROM p_expected_revision THEN
        RAISE EXCEPTION 'stale chat role revision' USING ERRCODE='23514';
    END IF;
    v_revision := p_expected_revision+1;
    INSERT INTO game_chat_role_receipts(audit_receipt,character_id,revision,group_id,
        account_type,group_access,decision_identity,issued_by)
    VALUES(p_receipt,p_character,v_revision,p_group,p_account,p_access,p_decision,session_user);
    INSERT INTO game_chat_roles(character_id,revision,group_id,account_type,group_access,audit_receipt)
    VALUES(p_character,v_revision,p_group,p_account,p_access,p_receipt)
    ON CONFLICT(character_id) DO UPDATE SET revision=EXCLUDED.revision,
        group_id=EXCLUDED.group_id,account_type=EXCLUDED.account_type,
        group_access=EXCLUDED.group_access,audit_receipt=EXCLUDED.audit_receipt;
    RETURN v_revision;
END; $$;
DO $$ BEGIN
    EXECUTE format('ALTER FUNCTION game_chat_role_issue(UUID,BIGINT,SMALLINT,SMALLINT,BOOLEAN,UUID,TEXT) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
ALTER FUNCTION game_chat_role_issue(UUID,BIGINT,SMALLINT,SMALLINT,BOOLEAN,UUID,TEXT) OWNER TO oteryn_game_control;
REVOKE ALL ON FUNCTION game_chat_role_issue(UUID,BIGINT,SMALLINT,SMALLINT,BOOLEAN,UUID,TEXT) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_chat_role_issue(UUID,BIGINT,SMALLINT,SMALLINT,BOOLEAN,UUID,TEXT) TO oteryn_game_control;

CREATE TABLE game_chat_help_mutes (
    character_id UUID PRIMARY KEY REFERENCES game_character_roots(character_id),
    world_id UUID NOT NULL,
    condition_until_ms BIGINT NOT NULL CHECK (condition_until_ms >= 0),
    speech_until_ms BIGINT NOT NULL CHECK (speech_until_ms >= 0 AND speech_until_ms <= condition_until_ms),
    moderator_character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    last_game_session_id UUID NOT NULL,
    last_command_id NUMERIC(20,0) NOT NULL CHECK (last_command_id > 0)
);
CREATE INDEX game_chat_help_mutes_moderator ON game_chat_help_mutes(moderator_character_id);
CREATE TABLE game_chat_help_receipts (
    game_session_id UUID NOT NULL,
    command_id NUMERIC(20,0) NOT NULL CHECK (command_id > 0),
    connection_generation NUMERIC(20,0) NOT NULL CHECK (connection_generation > 0),
    request_digest BYTEA NOT NULL CHECK (octet_length(request_digest)=32),
    disposition SMALLINT NOT NULL CHECK (disposition BETWEEN 1 AND 11),
    wait_seconds INTEGER NOT NULL CHECK (wait_seconds BETWEEN 0 AND 1280),
    CHECK ((disposition IN(2,3) AND wait_seconds>0) OR (disposition NOT IN(2,3) AND wait_seconds=0)),
    PRIMARY KEY(game_session_id,command_id)
);
REVOKE ALL ON game_chat_help_mutes,game_chat_help_receipts FROM PUBLIC;
GRANT SELECT,INSERT,UPDATE ON game_chat_help_mutes TO oteryn_game_runtime;
GRANT SELECT,INSERT ON game_chat_help_receipts TO oteryn_game_runtime;

CREATE FUNCTION game_chat_role_receipt_immutable() RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION 'chat role receipts are immutable' USING ERRCODE='23514';
END; $$;
CREATE TRIGGER chat_role_receipt_immutable BEFORE UPDATE OR DELETE ON game_chat_role_receipts
    FOR EACH ROW EXECUTE FUNCTION game_chat_role_receipt_immutable();
CREATE TRIGGER chat_help_receipt_immutable BEFORE UPDATE OR DELETE ON game_chat_help_receipts
    FOR EACH ROW EXECUTE FUNCTION game_chat_role_receipt_immutable();
DO $$ BEGIN
    EXECUTE format('ALTER FUNCTION game_chat_role_receipt_immutable() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_chat_role_receipt_immutable() FROM PUBLIC;

CREATE FUNCTION game_chat_role_projection_consistent() RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM game_chat_role_receipts r
        WHERE r.audit_receipt=NEW.audit_receipt AND r.character_id=NEW.character_id
          AND r.revision=NEW.revision AND r.group_id=NEW.group_id
          AND r.account_type=NEW.account_type AND r.group_access=NEW.group_access) THEN
        RAISE EXCEPTION 'chat role projection differs from its audit receipt' USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END; $$;
CREATE CONSTRAINT TRIGGER chat_role_projection_consistent AFTER INSERT OR UPDATE ON game_chat_roles
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_chat_role_projection_consistent();
DO $$ BEGIN
    EXECUTE format('ALTER FUNCTION game_chat_role_projection_consistent() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_chat_role_projection_consistent() FROM PUBLIC;
