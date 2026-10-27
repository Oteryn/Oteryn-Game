-- MAIL-1 durable occurrence, rate/ceiling policy and event owner. The posting
-- writer uses these rows in the same transaction as source-location deletion,
-- PRESERVE_INSTANCE definition transform and CharacterInbox insertion.
INSERT INTO game_character_inbox_cause_kinds(cause_kind) VALUES ('MAIL_POST');

CREATE TABLE game_mail_operations (
    mail_id UUID PRIMARY KEY CHECK (game_character_is_uuid_v7(mail_id)),
    game_session_id UUID NOT NULL CHECK (game_character_is_uuid_v7(game_session_id)),
    command_id NUMERIC(20,0) NOT NULL CHECK (command_id BETWEEN 1 AND 18446744073709551615),
    world_id UUID NOT NULL,
    channel_id UUID NOT NULL,
    sender_character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    recipient_character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    root_item_instance_id UUID NOT NULL REFERENCES game_item_instances(item_instance_id),
    mailbox_placement_key BYTEA NOT NULL CHECK (octet_length(mailbox_placement_key) BETWEEN 1 AND 128),
    item_count INTEGER NOT NULL CHECK (item_count BETWEEN 1 AND 499),
    request_sha256 BYTEA NOT NULL CHECK (octet_length(request_sha256)=32),
    transaction_id UUID NOT NULL UNIQUE CHECK (game_character_is_uuid_v7(transaction_id)),
    occurred_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    UNIQUE(game_session_id, command_id),
    UNIQUE(root_item_instance_id)
);

CREATE INDEX game_mail_operations_sender_rate
    ON game_mail_operations(sender_character_id, occurred_at DESC);

CREATE TABLE game_mail_outbox (
    mail_id UUID PRIMARY KEY REFERENCES game_mail_operations(mail_id) ON DELETE RESTRICT,
    event_type TEXT NOT NULL CHECK (event_type='MAIL_POSTED'),
    event_payload BYTEA NOT NULL CHECK (octet_length(event_payload) BETWEEN 1 AND 131072),
    created_at TIMESTAMPTZ NOT NULL DEFAULT transaction_timestamp(),
    published_at TIMESTAMPTZ
);

-- Defense in depth for the accepted one-post-per-second rule. Sender-root
-- serialization in the writer makes this predicate authoritative under concurrency.
CREATE FUNCTION game_mail_posting_rate_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF EXISTS (SELECT 1 FROM game_mail_operations o
                WHERE o.sender_character_id=NEW.sender_character_id
                  AND o.occurred_at > NEW.occurred_at - interval '1 second') THEN
        RAISE EXCEPTION 'mail posting rate exhausted' USING ERRCODE='OTM01';
    END IF;
    RETURN NEW;
END $$;
CREATE TRIGGER game_mail_posting_rate_guard
    BEFORE INSERT ON game_mail_operations FOR EACH ROW
    EXECUTE FUNCTION game_mail_posting_rate_guard();

REVOKE ALL ON game_mail_operations, game_mail_outbox FROM PUBLIC;
REVOKE ALL ON FUNCTION game_mail_posting_rate_guard() FROM PUBLIC;
GRANT SELECT, INSERT ON game_mail_operations, game_mail_outbox TO oteryn_game_runtime;
GRANT SELECT ON game_mail_operations, game_mail_outbox TO oteryn_game_control;
