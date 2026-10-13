-- ITEM-USE-1b (control-plane lease; decision `reviews/OTERYN_GAME_ITEM_USE0_USING_ITEMS_DECISION_2026-09-30.md`
-- §4-§5; DUR-03 item-use amendment, Type 2 operation tag 10 `item_use`).
--
-- Scope of this migration only: the durable record of one consumed unit of a stack in the
-- character's main backpack (food or a potion), in four shapes:
--   1 burn            one unit of the used stack S (S keeps its identity, or RETIRES at zero and
--                     its entry is deleted);
--   2 burn + missing  the burn of shape 1 for a potion whose flask had no room (PARITY_PENDING
--                     until ITEM-MOVE-2b); the audit records `flask_missing`;
--   3 flask in stack  the burn plus one unit added to a compatible flask stack E of the same
--                     backpack (E keeps its identity);
--   4 flask in entry  the burn plus a fresh flask item F of quantity 1 in a new entry of the same
--                     backpack (nothing is minted: the unit is conserved from S).
-- It is an item-only transaction under the rule 2 fence: it never advances CharacterRevision. The
-- fence (`connection_generation` = session generation, game session, character lease, scope
-- generation) is asserted by the writer in the same transaction; the record stores the binding
-- the audit event carries. One record per CommandRef (game session, command id), forever: a
-- replay is answered from the record and never burns twice. The record commits only with its
-- item changes, its entry changes and its one audit event (event type 2), all in the same
-- physical transaction; the shared item guards below admit each of those changes only with it.
-- Each replaced function keeps its latest prior body verbatim (0075; the MINT guard from 0077)
-- and only adds one admitting clause.
--
-- Rollback: applied migrations are immutable, so rollback is a new migration. Before any record
-- exists it drops the table, the guard and the four arms. After records exist they are retained
-- evidence of consumed items: a rollback keeps them and may only stop new writes by revoking the
-- INSERT grant.

CREATE TABLE game_item_use_records (
    transaction_id UUID PRIMARY KEY CHECK (game_character_is_uuid_v7(transaction_id)),
    event_id UUID NOT NULL UNIQUE CHECK (game_character_is_uuid_v7(event_id)),
    -- ItemUseCause: 1 = Food, 2 = Potion.
    cause_kind SMALLINT NOT NULL CHECK (cause_kind IN (1, 2)),
    -- The CommandRef that identifies the cause: one record per command, forever.
    game_session_id UUID NOT NULL CHECK (game_character_is_uuid_v7(game_session_id)),
    command_id NUMERIC(20,0) NOT NULL CHECK (command_id BETWEEN 1 AND 18446744073709551615),
    content_revision TEXT NOT NULL CHECK (octet_length(content_revision) BETWEEN 1 AND 512),
    ruleset_revision TEXT NOT NULL CHECK (octet_length(ruleset_revision) BETWEEN 1 AND 512),
    sim_revision TEXT NOT NULL CHECK (octet_length(sim_revision) BETWEEN 1 AND 512),
    -- SHA-256 of the semantic request; a changed request for the same CommandRef conflicts.
    request_binding BYTEA NOT NULL CHECK (octet_length(request_binding) = 32),
    character_id UUID NOT NULL REFERENCES game_character_roots (character_id),
    world_id UUID NOT NULL CHECK (game_character_is_uuid_v7(world_id)),
    channel_id UUID NOT NULL CHECK (game_character_is_uuid_v7(channel_id)),
    runtime_scope_ownership_generation NUMERIC(20,0) NOT NULL
        CHECK (runtime_scope_ownership_generation BETWEEN 1 AND 18446744073709551615),
    -- The rule 2 binding: the session generation fence (= connection_generation) of the use.
    expected_session_generation NUMERIC(20,0) NOT NULL
        CHECK (expected_session_generation BETWEEN 1 AND 18446744073709551615),
    shape SMALLINT NOT NULL CHECK (shape BETWEEN 1 AND 4),
    -- The used stack S, in an entry of the character's main backpack.
    source_item_instance_id UUID NOT NULL REFERENCES game_item_instances (item_instance_id),
    source_definition_family TEXT NOT NULL
        CHECK (octet_length(source_definition_family) BETWEEN 1 AND 128),
    source_definition_production_key TEXT NOT NULL
        CHECK (octet_length(source_definition_production_key) BETWEEN 1 AND 512),
    source_definition_revision_ref TEXT NOT NULL
        CHECK (octet_length(source_definition_revision_ref) BETWEEN 1 AND 512),
    source_parent_item_instance_id UUID NOT NULL,
    source_placement_ordinal NUMERIC(20,0) NOT NULL
        CHECK (source_placement_ordinal BETWEEN 1 AND 18446744073709551615),
    source_quantity_before BIGINT NOT NULL CHECK (source_quantity_before BETWEEN 1 AND 100),
    source_quantity_after BIGINT NOT NULL CHECK (source_quantity_after = source_quantity_before - 1),
    -- The flask output: a stack E growing by one (shape 3) or a fresh item F (shape 4).
    flask_item_instance_id UUID NULL REFERENCES game_item_instances (item_instance_id)
        DEFERRABLE INITIALLY DEFERRED,
    flask_definition_family TEXT NULL
        CHECK (octet_length(flask_definition_family) BETWEEN 1 AND 128),
    flask_definition_production_key TEXT NULL
        CHECK (octet_length(flask_definition_production_key) BETWEEN 1 AND 512),
    flask_definition_revision_ref TEXT NULL
        CHECK (octet_length(flask_definition_revision_ref) BETWEEN 1 AND 512),
    flask_placement_ordinal NUMERIC(20,0) NULL
        CHECK (flask_placement_ordinal BETWEEN 1 AND 18446744073709551615),
    flask_quantity_before BIGINT NULL CHECK (flask_quantity_before BETWEEN 1 AND 99),
    flask_quantity_after BIGINT NULL CHECK (flask_quantity_after BETWEEN 1 AND 100),
    occurred_at BIGINT NOT NULL CHECK (occurred_at > 0),
    envelope_sha256 BYTEA NOT NULL CHECK (octet_length(envelope_sha256) = 32),
    -- Stamped by 0011's trigger, never caller supplied.
    created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id(),
    UNIQUE (game_session_id, command_id),
    CHECK (event_id <> transaction_id),
    -- Shapes 1 and 2 have no flask item; 3 grows a stack by one; 4 plants a fresh unit.
    CHECK ((shape IN (1, 2)) = (flask_item_instance_id IS NULL)),
    CHECK ((shape IN (1, 2)) = (flask_definition_family IS NULL
                                AND flask_definition_production_key IS NULL
                                AND flask_definition_revision_ref IS NULL
                                AND flask_placement_ordinal IS NULL
                                AND flask_quantity_after IS NULL)),
    CHECK ((shape IN (1, 2, 4)) = (flask_quantity_before IS NULL)),
    CHECK (shape <> 3 OR flask_quantity_after = flask_quantity_before + 1),
    CHECK (shape <> 4 OR flask_quantity_after = 1),
    CHECK (flask_item_instance_id IS DISTINCT FROM source_item_instance_id),
    -- Only a potion has a flask, or misses one.
    CHECK (cause_kind = 2 OR shape = 1)
);

-- Commit-time atomicity of one item use. SECURITY INVOKER: the runtime role reads every relation
-- here already.
CREATE FUNCTION game_item_use_consistency_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    src game_item_instances%ROWTYPE;
    fl game_item_instances%ROWTYPE;
    slot_item UUID;
BEGIN
    SELECT * INTO src FROM game_item_instances WHERE item_instance_id = NEW.source_item_instance_id;
    SELECT s.item_instance_id INTO slot_item FROM game_item_container_slots s
     WHERE s.character_id = NEW.character_id;
    IF src.item_instance_id IS NULL
       -- The used stack is a direct entry of the Character's own main backpack.
       OR slot_item IS DISTINCT FROM NEW.source_parent_item_instance_id
       OR src.world_id <> NEW.world_id
       OR (src.definition_family, src.definition_production_key, src.definition_revision_ref)
          <> (NEW.source_definition_family, NEW.source_definition_production_key,
              NEW.source_definition_revision_ref)
       -- Exactly one unit left it, in this transaction.
       OR src.last_transaction_id IS DISTINCT FROM NEW.transaction_id
       OR src.quantity <> NEW.source_quantity_after
       OR src.lifecycle <> (CASE WHEN NEW.source_quantity_after = 0 THEN 2 ELSE 1 END)
       -- The real before-quantity, captured by the item guard.
       OR NOT EXISTS (
           SELECT 1 FROM game_item_transfer_quantity_evidence ev
            WHERE ev.item_instance_id = NEW.source_item_instance_id
              AND ev.transaction_id = NEW.transaction_id
              AND ev.quantity_before = NEW.source_quantity_before)
       -- A spent stack leaves no location (its entry removal is proven by the removal trigger);
       -- a stack with units left keeps its entry.
       OR (NEW.source_quantity_after = 0 AND (
               EXISTS (SELECT 1 FROM game_item_container_entries e
                        WHERE e.item_instance_id = NEW.source_item_instance_id)
            OR EXISTS (SELECT 1 FROM game_item_container_slots s
                        WHERE s.item_instance_id = NEW.source_item_instance_id)
            OR EXISTS (SELECT 1 FROM game_item_ground_locations g
                        WHERE g.item_instance_id = NEW.source_item_instance_id)
            OR EXISTS (SELECT 1 FROM game_item_corpse_container_entries ce
                        WHERE ce.item_instance_id = NEW.source_item_instance_id)))
       OR (NEW.source_quantity_after > 0 AND NOT EXISTS (
               SELECT 1 FROM game_item_container_entries e
                WHERE e.item_instance_id = NEW.source_item_instance_id
                  AND e.character_id = NEW.character_id
                  AND e.parent_item_instance_id = NEW.source_parent_item_instance_id
                  AND e.placement_ordinal = NEW.source_placement_ordinal))
       -- The Character is in this World.
       OR NOT EXISTS (
           SELECT 1 FROM game_character_roots cr
            WHERE cr.character_id = NEW.character_id AND cr.world_id = NEW.world_id)
       -- Its one audit event commits in this same physical transaction.
       OR NOT EXISTS (
           SELECT 1 FROM game_item_audit_outbox a
            WHERE a.event_id = NEW.event_id AND a.transaction_id = NEW.transaction_id
              AND a.item_instance_id = NEW.source_item_instance_id
              AND a.event_type_id = 2
              AND a.occurred_at = NEW.occurred_at
              AND a.envelope_sha256 = NEW.envelope_sha256
              AND a.created_xact_id = pg_current_xact_id()
              AND a.publication_state = 1
              AND a.published_at IS NULL) THEN
        RAISE EXCEPTION 'item use must commit with its used stack, its audit event and its custody together'
            USING ERRCODE = '23514';
    END IF;

    IF NEW.flask_item_instance_id IS NOT NULL THEN
        SELECT * INTO fl FROM game_item_instances WHERE item_instance_id = NEW.flask_item_instance_id;
        IF fl.item_instance_id IS NULL
           OR fl.world_id <> NEW.world_id
           OR fl.lifecycle <> 1
           OR fl.quantity <> NEW.flask_quantity_after
           OR (fl.definition_family, fl.definition_production_key, fl.definition_revision_ref)
              <> (NEW.flask_definition_family, NEW.flask_definition_production_key,
                  NEW.flask_definition_revision_ref)
           -- The flask lies in the same backpack, at its recorded entry.
           OR NOT EXISTS (
               SELECT 1 FROM game_item_container_entries e
                WHERE e.item_instance_id = NEW.flask_item_instance_id
                  AND e.character_id = NEW.character_id
                  AND e.parent_item_instance_id = NEW.source_parent_item_instance_id
                  AND e.placement_ordinal = NEW.flask_placement_ordinal)
           OR (NEW.shape = 3 AND (
                   fl.last_transaction_id IS DISTINCT FROM NEW.transaction_id
                OR NOT EXISTS (
                    SELECT 1 FROM game_item_transfer_quantity_evidence ev
                     WHERE ev.item_instance_id = NEW.flask_item_instance_id
                       AND ev.transaction_id = NEW.transaction_id
                       AND ev.quantity_before = NEW.flask_quantity_before)))
           -- A fresh flask item is created by this transaction, with its entry placed by it.
           OR (NEW.shape = 4 AND (
                   fl.minted_transaction_id <> NEW.transaction_id
                OR NOT EXISTS (
                    SELECT 1 FROM game_item_container_entries e
                     WHERE e.item_instance_id = NEW.flask_item_instance_id
                       AND e.placed_transaction_id = NEW.transaction_id))) THEN
            RAISE EXCEPTION 'item use flask must match its stack or fresh item and entry'
                USING ERRCODE = '23514';
        END IF;
    END IF;
    RETURN NULL;
END;
$$;


CREATE OR REPLACE FUNCTION game_item_instance_change_proven() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
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

CREATE OR REPLACE FUNCTION game_item_mint_consistency_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
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


CREATE CONSTRAINT TRIGGER game_item_use_consistent
    AFTER INSERT ON game_item_use_records
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
    EXECUTE FUNCTION game_item_use_consistency_guard();

CREATE TRIGGER game_item_use_record_immutable BEFORE UPDATE OR DELETE
    ON game_item_use_records FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_item_use_records_no_truncate BEFORE TRUNCATE ON game_item_use_records
    FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();
CREATE TRIGGER game_item_use_records_stamp_xact BEFORE INSERT
    ON game_item_use_records FOR EACH ROW EXECUTE FUNCTION game_item_stamp_created_xact_id();

-- CREATE OR REPLACE clears function-level settings; restore the search_path pins.
DO $$ BEGIN
    EXECUTE format('ALTER FUNCTION game_item_use_consistency_guard() SET search_path = %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_item_instance_change_proven() SET search_path = pg_catalog, %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_item_container_entry_removal_proven() SET search_path = pg_catalog, %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_item_placement_proven() SET search_path = pg_catalog, %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_item_mint_consistency_guard() SET search_path = pg_catalog, %I, pg_temp', current_schema());
END $$;

REVOKE ALL ON game_item_use_records FROM PUBLIC;
REVOKE ALL ON FUNCTION game_item_use_consistency_guard() FROM PUBLIC;
REVOKE ALL ON FUNCTION game_item_placement_proven() FROM PUBLIC;
-- Least privilege: the writer inserts the record; the item, entry and audit grants it already
-- holds (0011, 0023, 0031) are admitted for a use only by the arms above.
GRANT SELECT, INSERT ON game_item_use_records TO oteryn_game_runtime;
GRANT SELECT ON game_item_use_records TO oteryn_game_control;
