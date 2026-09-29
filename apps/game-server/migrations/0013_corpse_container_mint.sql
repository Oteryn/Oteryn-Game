-- DUR-03 §39.4 "Corpse container amendment (D3)" (child D3-1;
-- `reviews/OTERYN_GAME_D3_CORPSE_CONTAINER_LOOT_WINDOW_DECAY_DECISION_2026-09-29.md`
-- §4.1/§4.2/§4.4).
--
-- Scope of this migration only (every other §39 obligation is unchanged):
--   * two additive, nullable columns on the corpse's own MINT receipt
--     (`corpse_top_damage_character_id`, `materialized_at`), NOT NULL exactly
--     when `loot_purpose_key = 'CORPSE_MATERIALIZATION'`;
--   * the deferred constraint trigger that writes `materialized_at` from
--     `clock_timestamp()` immediately before commit, and its narrow one-way
--     immutability exception;
--   * the DB-level admission gate for a loot entry's MINT to establish a
--     fresh `Container(parent = <that death's corpse>)` entry instead of
--     Ground, gated on the parent carrying a live `CORPSE_MATERIALIZATION`
--     receipt for the *same* death (extends `game_item_mint_consistency_guard`,
--     the same way the 0012 reward-claim amendment did);
--   * the corpse-row-locked, concurrency-safe `GAMEITEM01-CORPSE-CONTAINER-
--     ENTRIES-MAX` = 16 ceiling on that Container family (mirrors
--     `game_item_placement_proven`'s backpack ceiling, migration 0011);
--   * the per-scope `oteryn:corpse-cap:` advisory transaction lock plus
--     commit-time live-corpse recount that authoritatively enforces the
--     already-accepted `COMBAT01-CORPSES-PER-SCOPE` = 64 (VSL resource-rows
--     decision §4.1 row 6). That lock/recount is Rust-side sequencing inside
--     `durability/item_mint.rs::commit_corpse_mint` (taken before
--     `fence_is_live` and before the corpse's own insert); this migration
--     supplies no new relation for it.
--
-- A full working Rust MINT-into-container caller (freeze/commit) is not
-- delivered here: DUR-03 §39.4 itself defers the `OneItemMintV1.destination`
-- proto widening beyond Ground-only to the D3-6 child, and the composing
-- caller (`combat/death_reward.rs`) is D3-2's. This migration's admission
-- gate and capacity ceiling are proven directly against the schema (D3-1's
-- required PG tests), ready for D3-2/D3-6 to wire a caller against.

-- The corpse's own MINT receipt (loot_purpose_key = 'CORPSE_MATERIALIZATION',
-- draw_ordinal = 0, DUR-03 §4.1) gets two additive nullable columns; every
-- other MINT shape (ordinary Ground loot, reward-claim, and the corpse's own
-- loot entries below) leaves both NULL.
ALTER TABLE game_item_mint_receipts
    ADD COLUMN corpse_top_damage_character_id UUID NULL
        CHECK (corpse_top_damage_character_id IS NULL
               OR game_character_is_uuid_v7(corpse_top_damage_character_id));
ALTER TABLE game_item_mint_receipts
    ADD CONSTRAINT game_item_mint_receipts_corpse_top_damage_scoped CHECK (
        (loot_purpose_key = 'CORPSE_MATERIALIZATION')
        = (corpse_top_damage_character_id IS NOT NULL));
-- D3 §4.1: the corpse's own MINT is always the reserved draw_ordinal = 0
-- sentinel; no ordinary loot entry's own cause ever reuses it. Rust
-- (`commit_corpse_mint`) validates this too, but the DB is the ground truth.
ALTER TABLE game_item_mint_receipts
    ADD CONSTRAINT game_item_mint_receipts_corpse_draw_ordinal_zero CHECK (
        loot_purpose_key <> 'CORPSE_MATERIALIZATION' OR draw_ordinal = 0);

-- D133/D135: the latest pre-commit anchor, written only by the deferred
-- trigger below (never by the runtime candidate, never by
-- statement_timestamp()). NULL at INSERT time even for a corpse receipt; the
-- trigger fills it before this transaction's own commit finalizes, so it is
-- always non-NULL by the time any other transaction can observe the row.
ALTER TABLE game_item_mint_receipts
    ADD COLUMN materialized_at BIGINT NULL CHECK (materialized_at IS NULL OR materialized_at >= 0);
ALTER TABLE game_item_mint_receipts
    ADD CONSTRAINT game_item_mint_receipts_materialized_at_scoped CHECK (
        materialized_at IS NULL OR loot_purpose_key = 'CORPSE_MATERIALIZATION');

-- The Container(parent = corpse) destination a loot entry's own MINT may
-- establish (DUR-03 §39.4 "Loot MINT into the corpse"). NULL for every Ground
-- MINT, including the corpse's own; set exactly for a loot entry minted
-- straight into its death's corpse container.
ALTER TABLE game_item_mint_receipts
    ADD COLUMN destination_parent_item_instance_id UUID NULL;
ALTER TABLE game_item_mint_receipts
    ADD COLUMN destination_ordinal NUMERIC(20,0) NULL
        CHECK (destination_ordinal IS NULL
               OR destination_ordinal BETWEEN 1 AND 18446744073709551615);
ALTER TABLE game_item_mint_receipts
    ADD CONSTRAINT game_item_mint_receipts_destination_paired CHECK (
        (destination_parent_item_instance_id IS NULL) = (destination_ordinal IS NULL));
-- The corpse's own MINT is unamended Ground custody (§4.1): it never itself
-- carries a Container destination.
ALTER TABLE game_item_mint_receipts
    ADD CONSTRAINT game_item_mint_receipts_corpse_is_ground_only CHECK (
        destination_parent_item_instance_id IS NULL
        OR loot_purpose_key <> 'CORPSE_MATERIALIZATION');

-- Container { parent_item_instance_id = <corpse ItemInstance>, entry }
-- (DUR-03 §5.2/§39.4, D131). Unlike the character-scoped
-- game_item_container_entries (migration 0011), a corpse loot entry is not
-- owned by a character until it is picked up (D134), so this is a distinct
-- physical relation for the same logical Container family, not a reuse of
-- the character-scoped table (which requires a live game_item_container_slots
-- parent). GAMEITEM01-CORPSE-PLACEMENT-DEPTH = 1: entries are direct children
-- of the corpse item only.
CREATE TABLE game_item_corpse_container_entries (
    item_instance_id UUID PRIMARY KEY,
    world_id UUID NOT NULL,
    parent_item_instance_id UUID NOT NULL,
    placement_ordinal NUMERIC(20,0) NOT NULL
        CHECK (placement_ordinal BETWEEN 1 AND 18446744073709551615),
    placed_transaction_id UUID NOT NULL UNIQUE
        CHECK (game_character_is_uuid_v7(placed_transaction_id)),
    CHECK (item_instance_id <> parent_item_instance_id),
    UNIQUE (parent_item_instance_id, placement_ordinal),
    FOREIGN KEY (item_instance_id, world_id)
        REFERENCES game_item_instances (item_instance_id, world_id),
    FOREIGN KEY (parent_item_instance_id, world_id)
        REFERENCES game_item_instances (item_instance_id, world_id)
);

-- D133/§39.4: fills `materialized_at` from `clock_timestamp()` (re-evaluated
-- on every call, unlike `statement_timestamp()`), immediately before this
-- transaction's own commit finalizes -- the latest reachable pre-commit
-- point, regardless of how many statements the caller issues afterward.
-- SECURITY DEFINER: the runtime role is granted no UPDATE on this column
-- (only this function, reading its own just-inserted row, ever writes it).
CREATE FUNCTION game_item_mint_receipt_materialize_corpse() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, public, pg_temp AS $$
BEGIN
    IF NEW.loot_purpose_key = 'CORPSE_MATERIALIZATION' THEN
        UPDATE game_item_mint_receipts
           SET materialized_at = floor(extract(epoch FROM clock_timestamp()) * 1000)::bigint
         WHERE death_world_id = NEW.death_world_id
           AND death_channel_id = NEW.death_channel_id
           AND death_scope_ownership_generation = NEW.death_scope_ownership_generation
           AND death_actor_local_id = NEW.death_actor_local_id
           AND death_actor_local_generation = NEW.death_actor_local_generation
           AND loot_table_family = NEW.loot_table_family
           AND loot_table_production_key = NEW.loot_table_production_key
           AND loot_table_revision_ref = NEW.loot_table_revision_ref
           AND loot_purpose_key = NEW.loot_purpose_key
           AND draw_ordinal = NEW.draw_ordinal
           AND materialized_at IS NULL;
    END IF;
    RETURN NULL;
END;
$$;

CREATE CONSTRAINT TRIGGER game_item_mint_receipt_materialize_corpse
    AFTER INSERT ON game_item_mint_receipts
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
    EXECUTE FUNCTION game_item_mint_receipt_materialize_corpse();

-- 0010 made every receipt row fully immutable. Narrow that to admit exactly
-- one one-way transition: `materialized_at` NULL -> a value, for a
-- CORPSE_MATERIALIZATION receipt only, with every other column unchanged --
-- the same idiom the 0011 audit-outbox publication mark already uses. Only
-- the SECURITY DEFINER trigger above performs it, so the runtime role needs
-- no direct UPDATE grant on the column.
DROP TRIGGER game_item_mint_receipt_immutable ON game_item_mint_receipts;
CREATE FUNCTION game_item_mint_receipt_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'UPDATE' AND OLD.loot_purpose_key = 'CORPSE_MATERIALIZATION'
       AND OLD.materialized_at IS NULL AND NEW.materialized_at IS NOT NULL
       AND (to_jsonb(NEW) - 'materialized_at') = (to_jsonb(OLD) - 'materialized_at') THEN
        RETURN NEW;
    END IF;
    RAISE EXCEPTION 'DUR-03 item MINT receipt is immutable except its one-way materialized_at mark'
        USING ERRCODE = '23514';
END;
$$;
CREATE TRIGGER game_item_mint_receipt_guard BEFORE UPDATE OR DELETE
    ON game_item_mint_receipts FOR EACH ROW EXECUTE FUNCTION game_item_mint_receipt_guard();

-- 0010/0012's guard admitted a fresh ItemInstance only as a Ground MINT or a
-- reward-claim-into-backpack MINT. A loot entry's MINT now also establishes
-- an ItemInstance as a fresh Container(parent = corpse) entry: the parent
-- must carry a live CORPSE_MATERIALIZATION receipt for the SAME death
-- (DUR-03 §39.4 "the parent must itself carry a live CORPSE_MATERIALIZATION
-- receipt for the same death key"), committed by an earlier transaction (the
-- corpse's own MINT is never combined with a loot entry's, §39.1).
CREATE OR REPLACE FUNCTION game_item_mint_consistency_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
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
    RAISE EXCEPTION 'item MINT must commit its location, cause receipt and audit event together as its reserved transaction'
        USING ERRCODE = '23514';
END;
$$;

-- The whole-plan preflight's corpse-row locking (D3-1 required scope,
-- §4.2 "the corpse's own entry-count check locks the corpse's row before
-- counting, the same FOR UPDATE-before-count pattern game_item_placement_
-- proven() already uses for the backpack") and GAMEITEM01-CORPSE-CONTAINER-
-- ENTRIES-MAX = 16. SECURITY DEFINER: the row lock needs UPDATE privilege on
-- game_item_mint_receipts, which the runtime role is not granted.
CREATE FUNCTION game_item_corpse_container_entry_proven() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, public, pg_temp AS $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM game_item_mint_receipts r
         WHERE r.transaction_id = NEW.placed_transaction_id
           AND r.item_instance_id = NEW.item_instance_id
           AND r.destination_parent_item_instance_id = NEW.parent_item_instance_id
           AND r.destination_ordinal = NEW.placement_ordinal
           AND r.created_xact_id = pg_current_xact_id()
           AND EXISTS (SELECT 1 FROM game_item_instances i
                        WHERE i.item_instance_id = NEW.item_instance_id AND i.lifecycle = 1)) THEN
        RAISE EXCEPTION 'corpse container placement must commit with its loot MINT receipt'
            USING ERRCODE = '23514';
    END IF;
    -- An item is never both a corpse container entry and on Ground (the
    -- single-location invariant): refuse if a Ground row for it already
    -- exists (whichever insertion lands second within the transaction).
    IF EXISTS (
        SELECT 1 FROM game_item_ground_locations g
         WHERE g.item_instance_id = NEW.item_instance_id) THEN
        RAISE EXCEPTION 'corpse container entry must not also hold a Ground location'
            USING ERRCODE = '23514';
    END IF;
    -- Lock the corpse's own receipt row (its unique CORPSE_MATERIALIZATION /
    -- draw_ordinal = 0 cause, found by the unique item_instance_id) before
    -- counting, so concurrent loot MINTs into the same corpse serialize
    -- instead of racing the count.
    PERFORM 1 FROM game_item_mint_receipts r
     WHERE r.item_instance_id = NEW.parent_item_instance_id
       AND r.loot_purpose_key = 'CORPSE_MATERIALIZATION'
       AND r.draw_ordinal = 0
     FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'corpse container entry must parent a materialized corpse'
            USING ERRCODE = '23514';
    END IF;
    IF (SELECT count(*) FROM game_item_corpse_container_entries e
         WHERE e.parent_item_instance_id = NEW.parent_item_instance_id) > 16 THEN
        RAISE EXCEPTION 'corpse container entry ceiling exceeded' USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END;
$$;

CREATE CONSTRAINT TRIGGER game_item_corpse_container_entry_proven
    AFTER INSERT ON game_item_corpse_container_entries
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
    EXECUTE FUNCTION game_item_corpse_container_entry_proven();

CREATE TRIGGER game_item_corpse_container_entry_immutable BEFORE UPDATE OR DELETE
    ON game_item_corpse_container_entries FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_item_corpse_container_entries_no_truncate BEFORE TRUNCATE
    ON game_item_corpse_container_entries
    FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();

-- 0011/0012's guard admitted a Ground location for a live, never-transferred
-- item with a MINT receipt of the current transaction, provided it is no
-- reward-claim item and holds no (character-scoped) container slot or entry.
-- A corpse container entry is the same kind of hazard (single-location
-- invariant): a forged Ground row for an item that already has a
-- game_item_corpse_container_entries row must also be refused.
CREATE OR REPLACE FUNCTION game_item_ground_insertion_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM game_item_instances i
         WHERE i.item_instance_id = NEW.item_instance_id
           AND i.world_id = NEW.world_id
           AND i.lifecycle = 1
           AND i.last_transaction_id IS NULL)
       OR NOT EXISTS (
        SELECT 1 FROM game_item_mint_receipts r
         WHERE r.item_instance_id = NEW.item_instance_id
           AND r.created_xact_id = pg_current_xact_id())
       OR EXISTS (
        SELECT 1 FROM game_reward_claim_mint_receipts rr
         WHERE rr.item_instance_id = NEW.item_instance_id)
       OR EXISTS (
        SELECT 1 FROM game_item_container_entries e
         WHERE e.item_instance_id = NEW.item_instance_id)
       OR EXISTS (
        SELECT 1 FROM game_item_container_slots s
         WHERE s.item_instance_id = NEW.item_instance_id)
       OR EXISTS (
        SELECT 1 FROM game_item_corpse_container_entries ce
         WHERE ce.item_instance_id = NEW.item_instance_id) THEN
        RAISE EXCEPTION 'Ground placement must be the item MINT of the current transaction'
            USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END;
$$;

DO $$ BEGIN
    EXECUTE format('ALTER FUNCTION game_item_mint_receipt_materialize_corpse() SET search_path = %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_item_mint_receipt_guard() SET search_path = %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_item_mint_consistency_guard() SET search_path = %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_item_corpse_container_entry_proven() SET search_path = %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_item_ground_insertion_guard() SET search_path = %I, pg_temp', current_schema());
END $$;

REVOKE ALL ON game_item_corpse_container_entries FROM PUBLIC;
REVOKE ALL ON FUNCTION
    game_item_mint_receipt_materialize_corpse(),
    game_item_mint_receipt_guard(),
    game_item_corpse_container_entry_proven(),
    game_item_ground_insertion_guard()
FROM PUBLIC;
GRANT SELECT, INSERT ON game_item_corpse_container_entries TO oteryn_game_runtime;
GRANT SELECT ON game_item_corpse_container_entries TO oteryn_game_control;

-- No new grant is needed for corpse_top_damage_character_id: it is written
-- only by the receipt's own INSERT (durability/item_mint.rs::
-- commit_corpse_mint), already covered by 0010's INSERT grant on
-- game_item_mint_receipts. Only materialized_at is ever UPDATEd after
-- insert, and only by the SECURITY DEFINER trigger above -- the runtime role
-- is granted no UPDATE on game_item_mint_receipts at all.
