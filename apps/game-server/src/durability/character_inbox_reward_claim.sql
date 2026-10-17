-- MAIL-1 / CP D1152, D1158, D1318: candidate privileged Inbox seam only.
-- This file is not a migration and is never run by the runtime module.
-- CP assigns the actual migration number after the predecessor slots merge.
--
-- The source owner writes and proves its claim, fresh items, binding rows and
-- reward_claim_mint audit in this same transaction. Its shared MINT receipt and
-- audit guards must admit the real Inbox destination before activation. The
-- current CHEST/backpack guards remain unchanged and therefore fail closed.
-- A source fixture in the native seam tests is not qualification of that bridge.

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
