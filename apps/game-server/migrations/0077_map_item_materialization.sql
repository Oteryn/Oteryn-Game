-- MAP-OVERLAY-1b map-item MINT (ADR-0021 §4.4; social/map packets §2.6).
--
-- A player pickup of one eligible base-map entry first mints one fresh live
-- ItemInstance onto Ground at the entry's own tile, then moves it with an
-- ordinary B3 TRANSFER in its own transaction. The item, its Ground line, the
-- receipt and the audit event commit together or not at all. Each entry is
-- materialized at most once per (World, Channel, base bundle digest, placement
-- key, reset epoch); another Channel or a later reset epoch can take it again.
-- Reset retirement and the reset record are not part of this migration.

-- One logical map-item MINT per CommandRef, forever. Committed in its own
-- transaction before the first commit pass, it binds the frozen
-- TransactionId, EventId, ItemInstanceId and trusted timestamp, the intent and
-- the DUR03-RL-08 work units already charged. The only mutation is a +1
-- work-unit charge up to 3. Reservations are not unique per entry: only a
-- committed receipt takes the entry.
CREATE TABLE game_map_item_mint_reservations (
    game_session_id UUID NOT NULL CHECK (game_character_is_uuid_v7(game_session_id)),
    command_id NUMERIC(20,0) NOT NULL CHECK (command_id BETWEEN 1 AND 18446744073709551615),
    character_id UUID NOT NULL CHECK (game_character_is_uuid_v7(character_id)),
    world_id UUID NOT NULL CHECK (game_character_is_uuid_v7(world_id)),
    channel_id UUID NOT NULL CHECK (game_character_is_uuid_v7(channel_id)),
    base_bundle_digest BYTEA NOT NULL CHECK (octet_length(base_bundle_digest) = 32),
    -- x<<32 | y<<16 | abs(floor)<<8 | ordinal, x and y u16, floor -15..0, ordinal < 64.
    placement_key NUMERIC(20,0) NOT NULL
        CHECK (placement_key BETWEEN 0 AND 281474976710655
               AND mod(div(placement_key, 256), 256) <= 15
               AND mod(placement_key, 256) < 64),
    reset_epoch NUMERIC(20,0) NOT NULL CHECK (reset_epoch BETWEEN 0 AND 18446744073709551615),
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
CREATE INDEX game_map_item_mint_reservations_entry
    ON game_map_item_mint_reservations (world_id, channel_id, base_bundle_digest, placement_key, reset_epoch);

-- Terminal result of one CommandRef and the one take of its entry.
CREATE TABLE game_map_item_mint_receipts (
    game_session_id UUID NOT NULL,
    command_id NUMERIC(20,0) NOT NULL,
    character_id UUID NOT NULL,
    world_id UUID NOT NULL,
    channel_id UUID NOT NULL,
    base_bundle_digest BYTEA NOT NULL CHECK (octet_length(base_bundle_digest) = 32),
    placement_key NUMERIC(20,0) NOT NULL,
    reset_epoch NUMERIC(20,0) NOT NULL,
    intent_binding BYTEA NOT NULL CHECK (octet_length(intent_binding) = 33),
    transaction_id UUID NOT NULL UNIQUE,
    event_id UUID NOT NULL UNIQUE,
    item_instance_id UUID NOT NULL UNIQUE REFERENCES game_item_instances (item_instance_id),
    definition_family TEXT NOT NULL CHECK (definition_family = 'Item'),
    definition_production_key TEXT NOT NULL
        CHECK (octet_length(definition_production_key) BETWEEN 1 AND 512),
    definition_revision_ref TEXT NOT NULL CHECK (octet_length(definition_revision_ref) BETWEEN 1 AND 512),
    quantity BIGINT NOT NULL CHECK (quantity BETWEEN 1 AND 100),
    runtime_scope_ownership_generation NUMERIC(20,0) NOT NULL
        CHECK (runtime_scope_ownership_generation BETWEEN 1 AND 18446744073709551615),
    spatial_position BYTEA NOT NULL,
    map_revision TEXT NOT NULL CHECK (map_revision = 'sha256:' || encode(base_bundle_digest, 'hex')),
    content_revision TEXT NOT NULL CHECK (octet_length(content_revision) BETWEEN 1 AND 512),
    occurred_at BIGINT NOT NULL CHECK (occurred_at > 0),
    envelope_sha256 BYTEA NOT NULL CHECK (octet_length(envelope_sha256) = 32),
    committed_at BIGINT NOT NULL CHECK (committed_at >= 0),
    -- The physical transaction that inserted this receipt; stamped by the
    -- 0011 trigger, never caller-supplied.
    created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id(),
    PRIMARY KEY (game_session_id, command_id),
    UNIQUE (world_id, channel_id, base_bundle_digest, placement_key, reset_epoch),
    -- The receipt's Ground tile is exactly the tile its placement key names.
    CHECK (spatial_position = int4send(div(placement_key, 4294967296)::integer)
                              || int4send(mod(div(placement_key, 65536), 65536)::integer)
                              || int2send((-mod(div(placement_key, 256), 256))::smallint))
);

CREATE FUNCTION game_map_item_mint_reservation_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'UPDATE' AND NEW.work_units_used = OLD.work_units_used + 1
       AND (to_jsonb(NEW) - 'work_units_used') = (to_jsonb(OLD) - 'work_units_used') THEN
        RETURN NEW;
    END IF;
    RAISE EXCEPTION 'map-item MINT reservation is immutable except its work-unit charge'
        USING ERRCODE = '23514';
END;
$$;

-- Commit-time atomicity of one map-item MINT: its reservation, its pending
-- audit event and the fresh live item with exactly one location, its Ground
-- line at the receipt's tile and frame, all in this physical transaction.
CREATE FUNCTION game_map_item_mint_consistency_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.created_xact_id <> pg_current_xact_id()
       OR NOT EXISTS (
           SELECT 1 FROM game_map_item_mint_reservations v
            WHERE (v.game_session_id, v.command_id, v.character_id, v.world_id, v.channel_id,
                   v.base_bundle_digest, v.placement_key, v.reset_epoch, v.intent_binding,
                   v.transaction_id, v.event_id, v.item_instance_id, v.occurred_at)
                = (NEW.game_session_id, NEW.command_id, NEW.character_id, NEW.world_id, NEW.channel_id,
                   NEW.base_bundle_digest, NEW.placement_key, NEW.reset_epoch, NEW.intent_binding,
                   NEW.transaction_id, NEW.event_id, NEW.item_instance_id, NEW.occurred_at))
       OR NOT EXISTS (
           SELECT 1 FROM game_item_audit_outbox a
            WHERE a.event_id = NEW.event_id AND a.transaction_id = NEW.transaction_id
              AND a.item_instance_id = NEW.item_instance_id
              AND a.occurred_at = NEW.occurred_at
              AND a.envelope_sha256 = NEW.envelope_sha256
              AND a.created_xact_id = pg_current_xact_id()
              AND a.publication_state = 1
              AND a.published_at IS NULL)
       -- The taking Character must be rooted in the entry's World.
       OR NOT EXISTS (
           SELECT 1 FROM game_character_roots cr
            WHERE cr.character_id = NEW.character_id AND cr.world_id = NEW.world_id)
       OR NOT EXISTS (
           SELECT 1 FROM game_item_instances i
             JOIN game_item_ground_locations g ON g.item_instance_id = i.item_instance_id
            WHERE i.item_instance_id = NEW.item_instance_id
              AND i.world_id = NEW.world_id
              AND i.minted_transaction_id = NEW.transaction_id
              AND i.last_transaction_id IS NULL
              AND i.lifecycle = 1
              AND i.state_revision = 1
              AND i.quantity = NEW.quantity
              AND (i.definition_family, i.definition_production_key, i.definition_revision_ref)
                  = (NEW.definition_family, NEW.definition_production_key, NEW.definition_revision_ref)
              AND g.world_id = NEW.world_id
              AND g.channel_id = NEW.channel_id
              AND g.runtime_scope_ownership_generation = NEW.runtime_scope_ownership_generation
              AND g.spatial_position = NEW.spatial_position
              AND g.corpse_ref = convert_to('map-item-materialization-v1', 'UTF8')
              AND g.map_revision = NEW.map_revision
              AND g.content_revision = NEW.content_revision
              AND g.native_room_placement_context = NEW.base_bundle_digest)
       OR EXISTS (SELECT 1 FROM game_item_container_entries e WHERE e.item_instance_id = NEW.item_instance_id)
       OR EXISTS (SELECT 1 FROM game_item_corpse_container_entries e WHERE e.item_instance_id = NEW.item_instance_id)
       OR EXISTS (SELECT 1 FROM game_item_container_slots s WHERE s.item_instance_id = NEW.item_instance_id) THEN
        RAISE EXCEPTION 'map-item MINT must commit its reservation, audit event, item and Ground line together'
            USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END;
$$;

CREATE CONSTRAINT TRIGGER game_map_item_mint_consistent
    AFTER INSERT ON game_map_item_mint_receipts
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
    EXECUTE FUNCTION game_map_item_mint_consistency_guard();
CREATE TRIGGER game_map_item_mint_receipts_stamp_xact BEFORE INSERT
    ON game_map_item_mint_receipts FOR EACH ROW
    EXECUTE FUNCTION game_item_stamp_created_xact_id();
CREATE TRIGGER game_map_item_mint_receipt_immutable BEFORE UPDATE OR DELETE
    ON game_map_item_mint_receipts FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_map_item_mint_reservation_guard BEFORE UPDATE OR DELETE
    ON game_map_item_mint_reservations FOR EACH ROW
    EXECUTE FUNCTION game_map_item_mint_reservation_guard();
CREATE TRIGGER game_map_item_mint_receipts_no_truncate BEFORE TRUNCATE
    ON game_map_item_mint_receipts
    FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();
CREATE TRIGGER game_map_item_mint_reservations_no_truncate BEFORE TRUNCATE
    ON game_map_item_mint_reservations
    FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();

-- 0049's Ground insertion guard, plus the map-item MINT arm before the
-- fallback. Every existing branch is unchanged.
CREATE OR REPLACE FUNCTION game_item_ground_insertion_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN

    IF EXISTS(SELECT 1 FROM game_native_map_item_receipts r JOIN game_item_instances i USING(item_instance_id) JOIN game_native_map_item_audit a USING(item_instance_id)
      WHERE r.item_instance_id=NEW.item_instance_id AND r.transaction_id=i.minted_transaction_id
      AND i.world_id=NEW.world_id AND i.quantity=r.quantity AND i.lifecycle=1 AND i.state_revision=1 AND i.last_transaction_id IS NULL
      AND i.definition_family=r.definition_family AND i.definition_production_key=r.definition_key AND i.definition_revision_ref=r.definition_revision
      AND NEW.world_id=r.world_id AND NEW.channel_id=r.channel_id AND NEW.runtime_scope_ownership_generation=r.scope_generation
      AND NEW.spatial_position=r.spatial_position AND NEW.map_revision=r.map_revision AND NEW.content_revision=r.content_revision
      AND NEW.native_room_placement_context=r.placement_context AND a.event_id=r.event_id AND a.transaction_id=r.transaction_id
      AND a.envelope=r.source_intent AND a.created_xact_id=pg_current_xact_id() AND r.created_xact_id=pg_current_xact_id()) THEN RETURN NULL; END IF;
    IF EXISTS(SELECT 1 FROM game_spell_item_lines l JOIN game_spell_item_receipts r USING(transaction_id)
       JOIN game_item_instances i ON i.item_instance_id=l.item_instance_id
       WHERE l.item_instance_id=NEW.item_instance_id AND l.operation_kind=1
         AND i.lifecycle=1 AND i.last_transaction_id IS NULL AND i.minted_transaction_id=l.transaction_id
         AND r.created_xact_id=pg_current_xact_id() AND l.created_xact_id=pg_current_xact_id()
         AND NOT EXISTS(SELECT 1 FROM game_item_container_entries e WHERE e.item_instance_id=i.item_instance_id)
         AND NOT EXISTS(SELECT 1 FROM game_item_corpse_container_entries e WHERE e.item_instance_id=i.item_instance_id)
         AND NOT EXISTS(SELECT 1 FROM game_item_container_slots s WHERE s.item_instance_id=i.item_instance_id)) THEN RETURN NULL; END IF;
    -- MAP-OVERLAY-1b: the Ground line of a map-item MINT of this transaction.
    IF EXISTS(SELECT 1 FROM game_map_item_mint_receipts r
       JOIN game_item_instances i ON i.item_instance_id=r.item_instance_id
       JOIN game_item_audit_outbox a ON a.event_id=r.event_id
       WHERE r.item_instance_id=NEW.item_instance_id AND r.created_xact_id=pg_current_xact_id()
         AND i.lifecycle=1 AND i.state_revision=1 AND i.last_transaction_id IS NULL
         AND i.minted_transaction_id=r.transaction_id AND i.world_id=r.world_id
         AND a.transaction_id=r.transaction_id AND a.item_instance_id=r.item_instance_id
         AND a.envelope_sha256=r.envelope_sha256 AND a.created_xact_id=pg_current_xact_id()
         AND NEW.world_id=r.world_id AND NEW.channel_id=r.channel_id
         AND NEW.runtime_scope_ownership_generation=r.runtime_scope_ownership_generation
         AND NEW.spatial_position=r.spatial_position
         AND NEW.corpse_ref=convert_to('map-item-materialization-v1','UTF8')
         AND NEW.map_revision=r.map_revision AND NEW.content_revision=r.content_revision
         AND NEW.native_room_placement_context=r.base_bundle_digest
         AND NOT EXISTS(SELECT 1 FROM game_item_container_entries e WHERE e.item_instance_id=i.item_instance_id)
         AND NOT EXISTS(SELECT 1 FROM game_item_corpse_container_entries e WHERE e.item_instance_id=i.item_instance_id)
         AND NOT EXISTS(SELECT 1 FROM game_item_container_slots s WHERE s.item_instance_id=i.item_instance_id)) THEN RETURN NULL; END IF;
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

-- 0075's MINT consistency guard, plus the map-item MINT arm before the RAISE.
-- Every existing branch is unchanged.
CREATE OR REPLACE FUNCTION game_item_mint_consistency_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN

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

-- CREATE OR REPLACE clears function-level settings; restore the search_path pins.
DO $$ DECLARE f TEXT; BEGIN
 FOREACH f IN ARRAY ARRAY['game_item_mint_consistency_guard','game_item_ground_insertion_guard','game_map_item_mint_reservation_guard','game_map_item_mint_consistency_guard'] LOOP
  EXECUTE format('ALTER FUNCTION %I() SET search_path = pg_catalog, %I, pg_temp',f,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %I() FROM PUBLIC',f);
 END LOOP;
END $$;

REVOKE ALL ON game_map_item_mint_reservations, game_map_item_mint_receipts FROM PUBLIC;
GRANT SELECT, INSERT ON game_map_item_mint_reservations, game_map_item_mint_receipts
    TO oteryn_game_runtime;
GRANT UPDATE (work_units_used) ON game_map_item_mint_reservations TO oteryn_game_runtime;
GRANT SELECT ON game_map_item_mint_reservations, game_map_item_mint_receipts
    TO oteryn_game_control;
