-- DUR-03 reward-claim MINT into the equipped main backpack (reward chest
-- decisions D40-D42, §5.1 D92; child CHEST-1).
--
-- A player USE of a reward chest mints exactly one fresh live ItemInstance
-- into a new direct entry of the character's equipped main backpack and
-- records the character's `once` RewardClaim. The item, its entry, the
-- RewardClaim, the receipt and the audit event commit together or not at all.
-- Each logical claim is reserved and committed under its full FND-02
-- CommandRef (game_session_id, command_id). There is no Ground location, no
-- mint into an existing stack, no container reward and no cooldown: the
-- RewardClaim's next_allowed_at stays NULL until the cooldown child. A claim
-- writes no Character root, progression or XP-receipt row (CHARACTER-
-- REVISION-ITEM-TRANSACTION-COMPOSITION-V1 §3.1).

-- D42: one RewardClaim per (character, claim), updated in place and never
-- duplicated. The claim identity is (family, production key); a later claim
-- revision never re-opens it. CHEST-1 admits `once` claims only.
CREATE TABLE game_reward_claims (
    character_id UUID NOT NULL REFERENCES game_character_roots (character_id),
    claim_family TEXT NOT NULL CHECK (octet_length(claim_family) BETWEEN 1 AND 128),
    claim_production_key TEXT NOT NULL
        CHECK (octet_length(claim_production_key) BETWEEN 1 AND 512),
    claim_revision_ref TEXT NOT NULL CHECK (octet_length(claim_revision_ref) BETWEEN 1 AND 512),
    -- Cooldown claims (a later child) store the next allowed time here.
    next_allowed_at BIGINT NULL CHECK (next_allowed_at IS NULL),
    claimed_transaction_id UUID NOT NULL UNIQUE
        CHECK (game_character_is_uuid_v7(claimed_transaction_id)),
    claimed_at BIGINT NOT NULL CHECK (claimed_at >= 0),
    PRIMARY KEY (character_id, claim_family, claim_production_key)
);

-- One logical reward-claim MINT per CommandRef, forever. Committed in its own
-- transaction before the first commit pass, it binds the frozen
-- TransactionId, EventId, ItemInstanceId and trusted timestamp, the intent and
-- the DUR03-RL-08 work units already charged. The only mutation is a +1
-- work-unit charge up to 3.
CREATE TABLE game_reward_claim_mint_reservations (
    game_session_id UUID NOT NULL CHECK (game_character_is_uuid_v7(game_session_id)),
    command_id NUMERIC(20,0) NOT NULL CHECK (command_id BETWEEN 1 AND 18446744073709551615),
    character_id UUID NOT NULL CHECK (game_character_is_uuid_v7(character_id)),
    world_id UUID NOT NULL CHECK (game_character_is_uuid_v7(world_id)),
    channel_id UUID NOT NULL CHECK (game_character_is_uuid_v7(channel_id)),
    claim_family TEXT NOT NULL CHECK (octet_length(claim_family) BETWEEN 1 AND 128),
    claim_production_key TEXT NOT NULL
        CHECK (octet_length(claim_production_key) BETWEEN 1 AND 512),
    claim_revision_ref TEXT NOT NULL CHECK (octet_length(claim_revision_ref) BETWEEN 1 AND 512),
    intent_binding BYTEA NOT NULL CHECK (octet_length(intent_binding) = 33),
    transaction_id UUID NOT NULL UNIQUE CHECK (game_character_is_uuid_v7(transaction_id)),
    event_id UUID NOT NULL UNIQUE CHECK (game_character_is_uuid_v7(event_id)),
    item_instance_id UUID NOT NULL UNIQUE CHECK (game_character_is_uuid_v7(item_instance_id)),
    occurred_at BIGINT NOT NULL CHECK (occurred_at > 0),
    -- DUR03-RL-08: work units per logical transaction.
    work_units_used SMALLINT NOT NULL CHECK (work_units_used BETWEEN 0 AND 3),
    reserved_at BIGINT NOT NULL CHECK (reserved_at >= 0),
    PRIMARY KEY (game_session_id, command_id)
);

-- Terminal result of one CommandRef: one fresh item of 1..100 units (D82) in
-- one new direct entry of the main backpack.
CREATE TABLE game_reward_claim_mint_receipts (
    game_session_id UUID NOT NULL,
    command_id NUMERIC(20,0) NOT NULL,
    character_id UUID NOT NULL,
    claim_family TEXT NOT NULL,
    claim_production_key TEXT NOT NULL,
    claim_revision_ref TEXT NOT NULL,
    intent_binding BYTEA NOT NULL CHECK (octet_length(intent_binding) = 33),
    transaction_id UUID NOT NULL UNIQUE,
    event_id UUID NOT NULL UNIQUE,
    item_instance_id UUID NOT NULL UNIQUE REFERENCES game_item_instances (item_instance_id),
    quantity BIGINT NOT NULL CHECK (quantity BETWEEN 1 AND 100),
    destination_parent_item_instance_id UUID NOT NULL,
    destination_ordinal NUMERIC(20,0) NOT NULL
        CHECK (destination_ordinal BETWEEN 1 AND 18446744073709551615),
    occurred_at BIGINT NOT NULL CHECK (occurred_at > 0),
    envelope_sha256 BYTEA NOT NULL CHECK (octet_length(envelope_sha256) = 32),
    committed_at BIGINT NOT NULL CHECK (committed_at >= 0),
    -- The physical transaction that inserted this receipt; stamped by the
    -- 0011 trigger, never caller-supplied.
    created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id(),
    PRIMARY KEY (game_session_id, command_id),
    CHECK (destination_parent_item_instance_id <> item_instance_id)
);

CREATE FUNCTION game_reward_claim_mint_reservation_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'UPDATE' AND NEW.work_units_used = OLD.work_units_used + 1
       AND (to_jsonb(NEW) - 'work_units_used') = (to_jsonb(OLD) - 'work_units_used') THEN
        RETURN NEW;
    END IF;
    RAISE EXCEPTION 'DUR-03 reward-claim MINT reservation is immutable except its work-unit charge'
        USING ERRCODE = '23514';
END;
$$;

-- A RewardClaim exists only as the claim of a reward-claim MINT receipt
-- inserted by the same physical transaction, for the same character and
-- claim identity.
CREATE FUNCTION game_reward_claim_proven() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM game_reward_claim_mint_receipts r
         WHERE r.transaction_id = NEW.claimed_transaction_id
           AND r.character_id = NEW.character_id
           AND r.claim_family = NEW.claim_family
           AND r.claim_production_key = NEW.claim_production_key
           AND r.claim_revision_ref = NEW.claim_revision_ref
           AND r.created_xact_id = pg_current_xact_id()) THEN
        RAISE EXCEPTION 'RewardClaim must commit with its reward-claim MINT receipt'
            USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END;
$$;

-- Commit-time atomicity of one reward-claim MINT: its reservation, its
-- pending audit event, the fresh live item with exactly one location (its new
-- entry directly in the character's main backpack) and the RewardClaim.
CREATE FUNCTION game_reward_claim_mint_consistency_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    item game_item_instances%ROWTYPE;
    slot_item UUID;
    item_places INTEGER;
BEGIN
    SELECT * INTO item FROM game_item_instances WHERE item_instance_id = NEW.item_instance_id;
    SELECT s.item_instance_id INTO slot_item FROM game_item_container_slots s
     WHERE s.character_id = NEW.character_id;
    SELECT (SELECT count(*) FROM game_item_ground_locations g
             WHERE g.item_instance_id = NEW.item_instance_id)
         + (SELECT count(*) FROM game_item_container_slots s
             WHERE s.item_instance_id = NEW.item_instance_id)
         + (SELECT count(*) FROM game_item_container_entries e
             WHERE e.item_instance_id = NEW.item_instance_id)
      INTO item_places;
    IF NOT EXISTS (
           SELECT 1 FROM game_reward_claim_mint_reservations v
            WHERE (v.game_session_id, v.command_id, v.character_id, v.claim_family,
                   v.claim_production_key, v.claim_revision_ref, v.intent_binding,
                   v.transaction_id, v.event_id, v.item_instance_id, v.occurred_at)
                = (NEW.game_session_id, NEW.command_id, NEW.character_id, NEW.claim_family,
                   NEW.claim_production_key, NEW.claim_revision_ref, NEW.intent_binding,
                   NEW.transaction_id, NEW.event_id, NEW.item_instance_id, NEW.occurred_at)
              AND v.world_id = item.world_id)
       OR NOT EXISTS (
           SELECT 1 FROM game_item_audit_outbox a
            WHERE a.event_id = NEW.event_id AND a.transaction_id = NEW.transaction_id
              AND a.item_instance_id = NEW.item_instance_id
              AND a.occurred_at = NEW.occurred_at
              AND a.envelope_sha256 = NEW.envelope_sha256
              AND a.created_xact_id = pg_current_xact_id()
              AND a.publication_state = 1
              AND a.published_at IS NULL)
       -- The claiming Character must be rooted in the minted item's World.
       OR NOT EXISTS (
           SELECT 1 FROM game_character_roots cr
            WHERE cr.character_id = NEW.character_id AND cr.world_id = item.world_id)
       OR item.item_instance_id IS NULL
       OR item.minted_transaction_id IS DISTINCT FROM NEW.transaction_id
       OR item.last_transaction_id IS NOT NULL
       OR item.lifecycle <> 1
       OR item.quantity <> NEW.quantity
       OR item_places <> 1
       OR slot_item IS NULL
       OR NOT EXISTS (
           SELECT 1 FROM game_item_container_entries e
            WHERE e.item_instance_id = NEW.item_instance_id
              AND e.character_id = NEW.character_id
              AND e.parent_item_instance_id = NEW.destination_parent_item_instance_id
              AND e.parent_item_instance_id = slot_item
              AND e.placement_ordinal = NEW.destination_ordinal
              AND e.placed_transaction_id = NEW.transaction_id)
       OR NOT EXISTS (
           SELECT 1 FROM game_reward_claims c
            WHERE c.character_id = NEW.character_id
              AND c.claim_family = NEW.claim_family
              AND c.claim_production_key = NEW.claim_production_key
              AND c.claim_revision_ref = NEW.claim_revision_ref
              AND c.claimed_transaction_id = NEW.transaction_id
              AND c.next_allowed_at IS NULL) THEN
        RAISE EXCEPTION 'reward-claim MINT must commit its reservation, audit event, item, backpack entry and RewardClaim together'
            USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END;
$$;

-- 0010's guard admitted only a Ground MINT. An ItemInstance now also exists
-- as the fresh item of a reward-claim MINT receipt inserted by the same
-- physical transaction, placed in its new backpack entry with its audit
-- event. The Ground branch is unchanged.
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
    RAISE EXCEPTION 'item MINT must commit its location, cause receipt and audit event together as its reserved transaction'
        USING ERRCODE = '23514';
END;
$$;

-- 0011's guard admitted a backpack entry only as a TRANSFER destination. An
-- entry now also exists as the new entry of a reward-claim MINT receipt
-- inserted by the same physical transaction, for a live item, at the
-- receipt's exact parent and ordinal. The container-slot branch and the
-- concurrency-safe GAMEITEM01-CONTAINER-ENTRIES-MAX count are unchanged.
CREATE OR REPLACE FUNCTION game_item_placement_proven() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, public, pg_temp AS $$
BEGIN
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
                            WHERE i.item_instance_id = NEW.item_instance_id AND i.lifecycle = 1)) THEN
            RAISE EXCEPTION 'item placement must commit with its TRANSFER or reward-claim MINT receipt'
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

-- 0011's guard admitted a Ground location for a live, never-transferred item
-- with a MINT receipt of the current transaction. A reward-claim item is also
-- live and never transferred, so a forged MINT receipt must not give it a
-- second, Ground location: Ground now also requires that the item is no
-- reward-claim item and holds no container slot or entry.
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
         WHERE s.item_instance_id = NEW.item_instance_id) THEN
        RAISE EXCEPTION 'Ground placement must be the item MINT of the current transaction'
            USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END;
$$;

CREATE CONSTRAINT TRIGGER game_reward_claim_mint_consistent
    AFTER INSERT ON game_reward_claim_mint_receipts
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
    EXECUTE FUNCTION game_reward_claim_mint_consistency_guard();
CREATE CONSTRAINT TRIGGER game_reward_claim_proven
    AFTER INSERT ON game_reward_claims
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
    EXECUTE FUNCTION game_reward_claim_proven();
CREATE TRIGGER game_reward_claim_mint_receipts_stamp_xact BEFORE INSERT
    ON game_reward_claim_mint_receipts FOR EACH ROW
    EXECUTE FUNCTION game_item_stamp_created_xact_id();

-- CHEST-1 claims are `once`: the RewardClaim is immutable until the cooldown
-- child narrows this to its in-place update.
CREATE TRIGGER game_reward_claim_immutable BEFORE UPDATE OR DELETE
    ON game_reward_claims FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_reward_claim_mint_receipt_immutable BEFORE UPDATE OR DELETE
    ON game_reward_claim_mint_receipts FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_reward_claim_mint_reservation_guard BEFORE UPDATE OR DELETE
    ON game_reward_claim_mint_reservations FOR EACH ROW
    EXECUTE FUNCTION game_reward_claim_mint_reservation_guard();

CREATE TRIGGER game_reward_claims_no_truncate BEFORE TRUNCATE ON game_reward_claims
    FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();
CREATE TRIGGER game_reward_claim_mint_receipts_no_truncate BEFORE TRUNCATE
    ON game_reward_claim_mint_receipts
    FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();
CREATE TRIGGER game_reward_claim_mint_reservations_no_truncate BEFORE TRUNCATE
    ON game_reward_claim_mint_reservations
    FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();

DO $$ BEGIN
    EXECUTE format('ALTER FUNCTION game_reward_claim_mint_reservation_guard() SET search_path = %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_reward_claim_proven() SET search_path = %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_reward_claim_mint_consistency_guard() SET search_path = %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_item_mint_consistency_guard() SET search_path = %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_item_placement_proven() SET search_path = %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_item_ground_insertion_guard() SET search_path = %I, pg_temp', current_schema());
END $$;

REVOKE ALL ON
    game_reward_claims,
    game_reward_claim_mint_reservations,
    game_reward_claim_mint_receipts
FROM PUBLIC;
REVOKE ALL ON FUNCTION
    game_reward_claim_mint_reservation_guard(),
    game_reward_claim_proven(),
    game_reward_claim_mint_consistency_guard(),
    game_item_mint_consistency_guard(),
    game_item_placement_proven(),
    game_item_ground_insertion_guard()
FROM PUBLIC;
GRANT SELECT, INSERT ON
    game_reward_claims,
    game_reward_claim_mint_reservations,
    game_reward_claim_mint_receipts
TO oteryn_game_runtime;
GRANT UPDATE (work_units_used) ON game_reward_claim_mint_reservations TO oteryn_game_runtime;
GRANT SELECT ON
    game_reward_claims,
    game_reward_claim_mint_reservations,
    game_reward_claim_mint_receipts
TO oteryn_game_control;
