-- TRADE-1 / PLAYER-TRADE0 §5-6: one same-instance, non-merging, two-Character swap.
-- Source union: main f010472bcb6794a2cbc56dfaf755bd122c10225e, through 0095.
-- Apply the main prerequisites before this migration. Older migrations are immutable.
--
-- These immutable rows describe one committed outcome; they do not issue consent or
-- replace either native current fence. The native Root writer checks both fences,
-- locks both roots in CharacterId order, both offered items in ItemInstanceId order,
-- and both container slots in CharacterId order before it writes any of these rows.
-- Both actual source removals, destination placements, item revisions and the single
-- pending type-2 audit event must commit in the same physical transaction.
-- No cause/reservation is committed before the successful atomic swap.
--
-- Fixed shape: two existing items, four location lines, two participants, six work
-- units. No MINT/BURN/merge, container tree, equipment, CharacterRevision or timed
-- row write. The existing 9,216-byte envelope bound and type-2 activation stay intact.
-- Rollback is a successor migration. Once an outcome exists its cause and receipts
-- remain durable; stopping new swaps revokes INSERT rather than deleting evidence.

CREATE TABLE game_player_trades (
    occurrence_id UUID PRIMARY KEY CHECK (game_character_is_uuid_v7(occurrence_id)),
    trade_id UUID NOT NULL UNIQUE CHECK (game_character_is_uuid_v7(trade_id)),
    transaction_id UUID NOT NULL UNIQUE CHECK (game_character_is_uuid_v7(transaction_id)),
    event_id UUID NOT NULL UNIQUE CHECK (game_character_is_uuid_v7(event_id)),
    first_game_session_id UUID NOT NULL CHECK (game_character_is_uuid_v7(first_game_session_id)),
    first_command_id NUMERIC(20,0) NOT NULL
        CHECK (first_command_id BETWEEN 1 AND 18446744073709551615),
    trigger_game_session_id UUID NOT NULL CHECK (game_character_is_uuid_v7(trigger_game_session_id)),
    trigger_command_id NUMERIC(20,0) NOT NULL
        CHECK (trigger_command_id BETWEEN 1 AND 18446744073709551615),
    request_binding BYTEA NOT NULL CHECK (octet_length(request_binding) = 32),
    world_id UUID NOT NULL CHECK (game_character_is_uuid_v7(world_id)),
    channel_id UUID NOT NULL CHECK (game_character_is_uuid_v7(channel_id)),
    runtime_scope_ownership_generation NUMERIC(20,0) NOT NULL
        CHECK (runtime_scope_ownership_generation BETWEEN 1 AND 18446744073709551615),
    content_digest BYTEA NOT NULL CHECK (octet_length(content_digest) = 32),
    occurred_at BIGINT NOT NULL CHECK (occurred_at > 0),
    envelope_sha256 BYTEA NOT NULL CHECK (octet_length(envelope_sha256) = 32),
    committed_at BIGINT NOT NULL CHECK (committed_at >= 0),
    created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id(),
    CHECK (first_game_session_id <> trigger_game_session_id),
    UNIQUE (first_game_session_id, first_command_id),
    UNIQUE (trigger_game_session_id, trigger_command_id)
);

CREATE TABLE game_player_trade_participants (
    occurrence_id UUID NOT NULL REFERENCES game_player_trades (occurrence_id),
    participant_ordinal SMALLINT NOT NULL CHECK (participant_ordinal IN (1, 2)),
    character_id UUID NOT NULL REFERENCES game_character_roots (character_id),
    account_id UUID NOT NULL CHECK (game_character_is_uuid_v7(account_id)),
    game_session_id UUID NOT NULL CHECK (game_character_is_uuid_v7(game_session_id)),
    command_id NUMERIC(20,0) NOT NULL CHECK (command_id BETWEEN 1 AND 18446744073709551615),
    connection_generation NUMERIC(20,0) NOT NULL
        CHECK (connection_generation BETWEEN 1 AND 18446744073709551615),
    current_connection_generation NUMERIC(20,0) NOT NULL
        CHECK (current_connection_generation BETWEEN 1 AND 18446744073709551615),
    character_lease_generation NUMERIC(20,0) NOT NULL
        CHECK (character_lease_generation BETWEEN 1 AND 18446744073709551615),
    character_revision NUMERIC(20,0) NOT NULL
        CHECK (character_revision BETWEEN 1 AND 18446744073709551615),
    backpack_item_instance_id UUID NOT NULL REFERENCES game_item_instances (item_instance_id),
    backpack_placed_transaction_id UUID NOT NULL
        CHECK (game_character_is_uuid_v7(backpack_placed_transaction_id)),
    backpack_state_revision NUMERIC(20,0) NOT NULL
        CHECK (backpack_state_revision BETWEEN 1 AND 18446744073709551615),
    backpack_capacity SMALLINT NOT NULL CHECK (backpack_capacity BETWEEN 1 AND 20),
    entry_count_before SMALLINT NOT NULL CHECK (entry_count_before BETWEEN 1 AND 19),
    max_ordinal_before NUMERIC(20,0) NOT NULL
        CHECK (max_ordinal_before BETWEEN 1 AND 18446744073709551614),
    created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id(),
    CHECK (entry_count_before < backpack_capacity),
    PRIMARY KEY (occurrence_id, participant_ordinal),
    UNIQUE (occurrence_id, character_id),
    UNIQUE (occurrence_id, account_id),
    UNIQUE (occurrence_id, game_session_id),
    -- One accepted CommandRef cannot commit again, including in the opposite role.
    UNIQUE (game_session_id, command_id)
);

CREATE TABLE game_player_trade_receipts (
    occurrence_id UUID NOT NULL REFERENCES game_player_trades (occurrence_id),
    line_ordinal SMALLINT NOT NULL CHECK (line_ordinal IN (1, 2)),
    item_instance_id UUID NOT NULL REFERENCES game_item_instances (item_instance_id),
    source_character_id UUID NOT NULL REFERENCES game_character_roots (character_id),
    source_parent_item_instance_id UUID NOT NULL REFERENCES game_item_instances (item_instance_id),
    source_placement_ordinal NUMERIC(20,0) NOT NULL
        CHECK (source_placement_ordinal BETWEEN 1 AND 18446744073709551615),
    source_placed_transaction_id UUID NOT NULL
        CHECK (game_character_is_uuid_v7(source_placed_transaction_id)),
    destination_character_id UUID NOT NULL REFERENCES game_character_roots (character_id),
    destination_parent_item_instance_id UUID NOT NULL REFERENCES game_item_instances (item_instance_id),
    destination_ordinal NUMERIC(20,0) NOT NULL
        CHECK (destination_ordinal BETWEEN 1 AND 18446744073709551615),
    definition_family TEXT NOT NULL CHECK (octet_length(definition_family) BETWEEN 1 AND 128),
    definition_production_key TEXT NOT NULL
        CHECK (octet_length(definition_production_key) BETWEEN 1 AND 512),
    definition_revision_ref TEXT NOT NULL
        CHECK (octet_length(definition_revision_ref) BETWEEN 1 AND 512),
    quantity BIGINT NOT NULL CHECK (quantity BETWEEN 1 AND 100),
    key_number INTEGER NOT NULL CHECK (key_number BETWEEN 0 AND 65535),
    state_revision_before NUMERIC(20,0) NOT NULL
        CHECK (state_revision_before BETWEEN 1 AND 18446744073709551614),
    timed_revision NUMERIC(20,0) NULL
        CHECK (timed_revision BETWEEN 1 AND 18446744073709551615),
    timed_charges BIGINT NULL CHECK (timed_charges BETWEEN 1 AND 65535),
    timed_remaining_ms BIGINT NULL CHECK (timed_remaining_ms BETWEEN 0 AND 604800000),
    timed_deadline_at BIGINT NULL CHECK (timed_deadline_at > 0),
    created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id(),
    CHECK (source_character_id <> destination_character_id),
    CHECK (source_parent_item_instance_id <> destination_parent_item_instance_id),
    CHECK (item_instance_id <> source_parent_item_instance_id
       AND item_instance_id <> destination_parent_item_instance_id),
    CHECK (timed_revision IS NOT NULL
        OR (timed_charges IS NULL AND timed_remaining_ms IS NULL AND timed_deadline_at IS NULL)),
    CHECK (timed_deadline_at IS NULL OR timed_remaining_ms IS NOT NULL),
    PRIMARY KEY (occurrence_id, line_ordinal),
    UNIQUE (occurrence_id, item_instance_id)
);

-- Runtime has no UPDATE privilege on the immutable slot table. This bounded
-- lock operation grants exactly the two ordered row locks needed after the
-- native writer has checked both fences and locked both Character roots/items.
-- Its count is DATA, never proof of a session, reservation or custody grant.
CREATE FUNCTION game_player_trade_lock_slots(p_first UUID, p_second UUID) RETURNS SMALLINT
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, public, pg_temp AS $$
DECLARE locked SMALLINT;
BEGIN
    IF p_first IS NULL OR p_second IS NULL
       OR NOT game_character_is_uuid_v7(p_first) OR NOT game_character_is_uuid_v7(p_second)
       OR p_first >= p_second THEN
        RAISE EXCEPTION 'player trade slot locks require two ordered Character identities'
            USING ERRCODE = '23514';
    END IF;
    PERFORM 1 FROM game_item_container_slots
     WHERE character_id IN (p_first, p_second) ORDER BY character_id FOR UPDATE;
    GET DIAGNOSTICS locked = ROW_COUNT;
    RETURN locked;
END;
$$;

CREATE FUNCTION game_player_trade_complete(p_occurrence UUID) RETURNS VOID
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, public, pg_temp AS $$
DECLARE
    t game_player_trades%ROWTYPE;
    p game_player_trade_participants%ROWTYPE;
    l game_player_trade_receipts%ROWTYPE;
    actual_entries BIGINT;
    actual_old_max NUMERIC(20,0);
    anchor UUID;
BEGIN
    SELECT * INTO t FROM game_player_trades WHERE occurrence_id = p_occurrence;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'player trade has no cause' USING ERRCODE = '23514';
    END IF;
    IF t.created_xact_id <> pg_current_xact_id()
       OR (SELECT count(*) FROM game_player_trade_participants p0
            WHERE p0.occurrence_id = t.occurrence_id
              AND p0.created_xact_id = pg_current_xact_id()) <> 2
       OR (SELECT count(*) FROM game_player_trade_receipts l0
            WHERE l0.occurrence_id = t.occurrence_id
              AND l0.created_xact_id = pg_current_xact_id()) <> 2 THEN
        RAISE EXCEPTION 'player trade requires two fresh participants and two fresh receipts'
            USING ERRCODE = '23514';
    END IF;
    IF EXISTS (
        SELECT 1 FROM game_player_trade_participants a
          JOIN game_player_trade_participants b USING (occurrence_id)
         WHERE a.occurrence_id = t.occurrence_id AND a.participant_ordinal = 1
           AND b.participant_ordinal = 2 AND a.character_id >= b.character_id)
       OR EXISTS (
        SELECT 1 FROM game_player_trade_receipts a
          JOIN game_player_trade_receipts b USING (occurrence_id)
         WHERE a.occurrence_id = t.occurrence_id AND a.line_ordinal = 1
           AND b.line_ordinal = 2 AND a.item_instance_id >= b.item_instance_id) THEN
        RAISE EXCEPTION 'player trade participants and item lines must be canonical'
            USING ERRCODE = '23514';
    END IF;

    FOR p IN SELECT * FROM game_player_trade_participants
              WHERE occurrence_id = t.occurrence_id ORDER BY character_id LOOP
        IF (p.game_session_id, p.command_id)
                = (t.first_game_session_id, t.first_command_id) THEN
            IF p.current_connection_generation < p.connection_generation THEN
                RAISE EXCEPTION 'first trade accept connection generation regressed'
                    USING ERRCODE = '23514';
            END IF;
        ELSIF (p.game_session_id, p.command_id)
                = (t.trigger_game_session_id, t.trigger_command_id) THEN
            IF p.current_connection_generation <> p.connection_generation THEN
                RAISE EXCEPTION 'trigger trade accept connection generation differs'
                    USING ERRCODE = '23514';
            END IF;
        ELSE
            RAISE EXCEPTION 'player trade participant does not name its accepted CommandRef'
                USING ERRCODE = '23514';
        END IF;
        IF NOT EXISTS (
            SELECT 1 FROM game_character_roots r
             WHERE r.character_id = p.character_id AND r.account_id = p.account_id
               AND r.world_id = t.world_id AND r.lifecycle = 1
               AND r.character_revision = p.character_revision)
           OR NOT EXISTS (
            SELECT 1 FROM game_item_container_slots s
              JOIN game_item_instances i ON i.item_instance_id = s.item_instance_id
             WHERE s.character_id = p.character_id AND s.world_id = t.world_id
               AND s.item_instance_id = p.backpack_item_instance_id
               AND s.placed_transaction_id = p.backpack_placed_transaction_id
               AND s.placed_transaction_id <> t.transaction_id
               AND i.world_id = t.world_id AND i.lifecycle = 1
               AND i.state_revision = p.backpack_state_revision)
           OR (SELECT count(*) FROM game_player_trade_receipts r
                WHERE r.occurrence_id = t.occurrence_id
                  AND r.source_character_id = p.character_id) <> 1
           OR (SELECT count(*) FROM game_player_trade_receipts r
                WHERE r.occurrence_id = t.occurrence_id
                  AND r.destination_character_id = p.character_id) <> 1 THEN
            RAISE EXCEPTION 'player trade must preserve both live roots and main backpacks'
                USING ERRCODE = '23514';
        END IF;
        -- The native writer already holds both slots in this CharacterId order.
        -- Retain the existing placement serialization/capacity backstop as well.
        PERFORM 1 FROM game_item_container_slots
         WHERE character_id = p.character_id FOR UPDATE;
        SELECT count(*) INTO actual_entries FROM game_item_container_entries
         WHERE parent_item_instance_id = p.backpack_item_instance_id;
        SELECT greatest(
            coalesce((SELECT max(e.placement_ordinal) FROM game_item_container_entries e
                       WHERE e.parent_item_instance_id = p.backpack_item_instance_id
                         AND e.placed_transaction_id <> t.transaction_id), 0),
            (SELECT r.source_placement_ordinal FROM game_player_trade_receipts r
              WHERE r.occurrence_id = t.occurrence_id
                AND r.source_character_id = p.character_id))
          INTO actual_old_max;
        IF actual_entries <> p.entry_count_before
           OR actual_entries >= p.backpack_capacity OR actual_entries > 20
           OR actual_old_max IS DISTINCT FROM p.max_ordinal_before THEN
            RAISE EXCEPTION 'player trade needs one real free entry before either source removal'
                USING ERRCODE = '23514';
        END IF;
    END LOOP;

    FOR l IN SELECT * FROM game_player_trade_receipts
              WHERE occurrence_id = t.occurrence_id ORDER BY line_ordinal LOOP
        IF NOT EXISTS (
            SELECT 1 FROM game_player_trade_participants src
              JOIN game_player_trade_participants dst USING (occurrence_id)
             WHERE src.occurrence_id = t.occurrence_id
               AND src.character_id = l.source_character_id
               AND dst.character_id = l.destination_character_id
               AND src.character_id <> dst.character_id
               AND src.backpack_item_instance_id = l.source_parent_item_instance_id
               AND dst.backpack_item_instance_id = l.destination_parent_item_instance_id
               AND l.destination_ordinal = dst.max_ordinal_before + 1
               AND l.source_placement_ordinal <= src.max_ordinal_before
               AND l.source_placed_transaction_id <> t.transaction_id)
           OR NOT EXISTS (
            SELECT 1 FROM game_item_instances i
              LEFT JOIN game_item_keys k USING (item_instance_id)
             WHERE i.item_instance_id = l.item_instance_id AND i.world_id = t.world_id
               AND i.lifecycle = 1 AND i.quantity = l.quantity
               AND (i.definition_family, i.definition_production_key, i.definition_revision_ref)
                   = (l.definition_family, l.definition_production_key, l.definition_revision_ref)
               AND coalesce(k.key_number, 0) = l.key_number
               AND i.state_revision = l.state_revision_before + 1
               AND i.last_transaction_id = t.transaction_id
               AND i.minted_transaction_id <> t.transaction_id)
           OR NOT EXISTS (
            SELECT 1 FROM game_item_transfer_quantity_evidence q
             WHERE q.item_instance_id = l.item_instance_id
               AND q.transaction_id = t.transaction_id AND q.quantity_before = l.quantity)
           OR NOT EXISTS (
            SELECT 1 FROM game_item_container_entries e
             WHERE e.item_instance_id = l.item_instance_id AND e.world_id = t.world_id
               AND e.character_id = l.destination_character_id
               AND e.parent_item_instance_id = l.destination_parent_item_instance_id
               AND e.placement_ordinal = l.destination_ordinal
               AND e.placed_transaction_id = t.transaction_id) THEN
            RAISE EXCEPTION 'player trade requires both exact unchanged items at reciprocal destinations'
                USING ERRCODE = '23514';
        END IF;
        IF (l.timed_revision IS NULL AND EXISTS (
                SELECT 1 FROM game_item_timed_states v WHERE v.item_instance_id = l.item_instance_id))
           OR (l.timed_revision IS NOT NULL AND NOT EXISTS (
                SELECT 1 FROM game_item_timed_states v
                 WHERE v.item_instance_id = l.item_instance_id
                   AND (v.state_revision, v.charges, v.remaining_ms, v.deadline_at)
                       IS NOT DISTINCT FROM
                       (l.timed_revision, l.timed_charges, l.timed_remaining_ms, l.timed_deadline_at)))
           OR EXISTS (
                SELECT 1 FROM game_item_timed_state_writes w
                 WHERE w.item_instance_id = l.item_instance_id
                   AND w.expected_revision = l.timed_revision - 1
                   AND w.created_xact_id = pg_current_xact_id()) THEN
            RAISE EXCEPTION 'player trade preserves the exact timed row or its proven absence'
                USING ERRCODE = '23514';
        END IF;
        IF EXISTS (SELECT 1 FROM game_item_ground_locations x WHERE x.item_instance_id = l.item_instance_id)
           OR EXISTS (SELECT 1 FROM game_item_container_slots x WHERE x.item_instance_id = l.item_instance_id)
           OR EXISTS (SELECT 1 FROM game_character_equipment_slots x WHERE x.item_instance_id = l.item_instance_id)
           OR EXISTS (SELECT 1 FROM game_item_corpse_container_entries x WHERE x.item_instance_id = l.item_instance_id)
           OR EXISTS (SELECT 1 FROM game_item_character_inbox_locations x WHERE x.item_instance_id = l.item_instance_id)
           OR EXISTS (SELECT 1 FROM game_item_house_interior_locations x WHERE x.item_instance_id = l.item_instance_id)
           OR EXISTS (SELECT 1 FROM game_item_house_reclaim_provenance x WHERE x.item_instance_id = l.item_instance_id)
           OR EXISTS (SELECT 1 FROM game_item_container_entries x WHERE x.parent_item_instance_id = l.item_instance_id)
           OR EXISTS (SELECT 1 FROM game_item_corpse_container_entries x WHERE x.parent_item_instance_id = l.item_instance_id)
           OR EXISTS (SELECT 1 FROM game_item_mint_receipts x
                       WHERE x.item_instance_id = l.item_instance_id
                         AND x.loot_purpose_key = 'CORPSE_MATERIALIZATION') THEN
            RAISE EXCEPTION 'player trade items must be two exclusive direct-entry leaves'
                USING ERRCODE = '23514';
        END IF;
    END LOOP;

    -- Both receipt items were checked above. The fresh-cause item-change guard
    -- below rejects every third item UPDATE using this TransactionId, avoiding
    -- an unbounded scan of the global item table by last_transaction_id.
    IF EXISTS (SELECT 1 FROM game_item_instances i WHERE i.minted_transaction_id = t.transaction_id)
       OR (SELECT count(*) FROM game_item_container_entries e
            WHERE e.placed_transaction_id = t.transaction_id) <> 2 THEN
        RAISE EXCEPTION 'player trade cannot mint or add an extra placement'
            USING ERRCODE = '23514';
    END IF;
    SELECT item_instance_id INTO anchor FROM game_player_trade_receipts
     WHERE occurrence_id = t.occurrence_id ORDER BY item_instance_id LIMIT 1;
    IF NOT EXISTS (
        SELECT 1 FROM game_item_audit_outbox a
         WHERE a.event_id = t.event_id AND a.transaction_id = t.transaction_id
           AND a.transaction_ordinal = 1 AND a.transaction_count = 1 AND a.event_type_id = 2
           AND a.item_instance_id = anchor AND a.occurred_at = t.occurred_at
           AND a.envelope_sha256 = t.envelope_sha256 AND octet_length(a.envelope) <= 9216
           AND a.created_xact_id = pg_current_xact_id()
           AND a.publication_state = 1 AND a.published_at IS NULL) THEN
        RAISE EXCEPTION 'player trade requires one exact fresh pending type-2 audit event'
            USING ERRCODE = '23514';
    END IF;
END;
$$;

CREATE FUNCTION game_player_trade_fact_proven() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, public, pg_temp AS $$
BEGIN
    PERFORM game_player_trade_complete(NEW.occurrence_id);
    RETURN NULL;
END;
$$;

CREATE TRIGGER game_player_trades_stamp_xact BEFORE INSERT ON game_player_trades
    FOR EACH ROW EXECUTE FUNCTION game_item_stamp_created_xact_id();
CREATE TRIGGER game_player_trade_participants_stamp_xact BEFORE INSERT ON game_player_trade_participants
    FOR EACH ROW EXECUTE FUNCTION game_item_stamp_created_xact_id();
CREATE TRIGGER game_player_trade_receipts_stamp_xact BEFORE INSERT ON game_player_trade_receipts
    FOR EACH ROW EXECUTE FUNCTION game_item_stamp_created_xact_id();

CREATE CONSTRAINT TRIGGER game_player_trades_proven AFTER INSERT ON game_player_trades
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_player_trade_fact_proven();
CREATE CONSTRAINT TRIGGER game_player_trade_participants_proven AFTER INSERT ON game_player_trade_participants
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_player_trade_fact_proven();
CREATE CONSTRAINT TRIGGER game_player_trade_receipts_proven AFTER INSERT ON game_player_trade_receipts
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_player_trade_fact_proven();

CREATE TRIGGER game_player_trades_immutable BEFORE UPDATE OR DELETE ON game_player_trades
    FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_player_trade_participants_immutable BEFORE UPDATE OR DELETE ON game_player_trade_participants
    FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_player_trade_receipts_immutable BEFORE UPDATE OR DELETE ON game_player_trade_receipts
    FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_player_trades_no_truncate BEFORE TRUNCATE ON game_player_trades
    FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();
CREATE TRIGGER game_player_trade_participants_no_truncate BEFORE TRUNCATE ON game_player_trade_participants
    FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();
CREATE TRIGGER game_player_trade_receipts_no_truncate BEFORE TRUNCATE ON game_player_trade_receipts
    FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();

-- The three complete 0095 guard bodies follow, with one TRADE arm prepended each.
-- Their older HOUSE and non-HOUSE arms are retained exactly.

CREATE OR REPLACE FUNCTION game_item_instance_change_proven() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    -- 0100 player-trade arm BEGIN: a fresh cause owns exactly its two item changes.
    IF EXISTS (SELECT 1 FROM game_player_trades t
                WHERE t.transaction_id = NEW.last_transaction_id
                  AND t.created_xact_id = pg_current_xact_id()) THEN
        IF TG_OP <> 'UPDATE' OR OLD.lifecycle <> 1 OR NEW.lifecycle <> 1
           OR NEW.state_revision <> OLD.state_revision + 1
           OR NEW.last_transaction_id IS NOT DISTINCT FROM OLD.last_transaction_id
           OR (to_jsonb(NEW) - 'last_transaction_id' - 'state_revision')
               IS DISTINCT FROM (to_jsonb(OLD) - 'last_transaction_id' - 'state_revision')
           OR NOT EXISTS (
                SELECT 1 FROM game_player_trades t
                  JOIN game_player_trade_receipts l USING (occurrence_id)
                 WHERE t.transaction_id = NEW.last_transaction_id
                   AND t.created_xact_id = pg_current_xact_id()
                   AND l.created_xact_id = pg_current_xact_id()
                   AND l.item_instance_id = OLD.item_instance_id
                   AND t.world_id = OLD.world_id
                   AND (l.definition_family, l.definition_production_key, l.definition_revision_ref)
                       = (OLD.definition_family, OLD.definition_production_key, OLD.definition_revision_ref)
                   AND l.quantity = OLD.quantity AND l.state_revision_before = OLD.state_revision) THEN
            RAISE EXCEPTION 'player trade must preserve the actual OLD item state and change only its revision and transaction'
                USING ERRCODE = '23514';
        END IF;
        PERFORM game_player_trade_complete((SELECT occurrence_id FROM game_player_trades
                                            WHERE transaction_id = NEW.last_transaction_id));
        RETURN NULL;
    END IF;
    -- 0100 player-trade arm END.

    -- 0095: exact owner-produced House shape evidence; all older arms below are unchanged.
    IF EXISTS(SELECT 1 FROM game_item_house_shape_proofs p WHERE p.transaction_id=NEW.last_transaction_id
     AND p.item_instance_id=NEW.item_instance_id AND p.created_xact_id=pg_current_xact_id()) THEN
     PERFORM game_house_shape_complete(NEW.last_transaction_id);RETURN NULL;END IF;

    -- 0082: the BURN line of an NPC trade of this same physical transaction.
    IF EXISTS(SELECT 1 FROM game_npc_trade_lines l
          JOIN game_npc_trades t ON t.transaction_id = l.transaction_id
                                AND t.created_xact_id = pg_current_xact_id()
         WHERE l.transaction_id = NEW.last_transaction_id
           AND l.item_instance_id = NEW.item_instance_id
           AND l.line_kind IN (1, 3)
           AND l.created_xact_id = pg_current_xact_id()) THEN RETURN NULL; END IF;

    -- 0083: the used stack (and the flask stack it grows) of an item-use record of this same
    -- physical transaction.
    IF EXISTS (SELECT 1 FROM game_item_use_records u
                WHERE u.transaction_id = NEW.last_transaction_id
                  AND u.created_xact_id = pg_current_xact_id()
                  AND (u.source_item_instance_id = NEW.item_instance_id
                       OR (u.shape = 3 AND u.flask_item_instance_id = NEW.item_instance_id))) THEN
        RETURN NULL;
    END IF;
    IF EXISTS(SELECT 1 FROM game_spell_field_temporal_receipts r JOIN game_spell_field_temporal_before b USING(transaction_id)
      WHERE r.transaction_id=NEW.last_transaction_id AND r.item_instance_id=NEW.item_instance_id
       AND NEW.state_revision=r.state_revision_before+1 AND r.created_xact_id=pg_current_xact_id() AND b.created_xact_id=r.created_xact_id) THEN RETURN NULL; END IF;

    IF EXISTS(SELECT 1 FROM game_spell_item_temporal_receipts r JOIN game_spell_item_temporal_before b USING(transaction_id)
      WHERE r.transaction_id=NEW.last_transaction_id AND r.item_instance_id=NEW.item_instance_id
       AND NEW.state_revision=r.state_revision_before+1 AND r.created_xact_id=pg_current_xact_id() AND b.created_xact_id=r.created_xact_id) THEN RETURN NULL; END IF;

    IF EXISTS(SELECT 1 FROM game_character_equipment_receipts r
       WHERE r.operation=1 AND r.item_instance_id=NEW.item_instance_id
         AND r.transaction_id=NEW.last_transaction_id AND r.state_revision_before=OLD.state_revision
         AND NEW.state_revision=OLD.state_revision+1 AND NEW.quantity=OLD.quantity
         AND NEW.lifecycle=OLD.lifecycle AND r.created_xact_id=pg_current_xact_id()) THEN
        RETURN NULL;
    END IF;
    IF EXISTS(SELECT 1 FROM game_spell_item_expiry_receipts e JOIN game_item_instances i USING(item_instance_id)
      WHERE e.item_instance_id=NEW.item_instance_id AND e.transaction_id=NEW.last_transaction_id
        AND i.last_transaction_id=e.transaction_id AND e.created_xact_id=pg_current_xact_id()) THEN RETURN NULL; END IF;
    IF EXISTS(SELECT 1 FROM game_spell_item_lines l JOIN game_spell_item_receipts r USING(transaction_id)
       WHERE l.item_instance_id=NEW.item_instance_id AND l.operation_kind<>1
         AND l.transaction_id=NEW.last_transaction_id AND r.created_xact_id=pg_current_xact_id()
         AND l.created_xact_id=pg_current_xact_id()) THEN RETURN NULL; END IF;
    IF NOT EXISTS (
        SELECT 1 FROM game_item_transfer_receipts r
         WHERE r.transaction_id = NEW.last_transaction_id
           AND (r.source_item_instance_id = NEW.item_instance_id
                OR r.receiver_item_instance_id = NEW.item_instance_id)
           -- Repair generation 2 (finding 2's root cause, applied here too):
           -- the matched receipt must be this SAME physical transaction's
           -- own receipt, never a historical one whose logical TransactionId
           -- is replayed from a different, already-committed transaction.
           AND r.created_xact_id = pg_current_xact_id())
       AND NOT EXISTS (
        SELECT 1 FROM game_item_decay_retire_receipts d
         WHERE d.transaction_id = NEW.last_transaction_id
           AND d.item_instance_id = NEW.item_instance_id
           AND d.created_xact_id = pg_current_xact_id())
       AND NOT EXISTS (
        SELECT 1 FROM game_item_fee_burn_lines l
          -- The line's fee record is this same physical transaction's own record, never a
          -- committed one a later line is appended to.
          JOIN game_item_fee_burns f ON f.transaction_id = l.transaction_id
                                    AND f.created_xact_id = pg_current_xact_id()
         WHERE l.transaction_id = NEW.last_transaction_id
           AND l.item_instance_id = NEW.item_instance_id
           AND l.created_xact_id = pg_current_xact_id())
       AND NOT EXISTS (
        SELECT 1 FROM game_item_timed_state_writes w
         WHERE w.transaction_id = NEW.last_transaction_id
           AND w.item_instance_id = NEW.item_instance_id
           AND w.cause IN (2, 3)
           AND w.created_xact_id = pg_current_xact_id())
       AND NOT EXISTS (
        SELECT 1 FROM game_account_bank_coin_lines l
          JOIN game_account_bank_operations o ON o.transaction_id = l.transaction_id
                                             AND o.created_xact_id = pg_current_xact_id()
         WHERE l.transaction_id = NEW.last_transaction_id
           AND l.item_instance_id = NEW.item_instance_id
           AND l.direction = 1
           AND l.created_xact_id = pg_current_xact_id()) THEN
        RAISE EXCEPTION 'item change must commit with its TRANSFER receipt'
            USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END;
$$;

CREATE OR REPLACE FUNCTION game_item_container_entry_removal_proven() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    -- 0100 player-trade arm BEGIN: bind the receipt to the actual deleted row.
    IF TG_OP = 'DELETE' AND EXISTS (
        SELECT 1 FROM game_item_instances i
          JOIN game_player_trades t ON t.transaction_id = i.last_transaction_id
         WHERE i.item_instance_id = OLD.item_instance_id
           AND t.created_xact_id = pg_current_xact_id()) THEN
        IF NOT EXISTS (
            SELECT 1 FROM game_item_instances i
              JOIN game_player_trades t ON t.transaction_id = i.last_transaction_id
              JOIN game_player_trade_receipts l USING (occurrence_id)
             WHERE i.item_instance_id = OLD.item_instance_id
               AND l.item_instance_id = OLD.item_instance_id AND t.world_id = OLD.world_id
               AND l.source_character_id = OLD.character_id
               AND l.source_parent_item_instance_id = OLD.parent_item_instance_id
               AND l.source_placement_ordinal = OLD.placement_ordinal
               AND l.source_placed_transaction_id = OLD.placed_transaction_id
               AND t.created_xact_id = pg_current_xact_id()
               AND l.created_xact_id = pg_current_xact_id()) THEN
            RAISE EXCEPTION 'player trade source must be the actual removed direct entry'
                USING ERRCODE = '23514';
        END IF;
        PERFORM game_player_trade_complete((SELECT t.occurrence_id FROM game_item_instances i
                                            JOIN game_player_trades t ON t.transaction_id = i.last_transaction_id
                                            WHERE i.item_instance_id = OLD.item_instance_id));
        RETURN NULL;
    END IF;
    -- 0100 player-trade arm END.

    -- 0095: exact owner-produced House shape evidence; all older arms below are unchanged.
    IF TG_OP='DELETE' THEN
     IF EXISTS(SELECT 1 FROM game_item_house_shape_proofs p WHERE p.shape=5 AND p.source_kind=1
      AND p.item_instance_id=OLD.item_instance_id AND p.character_id=OLD.character_id AND p.world_id=OLD.world_id
      AND p.source_parent=OLD.parent_item_instance_id AND p.source_ordinal=OLD.placement_ordinal
      AND p.created_xact_id=pg_current_xact_id()) THEN RETURN NULL;END IF;
    END IF;

    -- 0082: the whole BURN of an NPC trade line of this same physical transaction (NPC-0 §5.5).
    IF TG_OP='DELETE' AND EXISTS(SELECT 1 FROM game_npc_trade_lines l
          JOIN game_npc_trades t ON t.transaction_id = l.transaction_id
          JOIN game_item_instances i ON i.item_instance_id = l.item_instance_id
         WHERE l.item_instance_id = OLD.item_instance_id
           AND l.line_kind IN (1, 3)
           AND l.quantity_after = 0
           AND l.placement_ordinal = OLD.placement_ordinal
           AND t.character_id = OLD.character_id
           AND t.backpack_item_instance_id = OLD.parent_item_instance_id
           AND i.last_transaction_id = l.transaction_id
           AND i.lifecycle = 2
           AND l.created_xact_id = pg_current_xact_id()
           AND t.created_xact_id = pg_current_xact_id()) THEN RETURN NULL; END IF;

    -- 0083: the entry of a used stack spent to zero, by the item-use record of this same
    -- physical transaction.
    IF TG_OP = 'DELETE' AND EXISTS (
        SELECT 1 FROM game_item_use_records u
          JOIN game_item_instances i ON i.item_instance_id = u.source_item_instance_id
         WHERE u.source_item_instance_id = OLD.item_instance_id
           AND u.source_quantity_after = 0
           AND u.character_id = OLD.character_id
           AND u.source_parent_item_instance_id = OLD.parent_item_instance_id
           AND u.source_placement_ordinal = OLD.placement_ordinal
           AND i.last_transaction_id = u.transaction_id
           AND i.lifecycle = 2
           AND u.created_xact_id = pg_current_xact_id()) THEN
        RETURN NULL;
    END IF;
    IF TG_OP='DELETE' AND EXISTS(SELECT 1 FROM game_spell_field_temporal_receipts r JOIN game_spell_field_temporal_schedules p ON p.item_instance_id=r.item_instance_id AND p.source_stage=r.source_stage
      JOIN game_item_instances i ON i.item_instance_id=r.item_instance_id WHERE r.item_instance_id=OLD.item_instance_id AND r.custody_kind=2
        AND r.character_id=OLD.character_id AND r.source_parent=OLD.parent_item_instance_id AND r.source_ordinal=OLD.placement_ordinal AND p.target_family IS NULL AND i.lifecycle=2 AND i.quantity=0 AND i.last_transaction_id=r.transaction_id
        AND i.state_revision=r.state_revision_before+1 AND r.created_xact_id=pg_current_xact_id()) THEN RETURN NULL; END IF;

    IF TG_OP='DELETE' AND EXISTS(SELECT 1 FROM game_spell_item_temporal_receipts r JOIN game_spell_item_temporal_schedules p USING(item_instance_id)
      JOIN game_item_instances i USING(item_instance_id) WHERE r.item_instance_id=OLD.item_instance_id AND r.custody_kind=2
        AND r.character_id=OLD.character_id AND r.source_parent=OLD.parent_item_instance_id AND r.source_ordinal=OLD.placement_ordinal AND p.target_family IS NULL AND i.lifecycle=2 AND i.quantity=0 AND i.last_transaction_id=r.transaction_id
        AND i.state_revision=r.state_revision_before+1 AND r.created_xact_id=pg_current_xact_id()) THEN RETURN NULL; END IF;

    IF EXISTS(SELECT 1 FROM game_spell_item_lines l JOIN game_spell_item_receipts r USING(transaction_id) JOIN game_item_instances i ON i.item_instance_id=l.item_instance_id
      WHERE l.operation_kind IN(5,6) AND l.item_instance_id=OLD.item_instance_id AND l.quantity_after=0
       AND l.source_parent_item_instance_id=OLD.parent_item_instance_id AND l.source_placement_ordinal=OLD.placement_ordinal
       AND i.last_transaction_id=l.transaction_id AND i.lifecycle=2 AND i.quantity=0 AND i.state_revision=l.state_revision_before+1
       AND (l.operation_kind=6 OR r.character_id=OLD.character_id)
       AND l.created_xact_id=pg_current_xact_id() AND r.created_xact_id=pg_current_xact_id()) THEN RETURN NULL; END IF;

    IF EXISTS(SELECT 1 FROM game_character_equipment_receipts r JOIN game_item_instances i USING(item_instance_id)
       WHERE r.operation=1 AND r.to_slot IS NOT NULL AND r.item_instance_id=OLD.item_instance_id
         AND r.character_id=OLD.character_id AND r.backpack_item_instance_id=OLD.parent_item_instance_id
         AND r.backpack_ordinal=OLD.placement_ordinal AND i.last_transaction_id=r.transaction_id
         AND i.state_revision=r.state_revision_before+1 AND r.created_xact_id=pg_current_xact_id()) THEN
        RETURN NULL;
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM game_item_fee_burn_lines l
          JOIN game_item_fee_burns f ON f.transaction_id = l.transaction_id
          JOIN game_item_instances i ON i.item_instance_id = l.item_instance_id
         WHERE l.item_instance_id = OLD.item_instance_id
           AND l.quantity_after = 0
           AND l.placement_ordinal = OLD.placement_ordinal
           AND f.character_id = OLD.character_id
           AND f.backpack_item_instance_id = OLD.parent_item_instance_id
           AND i.last_transaction_id = l.transaction_id
           AND l.created_xact_id = pg_current_xact_id()
           AND f.created_xact_id = pg_current_xact_id())
       AND NOT EXISTS (
        SELECT 1 FROM game_item_timed_state_writes w
          JOIN game_item_instances i ON i.item_instance_id = w.item_instance_id
         WHERE w.item_instance_id = OLD.item_instance_id
           AND w.cause IN (2, 3)
           AND w.holder_character_id = OLD.character_id
           AND (w.definition_before_family, w.definition_before_production_key,
                w.definition_before_revision_ref)
             = (w.definition_after_family, w.definition_after_production_key,
                w.definition_after_revision_ref)
           AND i.lifecycle = 2
           AND i.last_transaction_id = w.transaction_id
           AND w.created_xact_id = pg_current_xact_id())
       AND NOT EXISTS (
        SELECT 1 FROM game_account_bank_coin_lines l
          JOIN game_account_bank_operations o ON o.transaction_id = l.transaction_id
          JOIN game_item_instances i ON i.item_instance_id = l.item_instance_id
         WHERE l.item_instance_id = OLD.item_instance_id
           AND l.direction = 1
           AND l.quantity_after = 0
           AND l.placement_ordinal = OLD.placement_ordinal
           AND o.acting_character_id = OLD.character_id
           AND o.backpack_item_instance_id = OLD.parent_item_instance_id
           AND i.last_transaction_id = l.transaction_id
           AND l.created_xact_id = pg_current_xact_id()
           AND o.created_xact_id = pg_current_xact_id()) THEN
        RAISE EXCEPTION 'backpack entry removal must commit with its fee BURN line'
            USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END;
$$;

CREATE OR REPLACE FUNCTION game_item_placement_proven() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, public, pg_temp AS $$
BEGIN
    -- 0100 player-trade arm BEGIN: bind both new entries and retain capacity locking.
    IF TG_TABLE_NAME = 'game_item_container_entries' THEN
        IF EXISTS (SELECT 1 FROM game_player_trades t
                    WHERE t.transaction_id = NEW.placed_transaction_id
                      AND t.created_xact_id = pg_current_xact_id()) THEN
            IF NOT EXISTS (
                SELECT 1 FROM game_player_trades t
                  JOIN game_player_trade_receipts l USING (occurrence_id)
                 WHERE t.transaction_id = NEW.placed_transaction_id
                   AND t.world_id = NEW.world_id AND l.item_instance_id = NEW.item_instance_id
                   AND l.destination_character_id = NEW.character_id
                   AND l.destination_parent_item_instance_id = NEW.parent_item_instance_id
                   AND l.destination_ordinal = NEW.placement_ordinal
                   AND t.created_xact_id = pg_current_xact_id()
                   AND l.created_xact_id = pg_current_xact_id()) THEN
                RAISE EXCEPTION 'player trade destination must be the exact new direct entry'
                    USING ERRCODE = '23514';
            END IF;
            PERFORM 1 FROM game_item_container_slots
             WHERE item_instance_id = NEW.parent_item_instance_id FOR UPDATE;
            IF (SELECT count(*) FROM game_item_container_entries
                 WHERE parent_item_instance_id = NEW.parent_item_instance_id) > 20 THEN
                RAISE EXCEPTION 'container entry ceiling exceeded' USING ERRCODE = '23514';
            END IF;
            PERFORM game_player_trade_complete((SELECT occurrence_id FROM game_player_trades
                                                WHERE transaction_id = NEW.placed_transaction_id));
            RETURN NULL;
        END IF;
    END IF;
    -- 0100 player-trade arm END.

    -- 0095: exact owner-produced House shape evidence; all older arms below are unchanged.
    IF TG_TABLE_NAME='game_item_container_entries' THEN
     IF EXISTS(SELECT 1 FROM game_item_house_shape_proofs p WHERE p.shape=6 AND p.transaction_id=NEW.placed_transaction_id
      AND p.item_instance_id=NEW.item_instance_id AND p.world_id=NEW.world_id AND p.character_id=NEW.character_id
      AND p.destination_parent=NEW.parent_item_instance_id AND p.destination_ordinal=NEW.placement_ordinal
      AND p.created_xact_id=pg_current_xact_id()) THEN RETURN NULL;END IF;
    END IF;

    IF TG_TABLE_NAME='game_item_container_entries' THEN
    -- 0082: the new entry of an NPC trade MINT line of this same physical transaction.
    IF EXISTS(
       SELECT 1 FROM game_npc_trade_lines l
         JOIN game_npc_trades t ON t.transaction_id = l.transaction_id
         JOIN game_item_instances i ON i.item_instance_id = l.item_instance_id
        WHERE l.transaction_id = NEW.placed_transaction_id
          AND l.item_instance_id = NEW.item_instance_id
          AND l.line_kind IN (2, 4)
          AND l.placement_ordinal = NEW.placement_ordinal
          AND t.character_id = NEW.character_id
          AND t.backpack_item_instance_id = NEW.parent_item_instance_id
          AND i.lifecycle = 1
          AND l.created_xact_id = pg_current_xact_id()
          AND t.created_xact_id = pg_current_xact_id()) THEN
       PERFORM 1 FROM game_item_container_slots WHERE item_instance_id=NEW.parent_item_instance_id FOR UPDATE;
       IF (SELECT count(*) FROM game_item_container_entries e WHERE e.parent_item_instance_id=NEW.parent_item_instance_id)>20 THEN
           RAISE EXCEPTION 'container entry ceiling exceeded' USING ERRCODE='23514';
       END IF;
       RETURN NULL;
    END IF;
    -- 0083: the fresh flask item of an item-use record of this same physical transaction.
    IF EXISTS (SELECT 1 FROM game_item_use_records u JOIN game_item_instances i ON i.item_instance_id = NEW.item_instance_id
       WHERE u.shape = 4 AND u.flask_item_instance_id = NEW.item_instance_id
         AND u.transaction_id = NEW.placed_transaction_id AND u.character_id = NEW.character_id
         AND u.source_parent_item_instance_id = NEW.parent_item_instance_id
         AND u.flask_placement_ordinal = NEW.placement_ordinal
         AND i.lifecycle = 1 AND u.created_xact_id = pg_current_xact_id()) THEN
       PERFORM 1 FROM game_item_container_slots WHERE item_instance_id = NEW.parent_item_instance_id FOR UPDATE;
       IF (SELECT count(*) FROM game_item_container_entries e WHERE e.parent_item_instance_id = NEW.parent_item_instance_id) > 20 THEN
           RAISE EXCEPTION 'container entry ceiling exceeded' USING ERRCODE = '23514';
       END IF;
       RETURN NULL;
    END IF;
    IF EXISTS(
       SELECT 1 FROM game_character_equipment_receipts r JOIN game_item_instances i USING(item_instance_id)
       WHERE r.operation=1 AND r.from_slot IS NOT NULL AND r.item_instance_id=NEW.item_instance_id
         AND r.transaction_id=NEW.placed_transaction_id AND r.character_id=NEW.character_id
         AND r.backpack_item_instance_id=NEW.parent_item_instance_id AND r.backpack_ordinal=NEW.placement_ordinal
         AND i.last_transaction_id=r.transaction_id AND i.state_revision=r.state_revision_before+1
         AND i.lifecycle=1 AND r.created_xact_id=pg_current_xact_id()) THEN
       PERFORM 1 FROM game_item_container_slots WHERE item_instance_id=NEW.parent_item_instance_id FOR UPDATE;
       IF (SELECT count(*) FROM game_item_container_entries e WHERE e.parent_item_instance_id=NEW.parent_item_instance_id)>20 THEN
           RAISE EXCEPTION 'equipment backpack entry ceiling exceeded' USING ERRCODE='23514';
       END IF;
       RETURN NULL;
    END IF;
    IF EXISTS(SELECT 1 FROM game_spell_item_lines l JOIN game_spell_item_receipts r USING(transaction_id)
       WHERE l.operation_kind=1 AND l.item_instance_id=NEW.item_instance_id AND l.transaction_id=NEW.placed_transaction_id
        AND l.destination_parent_item_instance_id=NEW.parent_item_instance_id AND l.destination_ordinal=NEW.placement_ordinal
        AND r.character_id=NEW.character_id AND r.created_xact_id=pg_current_xact_id() AND l.created_xact_id=pg_current_xact_id()) THEN
       PERFORM 1 FROM game_item_container_slots WHERE item_instance_id=NEW.parent_item_instance_id FOR UPDATE;
       IF (SELECT count(*) FROM game_item_container_entries e WHERE e.parent_item_instance_id=NEW.parent_item_instance_id)>20 THEN RAISE EXCEPTION 'container entry ceiling exceeded' USING ERRCODE='23514'; END IF;
       RETURN NULL;
    END IF;
    END IF;
    IF TG_TABLE_NAME = 'game_item_container_slots' THEN
        IF NOT EXISTS (
            SELECT 1 FROM game_item_transfer_receipts r
             WHERE r.transaction_id = NEW.placed_transaction_id
               AND r.character_id = NEW.character_id
               AND r.source_item_instance_id = NEW.item_instance_id
               AND r.shape = 1
               AND r.created_xact_id = pg_current_xact_id()
               AND EXISTS (SELECT 1 FROM game_item_instances i
                            WHERE i.item_instance_id = NEW.item_instance_id AND i.lifecycle = 1)) THEN
            RAISE EXCEPTION 'item placement must commit with its TRANSFER receipt'
                USING ERRCODE = '23514';
        END IF;
    ELSIF TG_TABLE_NAME = 'game_item_container_entries' THEN
        IF NOT EXISTS (
            SELECT 1 FROM game_item_transfer_receipts r
             WHERE r.transaction_id = NEW.placed_transaction_id
               AND r.character_id = NEW.character_id
               AND r.source_item_instance_id = NEW.item_instance_id
               AND r.shape IN (2,4)
               AND r.destination_parent_item_instance_id = NEW.parent_item_instance_id
               AND r.destination_ordinal = NEW.placement_ordinal
               AND r.created_xact_id = pg_current_xact_id()
               AND EXISTS (SELECT 1 FROM game_item_instances i
                            WHERE i.item_instance_id = NEW.item_instance_id AND i.lifecycle = 1))
           AND NOT EXISTS (
            SELECT 1 FROM game_reward_claim_mint_receipts r
             WHERE r.transaction_id = NEW.placed_transaction_id
               AND r.character_id = NEW.character_id
               AND r.item_instance_id = NEW.item_instance_id
               AND r.destination_parent_item_instance_id = NEW.parent_item_instance_id
               AND r.destination_ordinal = NEW.placement_ordinal
               AND r.created_xact_id = pg_current_xact_id()
               AND EXISTS (SELECT 1 FROM game_item_instances i
                            WHERE i.item_instance_id = NEW.item_instance_id AND i.lifecycle = 1))
           AND NOT EXISTS (
            SELECT 1 FROM game_item_fee_burns f
             WHERE f.transaction_id = NEW.placed_transaction_id
               AND f.character_id = NEW.character_id
               AND f.backpack_item_instance_id = NEW.parent_item_instance_id
               AND NEW.item_instance_id IN (f.change_platinum_item_instance_id,
                                            f.change_gold_item_instance_id)
               AND f.created_xact_id = pg_current_xact_id()
               AND EXISTS (SELECT 1 FROM game_item_instances i
                            WHERE i.item_instance_id = NEW.item_instance_id AND i.lifecycle = 1))
           AND NOT EXISTS (
            SELECT 1 FROM game_account_bank_coin_lines l
              JOIN game_account_bank_operations o ON o.transaction_id = l.transaction_id
             WHERE o.transaction_id = NEW.placed_transaction_id
               AND o.acting_character_id = NEW.character_id
               AND o.backpack_item_instance_id = NEW.parent_item_instance_id
               AND l.item_instance_id = NEW.item_instance_id
               AND l.direction = 2
               AND l.placement_ordinal = NEW.placement_ordinal
               AND l.created_xact_id = pg_current_xact_id()
               AND o.created_xact_id = pg_current_xact_id()
               AND EXISTS (SELECT 1 FROM game_item_instances i
                            WHERE i.item_instance_id = NEW.item_instance_id AND i.lifecycle = 1)) THEN
            RAISE EXCEPTION 'item placement must commit with its TRANSFER, reward-claim MINT or fee change receipt'
                USING ERRCODE = '23514';
        END IF;
        PERFORM 1 FROM game_item_container_slots
         WHERE item_instance_id = NEW.parent_item_instance_id FOR UPDATE;
        IF (SELECT count(*) FROM game_item_container_entries e
             WHERE e.parent_item_instance_id = NEW.parent_item_instance_id) > 20 THEN
            RAISE EXCEPTION 'container entry ceiling exceeded' USING ERRCODE = '23514';
        END IF;
    END IF;
    RETURN NULL;
END;
$$;

-- Preserve the installed-schema SECURITY DEFINER properties of the three 0095
-- shared guards. Outcome checks are trigger-only/internal. The separate
-- bounded lock entrypoint returns no authority and performs no write.
DO $$ DECLARE n TEXT; BEGIN
    FOREACH n IN ARRAY ARRAY[
        'game_player_trade_fact_proven', 'game_item_instance_change_proven',
        'game_item_container_entry_removal_proven', 'game_item_placement_proven'
    ] LOOP
        EXECUTE format('ALTER FUNCTION %I() SECURITY DEFINER', n);
        EXECUTE format('ALTER FUNCTION %I() SET search_path = pg_catalog, %I, pg_temp', n, current_schema());
        EXECUTE format('REVOKE ALL ON FUNCTION %I() FROM PUBLIC', n);
    END LOOP;
    EXECUTE format('ALTER FUNCTION game_player_trade_complete(UUID) SET search_path = pg_catalog, %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_player_trade_lock_slots(UUID,UUID) SET search_path = pg_catalog, %I, pg_temp', current_schema());
END $$;

REVOKE ALL ON FUNCTION game_player_trade_lock_slots(UUID,UUID)
    FROM PUBLIC, oteryn_game_runtime, oteryn_game_control;
GRANT EXECUTE ON FUNCTION game_player_trade_lock_slots(UUID,UUID) TO oteryn_game_runtime;
REVOKE ALL ON FUNCTION game_player_trade_complete(UUID), game_player_trade_fact_proven()
    FROM PUBLIC, oteryn_game_runtime, oteryn_game_control;
REVOKE ALL ON game_player_trades, game_player_trade_participants, game_player_trade_receipts
    FROM PUBLIC, oteryn_game_runtime, oteryn_game_control;
GRANT SELECT, INSERT ON game_player_trades, game_player_trade_participants, game_player_trade_receipts
    TO oteryn_game_runtime;
GRANT SELECT ON game_player_trades, game_player_trade_participants, game_player_trade_receipts
    TO oteryn_game_control;
