-- MAIL-1 / CP D1152, D1158, D1318, D1356: the standing migration 0101.
-- Install the text storage, native Inbox sink and complete fresh reward MINT
-- receipt/audit/grant union together through the actual migrations directory.
-- The source owns its claim/line/binding callback in the same native Root TX.
-- This schema supplies no DAILY admission, claim policy or production caller.
-- Production activation remains after #2197; resource ceilings are CANDIDATE
-- until the native prost/JSON qualification prints its measured byte sizes.

-- Existing owned MAIL-TEXT storage. The actual text/TRADE ingress and daily
-- scheduler retain their task-card gates; the native storage ports are below.
CREATE TABLE game_item_text_revisions (
 item_instance_id UUID PRIMARY KEY REFERENCES game_item_instances ON DELETE RESTRICT,
 last_revision NUMERIC(20,0) NOT NULL CHECK(last_revision BETWEEN 1 AND 18446744073709551615)
);
CREATE TABLE game_item_texts (
 item_instance_id UUID PRIMARY KEY REFERENCES game_item_text_revisions ON DELETE RESTRICT,
 text TEXT NOT NULL CHECK(octet_length(text) BETWEEN 1 AND 7996),
 writer_character_id UUID REFERENCES game_character_roots ON DELETE RESTRICT,
 written_at TIMESTAMPTZ NOT NULL,
 text_revision NUMERIC(20,0) NOT NULL CHECK(text_revision BETWEEN 1 AND 18446744073709551615)
);
CREATE TABLE game_item_text_write_receipts (
 game_session_id UUID NOT NULL,
 command_id NUMERIC(20,0) NOT NULL CHECK(command_id BETWEEN 1 AND 18446744073709551615),
 character_id UUID NOT NULL REFERENCES game_character_roots ON DELETE RESTRICT,
 request_binding BYTEA NOT NULL CHECK(octet_length(request_binding)=32),
 text_revision NUMERIC(20,0) NOT NULL CHECK(text_revision BETWEEN 0 AND 18446744073709551615),
 changed BOOLEAN NOT NULL,
 written_at TIMESTAMPTZ NOT NULL,
 PRIMARY KEY(game_session_id,command_id)
);
CREATE INDEX game_item_text_write_rate ON game_item_text_write_receipts(character_id,written_at DESC)
 WHERE changed;

-- Only the operations used by the existing fenced TEXT writer. An empty write
-- may remove text but never its revision; receipts stay immutable and retained.
REVOKE ALL ON game_item_text_revisions, game_item_texts, game_item_text_write_receipts
 FROM PUBLIC, oteryn_game_runtime, oteryn_game_control;
GRANT SELECT, INSERT, UPDATE(last_revision) ON game_item_text_revisions
 TO oteryn_game_runtime;
GRANT SELECT, INSERT, DELETE, UPDATE(text,writer_character_id,written_at,text_revision)
 ON game_item_texts TO oteryn_game_runtime;
GRANT SELECT, INSERT ON game_item_text_write_receipts TO oteryn_game_runtime;

-- MAIL0 §4 / WRITE0 §5.2: exactly one already-retired item in the caller's TX.
-- This is not a scheduler, World-owner permit or item-retirement operation.
-- The item row lock serializes cleanup with the existing write/retirement paths.
-- Runtime gets no direct revision DELETE; only this proven-retired branch may
-- remove it. CommandRef receipts are deliberately outside the deletion set.
CREATE FUNCTION game_item_text_cleanup_retired(p_world UUID, p_item UUID)
RETURNS TABLE(retired BOOLEAN, text_removed BOOLEAN, revision_removed BOOLEAN)
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE
 v_lifecycle SMALLINT;
 v_text_rows BIGINT;
 v_revision_rows BIGINT;
BEGIN
 IF p_world IS NULL OR p_item IS NULL THEN
  RAISE EXCEPTION 'text cleanup requires exact World and item identities'
   USING ERRCODE = '22023';
 END IF;
 SELECT lifecycle INTO v_lifecycle FROM game_item_instances
  WHERE item_instance_id=p_item AND world_id=p_world FOR UPDATE;
 IF NOT FOUND OR v_lifecycle<>2 THEN
  RETURN QUERY SELECT FALSE, FALSE, FALSE;
  RETURN;
 END IF;
 DELETE FROM game_item_texts WHERE item_instance_id=p_item;
 GET DIAGNOSTICS v_text_rows = ROW_COUNT;
 DELETE FROM game_item_text_revisions WHERE item_instance_id=p_item;
 GET DIAGNOSTICS v_revision_rows = ROW_COUNT;
 RETURN QUERY SELECT TRUE, v_text_rows=1, v_revision_rows=1;
END $$;
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_item_text_cleanup_retired(UUID,UUID) SET search_path = pg_catalog, %I, pg_temp', current_schema());
END $$;
REVOKE ALL ON FUNCTION game_item_text_cleanup_retired(UUID,UUID)
 FROM PUBLIC, oteryn_game_runtime, oteryn_game_control;
GRANT EXECUTE ON FUNCTION game_item_text_cleanup_retired(UUID,UUID)
 TO oteryn_game_runtime;

-- D1318: the narrow same-transaction Inbox seam.
-- The registry spelling is migration 0076's uppercase representation of the
-- existing reward_claim_mint operation. It adds no new audit Cause variant.
INSERT INTO game_character_inbox_cause_kinds(cause_kind)
VALUES ('REWARD_CLAIM_MINT') ON CONFLICT DO NOTHING;

-- A CommandRef identifies one full output plan even when every requested item
-- ID changes on retry. No scan through another character's unbounded history.
CREATE INDEX game_inbox_reward_claim_command
    ON game_character_inbox_deliveries
       ((split_part(cause_ref, ':', 1)), (split_part(cause_ref, ':', 2)))
    WHERE cause_kind = 'REWARD_CLAIM_MINT';

-- The only replay read granted to runtime. A foreign binding returns just a
-- conflict marker; it does not disclose the foreign character, item or ordinal.
-- Zero rows means no delivery. At most the source maximum + one is inspected.
CREATE FUNCTION game_character_inbox_reward_claim_mint_lookup(
    p_character_id UUID, p_world_id UUID, p_transaction_id UUID,
    p_binding BYTEA, p_game_session_id UUID, p_command_id NUMERIC, p_item_count SMALLINT
) RETURNS TABLE(disposition SMALLINT, item_instance_id UUID, ordinal NUMERIC)
LANGUAGE plpgsql SECURITY DEFINER STRICT AS $$
DECLARE
    reference TEXT;
    command_text TEXT;
BEGIN
    IF octet_length(p_binding) <> 32 OR p_item_count NOT BETWEEN 1 AND 20
       OR p_command_id NOT BETWEEN 1 AND 18446744073709551615
       OR p_command_id <> trunc(p_command_id) THEN
        RAISE EXCEPTION 'invalid reward Inbox plan' USING ERRCODE = '23514';
    END IF;
    command_text := p_command_id::NUMERIC(20,0)::TEXT;
    reference := p_game_session_id::TEXT || ':' || command_text || ':' ||
        p_transaction_id::TEXT || ':' || p_item_count::TEXT || ':' || encode(p_binding, 'hex');
    RETURN QUERY
        SELECT CASE WHEN d.cause_ref = reference AND d.character_id = p_character_id
                          AND d.world_id = p_world_id THEN 1 ELSE 2 END::SMALLINT,
               CASE WHEN d.cause_ref = reference AND d.character_id = p_character_id
                          AND d.world_id = p_world_id THEN d.item_instance_id ELSE NULL END,
               CASE WHEN d.cause_ref = reference AND d.character_id = p_character_id
                          AND d.world_id = p_world_id THEN d.ordinal ELSE NULL END
          FROM game_character_inbox_deliveries d
         WHERE d.cause_kind = 'REWARD_CLAIM_MINT'
           AND split_part(d.cause_ref, ':', 1) = p_game_session_id::TEXT
           AND split_part(d.cause_ref, ':', 2) = command_text
         LIMIT 21;
END;
$$;

-- No generic caller-selected cause or direct Inbox/table grant. This wrapper
-- can only place a live, unchanged fresh-MINT identity of this transaction.
-- It never removes an existing location; the existing exclusivity guard is
-- still mandatory. It takes the same CommandRef advisory key as the current
-- reward_claim_mint Rust writer, including the full unsigned u64 command ID.
CREATE FUNCTION game_character_inbox_reward_claim_mint_deliver(
    p_item_instance_id UUID, p_character_id UUID, p_world_id UUID,
    p_transaction_id UUID, p_binding BYTEA, p_game_session_id UUID,
    p_command_id NUMERIC, p_item_count SMALLINT
) RETURNS TABLE(character_id UUID, ordinal NUMERIC)
LANGUAGE plpgsql SECURITY DEFINER STRICT AS $$
DECLARE
    reference TEXT;
    command_text TEXT;
    command_bytes BYTEA;
    item game_item_instances%ROWTYPE;
    count_before INTEGER;
BEGIN
    IF octet_length(p_binding) <> 32 OR p_item_count NOT BETWEEN 1 AND 20
       OR p_command_id NOT BETWEEN 1 AND 18446744073709551615
       OR p_command_id <> trunc(p_command_id)
       OR NOT game_character_is_uuid_v7(p_transaction_id)
       OR NOT game_character_is_uuid_v7(p_game_session_id) THEN
        RAISE EXCEPTION 'invalid reward Inbox plan' USING ERRCODE = '23514';
    END IF;
    command_text := p_command_id::NUMERIC(20,0)::TEXT;
    command_bytes := int8send((CASE WHEN p_command_id > 9223372036854775807
        THEN p_command_id - 18446744073709551616 ELSE p_command_id END)::BIGINT);
    PERFORM pg_advisory_xact_lock(hashtextextended(
        'oteryn:reward-claim-mint:' || encode(uuid_send(p_game_session_id) || command_bytes, 'hex'), 0));
    reference := p_game_session_id::TEXT || ':' || command_text || ':' ||
        p_transaction_id::TEXT || ':' || p_item_count::TEXT || ':' || encode(p_binding, 'hex');
    IF EXISTS (
        SELECT 1 FROM game_character_inbox_deliveries d
         WHERE d.cause_kind = 'REWARD_CLAIM_MINT'
           AND split_part(d.cause_ref, ':', 1) = p_game_session_id::TEXT
           AND split_part(d.cause_ref, ':', 2) = command_text
           AND (d.cause_ref <> reference OR d.character_id <> p_character_id OR d.world_id <> p_world_id)
         LIMIT 1
    ) THEN
        RAISE EXCEPTION 'reward Inbox CommandRef already has another plan' USING ERRCODE = 'OTI05';
    END IF;
    SELECT count(*) INTO count_before FROM (
        SELECT 1 FROM game_character_inbox_deliveries d
         WHERE d.cause_kind = 'REWARD_CLAIM_MINT'
           AND split_part(d.cause_ref, ':', 1) = p_game_session_id::TEXT
           AND split_part(d.cause_ref, ':', 2) = command_text
         LIMIT 21
    ) AS bounded;
    IF count_before >= p_item_count THEN
        RAISE EXCEPTION 'reward Inbox plan is already delivered' USING ERRCODE = 'OTI05';
    END IF;
    -- Preserve the native Character root -> item -> Inbox counter lock order.
    PERFORM 1 FROM game_character_roots r
        WHERE r.character_id = p_character_id AND r.world_id = p_world_id FOR KEY SHARE;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'reward Inbox Character World mismatch' USING ERRCODE = 'OTI02';
    END IF;
    SELECT * INTO item FROM game_item_instances i
        WHERE i.item_instance_id = p_item_instance_id FOR NO KEY UPDATE;
    IF NOT FOUND OR item.world_id <> p_world_id THEN
        RAISE EXCEPTION 'reward Inbox item World mismatch' USING ERRCODE = 'OTI02';
    END IF;
    IF item.minted_transaction_id <> p_transaction_id OR item.lifecycle <> 1
       OR item.last_transaction_id IS NOT NULL THEN
        RETURN;
    END IF;
    RETURN QUERY SELECT * FROM game_character_inbox_deliver(
        p_item_instance_id, p_character_id, p_world_id, 'REWARD_CLAIM_MINT', reference);
END;
$$;

-- A direct call cannot commit a partial plan or a delivery borrowed from an
-- earlier mint/audit transaction. The source's existing MINT consistency guard
-- remains responsible for proving its full receipt and exact audit payload; this
-- trigger adds the Inbox side, without manufacturing a second value receipt.
CREATE FUNCTION game_character_inbox_reward_claim_mint_proven() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE
    source_transaction UUID;
    expected_count INTEGER;
    delivered_count INTEGER;
BEGIN
    IF NEW.cause_ref !~ '^[0-9a-f-]{36}:[0-9]{1,20}:[0-9a-f-]{36}:([1-9]|1[0-9]|20):[0-9a-f]{64}$' THEN
        RAISE EXCEPTION 'invalid reward Inbox delivery reference' USING ERRCODE = '23514';
    END IF;
    source_transaction := split_part(NEW.cause_ref, ':', 3)::UUID;
    expected_count := split_part(NEW.cause_ref, ':', 4)::INTEGER;
    SELECT count(*) INTO delivered_count FROM (
        SELECT 1 FROM game_character_inbox_deliveries d
         WHERE d.cause_kind = 'REWARD_CLAIM_MINT'
           AND split_part(d.cause_ref, ':', 1) = split_part(NEW.cause_ref, ':', 1)
           AND split_part(d.cause_ref, ':', 2) = split_part(NEW.cause_ref, ':', 2)
           AND d.cause_ref = NEW.cause_ref AND d.character_id = NEW.character_id
           AND d.world_id = NEW.world_id AND d.created_xact_id = pg_current_xact_id()
         LIMIT 21
    ) AS bounded;
    IF NEW.created_xact_id <> pg_current_xact_id() OR delivered_count <> expected_count
       OR NOT EXISTS (
           SELECT 1 FROM game_item_instances i
            WHERE i.item_instance_id = NEW.item_instance_id AND i.world_id = NEW.world_id
              AND i.minted_transaction_id = source_transaction AND i.lifecycle = 1
              AND i.last_transaction_id IS NULL)
       OR NOT EXISTS (
           SELECT 1 FROM game_item_character_inbox_locations l
            WHERE l.item_instance_id = NEW.item_instance_id AND l.world_id = NEW.world_id
              AND l.character_id = NEW.character_id AND l.ordinal = NEW.ordinal)
       OR NOT EXISTS (
           SELECT 1 FROM game_item_audit_outbox a
            WHERE a.transaction_id = source_transaction AND a.event_type_id = 2
              AND a.created_xact_id = pg_current_xact_id()
              AND a.publication_state = 1 AND a.published_at IS NULL) THEN
        RAISE EXCEPTION 'reward Inbox plan, fresh mint and audit must share one transaction'
            USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END;
$$;
CREATE CONSTRAINT TRIGGER game_character_inbox_reward_claim_mint_proven
    AFTER INSERT ON game_character_inbox_deliveries DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW WHEN (NEW.cause_kind = 'REWARD_CLAIM_MINT')
    EXECUTE FUNCTION game_character_inbox_reward_claim_mint_proven();

DO $$ BEGIN
    EXECUTE format('ALTER FUNCTION game_character_inbox_reward_claim_mint_lookup(UUID, UUID, UUID, BYTEA, UUID, NUMERIC, SMALLINT) SET search_path = pg_catalog, %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_character_inbox_reward_claim_mint_deliver(UUID, UUID, UUID, UUID, BYTEA, UUID, NUMERIC, SMALLINT) SET search_path = pg_catalog, %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_character_inbox_reward_claim_mint_proven() SET search_path = pg_catalog, %I, pg_temp', current_schema());
END $$;

REVOKE ALL ON FUNCTION
    game_character_inbox_reward_claim_mint_lookup(UUID, UUID, UUID, BYTEA, UUID, NUMERIC, SMALLINT),
    game_character_inbox_reward_claim_mint_deliver(UUID, UUID, UUID, UUID, BYTEA, UUID, NUMERIC, SMALLINT),
    game_character_inbox_reward_claim_mint_proven()
FROM PUBLIC;
GRANT EXECUTE ON FUNCTION
    game_character_inbox_reward_claim_mint_lookup(UUID, UUID, UUID, BYTEA, UUID, NUMERIC, SMALLINT),
    game_character_inbox_reward_claim_mint_deliver(UUID, UUID, UUID, UUID, BYTEA, UUID, NUMERIC, SMALLINT)
TO oteryn_game_runtime;
-- game_character_inbox_deliver and every Inbox table keep their prior grants.

-- D1356: fresh-MINT reservation, receipt and the complete guard union.
CREATE TABLE game_reward_claim_inbox_mint_reservations (
    game_session_id UUID NOT NULL CHECK (game_character_is_uuid_v7(game_session_id)),
    command_id NUMERIC(20,0) NOT NULL CHECK (command_id BETWEEN 1 AND 18446744073709551615),
    character_id UUID NOT NULL CHECK (game_character_is_uuid_v7(character_id)),
    world_id UUID NOT NULL CHECK (game_character_is_uuid_v7(world_id)),
    channel_id UUID NOT NULL CHECK (game_character_is_uuid_v7(channel_id)),
    transaction_id UUID NOT NULL UNIQUE CHECK (game_character_is_uuid_v7(transaction_id)),
    source_binding BYTEA NOT NULL CHECK (octet_length(source_binding) = 32),
    plan_binding BYTEA NOT NULL CHECK (octet_length(plan_binding) = 32),
    item_instance_ids UUID[] NOT NULL CHECK (cardinality(item_instance_ids) BETWEEN 1 AND 20),
    mint_spec BYTEA NOT NULL CHECK (octet_length(mint_spec) BETWEEN 1 AND 65536),
    mint_intent_sha256 BYTEA NOT NULL CHECK (mint_intent_sha256 = sha256(mint_spec)),
    event_id UUID NOT NULL UNIQUE CHECK (game_character_is_uuid_v7(event_id)),
    occurred_at BIGINT NOT NULL CHECK (occurred_at BETWEEN 1 AND 9223372029078775807),
    work_units_used SMALLINT NOT NULL CHECK (work_units_used BETWEEN 0 AND 3),
    PRIMARY KEY (game_session_id, command_id)
);

CREATE TABLE game_reward_claim_inbox_mint_receipts (
    game_session_id UUID NOT NULL,
    command_id NUMERIC(20,0) NOT NULL,
    transaction_id UUID NOT NULL UNIQUE,
    event_id UUID NOT NULL UNIQUE,
    character_id UUID NOT NULL,
    world_id UUID NOT NULL,
    channel_id UUID NOT NULL,
    expected_session_generation NUMERIC(20,0) NOT NULL
        CHECK (expected_session_generation BETWEEN 1 AND 18446744073709551615),
    expected_character_lease_generation NUMERIC(20,0) NOT NULL
        CHECK (expected_character_lease_generation BETWEEN 1 AND 18446744073709551615),
    source_binding BYTEA NOT NULL CHECK (octet_length(source_binding) = 32),
    plan_binding BYTEA NOT NULL CHECK (octet_length(plan_binding) = 32),
    mint_intent_sha256 BYTEA NOT NULL CHECK (octet_length(mint_intent_sha256) = 32),
    item_count SMALLINT NOT NULL CHECK (item_count BETWEEN 1 AND 20),
    occurred_at BIGINT NOT NULL,
    envelope_sha256 BYTEA NOT NULL CHECK (octet_length(envelope_sha256) = 32),
    created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id(),
    PRIMARY KEY (game_session_id, command_id),
    FOREIGN KEY (game_session_id, command_id)
        REFERENCES game_reward_claim_inbox_mint_reservations ON DELETE RESTRICT
);

CREATE TABLE game_reward_claim_inbox_mint_lines (
    transaction_id UUID NOT NULL REFERENCES game_reward_claim_inbox_mint_receipts(transaction_id),
    output_index SMALLINT NOT NULL CHECK (output_index BETWEEN 1 AND 20),
    item_instance_id UUID NOT NULL UNIQUE REFERENCES game_item_instances(item_instance_id),
    source_line BIGINT NOT NULL CHECK (source_line BETWEEN 0 AND 4294967295),
    definition_family TEXT NOT NULL,
    definition_production_key TEXT NOT NULL,
    definition_revision_ref TEXT NOT NULL,
    quantity BIGINT NOT NULL CHECK (quantity BETWEEN 1 AND 100),
    key_number INTEGER NOT NULL CHECK (key_number BETWEEN 0 AND 65535),
    ordinal NUMERIC(20,0) NOT NULL CHECK (ordinal BETWEEN 1 AND 18446744073709551615),
    created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id(),
    PRIMARY KEY (transaction_id, output_index)
);

CREATE FUNCTION game_reward_claim_inbox_mint_lock(p_session UUID, p_command NUMERIC)
RETURNS VOID LANGUAGE plpgsql STRICT AS $$
DECLARE command_bytes BYTEA;
BEGIN
    IF NOT game_character_is_uuid_v7(p_session)
       OR p_command NOT BETWEEN 1 AND 18446744073709551615
       OR p_command <> trunc(p_command) THEN
        RAISE EXCEPTION 'invalid reward-claim CommandRef' USING ERRCODE = '23514';
    END IF;
    command_bytes := int8send((CASE WHEN p_command > 9223372036854775807
        THEN p_command - 18446744073709551616 ELSE p_command END)::BIGINT);
    PERFORM pg_advisory_xact_lock(hashtextextended(
        'oteryn:reward-claim-mint:' || encode(uuid_send(p_session) || command_bytes, 'hex'), 0));
END;
$$;

-- Both destinations share the existing full-u64 cause key. An old CHEST caller
-- cannot reserve an already-owned Inbox command, including a never-committed
-- preparation. The old reservation and receipt representation stays unchanged.
CREATE FUNCTION game_reward_claim_legacy_inbox_exclusion() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
    PERFORM game_reward_claim_inbox_mint_lock(NEW.game_session_id, NEW.command_id);
    IF EXISTS (SELECT 1 FROM game_reward_claim_inbox_mint_reservations v
                WHERE (v.game_session_id, v.command_id) = (NEW.game_session_id, NEW.command_id)) THEN
        RAISE EXCEPTION 'reward-claim CommandRef is reserved for Inbox' USING ERRCODE = 'OTI05';
    END IF;
    RETURN NEW;
END;
$$;
CREATE TRIGGER game_reward_claim_legacy_inbox_exclusion
    BEFORE INSERT ON game_reward_claim_mint_reservations
    FOR EACH ROW EXECUTE FUNCTION game_reward_claim_legacy_inbox_exclusion();

CREATE FUNCTION game_reward_claim_inbox_mint_reserve(
    p_session UUID, p_command NUMERIC, p_character UUID, p_world UUID, p_channel UUID,
    p_transaction UUID, p_source_binding BYTEA, p_plan_binding BYTEA,
    p_items UUID[], p_spec BYTEA
) RETURNS TABLE(event_id UUID, occurred_at BIGINT, work_units_used SMALLINT)
LANGUAGE plpgsql SECURITY DEFINER STRICT AS $$
DECLARE
    spec JSONB;
    output JSONB;
    text_value TEXT;
    field TEXT;
    item UUID;
    item_bytes BYTEA := ''::BYTEA;
    command_bytes BYTEA;
    source_digest BYTEA;
    expected_binding BYTEA;
    expected_maximum NUMERIC;
    existing game_reward_claim_inbox_mint_reservations%ROWTYPE;
    index INTEGER;
BEGIN
    PERFORM game_reward_claim_inbox_mint_lock(p_session, p_command);
    IF NOT game_character_is_uuid_v7(p_character) OR NOT game_character_is_uuid_v7(p_world)
       OR NOT game_character_is_uuid_v7(p_channel) OR NOT game_character_is_uuid_v7(p_transaction)
       OR octet_length(p_source_binding) <> 32 OR octet_length(p_plan_binding) <> 32
       OR cardinality(p_items) NOT BETWEEN 1 AND 20 OR array_ndims(p_items) IS DISTINCT FROM 1
       OR array_lower(p_items, 1) IS DISTINCT FROM 1
       OR octet_length(p_spec) NOT BETWEEN 1 AND 65536 THEN
        RAISE EXCEPTION 'invalid bounded Inbox mint reservation' USING ERRCODE = '23514';
    END IF;
    FOREACH item IN ARRAY p_items LOOP
        IF item IS NULL OR NOT game_character_is_uuid_v7(item) THEN
            RAISE EXCEPTION 'invalid frozen Inbox item identity' USING ERRCODE = '23514';
        END IF;
        item_bytes := item_bytes || uuid_send(item);
    END LOOP;
    IF (SELECT count(DISTINCT value) FROM unnest(p_items) AS u(value)) <> cardinality(p_items) THEN
        RAISE EXCEPTION 'duplicate frozen Inbox item identity' USING ERRCODE = '23514';
    END IF;
    spec := convert_from(p_spec, 'UTF8')::JSONB;
    IF jsonb_typeof(spec) IS DISTINCT FROM 'object'
       OR (spec - ARRAY['version','claim','content_revision','ruleset_revision','sim_revision','outputs']) <> '{}'::JSONB
       OR spec->'version' IS DISTINCT FROM '1'::JSONB
       OR jsonb_typeof(spec->'claim') IS DISTINCT FROM 'object'
       OR ((spec->'claim') - ARRAY['family','key','revision']) <> '{}'::JSONB
       OR jsonb_typeof(spec->'outputs') IS DISTINCT FROM 'array'
       OR jsonb_array_length(spec->'outputs') <> cardinality(p_items) THEN
        RAISE EXCEPTION 'invalid Inbox mint spec shape' USING ERRCODE = '23514';
    END IF;
    FOREACH field IN ARRAY ARRAY['family','key','revision'] LOOP
        text_value := spec->'claim'->>field;
        IF jsonb_typeof(spec->'claim'->field) IS DISTINCT FROM 'string'
           OR octet_length(text_value) NOT BETWEEN 1 AND (CASE WHEN field = 'family' THEN 128 ELSE 512 END)
           OR text_value ~ '[[:cntrl:]]' THEN
            RAISE EXCEPTION 'invalid Inbox reward claim definition' USING ERRCODE = '23514';
        END IF;
    END LOOP;
    FOREACH field IN ARRAY ARRAY['content_revision','ruleset_revision','sim_revision'] LOOP
        text_value := spec->>field;
        IF jsonb_typeof(spec->field) IS DISTINCT FROM 'string'
           OR octet_length(text_value) NOT BETWEEN 1 AND 512 OR text_value ~ '[[:cntrl:]]' THEN
            RAISE EXCEPTION 'invalid Inbox mint interpretation' USING ERRCODE = '23514';
        END IF;
    END LOOP;
    FOR index IN 0..(cardinality(p_items)-1) LOOP
        output := spec->'outputs'->index;
        IF jsonb_typeof(output) IS DISTINCT FROM 'object'
           OR (output - ARRAY['source_line','family','key','revision','quantity','stack_kind',
                'stack_maximum','proven_stack_maximum','container_capacity',
                'container_slot_equip_pattern','key_number']) <> '{}'::JSONB
           OR output->'container_capacity' IS DISTINCT FROM 'null'::JSONB
           OR output->'container_slot_equip_pattern' IS DISTINCT FROM 'false'::JSONB THEN
            RAISE EXCEPTION 'unsupported Inbox mint output shape' USING ERRCODE = '23514';
        END IF;
        FOREACH field IN ARRAY ARRAY['family','key','revision'] LOOP
            text_value := output->>field;
            IF jsonb_typeof(output->field) IS DISTINCT FROM 'string'
               OR octet_length(text_value) NOT BETWEEN 1 AND (CASE WHEN field = 'family' THEN 128 ELSE 512 END)
               OR text_value ~ '[[:cntrl:]]' THEN
                RAISE EXCEPTION 'invalid Inbox item definition' USING ERRCODE = '23514';
            END IF;
        END LOOP;
        FOREACH field IN ARRAY ARRAY['source_line','quantity','stack_kind','stack_maximum'] LOOP
            IF jsonb_typeof(output->field) IS DISTINCT FROM 'number'
               OR (output->>field) !~ '^(0|[1-9][0-9]*)$' THEN
                RAISE EXCEPTION 'invalid Inbox output number' USING ERRCODE = '23514';
            END IF;
        END LOOP;
        IF (output->>'source_line')::NUMERIC NOT BETWEEN 0 AND 4294967295
           OR output->>'stack_kind' NOT IN ('0','1')
           OR (output->>'quantity')::NUMERIC NOT BETWEEN 1 AND 100 THEN
            RAISE EXCEPTION 'Inbox source line, class or quantity exceeds its bound' USING ERRCODE = '23514';
        END IF;
        IF output->'proven_stack_maximum' IS DISTINCT FROM 'null'::JSONB
           AND (jsonb_typeof(output->'proven_stack_maximum') IS DISTINCT FROM 'number'
                OR (output->>'proven_stack_maximum') !~ '^[1-9][0-9]*$'
                OR (output->>'proven_stack_maximum')::NUMERIC NOT BETWEEN 1 AND 100) THEN
            RAISE EXCEPTION 'invalid proven Inbox stack maximum' USING ERRCODE = '23514';
        END IF;
        expected_maximum := CASE WHEN output->>'stack_kind' = '0' THEN 1
            ELSE coalesce((output->>'proven_stack_maximum')::NUMERIC, 100) END;
        IF (output->>'stack_maximum')::NUMERIC <> expected_maximum
           OR (output->>'quantity')::NUMERIC > expected_maximum
           OR (output->>'stack_kind' = '0' AND output->'proven_stack_maximum' IS DISTINCT FROM 'null'::JSONB) THEN
            RAISE EXCEPTION 'Inbox quantity disagrees with its definition stack bound' USING ERRCODE = '23514';
        END IF;
        IF output->'key_number' IS DISTINCT FROM 'null'::JSONB
           AND (jsonb_typeof(output->'key_number') IS DISTINCT FROM 'number'
                OR (output->>'key_number') !~ '^[1-9][0-9]*$'
                OR (output->>'key_number')::NUMERIC NOT BETWEEN 1 AND 65535
                OR output->>'stack_kind' <> '0' OR output->>'quantity' <> '1') THEN
            RAISE EXCEPTION 'Inbox key must be one nonstackable item' USING ERRCODE = '23514';
        END IF;
    END LOOP;
    command_bytes := int8send((CASE WHEN p_command > 9223372036854775807
        THEN p_command - 18446744073709551616 ELSE p_command END)::BIGINT);
    source_digest := sha256(convert_to('oteryn:reward_claim_mint:inbox-source:v1','UTF8') ||
        decode('00','hex') || p_source_binding || sha256(p_spec));
    expected_binding := sha256(convert_to('oteryn:reward_claim_mint:inbox-plan:v1','UTF8') ||
        decode('00','hex') || uuid_send(p_session) || command_bytes || uuid_send(p_character) ||
        uuid_send(p_world) || uuid_send(p_channel) || uuid_send(p_transaction) || source_digest ||
        int4send(cardinality(p_items)) || item_bytes);
    IF p_plan_binding <> expected_binding THEN
        RAISE EXCEPTION 'Inbox plan does not bind its complete frozen mint spec' USING ERRCODE = '23514';
    END IF;
    IF EXISTS (SELECT 1 FROM game_reward_claim_mint_reservations v
                WHERE (v.game_session_id, v.command_id) = (p_session, p_command)) THEN
        RAISE EXCEPTION 'reward-claim CommandRef is reserved for the legacy destination' USING ERRCODE = 'OTI05';
    END IF;
    SELECT * INTO existing FROM game_reward_claim_inbox_mint_reservations v
     WHERE (v.game_session_id, v.command_id) = (p_session, p_command) FOR UPDATE;
    IF FOUND THEN
        IF (existing.character_id, existing.world_id, existing.channel_id, existing.transaction_id,
            existing.source_binding, existing.plan_binding, existing.item_instance_ids, existing.mint_spec)
           IS DISTINCT FROM (p_character, p_world, p_channel, p_transaction,
            p_source_binding, p_plan_binding, p_items, p_spec) THEN
            RAISE EXCEPTION 'reward Inbox CommandRef has another frozen mint' USING ERRCODE = 'OTI05';
        END IF;
    ELSE
        INSERT INTO game_reward_claim_inbox_mint_reservations
            (game_session_id, command_id, character_id, world_id, channel_id, transaction_id,
             source_binding, plan_binding, item_instance_ids, mint_spec, mint_intent_sha256,
             event_id, occurred_at, work_units_used)
        VALUES (p_session, p_command, p_character, p_world, p_channel, p_transaction,
                p_source_binding, p_plan_binding, p_items, p_spec, sha256(p_spec),
                game_character_uuid_v7(), floor(extract(epoch FROM clock_timestamp()) * 1000)::BIGINT, 0)
        RETURNING * INTO existing;
    END IF;
    RETURN QUERY SELECT existing.event_id, existing.occurred_at, existing.work_units_used;
END;
$$;

CREATE FUNCTION game_reward_claim_inbox_mint_charge(
    p_session UUID, p_command NUMERIC, p_transaction UUID, p_binding BYTEA
) RETURNS TABLE(work_units_used SMALLINT)
LANGUAGE plpgsql SECURITY DEFINER STRICT AS $$
DECLARE v game_reward_claim_inbox_mint_reservations%ROWTYPE;
BEGIN
    PERFORM game_reward_claim_inbox_mint_lock(p_session, p_command);
    SELECT * INTO v FROM game_reward_claim_inbox_mint_reservations r
     WHERE (r.game_session_id, r.command_id) = (p_session, p_command) FOR UPDATE;
    IF NOT FOUND OR v.transaction_id <> p_transaction OR v.plan_binding <> p_binding THEN
        RAISE EXCEPTION 'unreserved or changed Inbox mint charge' USING ERRCODE = 'OTI05';
    END IF;
    IF EXISTS (SELECT 1 FROM game_reward_claim_inbox_mint_receipts r
                WHERE r.transaction_id = p_transaction) THEN
        RETURN QUERY SELECT v.work_units_used;
        RETURN;
    END IF;
    IF v.work_units_used >= 3 THEN RETURN; END IF;
    UPDATE game_reward_claim_inbox_mint_reservations r SET work_units_used = r.work_units_used + 1
     WHERE (r.game_session_id, r.command_id) = (p_session, p_command)
     RETURNING r.work_units_used INTO v.work_units_used;
    RETURN QUERY SELECT v.work_units_used;
END;
$$;

CREATE FUNCTION game_reward_claim_inbox_mint_begin(
    p_session UUID, p_command NUMERIC, p_transaction UUID, p_binding BYTEA
) RETURNS VOID LANGUAGE plpgsql SECURITY DEFINER STRICT AS $$
DECLARE
    v game_reward_claim_inbox_mint_reservations%ROWTYPE;
    spec JSONB;
    output JSONB;
    index INTEGER;
BEGIN
    PERFORM game_reward_claim_inbox_mint_lock(p_session, p_command);
    SELECT * INTO v FROM game_reward_claim_inbox_mint_reservations r
     WHERE (r.game_session_id, r.command_id) = (p_session, p_command) FOR UPDATE;
    IF NOT FOUND OR v.transaction_id <> p_transaction OR v.plan_binding <> p_binding
       OR v.work_units_used = 0 THEN
        RAISE EXCEPTION 'fresh Inbox mint has no charged frozen reservation' USING ERRCODE = 'OTI05';
    END IF;
    IF EXISTS (SELECT 1 FROM game_reward_claim_inbox_mint_receipts r WHERE r.transaction_id = p_transaction)
       OR EXISTS (SELECT 1 FROM game_item_instances i WHERE i.item_instance_id = ANY(v.item_instance_ids)) THEN
        RAISE EXCEPTION 'Inbox mint identity is no longer fresh' USING ERRCODE = 'OTI05';
    END IF;
    spec := convert_from(v.mint_spec, 'UTF8')::JSONB;
    FOR index IN 1..cardinality(v.item_instance_ids) LOOP
        output := spec->'outputs'->(index-1);
        INSERT INTO game_item_instances
            (item_instance_id, world_id, definition_family, definition_production_key,
             definition_revision_ref, quantity, lifecycle, minted_transaction_id)
        VALUES (v.item_instance_ids[index], v.world_id, output->>'family', output->>'key',
                output->>'revision', (output->>'quantity')::BIGINT, 1, v.transaction_id);
        IF output->'key_number' <> 'null'::JSONB THEN
            INSERT INTO game_item_keys(item_instance_id, key_number, minted_transaction_id)
            VALUES (v.item_instance_ids[index], (output->>'key_number')::INTEGER, v.transaction_id);
        END IF;
    END LOOP;
END;
$$;

CREATE FUNCTION game_reward_claim_inbox_mint_finish(
    p_session UUID, p_command NUMERIC, p_transaction UUID, p_binding BYTEA,
    p_generation NUMERIC, p_lease_generation NUMERIC, p_envelope_sha256 BYTEA
) RETURNS VOID LANGUAGE plpgsql SECURITY DEFINER STRICT AS $$
DECLARE
    v game_reward_claim_inbox_mint_reservations%ROWTYPE;
    spec JSONB;
    output JSONB;
    reference TEXT;
    position NUMERIC;
    index INTEGER;
BEGIN
    PERFORM game_reward_claim_inbox_mint_lock(p_session, p_command);
    SELECT * INTO v FROM game_reward_claim_inbox_mint_reservations r
     WHERE (r.game_session_id, r.command_id) = (p_session, p_command) FOR UPDATE;
    IF NOT FOUND OR v.transaction_id <> p_transaction OR v.plan_binding <> p_binding
       OR v.work_units_used = 0 OR p_generation NOT BETWEEN 1 AND 18446744073709551615
       OR p_generation <> trunc(p_generation)
       OR p_lease_generation NOT BETWEEN 1 AND 18446744073709551615
       OR p_lease_generation <> trunc(p_lease_generation) OR octet_length(p_envelope_sha256) <> 32 THEN
        RAISE EXCEPTION 'invalid frozen Inbox mint completion' USING ERRCODE = '23514';
    END IF;
    reference := p_session::TEXT || ':' || p_command::NUMERIC(20,0)::TEXT || ':' ||
        p_transaction::TEXT || ':' || cardinality(v.item_instance_ids)::TEXT || ':' || encode(p_binding,'hex');
    spec := convert_from(v.mint_spec, 'UTF8')::JSONB;
    INSERT INTO game_reward_claim_inbox_mint_receipts
        (game_session_id, command_id, transaction_id, event_id, character_id, world_id, channel_id,
         expected_session_generation, expected_character_lease_generation, source_binding, plan_binding, mint_intent_sha256,
         item_count, occurred_at, envelope_sha256)
    VALUES (p_session, p_command, p_transaction, v.event_id, v.character_id, v.world_id, v.channel_id,
            p_generation, p_lease_generation, v.source_binding, v.plan_binding, v.mint_intent_sha256,
            cardinality(v.item_instance_ids), v.occurred_at, p_envelope_sha256);
    FOR index IN 1..cardinality(v.item_instance_ids) LOOP
        output := spec->'outputs'->(index-1);
        SELECT d.ordinal INTO position FROM game_character_inbox_deliveries d
         WHERE d.item_instance_id = v.item_instance_ids[index]
           AND d.character_id = v.character_id AND d.world_id = v.world_id
           AND d.cause_kind = 'REWARD_CLAIM_MINT' AND d.cause_ref = reference
           AND d.created_xact_id = pg_current_xact_id();
        IF NOT FOUND THEN
            RAISE EXCEPTION 'Inbox mint completion lacks its exact same-transaction delivery' USING ERRCODE = '23514';
        END IF;
        INSERT INTO game_reward_claim_inbox_mint_lines
            (transaction_id, output_index, item_instance_id, source_line, definition_family,
             definition_production_key, definition_revision_ref, quantity, key_number, ordinal)
        VALUES (p_transaction, index, v.item_instance_ids[index], (output->>'source_line')::BIGINT,
                output->>'family', output->>'key', output->>'revision', (output->>'quantity')::BIGINT,
                coalesce((output->>'key_number')::INTEGER, 0), position);
    END LOOP;
END;
$$;

-- Definer-only receipt proof joins every actual fresh instance, position,
-- journal line and pending native audit. No caller-supplied boolean is evidence.
CREATE FUNCTION game_reward_claim_inbox_mint_is_complete(p_transaction UUID)
RETURNS BOOLEAN LANGUAGE plpgsql SECURITY DEFINER STRICT AS $$
DECLARE
    r game_reward_claim_inbox_mint_receipts%ROWTYPE;
    v game_reward_claim_inbox_mint_reservations%ROWTYPE;
    spec JSONB;
    output JSONB;
    reference TEXT;
    index INTEGER;
BEGIN
    SELECT * INTO r FROM game_reward_claim_inbox_mint_receipts t WHERE t.transaction_id = p_transaction;
    IF NOT FOUND OR r.created_xact_id <> pg_current_xact_id() THEN RETURN FALSE; END IF;
    SELECT * INTO v FROM game_reward_claim_inbox_mint_reservations t
     WHERE (t.game_session_id, t.command_id) = (r.game_session_id, r.command_id);
    IF NOT FOUND OR v.work_units_used = 0 OR
       (r.transaction_id, r.event_id, r.character_id, r.world_id, r.channel_id, r.source_binding,
        r.plan_binding, r.mint_intent_sha256, r.item_count, r.occurred_at)
        IS DISTINCT FROM (v.transaction_id, v.event_id, v.character_id, v.world_id, v.channel_id,
        v.source_binding, v.plan_binding, v.mint_intent_sha256, cardinality(v.item_instance_ids), v.occurred_at)
       OR (SELECT count(*) FROM game_reward_claim_inbox_mint_lines l WHERE l.transaction_id = r.transaction_id) <> r.item_count
       OR NOT EXISTS (SELECT 1 FROM game_character_roots cr
                       WHERE cr.character_id = r.character_id AND cr.world_id = r.world_id)
       OR NOT EXISTS (SELECT 1 FROM game_item_audit_outbox a
                       WHERE a.event_id = r.event_id AND a.transaction_id = r.transaction_id
                         AND a.item_instance_id = v.item_instance_ids[1]
                         AND a.event_type_id = 2 AND a.transaction_ordinal = 1 AND a.transaction_count = 1
                         AND a.occurred_at = r.occurred_at AND a.expires_at = r.occurred_at + 7776000000
                         AND a.envelope_sha256 = r.envelope_sha256
                         AND octet_length(a.envelope) BETWEEN 1 AND 34816
                         AND a.created_xact_id = pg_current_xact_id()
                         AND a.publication_state = 1 AND a.published_at IS NULL) THEN
        RETURN FALSE;
    END IF;
    spec := convert_from(v.mint_spec, 'UTF8')::JSONB;
    reference := r.game_session_id::TEXT || ':' || r.command_id::TEXT || ':' || r.transaction_id::TEXT ||
        ':' || r.item_count::TEXT || ':' || encode(r.plan_binding,'hex');
    FOR index IN 1..r.item_count LOOP
        output := spec->'outputs'->(index-1);
        IF NOT EXISTS (
            SELECT 1 FROM game_reward_claim_inbox_mint_lines l
            JOIN game_item_instances i ON i.item_instance_id = l.item_instance_id
            JOIN game_item_character_inbox_locations p ON p.item_instance_id = l.item_instance_id
            JOIN game_character_inbox_deliveries d ON d.item_instance_id = l.item_instance_id
            LEFT JOIN game_item_keys k ON k.item_instance_id = l.item_instance_id
             WHERE l.transaction_id = r.transaction_id AND l.output_index = index
               AND l.item_instance_id = v.item_instance_ids[index]
               AND l.source_line = (output->>'source_line')::BIGINT
               AND (l.definition_family,l.definition_production_key,l.definition_revision_ref,l.quantity,l.key_number)
                   = (output->>'family',output->>'key',output->>'revision',(output->>'quantity')::BIGINT,
                      coalesce((output->>'key_number')::INTEGER,0))
               AND l.created_xact_id = pg_current_xact_id()
               AND i.world_id = r.world_id AND i.minted_transaction_id = r.transaction_id
               AND i.lifecycle = 1 AND i.last_transaction_id IS NULL
               AND (i.definition_family,i.definition_production_key,i.definition_revision_ref,i.quantity)
                   = (l.definition_family,l.definition_production_key,l.definition_revision_ref,l.quantity)
               AND coalesce(k.key_number,0) = l.key_number
               AND (l.key_number = 0 OR (k.minted_transaction_id = r.transaction_id
                    AND k.created_xact_id = pg_current_xact_id() AND i.quantity = 1))
               AND (p.character_id,p.world_id,p.ordinal) = (r.character_id,r.world_id,l.ordinal)
               AND (d.character_id,d.world_id,d.ordinal) = (r.character_id,r.world_id,l.ordinal)
               AND d.cause_kind = 'REWARD_CLAIM_MINT' AND d.cause_ref = reference
               AND d.created_xact_id = pg_current_xact_id()
        ) THEN RETURN FALSE; END IF;
        IF index > 1 AND NOT EXISTS (
            SELECT 1 FROM game_reward_claim_inbox_mint_lines previous_line
            JOIN game_reward_claim_inbox_mint_lines current_line ON current_line.transaction_id = previous_line.transaction_id
             WHERE current_line.transaction_id = r.transaction_id AND current_line.output_index = index
               AND previous_line.output_index = index-1 AND previous_line.ordinal < current_line.ordinal
        ) THEN RETURN FALSE; END IF;
    END LOOP;
    RETURN TRUE;
END;
$$;

CREATE FUNCTION game_reward_claim_inbox_item_proven(p_item UUID, p_transaction UUID)
RETURNS BOOLEAN LANGUAGE sql SECURITY DEFINER STRICT AS $$
    SELECT EXISTS (SELECT 1 FROM game_reward_claim_inbox_mint_lines l
                    WHERE l.transaction_id = p_transaction AND l.item_instance_id = p_item)
       AND game_reward_claim_inbox_mint_is_complete(p_transaction)
$$;

CREATE FUNCTION game_reward_claim_inbox_mint_proven() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
    IF NEW.created_xact_id <> pg_current_xact_id()
       OR NOT game_reward_claim_inbox_mint_is_complete(NEW.transaction_id) THEN
        RAISE EXCEPTION 'fresh reward Inbox mint, full plan, immutable receipt and native audit must commit together'
            USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END;
$$;

CREATE FUNCTION game_reward_claim_inbox_delivery_receipt_proven() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
    IF NEW.created_xact_id <> pg_current_xact_id()
       OR NOT game_reward_claim_inbox_item_proven(NEW.item_instance_id, split_part(NEW.cause_ref,':',3)::UUID) THEN
        RAISE EXCEPTION 'reward Inbox delivery lacks its actual same-transaction mint receipt'
            USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END;
$$;

CREATE TRIGGER game_reward_claim_inbox_reservation_immutable
    BEFORE UPDATE OR DELETE ON game_reward_claim_inbox_mint_reservations
    FOR EACH ROW EXECUTE FUNCTION game_reward_claim_mint_reservation_guard();
CREATE TRIGGER game_reward_claim_inbox_reservation_no_truncate
    BEFORE TRUNCATE ON game_reward_claim_inbox_mint_reservations
    FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();
CREATE TRIGGER game_reward_claim_inbox_receipt_stamp
    BEFORE INSERT ON game_reward_claim_inbox_mint_receipts
    FOR EACH ROW EXECUTE FUNCTION game_item_stamp_created_xact_id();
CREATE TRIGGER game_reward_claim_inbox_line_stamp
    BEFORE INSERT ON game_reward_claim_inbox_mint_lines
    FOR EACH ROW EXECUTE FUNCTION game_item_stamp_created_xact_id();
CREATE TRIGGER game_reward_claim_inbox_receipt_immutable
    BEFORE UPDATE OR DELETE ON game_reward_claim_inbox_mint_receipts
    FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_reward_claim_inbox_line_immutable
    BEFORE UPDATE OR DELETE ON game_reward_claim_inbox_mint_lines
    FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_reward_claim_inbox_receipt_no_truncate
    BEFORE TRUNCATE ON game_reward_claim_inbox_mint_receipts
    FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();
CREATE TRIGGER game_reward_claim_inbox_line_no_truncate
    BEFORE TRUNCATE ON game_reward_claim_inbox_mint_lines
    FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();
CREATE CONSTRAINT TRIGGER game_reward_claim_inbox_receipt_proven
    AFTER INSERT ON game_reward_claim_inbox_mint_receipts DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW EXECUTE FUNCTION game_reward_claim_inbox_mint_proven();
CREATE CONSTRAINT TRIGGER game_reward_claim_inbox_line_proven
    AFTER INSERT ON game_reward_claim_inbox_mint_lines DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW EXECUTE FUNCTION game_reward_claim_inbox_mint_proven();
CREATE CONSTRAINT TRIGGER game_reward_claim_inbox_delivery_receipt_proven
    AFTER INSERT ON game_character_inbox_deliveries DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW WHEN (NEW.cause_kind = 'REWARD_CLAIM_MINT')
    EXECUTE FUNCTION game_reward_claim_inbox_delivery_receipt_proven();

-- Exact shared guard amendments are appended below from their authoritative
-- main definitions. No prior branch is deleted or made unconditional.

CREATE OR REPLACE FUNCTION game_item_mint_consistency_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    -- MAIL Inbox: complete native receipt and actual locations/audit in this xact.
    IF game_reward_claim_inbox_item_proven(NEW.item_instance_id, NEW.minted_transaction_id) THEN
        RETURN NULL;
    END IF;
    -- 0082: an NPC trade MINT line of this same physical transaction, placed in its new entry.
    IF EXISTS(SELECT 1 FROM game_npc_trade_lines l
         JOIN game_npc_trades t ON t.transaction_id = l.transaction_id
         JOIN game_item_container_entries e ON e.item_instance_id = l.item_instance_id
        WHERE l.item_instance_id = NEW.item_instance_id
          AND l.transaction_id = NEW.minted_transaction_id
          AND l.line_kind IN (2, 4)
          AND e.placed_transaction_id = l.transaction_id
          AND l.created_xact_id = pg_current_xact_id()
          AND t.created_xact_id = pg_current_xact_id()) THEN RETURN NULL; END IF;

    -- 0083: the fresh flask item of an item-use record of this same physical transaction.
    IF EXISTS (SELECT 1 FROM game_item_use_records u
                 JOIN game_item_container_entries e ON e.item_instance_id = NEW.item_instance_id
                WHERE u.shape = 4 AND u.flask_item_instance_id = NEW.item_instance_id
                  AND u.transaction_id = NEW.minted_transaction_id
                  AND u.created_xact_id = pg_current_xact_id()
                  AND e.placed_transaction_id = u.transaction_id
                  AND NEW.quantity = 1
                  AND (u.flask_definition_family, u.flask_definition_production_key,
                       u.flask_definition_revision_ref)
                    = (NEW.definition_family, NEW.definition_production_key,
                       NEW.definition_revision_ref)) THEN
        RETURN NULL;
    END IF;


    IF EXISTS(SELECT 1 FROM game_native_map_item_receipts r JOIN game_item_ground_locations g USING(item_instance_id) JOIN game_native_map_item_audit a USING(item_instance_id)
      WHERE r.item_instance_id=NEW.item_instance_id AND r.transaction_id=NEW.minted_transaction_id
      AND r.world_id=NEW.world_id AND r.definition_family=NEW.definition_family AND r.definition_key=NEW.definition_production_key
      AND r.definition_revision=NEW.definition_revision_ref AND r.quantity=NEW.quantity
      AND g.world_id=r.world_id AND g.channel_id=r.channel_id AND g.runtime_scope_ownership_generation=r.scope_generation
      AND g.spatial_position=r.spatial_position AND g.map_revision=r.map_revision AND g.content_revision=r.content_revision
      AND g.native_room_placement_context=r.placement_context AND a.event_id=r.event_id AND a.transaction_id=r.transaction_id
      AND a.envelope=r.source_intent AND a.created_xact_id=pg_current_xact_id() AND r.created_xact_id=pg_current_xact_id()) THEN RETURN NULL; END IF;
    IF EXISTS(SELECT 1 FROM game_spell_item_lines l JOIN game_spell_item_receipts r USING(transaction_id)
       WHERE l.item_instance_id=NEW.item_instance_id AND l.operation_kind=1
         AND l.transaction_id=NEW.minted_transaction_id AND r.created_xact_id=pg_current_xact_id()
         AND l.created_xact_id=pg_current_xact_id()) THEN RETURN NULL; END IF;
    IF EXISTS (
        SELECT 1
          FROM game_item_mint_receipts r
          JOIN game_item_ground_locations g ON g.item_instance_id = r.item_instance_id
          JOIN game_item_audit_outbox a ON a.event_id = r.event_id
         WHERE r.item_instance_id = NEW.item_instance_id
           AND r.transaction_id = NEW.minted_transaction_id
           AND r.death_world_id = NEW.world_id
           AND r.destination_parent_item_instance_id IS NULL
           AND g.world_id = NEW.world_id
           AND g.channel_id = r.death_channel_id
           AND g.runtime_scope_ownership_generation = r.death_scope_ownership_generation
           AND a.transaction_id = r.transaction_id
           AND a.item_instance_id = r.item_instance_id
           AND a.occurred_at = r.occurred_at
           AND a.envelope_sha256 = r.envelope_sha256
           AND EXISTS (
               SELECT 1 FROM game_item_mint_reservations v
                WHERE (v.death_world_id, v.death_channel_id, v.death_scope_ownership_generation,
                       v.death_actor_local_id, v.death_actor_local_generation,
                       v.loot_table_family, v.loot_table_production_key,
                       v.loot_table_revision_ref, v.loot_purpose_key, v.draw_ordinal,
                       v.intent_binding, v.transaction_id, v.event_id, v.item_instance_id,
                       v.occurred_at)
                    = (r.death_world_id, r.death_channel_id, r.death_scope_ownership_generation,
                       r.death_actor_local_id, r.death_actor_local_generation,
                       r.loot_table_family, r.loot_table_production_key,
                       r.loot_table_revision_ref, r.loot_purpose_key, r.draw_ordinal,
                       r.intent_binding, r.transaction_id, r.event_id, r.item_instance_id,
                       r.occurred_at)
                  AND sha256(v.envelope) = r.envelope_sha256)) THEN
        RETURN NULL;
    END IF;
    IF EXISTS (
        SELECT 1
          FROM game_reward_claim_mint_receipts r
          JOIN game_item_container_entries e ON e.item_instance_id = r.item_instance_id
          JOIN game_item_audit_outbox a ON a.event_id = r.event_id
         WHERE r.item_instance_id = NEW.item_instance_id
           AND r.transaction_id = NEW.minted_transaction_id
           AND r.created_xact_id = pg_current_xact_id()
           AND e.placed_transaction_id = r.transaction_id
           AND a.transaction_id = r.transaction_id
           AND a.item_instance_id = r.item_instance_id
           AND a.created_xact_id = pg_current_xact_id()) THEN
        RETURN NULL;
    END IF;
    IF EXISTS (
        SELECT 1
          FROM game_item_mint_receipts r
          JOIN game_item_corpse_container_entries e ON e.item_instance_id = r.item_instance_id
          JOIN game_item_audit_outbox a ON a.event_id = r.event_id
         WHERE r.item_instance_id = NEW.item_instance_id
           AND r.transaction_id = NEW.minted_transaction_id
           AND r.death_world_id = NEW.world_id
           AND r.destination_parent_item_instance_id = e.parent_item_instance_id
           AND r.destination_ordinal = e.placement_ordinal
           AND e.world_id = NEW.world_id
           AND e.placed_transaction_id = r.transaction_id
           AND a.transaction_id = r.transaction_id
           AND a.item_instance_id = r.item_instance_id
           AND a.occurred_at = r.occurred_at
           AND a.envelope_sha256 = r.envelope_sha256
           AND EXISTS (
               SELECT 1 FROM game_item_mint_reservations v
                WHERE (v.death_world_id, v.death_channel_id, v.death_scope_ownership_generation,
                       v.death_actor_local_id, v.death_actor_local_generation,
                       v.loot_table_family, v.loot_table_production_key,
                       v.loot_table_revision_ref, v.loot_purpose_key, v.draw_ordinal,
                       v.intent_binding, v.transaction_id, v.event_id, v.item_instance_id,
                       v.occurred_at)
                    = (r.death_world_id, r.death_channel_id, r.death_scope_ownership_generation,
                       r.death_actor_local_id, r.death_actor_local_generation,
                       r.loot_table_family, r.loot_table_production_key,
                       r.loot_table_revision_ref, r.loot_purpose_key, r.draw_ordinal,
                       r.intent_binding, r.transaction_id, r.event_id, r.item_instance_id,
                       r.occurred_at)
                  AND sha256(v.envelope) = r.envelope_sha256)
           -- The parent must be a LIVE corpse of the SAME death (§39.4): its
           -- own CORPSE_MATERIALIZATION receipt, same full death tuple, still
           -- live on Ground (never yet DECAY_RETIREd).
           AND EXISTS (
               SELECT 1 FROM game_item_mint_receipts cr
                 JOIN game_item_ground_locations cg ON cg.item_instance_id = cr.item_instance_id
                 JOIN game_item_instances ci ON ci.item_instance_id = cr.item_instance_id
                WHERE cr.item_instance_id = e.parent_item_instance_id
                  AND cr.loot_purpose_key = 'CORPSE_MATERIALIZATION'
                  AND cr.death_world_id = r.death_world_id
                  AND cr.death_channel_id = r.death_channel_id
                  AND cr.death_scope_ownership_generation = r.death_scope_ownership_generation
                  AND cr.death_actor_local_id = r.death_actor_local_id
                  AND cr.death_actor_local_generation = r.death_actor_local_generation
                  AND ci.lifecycle = 1)) THEN
        RETURN NULL;
    END IF;
    IF EXISTS (
        SELECT 1
          FROM game_item_fee_burns f
          JOIN game_item_container_entries e ON e.item_instance_id = NEW.item_instance_id
         WHERE f.transaction_id = NEW.minted_transaction_id
           AND f.created_xact_id = pg_current_xact_id()
           AND NEW.item_instance_id IN (f.change_platinum_item_instance_id,
                                        f.change_gold_item_instance_id)
           AND e.placed_transaction_id = f.transaction_id) THEN
        RETURN NULL;
    END IF;
    IF EXISTS (
        SELECT 1
          FROM game_account_bank_coin_lines l
          JOIN game_account_bank_operations o ON o.transaction_id = l.transaction_id
          JOIN game_item_container_entries e ON e.item_instance_id = NEW.item_instance_id
         WHERE l.transaction_id = NEW.minted_transaction_id
           AND l.item_instance_id = NEW.item_instance_id
           AND l.direction = 2
           AND l.created_xact_id = pg_current_xact_id()
           AND o.created_xact_id = pg_current_xact_id()
           AND e.placed_transaction_id = o.transaction_id) THEN
        RETURN NULL;
    END IF;
    -- MAP-OVERLAY-1b: a map-item MINT with its Ground line, receipt and audit
    -- event in this transaction, exactly as reserved.
    IF EXISTS (
        SELECT 1
          FROM game_map_item_mint_receipts r
          JOIN game_item_ground_locations g ON g.item_instance_id = r.item_instance_id
          JOIN game_item_audit_outbox a ON a.event_id = r.event_id
          JOIN game_map_item_mint_reservations v
            ON (v.game_session_id, v.command_id) = (r.game_session_id, r.command_id)
         WHERE r.item_instance_id = NEW.item_instance_id
           AND r.transaction_id = NEW.minted_transaction_id
           AND r.world_id = NEW.world_id
           AND r.created_xact_id = pg_current_xact_id()
           AND (r.definition_family, r.definition_production_key, r.definition_revision_ref)
               = (NEW.definition_family, NEW.definition_production_key, NEW.definition_revision_ref)
           AND r.quantity = NEW.quantity
           AND g.world_id = r.world_id
           AND g.channel_id = r.channel_id
           AND g.runtime_scope_ownership_generation = r.runtime_scope_ownership_generation
           AND g.spatial_position = r.spatial_position
           AND g.map_revision = r.map_revision
           AND g.native_room_placement_context = r.base_bundle_digest
           AND a.transaction_id = r.transaction_id
           AND a.item_instance_id = r.item_instance_id
           AND a.occurred_at = r.occurred_at
           AND a.created_xact_id = pg_current_xact_id()
           AND sha256(a.envelope) = r.envelope_sha256
           AND (v.character_id, v.world_id, v.channel_id, v.base_bundle_digest, v.placement_key,
                v.reset_epoch, v.intent_binding, v.transaction_id, v.event_id,
                v.item_instance_id, v.occurred_at)
               = (r.character_id, r.world_id, r.channel_id, r.base_bundle_digest, r.placement_key,
                  r.reset_epoch, r.intent_binding, r.transaction_id, r.event_id,
                  r.item_instance_id, r.occurred_at)) THEN
        RETURN NULL;
    END IF;
    RAISE EXCEPTION 'item MINT must commit its location, cause receipt and audit event together as its reserved transaction'
        USING ERRCODE = '23514';
END;
$$;

CREATE OR REPLACE FUNCTION game_item_audit_envelope_size_proven() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF octet_length(NEW.envelope) <= 34816
       AND game_reward_claim_inbox_item_proven(NEW.item_instance_id, NEW.transaction_id) THEN
        RETURN NULL;
    END IF;
    IF octet_length(NEW.envelope) <= 9216 THEN
        RETURN NULL;
    END IF;
    IF octet_length(NEW.envelope) <= 27408 AND EXISTS (
        SELECT 1 FROM game_npc_trades t
         WHERE t.event_id = NEW.event_id
           AND t.transaction_id = NEW.transaction_id
           AND t.created_xact_id = pg_current_xact_id()) THEN
        RETURN NULL;
    END IF;
    IF octet_length(NEW.envelope) <= 25712 AND EXISTS (
        SELECT 1 FROM game_item_fee_burns f
         WHERE f.event_id = NEW.event_id
           AND f.transaction_id = NEW.transaction_id
           AND f.created_xact_id = pg_current_xact_id()) THEN
        RETURN NULL;
    END IF;
    RAISE EXCEPTION 'DUR-03 audit envelope exceeds its registered shape ceiling'
        USING ERRCODE = '23514';
END;
$$;

CREATE OR REPLACE FUNCTION game_item_key_proven() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.created_xact_id = pg_current_xact_id()
       AND game_reward_claim_inbox_item_proven(NEW.item_instance_id, NEW.minted_transaction_id) THEN
        RETURN NULL;
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM game_item_instances i
          JOIN game_reward_claim_mint_receipts r
            ON r.item_instance_id = i.item_instance_id
           AND r.transaction_id = i.minted_transaction_id
         WHERE i.item_instance_id = NEW.item_instance_id
           AND i.minted_transaction_id = NEW.minted_transaction_id
           AND i.quantity = 1
           AND r.quantity = 1
           AND r.created_xact_id = pg_current_xact_id()
           AND NEW.created_xact_id = pg_current_xact_id()) THEN
        RAISE EXCEPTION 'a key number is set only by the reward-claim MINT of its one item'
            USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END;
$$;

-- Several fresh keys can share one Inbox MINT transaction. Item identity stays
-- unique and immutable; the key proof above still rejects every unrelated key.
ALTER TABLE game_item_keys DROP CONSTRAINT game_item_keys_minted_transaction_id_key;
CREATE INDEX game_item_keys_minted_transaction ON game_item_keys(minted_transaction_id);

-- A larger stored column is necessary for this bounded aggregate shape. Every
-- smaller original shape still passes its own unchanged deferred size arm.
ALTER TABLE game_item_audit_outbox DROP CONSTRAINT game_item_audit_outbox_envelope_check;
ALTER TABLE game_item_audit_outbox ADD CONSTRAINT game_item_audit_outbox_envelope_check
    CHECK (octet_length(envelope) BETWEEN 1 AND 34816);

DO $$ BEGIN
    EXECUTE format('ALTER FUNCTION game_reward_claim_inbox_mint_lock(UUID, NUMERIC) SET search_path = pg_catalog, %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_reward_claim_legacy_inbox_exclusion() SET search_path = pg_catalog, %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_reward_claim_inbox_mint_reserve(UUID, NUMERIC, UUID, UUID, UUID, UUID, BYTEA, BYTEA, UUID[], BYTEA) SET search_path = pg_catalog, %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_reward_claim_inbox_mint_charge(UUID, NUMERIC, UUID, BYTEA) SET search_path = pg_catalog, %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_reward_claim_inbox_mint_begin(UUID, NUMERIC, UUID, BYTEA) SET search_path = pg_catalog, %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_reward_claim_inbox_mint_finish(UUID, NUMERIC, UUID, BYTEA, NUMERIC, NUMERIC, BYTEA) SET search_path = pg_catalog, %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_reward_claim_inbox_mint_is_complete(UUID) SET search_path = pg_catalog, %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_reward_claim_inbox_item_proven(UUID, UUID) SET search_path = pg_catalog, %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_reward_claim_inbox_mint_proven() SET search_path = pg_catalog, %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_reward_claim_inbox_delivery_receipt_proven() SET search_path = pg_catalog, %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_item_mint_consistency_guard() SET search_path = pg_catalog, %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_item_audit_envelope_size_proven() SET search_path = pg_catalog, %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_item_key_proven() SET search_path = pg_catalog, %I, pg_temp', current_schema());
END $$;

REVOKE ALL ON game_reward_claim_inbox_mint_reservations,
    game_reward_claim_inbox_mint_receipts, game_reward_claim_inbox_mint_lines
FROM PUBLIC, oteryn_game_runtime, oteryn_game_control;
REVOKE ALL ON FUNCTION
    game_reward_claim_inbox_mint_lock(UUID, NUMERIC),
    game_reward_claim_legacy_inbox_exclusion(),
    game_reward_claim_inbox_mint_reserve(UUID, NUMERIC, UUID, UUID, UUID, UUID, BYTEA, BYTEA, UUID[], BYTEA),
    game_reward_claim_inbox_mint_charge(UUID, NUMERIC, UUID, BYTEA),
    game_reward_claim_inbox_mint_begin(UUID, NUMERIC, UUID, BYTEA),
    game_reward_claim_inbox_mint_finish(UUID, NUMERIC, UUID, BYTEA, NUMERIC, NUMERIC, BYTEA),
    game_reward_claim_inbox_mint_is_complete(UUID),
    game_reward_claim_inbox_item_proven(UUID, UUID),
    game_reward_claim_inbox_mint_proven(),
    game_reward_claim_inbox_delivery_receipt_proven()
FROM PUBLIC, oteryn_game_runtime, oteryn_game_control;
GRANT EXECUTE ON FUNCTION
    game_reward_claim_inbox_mint_reserve(UUID, NUMERIC, UUID, UUID, UUID, UUID, BYTEA, BYTEA, UUID[], BYTEA),
    game_reward_claim_inbox_mint_charge(UUID, NUMERIC, UUID, BYTEA),
    game_reward_claim_inbox_mint_begin(UUID, NUMERIC, UUID, BYTEA),
    game_reward_claim_inbox_mint_finish(UUID, NUMERIC, UUID, BYTEA, NUMERIC, NUMERIC, BYTEA),
    game_reward_claim_inbox_item_proven(UUID, UUID)
TO oteryn_game_runtime;
-- The last helper returns only the guard's boolean result; no receipt/table read
-- grant or raw Inbox mutation becomes available to the runtime role.
