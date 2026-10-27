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
    before_item JSONB NOT NULL,
    source_entry JSONB NOT NULL,
    stamped_production_key TEXT NOT NULL CHECK (octet_length(stamped_production_key) BETWEEN 1 AND 512),
    created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
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

-- MAIL joins the existing item proof union. The initial implementation lacked
-- these arms, so native guards rejected its stamp and source removal at commit.
CREATE FUNCTION game_mail_posting_admit() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE i game_item_instances%ROWTYPE; e game_item_container_entries%ROWTYPE;
BEGIN
    SELECT * INTO i FROM game_item_instances WHERE item_instance_id=NEW.root_item_instance_id FOR UPDATE;
    SELECT * INTO e FROM game_item_container_entries WHERE item_instance_id=NEW.root_item_instance_id FOR UPDATE;
    IF i.item_instance_id IS NULL OR e.item_instance_id IS NULL
       OR to_jsonb(i) IS DISTINCT FROM NEW.before_item OR to_jsonb(e) IS DISTINCT FROM NEW.source_entry
       OR i.lifecycle<>1 OR i.quantity<>1 OR i.definition_family<>'Item'
       OR i.world_id<>NEW.world_id OR e.world_id<>NEW.world_id OR e.character_id<>NEW.sender_character_id
       OR NEW.item_count<>1
       OR NOT EXISTS(SELECT 1 FROM game_character_roots WHERE character_id=NEW.recipient_character_id AND world_id=NEW.world_id AND lifecycle=1)
       OR EXISTS(SELECT 1 FROM game_item_container_entries WHERE parent_item_instance_id=i.item_instance_id)
       OR EXISTS(SELECT 1 FROM game_item_nested_entries WHERE parent_item_instance_id=i.item_instance_id)
       OR EXISTS(SELECT 1 FROM game_item_corpse_container_entries WHERE parent_item_instance_id=i.item_instance_id)
    THEN RAISE EXCEPTION 'mail requires unchanged single carried item without contents' USING ERRCODE='23514'; END IF;
    NEW.created_xact_id:=pg_current_xact_id();
    NEW.occurred_at:=clock_timestamp();
    RETURN NEW;
END $$;

CREATE FUNCTION game_mail_posting_complete(p_mail UUID) RETURNS void LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE r game_mail_operations%ROWTYPE; i game_item_instances%ROWTYPE;
BEGIN
    SELECT * INTO r FROM game_mail_operations WHERE mail_id=p_mail;
    SELECT * INTO i FROM game_item_instances WHERE item_instance_id=r.root_item_instance_id;
    IF r.mail_id IS NULL OR i.item_instance_id IS NULL OR r.created_xact_id<>pg_current_xact_id()
       OR i.definition_production_key<>r.stamped_production_key
       OR (to_jsonb(i)-'definition_production_key') IS DISTINCT FROM (r.before_item-'definition_production_key')
       OR EXISTS(SELECT 1 FROM game_item_container_entries WHERE item_instance_id=i.item_instance_id)
       OR NOT EXISTS(SELECT 1 FROM game_item_character_inbox_locations l JOIN game_character_inbox_deliveries d
          USING(item_instance_id,world_id,character_id,ordinal)
          WHERE l.item_instance_id=i.item_instance_id AND l.world_id=r.world_id AND l.character_id=r.recipient_character_id
            AND d.cause_kind='MAIL_POST' AND d.cause_ref=replace(r.mail_id::text,'-','') AND d.created_xact_id=pg_current_xact_id())
       OR NOT EXISTS(SELECT 1 FROM game_mail_outbox WHERE mail_id=r.mail_id AND event_type='MAIL_POSTED')
    THEN RAISE EXCEPTION 'incomplete atomic mail post' USING ERRCODE='23514'; END IF;
END $$;

CREATE FUNCTION game_mail_posting_consistent() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN PERFORM game_mail_posting_complete(NEW.mail_id); RETURN NULL; END $$;

DO $migration$
DECLARE n TEXT; body TEXT; arm TEXT; begin_at INT;
BEGIN
    FOREACH n IN ARRAY ARRAY['game_item_instance_guard','game_item_instance_change_proven','game_item_container_entry_removal_proven'] LOOP
        body:=pg_get_functiondef((n||'()')::regprocedure);
        begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
        IF begin_at=0 THEN RAISE EXCEPTION 'unexpected mail proof body: %',n; END IF;
        IF n='game_item_instance_guard' THEN
            arm:=$arm$
    IF TG_OP='UPDATE' AND EXISTS(SELECT 1 FROM game_mail_operations r
        WHERE r.root_item_instance_id=OLD.item_instance_id AND r.before_item=to_jsonb(OLD)
          AND r.created_xact_id=pg_current_xact_id() AND r.stamped_production_key=NEW.definition_production_key
          AND (to_jsonb(NEW)-'definition_production_key')=(to_jsonb(OLD)-'definition_production_key'))
    THEN RETURN NEW; END IF;
$arm$;
        ELSIF n='game_item_instance_change_proven' THEN
            arm:=$arm$
    IF EXISTS(SELECT 1 FROM game_mail_operations r WHERE r.root_item_instance_id=NEW.item_instance_id
        AND r.created_xact_id=pg_current_xact_id()) THEN
        PERFORM game_mail_posting_complete((SELECT mail_id FROM game_mail_operations WHERE root_item_instance_id=NEW.item_instance_id));
        RETURN NULL;
    END IF;
$arm$;
        ELSE
            arm:=$arm$
    IF TG_OP='DELETE' AND EXISTS(SELECT 1 FROM game_mail_operations r
        WHERE r.root_item_instance_id=OLD.item_instance_id AND r.source_entry=to_jsonb(OLD)
          AND r.created_xact_id=pg_current_xact_id()) THEN
        PERFORM game_mail_posting_complete((SELECT mail_id FROM game_mail_operations WHERE root_item_instance_id=OLD.item_instance_id));
        RETURN NULL;
    END IF;
$arm$;
        END IF;
        EXECUTE overlay(body placing E'BEGIN\n'||arm from begin_at for 5);
    END LOOP;
END $migration$;

CREATE TRIGGER game_mail_posting_admit BEFORE INSERT ON game_mail_operations
    FOR EACH ROW EXECUTE FUNCTION game_mail_posting_admit();
CREATE CONSTRAINT TRIGGER game_mail_posting_consistent AFTER INSERT ON game_mail_operations
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_mail_posting_consistent();
CREATE TRIGGER game_mail_operations_immutable BEFORE UPDATE OR DELETE ON game_mail_operations
    FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_mail_operations_no_truncate BEFORE TRUNCATE ON game_mail_operations
    FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();
CREATE TRIGGER game_mail_outbox_no_truncate BEFORE TRUNCATE ON game_mail_outbox
    FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();
DO $$ DECLARE n TEXT; BEGIN
    FOREACH n IN ARRAY ARRAY['game_mail_posting_rate_guard','game_mail_posting_admit','game_mail_posting_consistent'] LOOP
        EXECUTE format('ALTER FUNCTION %I() SET search_path=pg_catalog,%I,pg_temp',n,current_schema());
        EXECUTE format('REVOKE ALL ON FUNCTION %I() FROM PUBLIC',n);
    END LOOP;
    EXECUTE format('ALTER FUNCTION game_mail_posting_complete(UUID) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_mail_posting_complete(UUID) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_mail_posting_complete(UUID) TO oteryn_game_runtime;

-- Inbox remains a closed owner: runtime receives EXECUTE on this MAIL-only
-- transaction, never generic Inbox table writes or counter mutation privileges.
CREATE FUNCTION game_mail_posting_apply(
    p_mail UUID, p_session UUID, p_command NUMERIC, p_world UUID, p_channel UUID,
    p_sender UUID, p_recipient UUID, p_item UUID, p_stamp TEXT, p_mailbox BYTEA,
    p_hash BYTEA, p_transaction UUID, p_event BYTEA, p_before JSONB, p_source JSONB
) RETURNS BOOLEAN LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE c game_character_inbox_counters%ROWTYPE; affected BIGINT;
BEGIN
    PERFORM 1 FROM game_item_instances WHERE item_instance_id=p_item FOR UPDATE;
    PERFORM 1 FROM game_item_container_entries WHERE item_instance_id=p_item FOR UPDATE;
    INSERT INTO game_character_inbox_counters(character_id) VALUES(p_recipient) ON CONFLICT DO NOTHING;
    SELECT * INTO STRICT c FROM game_character_inbox_counters WHERE character_id=p_recipient FOR UPDATE;
    IF c.committed>=50000 THEN RETURN FALSE; END IF;
    INSERT INTO game_mail_operations(mail_id,game_session_id,command_id,world_id,channel_id,
        sender_character_id,recipient_character_id,root_item_instance_id,mailbox_placement_key,
        item_count,request_sha256,transaction_id,before_item,source_entry,stamped_production_key)
    VALUES(p_mail,p_session,p_command,p_world,p_channel,p_sender,p_recipient,p_item,p_mailbox,
        1,p_hash,p_transaction,p_before,p_source,p_stamp);
    UPDATE game_item_instances SET definition_production_key=p_stamp
        WHERE item_instance_id=p_item AND world_id=p_world;
    GET DIAGNOSTICS affected=ROW_COUNT;
    IF affected<>1 THEN
        RAISE EXCEPTION 'mail stamp did not update exactly one item' USING ERRCODE='OTM02';
    END IF;
    DELETE FROM game_item_container_entries WHERE item_instance_id=p_item AND character_id=p_sender;
    GET DIAGNOSTICS affected=ROW_COUNT;
    IF affected<>1 THEN
        RAISE EXCEPTION 'mail source removal did not remove exactly one entry' USING ERRCODE='OTM02';
    END IF;
    INSERT INTO game_character_inbox_deliveries(item_instance_id,cause_kind,cause_ref,world_id,character_id,ordinal)
        VALUES(p_item,'MAIL_POST',replace(p_mail::text,'-',''),p_world,p_recipient,c.next_ordinal);
    INSERT INTO game_item_character_inbox_locations(item_instance_id,world_id,character_id,ordinal)
        VALUES(p_item,p_world,p_recipient,c.next_ordinal);
    -- game_character_inbox_count increments committed on the location INSERT.
    -- Other Inbox writers also advance only next_ordinal explicitly.
    UPDATE game_character_inbox_counters SET next_ordinal=next_ordinal+1 WHERE character_id=p_recipient;
    INSERT INTO game_mail_outbox(mail_id,event_type,event_payload) VALUES(p_mail,'MAIL_POSTED',p_event);
    RETURN TRUE;
END $$;
DO $$ BEGIN
    EXECUTE format('ALTER FUNCTION game_mail_posting_apply(UUID,UUID,NUMERIC,UUID,UUID,UUID,UUID,UUID,TEXT,BYTEA,BYTEA,UUID,BYTEA,JSONB,JSONB) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_mail_posting_apply(UUID,UUID,NUMERIC,UUID,UUID,UUID,UUID,UUID,TEXT,BYTEA,BYTEA,UUID,BYTEA,JSONB,JSONB) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_mail_posting_apply(UUID,UUID,NUMERIC,UUID,UUID,UUID,UUID,UUID,TEXT,BYTEA,BYTEA,UUID,BYTEA,JSONB,JSONB) TO oteryn_game_runtime;
