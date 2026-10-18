-- ITEM-LOOP-1 (CP D1438): additive migration for the live item loop.
-- - BAGS-1: nested container entries below the main backpack's direct entries (depth <= 8,
--   at most min(capacity, 20) entries per container, at most 500 items per carried tree).
-- - ITEM-MOVE-2a/2b, STACK-1: one fenced move receipt per CommandRef with one line per touched
--   item (move, merge, split, drop to Ground, pickup from Ground).
-- - STARTER-1: one durable starter grant per Character and template (MINTED or SKIPPED).
-- The existing proof triggers keep their functions unchanged; each is recreated with a WHEN that
-- hands an item carrying a current loop line (or a current starter grant) to the proofs below.
-- PARITY_PENDING: container capacity is a receipt-attested Rust Content fact bound to
-- content_digest, as in 0113; SQL enforces the registered ceilings independently.

CREATE TABLE game_item_nested_entries (
    item_instance_id UUID PRIMARY KEY,
    world_id UUID NOT NULL,
    parent_item_instance_id UUID NOT NULL,
    placement_ordinal NUMERIC(20,0) NOT NULL
        CHECK (placement_ordinal BETWEEN 1 AND 18446744073709551615),
    placed_transaction_id UUID NOT NULL
        CHECK (game_character_is_uuid_v7(placed_transaction_id)),
    CHECK (item_instance_id <> parent_item_instance_id),
    UNIQUE (parent_item_instance_id, placement_ordinal),
    UNIQUE (placed_transaction_id, item_instance_id),
    FOREIGN KEY (item_instance_id, world_id)
        REFERENCES game_item_instances (item_instance_id, world_id),
    FOREIGN KEY (parent_item_instance_id, world_id)
        REFERENCES game_item_instances (item_instance_id, world_id)
);
CREATE INDEX game_item_nested_entries_parent ON game_item_nested_entries (parent_item_instance_id);

CREATE TABLE game_item_loop_move_receipts (
    transaction_id UUID PRIMARY KEY CHECK (game_character_is_uuid_v7(transaction_id)),
    event_id UUID NOT NULL UNIQUE CHECK (game_character_is_uuid_v7(event_id)),
    character_id UUID NOT NULL REFERENCES game_character_roots (character_id),
    game_session_id UUID NOT NULL,
    command_id NUMERIC(20,0) NOT NULL CHECK (command_id BETWEEN 1 AND 18446744073709551615),
    connection_generation NUMERIC(20,0) NOT NULL CHECK (connection_generation > 0),
    lease_generation NUMERIC(20,0) NOT NULL CHECK (lease_generation > 0),
    world_id UUID NOT NULL,
    channel_id UUID NOT NULL,
    scope_generation NUMERIC(20,0) NOT NULL CHECK (scope_generation > 0),
    content_digest BYTEA NOT NULL CHECK (octet_length(content_digest) = 32),
    -- 1 move, 2 merge, 3 split, 4 drop to Ground, 5 pickup from Ground.
    operation SMALLINT NOT NULL CHECK (operation BETWEEN 1 AND 5),
    line_count SMALLINT NOT NULL CHECK (line_count BETWEEN 1 AND 3),
    intent BYTEA NOT NULL CHECK (octet_length(intent) BETWEEN 1 AND 4096),
    intent_sha256 BYTEA NOT NULL CHECK (intent_sha256 = sha256(intent)),
    created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    UNIQUE (game_session_id, command_id)
);

-- Location kinds: 0 none (a fresh item before, a retired item after), 1 main-backpack entry,
-- 2 nested entry, 3 Ground, 9 unchanged (a merge receiver or a split source).
CREATE TABLE game_item_loop_move_lines (
    transaction_id UUID NOT NULL REFERENCES game_item_loop_move_receipts (transaction_id),
    line_ordinal SMALLINT NOT NULL CHECK (line_ordinal BETWEEN 1 AND 3),
    item_instance_id UUID NOT NULL,
    definition_family TEXT NOT NULL,
    definition_production_key TEXT NOT NULL,
    definition_revision_ref TEXT NOT NULL,
    from_kind SMALLINT NOT NULL CHECK (from_kind IN (0, 1, 2, 3, 9)),
    to_kind SMALLINT NOT NULL CHECK (to_kind IN (0, 1, 2, 3, 9)),
    to_parent_item_instance_id UUID,
    to_capacity SMALLINT CHECK (to_capacity BETWEEN 1 AND 20),
    to_spatial_position BYTEA CHECK (octet_length(to_spatial_position) BETWEEN 1 AND 128),
    quantity_before BIGINT NOT NULL CHECK (quantity_before BETWEEN 0 AND 4294967295),
    quantity_after BIGINT NOT NULL CHECK (quantity_after BETWEEN 0 AND 4294967295),
    state_revision_before NUMERIC(20,0)
        CHECK (state_revision_before BETWEEN 1 AND 18446744073709551615),
    created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
    PRIMARY KEY (transaction_id, line_ordinal),
    UNIQUE (transaction_id, item_instance_id),
    FOREIGN KEY (item_instance_id) REFERENCES game_item_instances (item_instance_id)
        DEFERRABLE INITIALLY DEFERRED,
    CHECK ((from_kind = 0) = (state_revision_before IS NULL AND quantity_before = 0)),
    CHECK (from_kind = 0 OR quantity_before > 0),
    CHECK ((to_kind = 0) = (quantity_after = 0)),
    CHECK (from_kind <> 0 OR to_kind <> 0),
    CHECK ((to_kind IN (1, 2)) = (to_parent_item_instance_id IS NOT NULL AND to_capacity IS NOT NULL)),
    CHECK ((to_kind = 3) = (to_spatial_position IS NOT NULL)),
    CHECK ((from_kind = 9) = (to_kind = 9) OR to_kind = 0),
    CHECK (from_kind <> 3 OR to_kind IN (0, 1, 2)),
    CHECK (to_kind <> 3 OR from_kind IN (0, 1, 2))
);

CREATE TABLE game_character_starter_grants (
    character_id UUID NOT NULL REFERENCES game_character_roots (character_id),
    template_key TEXT NOT NULL CHECK (template_key ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'),
    template_revision TEXT NOT NULL CHECK (template_revision ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'),
    world_id UUID NOT NULL,
    definition_family TEXT NOT NULL CHECK (octet_length(definition_family) BETWEEN 1 AND 128),
    definition_production_key TEXT NOT NULL
        CHECK (octet_length(definition_production_key) BETWEEN 1 AND 512),
    definition_revision_ref TEXT NOT NULL
        CHECK (octet_length(definition_revision_ref) BETWEEN 1 AND 512),
    quantity BIGINT NOT NULL CHECK (quantity BETWEEN 1 AND 100),
    -- 1 MINTED into the empty main-backpack slot, 2 SKIPPED (the slot was already occupied).
    outcome SMALLINT NOT NULL CHECK (outcome IN (1, 2)),
    item_instance_id UUID,
    transaction_id UUID NOT NULL UNIQUE CHECK (game_character_is_uuid_v7(transaction_id)),
    event_id UUID NOT NULL UNIQUE CHECK (game_character_is_uuid_v7(event_id)),
    grant_envelope BYTEA NOT NULL CHECK (octet_length(grant_envelope) BETWEEN 1 AND 4096),
    grant_envelope_sha256 BYTEA NOT NULL CHECK (grant_envelope_sha256 = sha256(grant_envelope)),
    created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    PRIMARY KEY (character_id, template_key),
    UNIQUE (item_instance_id),
    FOREIGN KEY (item_instance_id) REFERENCES game_item_instances (item_instance_id)
        DEFERRABLE INITIALLY DEFERRED,
    CHECK ((outcome = 1) = (item_instance_id IS NOT NULL))
);

-- Whether `p_item` carries a loop line of a receipt created in this physical transaction.
-- Evaluated immediately by the proof triggers' WHEN, so a writer inserts its receipt and lines
-- before any custody write of the items they name.
CREATE FUNCTION game_item_loop_line_current(p_item UUID) RETURNS BOOLEAN
LANGUAGE sql STABLE SECURITY DEFINER AS $$
    SELECT EXISTS (
        SELECT 1 FROM game_item_loop_move_lines l
          JOIN game_item_loop_move_receipts r USING (transaction_id)
         WHERE l.item_instance_id = p_item
           AND l.created_xact_id = pg_current_xact_id()
           AND r.created_xact_id = pg_current_xact_id())
$$;

CREATE FUNCTION game_item_starter_grant_current(p_item UUID) RETURNS BOOLEAN
LANGUAGE sql STABLE SECURITY DEFINER AS $$
    SELECT EXISTS (
        SELECT 1 FROM game_character_starter_grants g
         WHERE g.item_instance_id = p_item AND g.outcome = 1
           AND g.created_xact_id = pg_current_xact_id())
$$;

-- The Character whose main backpack tree holds `p_item`, and the item's depth in it (the main
-- backpack 0, its direct entries 1). NULL owner when the item is not in a carried tree, or the
-- chain is deeper than any admitted tree (a cycle included).
CREATE FUNCTION game_item_loop_owner(p_item UUID, OUT owner_character UUID, OUT depth INT)
LANGUAGE plpgsql STABLE SECURITY DEFINER AS $$
DECLARE
    v_item UUID := p_item;
    v_parent UUID;
    v_character UUID;
    v_hops INT := 0;
BEGIN
    owner_character := NULL;
    depth := NULL;
    LOOP
        SELECT character_id INTO v_character FROM game_item_container_entries
         WHERE item_instance_id = v_item;
        IF FOUND THEN
            owner_character := v_character;
            depth := v_hops + 1;
            RETURN;
        END IF;
        SELECT character_id INTO v_character FROM game_item_container_slots
         WHERE item_instance_id = v_item;
        IF FOUND THEN
            owner_character := v_character;
            depth := v_hops;
            RETURN;
        END IF;
        SELECT parent_item_instance_id INTO v_parent FROM game_item_nested_entries
         WHERE item_instance_id = v_item;
        IF NOT FOUND THEN
            RETURN;
        END IF;
        v_hops := v_hops + 1;
        IF v_hops > 16 THEN
            RETURN;
        END IF;
        v_item := v_parent;
    END LOOP;
END;
$$;

-- The nested subtree below `p_item`: its height (0 without contents) and size (root included).
CREATE FUNCTION game_item_loop_subtree(p_item UUID, OUT height INT, OUT size BIGINT)
LANGUAGE sql STABLE SECURITY DEFINER AS $$
    WITH RECURSIVE t(item, level, path) AS (
        SELECT p_item, 0, ARRAY[p_item]
        UNION ALL
        SELECT n.item_instance_id, t.level + 1, t.path || n.item_instance_id
          FROM t JOIN game_item_nested_entries n ON n.parent_item_instance_id = t.item
         WHERE t.level < 16 AND NOT n.item_instance_id = ANY (t.path))
    SELECT max(level), count(*) FROM t
$$;

-- Every item below the Character's main backpack.
CREATE FUNCTION game_item_loop_tree_size(p_character UUID) RETURNS BIGINT
LANGUAGE sql STABLE SECURITY DEFINER AS $$
    WITH RECURSIVE t(item, level) AS (
        SELECT item_instance_id, 1 FROM game_item_container_entries WHERE character_id = p_character
        UNION ALL
        SELECT n.item_instance_id, t.level + 1
          FROM t JOIN game_item_nested_entries n ON n.parent_item_instance_id = t.item
         WHERE t.level < 16)
    SELECT count(*) FROM t
$$;

CREATE FUNCTION game_item_loop_drop_ref(p_ref BYTEA) RETURNS BOOLEAN
LANGUAGE sql IMMUTABLE SET search_path = pg_catalog, pg_temp AS $$
    SELECT position(convert_to('OTERYN_ITEM_LOOP_DROP/v1:', 'UTF8') IN p_ref) = 1
$$;

-- Immediate: a line names its item's exact current state and source location at insertion,
-- before any custody write of this transaction touches the item.
CREATE FUNCTION game_item_loop_line_admit() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE
    r game_item_loop_move_receipts%ROWTYPE;
    i game_item_instances%ROWTYPE;
    g game_item_ground_locations%ROWTYPE;
    v_owner UUID;
    v_depth INT;
BEGIN
    SELECT * INTO r FROM game_item_loop_move_receipts
     WHERE transaction_id = NEW.transaction_id AND created_xact_id = pg_current_xact_id();
    IF NOT FOUND OR NEW.created_xact_id <> pg_current_xact_id() THEN
        RAISE EXCEPTION 'item loop line requires its receipt of this transaction' USING ERRCODE = '23514';
    END IF;
    IF NEW.from_kind = 0 THEN
        IF EXISTS (SELECT 1 FROM game_item_instances WHERE item_instance_id = NEW.item_instance_id) THEN
            RAISE EXCEPTION 'item loop fresh item already exists' USING ERRCODE = '23514';
        END IF;
        RETURN NEW;
    END IF;
    SELECT * INTO i FROM game_item_instances WHERE item_instance_id = NEW.item_instance_id;
    IF NOT FOUND OR i.lifecycle <> 1 OR i.world_id <> r.world_id
       OR i.quantity <> NEW.quantity_before OR i.state_revision <> NEW.state_revision_before
       OR (i.definition_family, i.definition_production_key, i.definition_revision_ref)
          <> (NEW.definition_family, NEW.definition_production_key, NEW.definition_revision_ref) THEN
        RAISE EXCEPTION 'item loop line must name the exact current item' USING ERRCODE = '23514';
    END IF;
    -- The Game node hosts no timed or temporal lane: such an item never moves through the loop.
    IF EXISTS (SELECT 1 FROM game_item_timed_states WHERE item_instance_id = NEW.item_instance_id)
       OR EXISTS (SELECT 1 FROM game_spell_item_temporal_schedules WHERE item_instance_id = NEW.item_instance_id)
       OR EXISTS (SELECT 1 FROM game_spell_field_temporal_schedules WHERE item_instance_id = NEW.item_instance_id) THEN
        RAISE EXCEPTION 'item loop timed item not hosted' USING ERRCODE = '23514';
    END IF;
    IF NEW.from_kind = 3 THEN
        SELECT * INTO g FROM game_item_ground_locations WHERE item_instance_id = NEW.item_instance_id;
        IF NOT FOUND OR g.world_id <> r.world_id OR g.channel_id <> r.channel_id
           OR NOT game_item_loop_drop_ref(g.corpse_ref) THEN
            RAISE EXCEPTION 'item loop pickup source is not a loop Ground item of this Channel'
                USING ERRCODE = '23514';
        END IF;
        RETURN NEW;
    END IF;
    SELECT o.owner_character, o.depth INTO v_owner, v_depth FROM game_item_loop_owner(NEW.item_instance_id) o;
    IF v_owner IS DISTINCT FROM r.character_id OR v_depth < 1
       OR (NEW.from_kind = 1 AND NOT EXISTS (SELECT 1 FROM game_item_container_entries
            WHERE item_instance_id = NEW.item_instance_id AND character_id = r.character_id))
       OR (NEW.from_kind = 2 AND NOT EXISTS (SELECT 1 FROM game_item_nested_entries
            WHERE item_instance_id = NEW.item_instance_id)) THEN
        RAISE EXCEPTION 'item loop source is not in the Character tree' USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END;
$$;

-- Deferred: the item's final state and location are exactly the line's.
CREATE FUNCTION game_item_loop_line_proven() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE
    r game_item_loop_move_receipts%ROWTYPE;
    i game_item_instances%ROWTYPE;
    e game_item_container_entries%ROWTYPE;
    n game_item_nested_entries%ROWTYPE;
    g game_item_ground_locations%ROWTYPE;
    v_owner UUID;
    v_depth INT;
    v_height INT;
    v_size BIGINT;
BEGIN
    SELECT * INTO STRICT r FROM game_item_loop_move_receipts WHERE transaction_id = NEW.transaction_id;
    SELECT * INTO i FROM game_item_instances WHERE item_instance_id = NEW.item_instance_id;
    IF NOT FOUND OR i.world_id <> r.world_id
       OR (i.definition_family, i.definition_production_key, i.definition_revision_ref)
          <> (NEW.definition_family, NEW.definition_production_key, NEW.definition_revision_ref) THEN
        RAISE EXCEPTION 'item loop line item is missing' USING ERRCODE = '23514';
    END IF;
    IF NEW.from_kind = 0 THEN
        IF i.minted_transaction_id <> NEW.transaction_id OR i.last_transaction_id IS NOT NULL
           OR i.state_revision <> 1 THEN
            RAISE EXCEPTION 'item loop fresh item must be minted by its receipt' USING ERRCODE = '23514';
        END IF;
    ELSIF i.last_transaction_id IS DISTINCT FROM NEW.transaction_id
          OR i.state_revision <> NEW.state_revision_before + 1 THEN
        RAISE EXCEPTION 'item loop item must change exactly once in its receipt' USING ERRCODE = '23514';
    END IF;
    IF NEW.to_kind = 0 THEN
        IF i.lifecycle <> 2 OR i.quantity <> 0 THEN
            RAISE EXCEPTION 'item loop retired item must be retired' USING ERRCODE = '23514';
        END IF;
        RETURN NULL;
    END IF;
    IF i.lifecycle <> 1 OR i.quantity <> NEW.quantity_after OR NEW.quantity_after > 100 THEN
        RAISE EXCEPTION 'item loop item quantity differs from its line' USING ERRCODE = '23514';
    END IF;
    IF NEW.to_kind = 1 THEN
        SELECT * INTO e FROM game_item_container_entries WHERE item_instance_id = NEW.item_instance_id;
        IF NOT FOUND OR e.character_id <> r.character_id OR e.world_id <> r.world_id
           OR e.parent_item_instance_id <> NEW.to_parent_item_instance_id
           OR e.placed_transaction_id <> NEW.transaction_id
           OR NOT EXISTS (SELECT 1 FROM game_item_container_slots s WHERE s.character_id = r.character_id
                AND s.item_instance_id = NEW.to_parent_item_instance_id)
           OR (SELECT count(*) FROM game_item_container_entries
                WHERE parent_item_instance_id = NEW.to_parent_item_instance_id) > NEW.to_capacity THEN
            RAISE EXCEPTION 'item loop main-backpack destination differs from its line' USING ERRCODE = '23514';
        END IF;
        v_depth := 0;
    ELSIF NEW.to_kind = 2 THEN
        SELECT * INTO n FROM game_item_nested_entries WHERE item_instance_id = NEW.item_instance_id;
        SELECT o.owner_character, o.depth INTO v_owner, v_depth
          FROM game_item_loop_owner(NEW.to_parent_item_instance_id) o;
        IF NOT FOUND OR n.item_instance_id IS NULL OR n.world_id <> r.world_id
           OR n.parent_item_instance_id <> NEW.to_parent_item_instance_id
           OR n.placed_transaction_id <> NEW.transaction_id
           OR v_owner IS DISTINCT FROM r.character_id OR v_depth < 1
           OR (SELECT count(*) FROM game_item_nested_entries
                WHERE parent_item_instance_id = NEW.to_parent_item_instance_id) > NEW.to_capacity THEN
            RAISE EXCEPTION 'item loop container destination differs from its line' USING ERRCODE = '23514';
        END IF;
    ELSIF NEW.to_kind = 3 THEN
        SELECT * INTO g FROM game_item_ground_locations WHERE item_instance_id = NEW.item_instance_id;
        IF NOT FOUND OR g.world_id <> r.world_id OR g.channel_id <> r.channel_id
           OR g.runtime_scope_ownership_generation <> r.scope_generation
           OR g.spatial_position <> NEW.to_spatial_position OR NOT game_item_loop_drop_ref(g.corpse_ref)
           OR (SELECT count(*) FROM game_item_ground_locations x WHERE x.world_id = g.world_id
                AND x.channel_id = g.channel_id AND x.spatial_position = g.spatial_position) > 64 THEN
            RAISE EXCEPTION 'item loop Ground destination differs from its line' USING ERRCODE = '23514';
        END IF;
        RETURN NULL;
    ELSE
        SELECT o.owner_character, o.depth INTO v_owner, v_depth FROM game_item_loop_owner(NEW.item_instance_id) o;
        IF v_owner IS DISTINCT FROM r.character_id OR v_depth < 1
           OR EXISTS (SELECT 1 FROM game_item_container_entries WHERE item_instance_id = NEW.item_instance_id
                AND placed_transaction_id = NEW.transaction_id)
           OR EXISTS (SELECT 1 FROM game_item_nested_entries WHERE item_instance_id = NEW.item_instance_id
                AND placed_transaction_id = NEW.transaction_id) THEN
            RAISE EXCEPTION 'item loop unchanged item moved' USING ERRCODE = '23514';
        END IF;
        RETURN NULL;
    END IF;
    -- BAGS-1 ceilings: depth <= 8 for every descendant, <= 500 items in the carried tree.
    SELECT t.height, t.size INTO v_height, v_size FROM game_item_loop_subtree(NEW.item_instance_id) t;
    IF v_depth + 1 + v_height > 8
       OR game_item_loop_tree_size(r.character_id) > 500 THEN
        RAISE EXCEPTION 'item loop container tree exceeds its ceilings' USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END;
$$;

-- Deferred: the receipt's lines are complete, of one definition, and conserve quantity.
CREATE FUNCTION game_item_loop_receipt_proven() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE
    v_lines BIGINT;
    v_first SMALLINT;
    v_last SMALLINT;
    v_before NUMERIC;
    v_after NUMERIC;
    v_definitions BIGINT;
    v_fresh BIGINT;
BEGIN
    SELECT count(*), min(line_ordinal), max(line_ordinal), sum(quantity_before), sum(quantity_after),
           count(DISTINCT (definition_family, definition_production_key, definition_revision_ref)),
           count(*) FILTER (WHERE from_kind = 0)
      INTO v_lines, v_first, v_last, v_before, v_after, v_definitions, v_fresh
      FROM game_item_loop_move_lines WHERE transaction_id = NEW.transaction_id;
    IF v_lines <> NEW.line_count OR v_first <> 1 OR v_last <> NEW.line_count
       OR v_before <> v_after OR v_definitions <> 1 OR v_fresh > 1
       OR NOT EXISTS (SELECT 1 FROM game_character_roots c WHERE c.character_id = NEW.character_id
            AND c.world_id = NEW.world_id) THEN
        RAISE EXCEPTION 'item loop receipt lines are incomplete or do not conserve quantity'
            USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END;
$$;

-- Deferred: nested custody changes only under a current loop line of the same item.
CREATE FUNCTION game_item_nested_entry_proven() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
    IF TG_OP = 'INSERT' AND game_item_loop_line_current(NEW.item_instance_id) THEN
        RETURN NULL;
    END IF;
    IF TG_OP = 'DELETE' AND game_item_loop_line_current(OLD.item_instance_id) THEN
        RETURN NULL;
    END IF;
    RAISE EXCEPTION 'nested container custody changes only by an item loop move' USING ERRCODE = '23514';
END;
$$;

-- Deferred: a MINTED grant's item is fresh and is the Character's main backpack; a SKIPPED grant
-- found the slot occupied.
CREATE FUNCTION game_character_starter_grant_proven() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE
    i game_item_instances%ROWTYPE;
BEGIN
    IF NOT EXISTS (SELECT 1 FROM game_character_roots c WHERE c.character_id = NEW.character_id
         AND c.world_id = NEW.world_id) THEN
        RAISE EXCEPTION 'starter grant Character world' USING ERRCODE = '23514';
    END IF;
    IF NEW.outcome = 2 THEN
        IF NOT EXISTS (SELECT 1 FROM game_item_container_slots WHERE character_id = NEW.character_id) THEN
            RAISE EXCEPTION 'starter grant skipped an empty main-backpack slot' USING ERRCODE = '23514';
        END IF;
        RETURN NULL;
    END IF;
    SELECT * INTO i FROM game_item_instances WHERE item_instance_id = NEW.item_instance_id;
    IF NOT FOUND OR i.world_id <> NEW.world_id OR i.lifecycle <> 1 OR i.quantity <> NEW.quantity
       OR i.minted_transaction_id <> NEW.transaction_id OR i.last_transaction_id IS NOT NULL
       OR i.state_revision <> 1
       OR (i.definition_family, i.definition_production_key, i.definition_revision_ref)
          <> (NEW.definition_family, NEW.definition_production_key, NEW.definition_revision_ref)
       OR NOT EXISTS (SELECT 1 FROM game_item_container_slots s WHERE s.character_id = NEW.character_id
            AND s.item_instance_id = NEW.item_instance_id AND s.world_id = NEW.world_id
            AND s.placed_transaction_id = NEW.transaction_id) THEN
        RAISE EXCEPTION 'starter grant item is not its fresh main backpack' USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END;
$$;

-- Exclusivity (0076 owner) extended by the nested entries: a nested entry is a location, a
-- HouseInterior or CharacterInbox item holds none, and only a live item holds any.
CREATE OR REPLACE FUNCTION game_item_location_exclusive(p_item UUID) RETURNS void
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE
    v_lifecycle SMALLINT;
    v_locations BIGINT;
BEGIN
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
        + (SELECT count(*) FROM game_item_character_inbox_locations WHERE item_instance_id = p_item)
        + (SELECT count(*) FROM game_item_nested_entries WHERE item_instance_id = p_item);
    IF v_locations <> (v_lifecycle = 1)::INT THEN
        RAISE EXCEPTION 'DUR-03 item must have exactly one location while live and none when retired'
            USING ERRCODE = '23514';
    END IF;
    IF v_lifecycle <> 1 AND EXISTS (SELECT 1 FROM game_item_nested_entries
                                     WHERE parent_item_instance_id = p_item) THEN
        RAISE EXCEPTION 'a retired item cannot hold nested contents' USING ERRCODE = '23514';
    END IF;
    IF EXISTS (SELECT 1 FROM game_item_house_interior_locations WHERE item_instance_id = p_item)
       AND (EXISTS (SELECT 1 FROM game_item_container_entries
                     WHERE parent_item_instance_id = p_item)
            OR EXISTS (SELECT 1 FROM game_item_corpse_container_entries
                        WHERE parent_item_instance_id = p_item)
            OR EXISTS (SELECT 1 FROM game_item_nested_entries
                        WHERE parent_item_instance_id = p_item)) THEN
        RAISE EXCEPTION 'HouseInterior item cannot have contents' USING ERRCODE = '23514';
    END IF;
    IF EXISTS (SELECT 1 FROM game_item_character_inbox_locations WHERE item_instance_id = p_item)
       AND (EXISTS (SELECT 1 FROM game_item_container_entries
                     WHERE parent_item_instance_id = p_item)
            OR EXISTS (SELECT 1 FROM game_item_corpse_container_entries
                        WHERE parent_item_instance_id = p_item)
            OR EXISTS (SELECT 1 FROM game_item_nested_entries
                        WHERE parent_item_instance_id = p_item)) THEN
        RAISE EXCEPTION 'CharacterInbox item cannot have contents' USING ERRCODE = '23514';
    END IF;
END;
$$;

CREATE OR REPLACE FUNCTION game_item_location_exclusivity_guard() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
    IF TG_OP <> 'DELETE' THEN
        PERFORM game_item_location_exclusive(NEW.item_instance_id);
        IF TG_TABLE_NAME IN ('game_item_container_entries', 'game_item_corpse_container_entries',
                             'game_item_nested_entries') THEN
            PERFORM game_item_location_exclusive(NEW.parent_item_instance_id);
        END IF;
    END IF;
    IF TG_OP <> 'INSERT' THEN
        PERFORM game_item_location_exclusive(OLD.item_instance_id);
        IF TG_TABLE_NAME = 'game_item_nested_entries' THEN
            PERFORM game_item_location_exclusive(OLD.parent_item_instance_id);
        END IF;
    END IF;
    RETURN NULL;
END;
$$;

-- A loop item can leave the Ground more than once: the evidence keeps the latest removal.
CREATE OR REPLACE FUNCTION game_item_ground_removal_evidence_capture() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, public, pg_temp AS $$
BEGIN
    INSERT INTO game_item_ground_removal_evidence (item_instance_id, world_id, channel_id)
    VALUES (OLD.item_instance_id, OLD.world_id, OLD.channel_id)
    ON CONFLICT (item_instance_id) DO UPDATE
       SET world_id = EXCLUDED.world_id, channel_id = EXCLUDED.channel_id;
    RETURN OLD;
END;
$$;

CREATE FUNCTION game_item_ground_removal_evidence_guard() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
    IF TG_OP = 'UPDATE' AND NEW.item_instance_id = OLD.item_instance_id
       AND EXISTS (SELECT 1 FROM game_item_ground_locations g
                    WHERE g.item_instance_id = NEW.item_instance_id
                      AND g.world_id = NEW.world_id AND g.channel_id = NEW.channel_id) THEN
        RETURN NEW;
    END IF;
    RAISE EXCEPTION 'DUR-03 item record is immutable' USING ERRCODE = '23514';
END;
$$;

DROP TRIGGER game_item_ground_removal_evidence_immutable ON game_item_ground_removal_evidence;
CREATE TRIGGER game_item_ground_removal_evidence_immutable
    BEFORE UPDATE OR DELETE ON game_item_ground_removal_evidence
    FOR EACH ROW EXECUTE FUNCTION game_item_ground_removal_evidence_guard();

-- The existing proofs, each unchanged, now skip an item its loop line or starter grant proves.
DROP TRIGGER game_item_instance_change_proven ON game_item_instances;
CREATE CONSTRAINT TRIGGER game_item_instance_change_proven
    AFTER UPDATE ON game_item_instances DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW WHEN (NOT game_item_loop_line_current(NEW.item_instance_id))
    EXECUTE FUNCTION game_item_instance_change_proven();
DROP TRIGGER game_item_mint_consistent ON game_item_instances;
CREATE CONSTRAINT TRIGGER game_item_mint_consistent
    AFTER INSERT ON game_item_instances DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW WHEN (NOT game_item_loop_line_current(NEW.item_instance_id)
                       AND NOT game_item_starter_grant_current(NEW.item_instance_id))
    EXECUTE FUNCTION game_item_mint_consistency_guard();
DROP TRIGGER game_item_container_entry_proven ON game_item_container_entries;
CREATE CONSTRAINT TRIGGER game_item_container_entry_proven
    AFTER INSERT ON game_item_container_entries DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW WHEN (NOT game_item_loop_line_current(NEW.item_instance_id))
    EXECUTE FUNCTION game_item_placement_proven();
DROP TRIGGER game_item_container_entry_removal_proven ON game_item_container_entries;
CREATE CONSTRAINT TRIGGER game_item_container_entry_removal_proven
    AFTER DELETE ON game_item_container_entries DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW WHEN (NOT game_item_loop_line_current(OLD.item_instance_id))
    EXECUTE FUNCTION game_item_container_entry_removal_proven();
DROP TRIGGER game_item_ground_location_insert_proven ON game_item_ground_locations;
CREATE CONSTRAINT TRIGGER game_item_ground_location_insert_proven
    AFTER INSERT ON game_item_ground_locations DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW WHEN (NOT game_item_loop_line_current(NEW.item_instance_id))
    EXECUTE FUNCTION game_item_ground_insertion_guard();
DROP TRIGGER game_item_ground_removal_proven ON game_item_ground_locations;
CREATE CONSTRAINT TRIGGER game_item_ground_removal_proven
    AFTER DELETE ON game_item_ground_locations DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW WHEN (NOT game_item_loop_line_current(OLD.item_instance_id))
    EXECUTE FUNCTION game_item_ground_removal_proven();
DROP TRIGGER game_item_container_slot_proven ON game_item_container_slots;
CREATE CONSTRAINT TRIGGER game_item_container_slot_proven
    AFTER INSERT ON game_item_container_slots DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW WHEN (NOT game_item_starter_grant_current(NEW.item_instance_id))
    EXECUTE FUNCTION game_item_placement_proven();

CREATE TRIGGER game_item_loop_line_admit
    BEFORE INSERT ON game_item_loop_move_lines
    FOR EACH ROW EXECUTE FUNCTION game_item_loop_line_admit();
CREATE CONSTRAINT TRIGGER game_item_loop_line_proven
    AFTER INSERT ON game_item_loop_move_lines DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW EXECUTE FUNCTION game_item_loop_line_proven();
CREATE CONSTRAINT TRIGGER game_item_loop_receipt_proven
    AFTER INSERT ON game_item_loop_move_receipts DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW EXECUTE FUNCTION game_item_loop_receipt_proven();
CREATE CONSTRAINT TRIGGER game_item_nested_entry_exclusive
    AFTER INSERT OR UPDATE OR DELETE ON game_item_nested_entries DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW EXECUTE FUNCTION game_item_location_exclusivity_guard();
CREATE CONSTRAINT TRIGGER game_item_nested_entry_proven
    AFTER INSERT OR DELETE ON game_item_nested_entries DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW EXECUTE FUNCTION game_item_nested_entry_proven();
CREATE TRIGGER game_item_nested_entries_immutable
    BEFORE UPDATE ON game_item_nested_entries
    FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE CONSTRAINT TRIGGER game_character_starter_grant_proven
    AFTER INSERT ON game_character_starter_grants DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW EXECUTE FUNCTION game_character_starter_grant_proven();

DO $$ DECLARE n TEXT; BEGIN
    FOREACH n IN ARRAY ARRAY['game_item_loop_move_receipts', 'game_item_loop_move_lines',
                             'game_character_starter_grants'] LOOP
        EXECUTE format('CREATE TRIGGER %I BEFORE UPDATE OR DELETE ON %I FOR EACH ROW EXECUTE FUNCTION game_item_immutable()', n || '_immutable', n);
    END LOOP;
    FOREACH n IN ARRAY ARRAY['game_item_nested_entries', 'game_item_loop_move_receipts',
                             'game_item_loop_move_lines', 'game_character_starter_grants'] LOOP
        EXECUTE format('CREATE TRIGGER %I BEFORE TRUNCATE ON %I FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate()', n || '_no_truncate', n);
    END LOOP;
    FOREACH n IN ARRAY ARRAY['game_item_loop_line_admit', 'game_item_loop_line_proven',
                             'game_item_loop_receipt_proven', 'game_item_nested_entry_proven',
                             'game_character_starter_grant_proven',
                             'game_item_ground_removal_evidence_guard',
                             'game_item_location_exclusivity_guard'] LOOP
        EXECUTE format('ALTER FUNCTION %I() SET search_path = %I, pg_temp', n, current_schema());
        EXECUTE format('REVOKE ALL ON FUNCTION %I() FROM PUBLIC', n);
    END LOOP;
    EXECUTE format('ALTER FUNCTION game_item_location_exclusive(UUID) SET search_path = %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_item_loop_line_current(UUID) SET search_path = %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_item_starter_grant_current(UUID) SET search_path = %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_item_loop_owner(UUID) SET search_path = %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_item_loop_subtree(UUID) SET search_path = %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_item_loop_tree_size(UUID) SET search_path = %I, pg_temp', current_schema());
END $$;
REVOKE ALL ON FUNCTION game_item_location_exclusive(UUID) FROM PUBLIC;
REVOKE ALL ON FUNCTION game_item_loop_line_current(UUID) FROM PUBLIC;
REVOKE ALL ON FUNCTION game_item_starter_grant_current(UUID) FROM PUBLIC;
REVOKE ALL ON FUNCTION game_item_loop_owner(UUID) FROM PUBLIC;
REVOKE ALL ON FUNCTION game_item_loop_subtree(UUID) FROM PUBLIC;
REVOKE ALL ON FUNCTION game_item_loop_tree_size(UUID) FROM PUBLIC;
-- The proof triggers' WHEN runs as the writing role.
GRANT EXECUTE ON FUNCTION game_item_loop_line_current(UUID) TO oteryn_game_runtime;
GRANT EXECUTE ON FUNCTION game_item_starter_grant_current(UUID) TO oteryn_game_runtime;
-- The writer reads the carried tree's owner, subtree and size under its locks.
GRANT EXECUTE ON FUNCTION game_item_loop_owner(UUID), game_item_loop_subtree(UUID),
    game_item_loop_tree_size(UUID) TO oteryn_game_runtime;

REVOKE ALL ON game_item_nested_entries, game_item_loop_move_receipts, game_item_loop_move_lines,
    game_character_starter_grants FROM PUBLIC;
GRANT SELECT, INSERT, DELETE ON game_item_nested_entries TO oteryn_game_runtime;
GRANT SELECT, INSERT ON game_item_loop_move_receipts, game_item_loop_move_lines,
    game_character_starter_grants TO oteryn_game_runtime;
GRANT SELECT ON game_item_nested_entries, game_item_loop_move_receipts, game_item_loop_move_lines,
    game_character_starter_grants TO oteryn_game_control;
