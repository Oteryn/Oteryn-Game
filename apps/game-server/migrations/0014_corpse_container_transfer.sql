-- DUR-03 §39.4 "Corpse container amendment (D3)" pickup source (child D3-4;
-- `reviews/OTERYN_GAME_D3_CORPSE_CONTAINER_LOOT_WINDOW_DECAY_DECISION_2026-09-29.md`
-- §4.4/§4.5, decisions D133/D134).
--
-- Scope of this migration only (every other §39 obligation is unchanged):
--   * TRANSFER additionally admits `Container(parent = a live corpse
--     ItemInstance)` as its source (destination unchanged, D80-D83). A corpse
--     entry row (migration 0013, previously fully immutable) may now be
--     DELETEd, but only as the source of a TRANSFER committed in the same
--     physical transaction, with the real pre-DELETE facts captured as guarded
--     evidence by a SECURITY DEFINER trigger (the same idiom as the Ground
--     removal evidence of migration 0011);
--   * the D133 exclusivity window, enforced in the DB commit path with the
--     database clock: during [materialized_at, materialized_at + 10 s) only the
--     corpse's `corpse_top_damage_character_id` may take an entry out;
--   * the corpse ItemInstance itself is never a legal TRANSFER source, whatever
--     it holds (D134 `CorpseNotPickupable`): its Ground row can never be
--     deleted by a TRANSFER, enforced by the Ground-removal constraint trigger
--     and independently by the TRANSFER consistency guard;
--   * the single-location invariant is extended to the new source: a TRANSFER
--     leaves the item in exactly one location (a corpse entry counts as one).
-- Nothing here weakens an existing guard; the guards below only add clauses.

-- Guarded evidence of the real pre-DELETE facts of a corpse container entry
-- and of its parent corpse's live Ground World/Channel, captured from the
-- actual OLD row (never a caller-supplied value). An item leaves a corpse at
-- most once, so at most one row exists per item. Only the SECURITY DEFINER
-- capture trigger writes it.
CREATE TABLE game_item_corpse_entry_removal_evidence (
    item_instance_id UUID PRIMARY KEY REFERENCES game_item_instances (item_instance_id),
    parent_item_instance_id UUID NOT NULL,
    placement_ordinal NUMERIC(20,0) NOT NULL
        CHECK (placement_ordinal BETWEEN 1 AND 18446744073709551615),
    world_id UUID NOT NULL,
    channel_id UUID NOT NULL,
    created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id()
);

CREATE FUNCTION game_item_corpse_entry_removal_evidence_capture() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, public, pg_temp AS $$
BEGIN
    -- The corpse must still be live on Ground: a decayed or retired corpse
    -- (no Ground row) cannot be picked from, and the Ground row is the scope
    -- authority the reservation is later bound to.
    INSERT INTO game_item_corpse_entry_removal_evidence
        (item_instance_id, parent_item_instance_id, placement_ordinal, world_id, channel_id)
    SELECT OLD.item_instance_id, OLD.parent_item_instance_id, OLD.placement_ordinal,
           g.world_id, g.channel_id
      FROM game_item_ground_locations g
      JOIN game_item_instances ci ON ci.item_instance_id = g.item_instance_id
     WHERE g.item_instance_id = OLD.parent_item_instance_id
       AND g.world_id = OLD.world_id
       AND ci.lifecycle = 1;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'corpse container entry removal requires its corpse live on Ground'
            USING ERRCODE = '23514';
    END IF;
    RETURN OLD;
END;
$$;

-- A corpse entry's custody ends only as the source of a TRANSFER committed in
-- the same physical transaction (symmetric to game_item_ground_removal_proven).
-- The receipt's own deferred consistency guard below proves the rest,
-- including the D133 window.
CREATE FUNCTION game_item_corpse_entry_removal_proven() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM game_item_transfer_receipts r
          JOIN game_item_instances i ON i.item_instance_id = r.source_item_instance_id
         WHERE r.source_item_instance_id = OLD.item_instance_id
           AND i.last_transaction_id = r.transaction_id
           AND r.created_xact_id = pg_current_xact_id()) THEN
        RAISE EXCEPTION 'corpse container entry removal must commit with its TRANSFER receipt'
            USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END;
$$;

-- 0013 made the corpse entry table fully immutable. Narrow that: UPDATE stays
-- rejected; DELETE is admitted only through the capture + proof triggers.
DROP TRIGGER game_item_corpse_container_entry_immutable ON game_item_corpse_container_entries;
CREATE TRIGGER game_item_corpse_container_entry_immutable BEFORE UPDATE
    ON game_item_corpse_container_entries FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_item_corpse_entry_removal_evidence_capture BEFORE DELETE
    ON game_item_corpse_container_entries FOR EACH ROW
    EXECUTE FUNCTION game_item_corpse_entry_removal_evidence_capture();
CREATE CONSTRAINT TRIGGER game_item_corpse_entry_removal_proven
    AFTER DELETE ON game_item_corpse_container_entries
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
    EXECUTE FUNCTION game_item_corpse_entry_removal_proven();

CREATE TRIGGER game_item_corpse_entry_removal_evidence_immutable BEFORE UPDATE OR DELETE
    ON game_item_corpse_entry_removal_evidence
    FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_item_corpse_entry_removal_evidence_no_truncate BEFORE TRUNCATE
    ON game_item_corpse_entry_removal_evidence
    FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();

-- D134: a Ground location ends only as the source of a TRANSFER committed in
-- the same transaction (0011) -- and never for a corpse: the corpse
-- ItemInstance is not a legal TRANSFER source, so its Ground row cannot be
-- deleted by a TRANSFER by construction, whether or not it still holds
-- entries. A corpse leaves Ground only through DECAY_RETIRE (child D3-6),
-- which extends this proof for its own transaction.
CREATE OR REPLACE FUNCTION game_item_ground_removal_proven() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM game_item_mint_receipts m
         WHERE m.item_instance_id = OLD.item_instance_id
           AND m.loot_purpose_key = 'CORPSE_MATERIALIZATION') THEN
        RAISE EXCEPTION 'a corpse Ground location is never removed by a TRANSFER'
            USING ERRCODE = '23514';
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM game_item_transfer_receipts r
          JOIN game_item_instances i ON i.item_instance_id = r.source_item_instance_id
         WHERE r.source_item_instance_id = OLD.item_instance_id
           AND i.last_transaction_id = r.transaction_id
           AND r.created_xact_id = pg_current_xact_id()) THEN
        RAISE EXCEPTION 'Ground removal must commit with its TRANSFER receipt'
            USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END;
$$;

-- Commit-time atomicity of one TRANSFER (0011), extended for the corpse
-- container source: the source's actual custody removal is proven from
-- guarded evidence for exactly one of the two source families and matches the
-- reservation's World/Channel; exactly one immediate location remains
-- (a corpse entry counts as a location); the corpse item itself is refused;
-- and a corpse-entry source passes the D133 exclusivity window, judged by the
-- DATABASE clock at commit (`clock_timestamp()`), against the corpse's own
-- committed `materialized_at` and `corpse_top_damage_character_id`.
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

DO $$ BEGIN
    EXECUTE format('ALTER FUNCTION game_item_corpse_entry_removal_evidence_capture() SET search_path = %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_item_corpse_entry_removal_proven() SET search_path = %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_item_ground_removal_proven() SET search_path = %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_item_transfer_consistency_guard() SET search_path = %I, pg_temp', current_schema());
END $$;

REVOKE ALL ON game_item_corpse_entry_removal_evidence FROM PUBLIC;
REVOKE ALL ON FUNCTION
    game_item_corpse_entry_removal_evidence_capture(),
    game_item_corpse_entry_removal_proven()
FROM PUBLIC;
-- Least privilege: the runtime role may DELETE a corpse entry (and only the
-- proof trigger above admits it, only as a TRANSFER source) and read the
-- evidence; only the SECURITY DEFINER capture trigger ever writes it.
GRANT DELETE ON game_item_corpse_container_entries TO oteryn_game_runtime;
GRANT SELECT ON game_item_corpse_entry_removal_evidence TO oteryn_game_runtime;
GRANT SELECT ON game_item_corpse_entry_removal_evidence TO oteryn_game_control;
