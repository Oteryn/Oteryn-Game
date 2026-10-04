-- INBOX-1a (HOUSE-RT-INBOX-PACKETS §2.4 and §1.6; MARKET-0 §5, §7 and §8;
-- SOCIAL-MAP-PACKETS-1 §1.12): the `CharacterInbox` location family, its
-- counter, its delivery records and one unreserved delivery function.
--
-- There are no out-shapes, no event, no reservation and no ceiling here: the
-- out-shapes are INBOX-1b's, reservations and the `MARKET0-RL-06` refusal are
-- MARKET-1's. Until a later child adds its own removal clause, an Inbox row
-- has no delete path. Death, `WorldReset` and channel changes never touch the
-- Inbox: no existing writer gains a clause on it.

-- The registry of delivery causes. Created empty: each caller inserts its own
-- kind row in its own migration (HOUSE-1b `HOUSE_DISPOSITION`, MARKET-1 its
-- own). No migration deletes or renames a kind row; the delivery record's FK
-- is RESTRICT. No runtime role has a grant on it.
CREATE TABLE game_character_inbox_cause_kinds (
    cause_kind TEXT PRIMARY KEY CHECK (cause_kind ~ '^[A-Z][A-Z0-9_]{0,63}$')
);

-- One row per Character. `committed` equals the Character's Inbox entry
-- count (kept by trigger; MARKET-1 adds its reservations to it in its own
-- delta). `next_ordinal` only rises, so an ordinal is never reused. There is
-- no refusal on the counter here: `MARKET0-RL-06` is MARKET-1's.
CREATE TABLE game_character_inbox_counters (
    character_id UUID PRIMARY KEY
        REFERENCES game_character_roots (character_id) ON DELETE RESTRICT,
    committed NUMERIC(20,0) NOT NULL DEFAULT 0
        CHECK (committed BETWEEN 0 AND 18446744073709551615),
    next_ordinal NUMERIC(20,0) NOT NULL DEFAULT 1
        CHECK (next_ordinal BETWEEN 1 AND 18446744073709551615)
);

-- CharacterInbox { character_id, ordinal }: one row per whole item without
-- contents. Character + World scope, no ChannelId. Rows are immutable.
CREATE TABLE game_item_character_inbox_locations (
    item_instance_id UUID PRIMARY KEY,
    world_id UUID NOT NULL,
    character_id UUID NOT NULL
        REFERENCES game_character_roots (character_id) ON DELETE RESTRICT,
    ordinal NUMERIC(20,0) NOT NULL
        CHECK (ordinal BETWEEN 1 AND 18446744073709551615),
    UNIQUE (character_id, ordinal),
    FOREIGN KEY (item_instance_id, world_id)
        REFERENCES game_item_instances (item_instance_id, world_id)
);

-- The placement proof: one record per delivery, keyed by (item, cause),
-- written by the delivery function in the same transaction as the Inbox row.
-- Immutable.
CREATE TABLE game_character_inbox_deliveries (
    item_instance_id UUID NOT NULL,
    cause_kind TEXT NOT NULL
        REFERENCES game_character_inbox_cause_kinds (cause_kind)
        ON DELETE RESTRICT ON UPDATE RESTRICT,
    cause_ref TEXT NOT NULL CHECK (octet_length(cause_ref) BETWEEN 1 AND 512),
    world_id UUID NOT NULL,
    character_id UUID NOT NULL
        REFERENCES game_character_roots (character_id) ON DELETE RESTRICT,
    ordinal NUMERIC(20,0) NOT NULL
        CHECK (ordinal BETWEEN 1 AND 18446744073709551615),
    created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id(),
    PRIMARY KEY (item_instance_id, cause_kind, cause_ref),
    UNIQUE (character_id, ordinal),
    FOREIGN KEY (item_instance_id, world_id)
        REFERENCES game_item_instances (item_instance_id, world_id)
);

-- Every location table, the Inbox included, joins this function (0035 body,
-- the union of 0025 and the equipment slots, plus the Inbox count and the
-- Inbox contents rule).
CREATE OR REPLACE FUNCTION game_item_location_exclusive(p_item UUID) RETURNS void
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE
    v_lifecycle SMALLINT;
    v_locations BIGINT;
BEGIN
    -- The row lock serializes two transactions that each place the same item
    -- in a different location table: the later one waits here and then
    -- counts the earlier one's committed row. Every existing writer already
    -- holds this lock from its own item UPDATE.
    SELECT lifecycle INTO v_lifecycle FROM game_item_instances
     WHERE item_instance_id = p_item FOR NO KEY UPDATE;
    IF NOT FOUND THEN
        RETURN;
    END IF;
    v_locations :=
          (SELECT count(*) FROM game_item_ground_locations WHERE item_instance_id = p_item)
        + (SELECT count(*) FROM game_item_container_slots WHERE item_instance_id = p_item)
        + (SELECT count(*) FROM game_item_container_entries WHERE item_instance_id = p_item)
        + (SELECT count(*) FROM game_item_corpse_container_entries WHERE item_instance_id = p_item)
        + (SELECT count(*) FROM game_item_house_interior_locations WHERE item_instance_id = p_item)
        + (SELECT count(*) FROM game_character_equipment_slots WHERE item_instance_id = p_item)
        + (SELECT count(*) FROM game_item_character_inbox_locations WHERE item_instance_id = p_item);
    IF v_locations <> (v_lifecycle = 1)::INT THEN
        RAISE EXCEPTION 'DUR-03 item must have exactly one location while live and none when retired'
            USING ERRCODE = '23514';
    END IF;
    IF EXISTS (SELECT 1 FROM game_item_house_interior_locations WHERE item_instance_id = p_item)
       AND (EXISTS (SELECT 1 FROM game_item_container_entries
                     WHERE parent_item_instance_id = p_item)
            OR EXISTS (SELECT 1 FROM game_item_corpse_container_entries
                        WHERE parent_item_instance_id = p_item)) THEN
        RAISE EXCEPTION 'HouseInterior item cannot have contents' USING ERRCODE = '23514';
    END IF;
    IF EXISTS (SELECT 1 FROM game_item_character_inbox_locations WHERE item_instance_id = p_item)
       AND (EXISTS (SELECT 1 FROM game_item_container_entries
                     WHERE parent_item_instance_id = p_item)
            OR EXISTS (SELECT 1 FROM game_item_corpse_container_entries
                        WHERE parent_item_instance_id = p_item)) THEN
        RAISE EXCEPTION 'CharacterInbox item cannot have contents' USING ERRCODE = '23514';
    END IF;
END;
$$;

-- The Inbox table's own exclusivity trigger. A separate function, not
-- `game_item_location_exclusivity_guard`, so a refused delivery surfaces as
-- a typed Inbox error (OTI01 contents, OTI03 another location) and not as the
-- generic DUR-03 violation.
CREATE FUNCTION game_character_inbox_location_exclusive() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
    IF TG_OP = 'INSERT' THEN
        BEGIN
            PERFORM game_item_location_exclusive(NEW.item_instance_id);
        EXCEPTION WHEN check_violation THEN
            IF EXISTS (SELECT 1 FROM game_item_container_entries
                        WHERE parent_item_instance_id = NEW.item_instance_id)
               OR EXISTS (SELECT 1 FROM game_item_corpse_container_entries
                           WHERE parent_item_instance_id = NEW.item_instance_id) THEN
                RAISE EXCEPTION 'CharacterInbox delivery refused: the item has contents'
                    USING ERRCODE = 'OTI01';
            END IF;
            RAISE EXCEPTION 'CharacterInbox delivery refused: the item is still in another location at commit'
                USING ERRCODE = 'OTI03';
        END;
    ELSE
        PERFORM game_item_location_exclusive(OLD.item_instance_id);
    END IF;
    RETURN NULL;
END;
$$;

-- An Inbox row and its delivery record are written together by the
-- delivery function, in one transaction, naming the same item, Character,
-- World and ordinal; the Character is of that World.
CREATE FUNCTION game_character_inbox_placement_proven() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
    IF NOT EXISTS (
           SELECT 1
             FROM game_item_character_inbox_locations l
             JOIN game_character_inbox_deliveries d
               ON (d.item_instance_id, d.character_id, d.world_id, d.ordinal)
                = (l.item_instance_id, l.character_id, l.world_id, l.ordinal)
             JOIN game_character_roots r
               ON r.character_id = l.character_id AND r.world_id = l.world_id
            WHERE l.item_instance_id = NEW.item_instance_id
              AND l.character_id = NEW.character_id
              AND l.ordinal = NEW.ordinal
              AND d.created_xact_id = pg_current_xact_id()) THEN
        RAISE EXCEPTION 'CharacterInbox row and delivery record must be written together by the delivery'
            USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END;
$$;

-- `committed` follows the entry count.
CREATE FUNCTION game_character_inbox_count() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'INSERT' THEN
        UPDATE game_character_inbox_counters SET committed = committed + 1
         WHERE character_id = NEW.character_id;
        IF NOT FOUND THEN
            RAISE EXCEPTION 'CharacterInbox row requires its Character counter'
                USING ERRCODE = '23514';
        END IF;
    ELSE
        UPDATE game_character_inbox_counters SET committed = committed - 1
         WHERE character_id = OLD.character_id;
    END IF;
    RETURN NULL;
END;
$$;

-- A counter keeps its Character, and its next ordinal never falls.
CREATE FUNCTION game_character_inbox_counter_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'CharacterInbox counter cannot be deleted' USING ERRCODE = '23514';
    END IF;
    IF NEW.character_id <> OLD.character_id OR NEW.next_ordinal < OLD.next_ordinal THEN
        RAISE EXCEPTION 'CharacterInbox counter keeps its Character and never reuses an ordinal'
            USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END;
$$;

-- The delivery (SOCIAL-MAP-PACKETS-1 §1.12): one call places one whole item
-- in the Character's Inbox and returns its location for the caller's own
-- event. Unreserved and never refused for capacity; no session fence, so the
-- recipient may be offline or on another channel. Lock order (MARKET-0 §7,
-- rule 4 extended): the Character root FOR KEY SHARE, the item, then the
-- counter row FOR UPDATE in place of rule 4's `character_root` lock. The key
-- share only keeps the root from being deleted; it does not conflict with a
-- writer's root lock taken in the same order. Refusals, each typed:
--   OTI01 the item has contents (also re-checked at commit);
--   OTI02 the item or the Character is not of the given World;
--   OTI03 the item is already in an Inbox, or still in another location at commit;
--   OTI04 the cause kind has no registry row;
--   OTI05 this (item, cause) was already delivered (the record's key).
-- No event: the caller audits the delivery. The item row is not written.
CREATE FUNCTION game_character_inbox_deliver(
    p_item_instance_id UUID,
    p_character_id UUID,
    p_world_id UUID,
    p_cause_kind TEXT,
    p_cause_ref TEXT
) RETURNS TABLE (character_id UUID, ordinal NUMERIC(20,0))
LANGUAGE plpgsql SECURITY DEFINER AS $$
#variable_conflict use_column
DECLARE
    v_root_world UUID;
    v_item_world UUID;
    v_ordinal NUMERIC(20,0);
BEGIN
    IF p_cause_kind IS NULL OR NOT EXISTS (
           SELECT 1 FROM game_character_inbox_cause_kinds k
            WHERE k.cause_kind = p_cause_kind) THEN
        RAISE EXCEPTION 'CharacterInbox delivery refused: unknown cause kind'
            USING ERRCODE = 'OTI04';
    END IF;
    SELECT r.world_id INTO v_root_world FROM game_character_roots r
     WHERE r.character_id = p_character_id FOR KEY SHARE;
    SELECT i.world_id INTO v_item_world FROM game_item_instances i
     WHERE i.item_instance_id = p_item_instance_id FOR NO KEY UPDATE;
    IF p_world_id IS NULL
       OR v_root_world IS DISTINCT FROM p_world_id
       OR v_item_world IS DISTINCT FROM p_world_id THEN
        RAISE EXCEPTION 'CharacterInbox delivery refused: the item and the Character must be of the given World'
            USING ERRCODE = 'OTI02';
    END IF;
    IF EXISTS (SELECT 1 FROM game_item_container_entries e
                WHERE e.parent_item_instance_id = p_item_instance_id)
       OR EXISTS (SELECT 1 FROM game_item_corpse_container_entries e
                   WHERE e.parent_item_instance_id = p_item_instance_id) THEN
        RAISE EXCEPTION 'CharacterInbox delivery refused: the item has contents'
            USING ERRCODE = 'OTI01';
    END IF;
    IF EXISTS (SELECT 1 FROM game_character_inbox_deliveries d
                WHERE d.item_instance_id = p_item_instance_id
                  AND d.cause_kind = p_cause_kind
                  AND d.cause_ref = p_cause_ref) THEN
        RAISE EXCEPTION 'CharacterInbox delivery refused: this item was already delivered for this cause'
            USING ERRCODE = 'OTI05';
    END IF;
    IF EXISTS (SELECT 1 FROM game_item_character_inbox_locations l
                WHERE l.item_instance_id = p_item_instance_id) THEN
        RAISE EXCEPTION 'CharacterInbox delivery refused: the item is already in an Inbox'
            USING ERRCODE = 'OTI03';
    END IF;
    INSERT INTO game_character_inbox_counters (character_id) VALUES (p_character_id)
        ON CONFLICT DO NOTHING;
    SELECT c.next_ordinal INTO v_ordinal FROM game_character_inbox_counters c
     WHERE c.character_id = p_character_id FOR UPDATE;
    UPDATE game_character_inbox_counters c SET next_ordinal = c.next_ordinal + 1
     WHERE c.character_id = p_character_id;
    INSERT INTO game_character_inbox_deliveries
        (item_instance_id, cause_kind, cause_ref, world_id, character_id, ordinal)
    VALUES (p_item_instance_id, p_cause_kind, p_cause_ref, p_world_id, p_character_id, v_ordinal);
    INSERT INTO game_item_character_inbox_locations
        (item_instance_id, world_id, character_id, ordinal)
    VALUES (p_item_instance_id, p_world_id, p_character_id, v_ordinal);
    RETURN QUERY SELECT p_character_id, v_ordinal;
END;
$$;

-- The consistency guard counts the Inbox among the source's locations (0014
-- body plus the Inbox count).
CREATE OR REPLACE FUNCTION game_item_transfer_consistency_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    src game_item_instances%ROWTYPE;
    rcv game_item_instances%ROWTYPE;
    slot_item UUID;
    src_places INTEGER;
    corpse_ev game_item_corpse_entry_removal_evidence%ROWTYPE;
    from_corpse BOOLEAN;
    corpse_top UUID;
    corpse_materialized BIGINT;
BEGIN
    SELECT * INTO src FROM game_item_instances WHERE item_instance_id = NEW.source_item_instance_id;
    -- D134: the corpse ItemInstance itself is never a legal TRANSFER source.
    IF EXISTS (
        SELECT 1 FROM game_item_mint_receipts m
         WHERE m.item_instance_id = NEW.source_item_instance_id
           AND m.loot_purpose_key = 'CORPSE_MATERIALIZATION') THEN
        RAISE EXCEPTION 'a corpse item is never a TRANSFER source' USING ERRCODE = '23514';
    END IF;
    SELECT s.item_instance_id INTO slot_item FROM game_item_container_slots s
     WHERE s.character_id = NEW.character_id;
    SELECT (SELECT count(*) FROM game_item_ground_locations g
             WHERE g.item_instance_id = src.item_instance_id)
         + (SELECT count(*) FROM game_item_container_slots s
             WHERE s.item_instance_id = src.item_instance_id)
         + (SELECT count(*) FROM game_item_container_entries e
             WHERE e.item_instance_id = src.item_instance_id)
         + (SELECT count(*) FROM game_item_corpse_container_entries ce
             WHERE ce.item_instance_id = src.item_instance_id)
         + (SELECT count(*) FROM game_item_character_inbox_locations ib
             WHERE ib.item_instance_id = src.item_instance_id)
      INTO src_places;
    SELECT * INTO corpse_ev FROM game_item_corpse_entry_removal_evidence ev
     WHERE ev.item_instance_id = NEW.source_item_instance_id
       AND ev.created_xact_id = pg_current_xact_id();
    from_corpse := FOUND;
    IF NOT EXISTS (
           SELECT 1 FROM game_item_transfer_reservations v
            WHERE (v.game_session_id, v.command_id, v.character_id, v.source_item_instance_id,
                   v.intent_binding, v.transaction_id, v.event_id, v.occurred_at)
                = (NEW.game_session_id, NEW.command_id, NEW.character_id,
                   NEW.source_item_instance_id, NEW.intent_binding, NEW.transaction_id,
                   NEW.event_id, NEW.occurred_at)
              AND v.world_id = src.world_id
              AND (v.destination_kind = 1) = (NEW.shape = 1))
       OR NOT EXISTS (
           SELECT 1 FROM game_item_audit_outbox a
            WHERE a.event_id = NEW.event_id AND a.transaction_id = NEW.transaction_id
              AND a.item_instance_id = NEW.source_item_instance_id
              AND a.occurred_at = NEW.occurred_at
              AND a.envelope_sha256 = NEW.envelope_sha256
              AND a.created_xact_id = pg_current_xact_id()
              -- A new TRANSFER's audit event starts pending and unpublished;
              -- only the publisher's acknowledgement may advance it.
              AND a.publication_state = 1
              AND a.published_at IS NULL)
       -- The destination Character must be rooted in the source item's World.
       OR NOT EXISTS (
           SELECT 1 FROM game_character_roots cr
            WHERE cr.character_id = NEW.character_id AND cr.world_id = src.world_id)
       OR NOT EXISTS (
           SELECT 1 FROM game_item_transfer_quantity_evidence ev
            WHERE ev.item_instance_id = NEW.source_item_instance_id
              AND ev.transaction_id = NEW.transaction_id
              AND ev.quantity_before = NEW.source_quantity_before)
       -- The reservation's World and Channel must equal the custody this
       -- TRANSFER actually removed, proven from the guarded pre-DELETE
       -- evidence (the source row is gone by now): a Ground row, or a corpse
       -- entry together with its corpse's live Ground.
       OR NOT EXISTS (
           SELECT 1 FROM game_item_transfer_reservations v
            WHERE v.game_session_id = NEW.game_session_id AND v.command_id = NEW.command_id
              AND (EXISTS (
                     SELECT 1 FROM game_item_ground_removal_evidence gre
                      WHERE gre.item_instance_id = NEW.source_item_instance_id
                        AND gre.world_id = v.world_id
                        AND gre.channel_id = v.channel_id)
                   OR (from_corpse
                       AND corpse_ev.world_id = v.world_id
                       AND corpse_ev.channel_id = v.channel_id)))
       OR src.last_transaction_id IS DISTINCT FROM NEW.transaction_id
       OR src.quantity <> NEW.source_quantity_after
       OR src.lifecycle <> (CASE WHEN NEW.source_quantity_after = 0 THEN 2 ELSE 1 END)
       OR src_places <> (CASE WHEN NEW.shape = 3 THEN 0 ELSE 1 END)
       OR (NEW.shape = 1 AND NOT EXISTS (
           SELECT 1 FROM game_item_container_slots s
            WHERE s.character_id = NEW.character_id
              AND s.item_instance_id = src.item_instance_id
              AND s.placed_transaction_id = NEW.transaction_id))
       OR (NEW.shape IN (2,4) AND NOT EXISTS (
           SELECT 1 FROM game_item_container_entries e
            WHERE e.item_instance_id = src.item_instance_id
              AND e.character_id = NEW.character_id
              AND e.parent_item_instance_id = NEW.destination_parent_item_instance_id
              AND e.parent_item_instance_id = slot_item
              AND e.placement_ordinal = NEW.destination_ordinal
              AND e.placed_transaction_id = NEW.transaction_id)) THEN
        RAISE EXCEPTION 'item TRANSFER must commit its reservation, audit event and complete item state together'
            USING ERRCODE = '23514';
    END IF;
    -- D133: during [materialized_at, materialized_at + 10 s) only the corpse's
    -- top-damage Character may take an entry out; afterwards anyone may. The
    -- deadline is judged by the database clock, never a caller-supplied time.
    IF from_corpse THEN
        SELECT r.corpse_top_damage_character_id, r.materialized_at
          INTO corpse_top, corpse_materialized
          FROM game_item_mint_receipts r
         WHERE r.item_instance_id = corpse_ev.parent_item_instance_id
           AND r.loot_purpose_key = 'CORPSE_MATERIALIZATION'
           AND r.draw_ordinal = 0;
        IF NOT FOUND OR corpse_top IS NULL OR corpse_materialized IS NULL THEN
            RAISE EXCEPTION 'corpse container entry source requires a materialized corpse'
                USING ERRCODE = '23514';
        END IF;
        IF NEW.character_id IS DISTINCT FROM corpse_top
           AND floor(extract(epoch FROM clock_timestamp()) * 1000)::bigint
               < corpse_materialized + 10000 THEN
            RAISE EXCEPTION 'corpse loot is exclusive to its top-damage character until the D133 window closes'
                USING ERRCODE = '23514';
        END IF;
    END IF;
    IF NEW.receiver_item_instance_id IS NOT NULL THEN
        SELECT * INTO rcv FROM game_item_instances
         WHERE item_instance_id = NEW.receiver_item_instance_id;
        IF rcv.last_transaction_id IS DISTINCT FROM NEW.transaction_id
           OR rcv.lifecycle <> 1
           OR rcv.quantity <> NEW.receiver_quantity_after
           OR (rcv.world_id, rcv.definition_family, rcv.definition_production_key,
               rcv.definition_revision_ref)
              IS DISTINCT FROM (src.world_id, src.definition_family,
                                src.definition_production_key, src.definition_revision_ref)
           OR NOT EXISTS (
               SELECT 1 FROM game_item_container_entries e
                WHERE e.item_instance_id = rcv.item_instance_id
                  AND e.character_id = NEW.character_id
                  AND e.parent_item_instance_id = slot_item)
           OR NOT EXISTS (
               SELECT 1 FROM game_item_transfer_quantity_evidence ev
                WHERE ev.item_instance_id = rcv.item_instance_id
                  AND ev.transaction_id = NEW.transaction_id
                  AND ev.quantity_before = NEW.receiver_quantity_before) THEN
            RAISE EXCEPTION 'item TRANSFER receiver must be a compatible stack in the main backpack'
                USING ERRCODE = '23514';
        END IF;
    END IF;
    RETURN NULL;
END;
$$;


CREATE CONSTRAINT TRIGGER game_character_inbox_location_exclusive
    AFTER INSERT OR UPDATE OR DELETE ON game_item_character_inbox_locations
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
    EXECUTE FUNCTION game_character_inbox_location_exclusive();
CREATE CONSTRAINT TRIGGER game_character_inbox_placement_proven
    AFTER INSERT ON game_item_character_inbox_locations
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
    EXECUTE FUNCTION game_character_inbox_placement_proven();
CREATE TRIGGER game_character_inbox_count AFTER INSERT OR DELETE
    ON game_item_character_inbox_locations FOR EACH ROW
    EXECUTE FUNCTION game_character_inbox_count();
CREATE TRIGGER game_item_character_inbox_location_immutable BEFORE UPDATE OR DELETE
    ON game_item_character_inbox_locations FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_item_character_inbox_locations_no_truncate BEFORE TRUNCATE
    ON game_item_character_inbox_locations
    FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();
CREATE TRIGGER game_character_inbox_delivery_immutable BEFORE UPDATE OR DELETE
    ON game_character_inbox_deliveries FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_character_inbox_deliveries_no_truncate BEFORE TRUNCATE
    ON game_character_inbox_deliveries
    FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();
CREATE TRIGGER game_character_inbox_counter_guard BEFORE UPDATE OR DELETE
    ON game_character_inbox_counters FOR EACH ROW
    EXECUTE FUNCTION game_character_inbox_counter_guard();
CREATE TRIGGER game_character_inbox_counters_no_truncate BEFORE TRUNCATE
    ON game_character_inbox_counters
    FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();
CREATE TRIGGER game_character_inbox_cause_kinds_no_truncate BEFORE TRUNCATE
    ON game_character_inbox_cause_kinds
    FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();

DO $$ BEGIN
    EXECUTE format('ALTER FUNCTION game_item_location_exclusive(UUID) SET search_path = %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_item_transfer_consistency_guard() SET search_path = %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_character_inbox_location_exclusive() SET search_path = %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_character_inbox_placement_proven() SET search_path = %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_character_inbox_count() SET search_path = %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_character_inbox_counter_guard() SET search_path = %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_character_inbox_deliver(UUID, UUID, UUID, TEXT, TEXT) SET search_path = %I, pg_temp', current_schema());
END $$;

-- The guard does not re-check rows that already exist, so the migration
-- does it once and fails closed on any existing violation.
DO $$
DECLARE
    v_item UUID;
BEGIN
    FOR v_item IN SELECT item_instance_id FROM game_item_instances LOOP
        PERFORM game_item_location_exclusive(v_item);
    END LOOP;
END $$;

REVOKE ALL ON
    game_character_inbox_cause_kinds,
    game_character_inbox_counters,
    game_item_character_inbox_locations,
    game_character_inbox_deliveries
FROM PUBLIC;
REVOKE ALL ON FUNCTION
    game_item_location_exclusive(UUID),
    game_item_transfer_consistency_guard(),
    game_character_inbox_location_exclusive(),
    game_character_inbox_placement_proven(),
    game_character_inbox_count(),
    game_character_inbox_counter_guard(),
    game_character_inbox_deliver(UUID, UUID, UUID, TEXT, TEXT)
FROM PUBLIC;
-- No runtime role executes the delivery or writes any Inbox table: only a
-- caller's own SECURITY DEFINER function, owned by the migration owner,
-- delivers. The runtime role reads the location table only because the
-- consistency guard, which runs as the TRANSFER writer, counts it.
GRANT SELECT ON game_item_character_inbox_locations TO oteryn_game_runtime;
GRANT SELECT ON
    game_character_inbox_cause_kinds,
    game_character_inbox_counters,
    game_item_character_inbox_locations,
    game_character_inbox_deliveries
TO oteryn_game_control;
