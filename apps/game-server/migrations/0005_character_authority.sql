CREATE TABLE game_character_account_guards (
    account_id UUID PRIMARY KEY CHECK (account_id <> '00000000-0000-0000-0000-000000000000'::uuid)
);

CREATE TABLE game_character_roots (
    character_id UUID PRIMARY KEY,
    account_id UUID NOT NULL REFERENCES game_character_account_guards(account_id),
    world_id UUID NOT NULL,
    lifecycle SMALLINT NOT NULL CHECK (lifecycle = 1),
    character_revision NUMERIC(20,0) NOT NULL CHECK (character_revision BETWEEN 1 AND 18446744073709551615),
    profile_revision TEXT NOT NULL CHECK (octet_length(profile_revision) BETWEEN 1 AND 128),
    ruleset_revision TEXT NOT NULL CHECK (octet_length(ruleset_revision) BETWEEN 1 AND 128),
    content_revision TEXT NOT NULL CHECK (octet_length(content_revision) BETWEEN 1 AND 128),
    starter_template_revision TEXT NOT NULL CHECK (octet_length(starter_template_revision) BETWEEN 1 AND 128),
    CHECK (get_byte(uuid_send(character_id), 6) >> 4 = 7),
    CHECK (get_byte(uuid_send(account_id), 6) >> 4 = 7),
    CHECK (get_byte(uuid_send(world_id), 6) >> 4 = 7)
);

CREATE TABLE game_character_audit_outbox (
    event_id UUID PRIMARY KEY,
    transaction_id UUID NOT NULL,
    transaction_ordinal INTEGER NOT NULL CHECK (transaction_ordinal = 1),
    transaction_count INTEGER NOT NULL CHECK (transaction_count = 1),
    event_type_id BIGINT NOT NULL CHECK (event_type_id = 1),
    schema_revision BIGINT NOT NULL CHECK (schema_revision = 1),
    retention_profile_id TEXT NOT NULL CHECK (retention_profile_id = 'CHARACTER_AUTHORITY_DURABLE_AUDIT_RETENTION_V1'),
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    occurred_at BIGINT NOT NULL CHECK (occurred_at >= 0),
    payload BYTEA NOT NULL CHECK (octet_length(payload) BETWEEN 1 AND 8192),
    payload_sha256 BYTEA NOT NULL CHECK (octet_length(payload_sha256) = 32),
    publication_state SMALLINT NOT NULL CHECK (publication_state IN (1,2)),
    CHECK (get_byte(uuid_send(event_id), 6) >> 4 = 7),
    CHECK (get_byte(uuid_send(transaction_id), 6) >> 4 = 7),
    UNIQUE (transaction_id, transaction_ordinal),
    UNIQUE (character_id, event_type_id)
);

CREATE TABLE game_character_operation_receipts (
    operation_id UUID PRIMARY KEY,
    command_binding BYTEA NOT NULL CHECK (octet_length(command_binding) BETWEEN 1 AND 1024),
    account_id UUID NOT NULL,
    character_id UUID NOT NULL UNIQUE REFERENCES game_character_roots(character_id),
    world_id UUID NOT NULL,
    character_revision NUMERIC(20,0) NOT NULL CHECK (character_revision = 1),
    event_id UUID NOT NULL UNIQUE REFERENCES game_character_audit_outbox(event_id),
    transaction_id UUID NOT NULL,
    payload BYTEA NOT NULL CHECK (octet_length(payload) BETWEEN 1 AND 8192),
    CHECK (get_byte(uuid_send(operation_id), 6) >> 4 = 7)
);

CREATE FUNCTION game_character_immutable() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN
    RAISE EXCEPTION 'Character first-slice authority history is immutable' USING ERRCODE = '23514';
END; $$;
CREATE TRIGGER game_character_root_immutable BEFORE UPDATE OR DELETE ON game_character_roots FOR EACH ROW EXECUTE FUNCTION game_character_immutable();
CREATE TRIGGER game_character_receipt_immutable BEFORE UPDATE OR DELETE ON game_character_operation_receipts FOR EACH ROW EXECUTE FUNCTION game_character_immutable();
CREATE TRIGGER game_character_audit_immutable BEFORE UPDATE OR DELETE ON game_character_audit_outbox FOR EACH ROW EXECUTE FUNCTION game_character_immutable();

