-- QUICKLOOT-1: Character-owned manual quick-loot routing.
-- Container ids are references to the existing item-instance truth; this table owns only the
-- player's routing preference.  A NULL target is the main container.
CREATE TABLE game_character_quick_loot_settings (
    character_id UUID PRIMARY KEY REFERENCES game_character_roots(character_id),
    unassigned_container_id UUID,
    unassigned_gold_pouch BOOLEAN NOT NULL DEFAULT FALSE,
    fallback_to_main BOOLEAN NOT NULL DEFAULT TRUE,
    committed_character_revision NUMERIC(20,0) NOT NULL CHECK(committed_character_revision>=1),
    CHECK(NOT unassigned_gold_pouch OR unassigned_container_id IS NOT NULL)
);

CREATE TABLE game_character_quick_loot_categories (
    character_id UUID NOT NULL REFERENCES game_character_quick_loot_settings(character_id)
        ON DELETE CASCADE,
    category SMALLINT NOT NULL CHECK(category BETWEEN 0 AND 4),
    container_id UUID,
    gold_pouch BOOLEAN NOT NULL DEFAULT FALSE,
    PRIMARY KEY(character_id,category),
    CHECK(NOT gold_pouch OR container_id IS NOT NULL)
);

REVOKE ALL ON game_character_quick_loot_settings,game_character_quick_loot_categories FROM PUBLIC;
GRANT SELECT,INSERT,UPDATE,DELETE ON game_character_quick_loot_settings,game_character_quick_loot_categories
    TO oteryn_game_runtime;

-- One terminal outcome per manual command; every child is an existing conserved loop move.
CREATE TABLE game_quick_loot_receipts (
    game_session_id UUID NOT NULL,
    command_id NUMERIC(20,0) NOT NULL CHECK(command_id BETWEEN 1 AND 18446744073709551615),
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    request BYTEA NOT NULL CHECK(octet_length(request) BETWEEN 1 AND 64),
    result BYTEA NOT NULL CHECK(octet_length(result) <= 32768),
    created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
    PRIMARY KEY(game_session_id,command_id)
);
CREATE TRIGGER game_quick_loot_receipts_immutable BEFORE UPDATE OR DELETE ON game_quick_loot_receipts
    FOR EACH ROW EXECUTE FUNCTION game_character_immutable();
REVOKE ALL ON game_quick_loot_receipts FROM PUBLIC;
GRANT SELECT,INSERT ON game_quick_loot_receipts TO oteryn_game_runtime;
ALTER TABLE game_item_loop_move_receipts
    ADD COLUMN quick_loot_ordinal SMALLINT NOT NULL DEFAULT 0 CHECK(quick_loot_ordinal BETWEEN 0 AND 1280),
    DROP CONSTRAINT game_item_loop_move_receipts_game_session_id_command_id_key,
    ADD UNIQUE(game_session_id,command_id,quick_loot_ordinal);
ALTER TABLE game_item_loop_move_lines
    DROP CONSTRAINT game_item_loop_move_lines_from_kind_check,
    ADD CHECK(from_kind IN(0,1,2,3,4,5,9)),
    ADD CHECK(from_kind <> 5 OR to_kind IN(0,1,2)),
    DROP CONSTRAINT game_item_loop_move_lines_to_capacity_check,
    ADD CHECK(to_capacity BETWEEN 0 AND 20);

CREATE OR REPLACE FUNCTION game_item_loop_line_admit() RETURNS trigger
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
    IF NEW.to_capacity = 0 AND NOT EXISTS (
        SELECT 1 FROM game_item_instances p JOIN game_item_character_inbox_locations loc USING(item_instance_id)
        WHERE p.item_instance_id=NEW.to_parent_item_instance_id AND p.definition_production_key='oteryn:item.tibia.i23721'
          AND p.world_id=r.world_id AND loc.character_id=r.character_id AND NEW.to_kind=2
          AND NEW.definition_production_key IN('oteryn:item.tibia.i3031','oteryn:item.tibia.i3035','oteryn:item.tibia.i3043')) THEN
        RAISE EXCEPTION 'unlimited slots require the Character coin pouch in Inbox' USING ERRCODE='23514';
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
    IF NEW.from_kind = 5 THEN
        IF r.quick_loot_ordinal = 0 OR NOT EXISTS (
            SELECT 1 FROM game_item_corpse_container_entries e
            JOIN game_item_ground_locations cg ON cg.item_instance_id=e.parent_item_instance_id
            JOIN game_item_instances ci ON ci.item_instance_id=cg.item_instance_id
            JOIN game_item_mint_receipts cr ON cr.item_instance_id=ci.item_instance_id
            WHERE e.item_instance_id=NEW.item_instance_id AND ci.lifecycle=1
              AND cg.world_id=r.world_id AND cg.channel_id=r.channel_id
              AND cg.runtime_scope_ownership_generation=r.scope_generation
              AND cr.loot_purpose_key='CORPSE_MATERIALIZATION' AND cr.draw_ordinal=0
              AND (floor(extract(epoch FROM clock_timestamp())*1000)::bigint >= cr.materialized_at + 10000
                OR cr.corpse_top_damage_character_id=r.character_id
                OR EXISTS(SELECT 1 FROM game_party_members a JOIN game_party_members b USING(party_id)
                    WHERE a.character_id=cr.corpse_top_damage_character_id AND b.character_id=r.character_id))) THEN
            RAISE EXCEPTION 'quick loot requires live same-scope corpse and current loot rights' USING ERRCODE='23514';
        END IF;
        RETURN NEW;
    END IF;
    IF NEW.from_kind = 4 THEN
        IF NOT EXISTS (SELECT 1 FROM game_item_character_depot_locations d
             WHERE d.item_instance_id = NEW.item_instance_id AND d.world_id = r.world_id
               AND d.character_id = r.character_id AND d.town_id = r.depot_town
               AND d.box_number = r.depot_box) THEN
            RAISE EXCEPTION 'depot withdrawal requires the exact Character town and box'
                USING ERRCODE = '23514';
        END IF;
        RETURN NEW;
    END IF;
    IF NEW.from_kind = 3 THEN
        SELECT * INTO g FROM game_item_ground_locations WHERE item_instance_id = NEW.item_instance_id;
        IF NOT FOUND OR g.world_id <> r.world_id OR g.channel_id <> r.channel_id
           OR NOT (game_item_loop_drop_ref(g.corpse_ref)
                   OR game_item_loop_map_mint_source(NEW.item_instance_id)) THEN
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
CREATE OR REPLACE FUNCTION game_item_corpse_entry_removal_proven() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    -- Preserve the same-transaction reset proof installed by 0148_world_reset.sql.
    IF game_world_reset_item_proven(OLD.item_instance_id) THEN
        RETURN NULL;
    END IF;
    IF EXISTS(SELECT 1 FROM game_item_loop_move_lines l JOIN game_item_loop_move_receipts r USING(transaction_id)
        WHERE l.item_instance_id=OLD.item_instance_id AND l.from_kind=5 AND r.quick_loot_ordinal>0
          AND l.created_xact_id=pg_current_xact_id() AND r.created_xact_id=pg_current_xact_id()) THEN
        RETURN NULL;
    END IF;
    IF EXISTS(SELECT 1 FROM game_spell_item_lines l JOIN game_spell_item_receipts r USING(transaction_id) JOIN game_item_instances i ON i.item_instance_id=l.item_instance_id
      WHERE l.operation_kind IN(5,6) AND l.item_instance_id=OLD.item_instance_id AND l.quantity_after=0
       AND l.source_parent_item_instance_id=OLD.parent_item_instance_id AND l.source_placement_ordinal=OLD.placement_ordinal
       AND i.last_transaction_id=l.transaction_id AND i.lifecycle=2 AND i.quantity=0 AND i.state_revision=l.state_revision_before+1
       AND l.operation_kind=6 AND l.source_entry_is_corpse
       AND l.created_xact_id=pg_current_xact_id() AND r.created_xact_id=pg_current_xact_id()) THEN RETURN NULL; END IF;

    IF NOT EXISTS (
        SELECT 1 FROM game_item_transfer_receipts r
          JOIN game_item_instances i ON i.item_instance_id = r.source_item_instance_id
         WHERE r.source_item_instance_id = OLD.item_instance_id
           AND i.last_transaction_id = r.transaction_id
           AND r.created_xact_id = pg_current_xact_id())
       AND NOT EXISTS (
        SELECT 1 FROM game_item_decay_retire_receipts d
          JOIN game_item_instances i ON i.item_instance_id = d.item_instance_id
         WHERE d.item_instance_id = OLD.item_instance_id
           AND d.corpse_item_instance_id = OLD.parent_item_instance_id
           AND d.item_instance_id <> d.corpse_item_instance_id
           AND i.last_transaction_id = d.transaction_id
           AND d.created_xact_id = pg_current_xact_id()) THEN
        RAISE EXCEPTION 'corpse container entry removal must commit with its TRANSFER receipt'
            USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END;
$$;
CREATE OR REPLACE FUNCTION game_item_loop_owner(p_item UUID, OUT owner_character UUID, OUT depth INT)
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
        SELECT e.character_id INTO v_character FROM game_character_equipment_slots e
        JOIN game_item_instances i USING(item_instance_id)
        WHERE e.item_instance_id=v_item AND e.slot IN(6,10)
          AND i.definition_production_key IN('oteryn:item.tibia.i35562','oteryn:item.tibia.i35848',
            'oteryn:item.tibia.i35849','oteryn:item.tibia.i35524','oteryn:item.tibia.i36666',
            'oteryn:item.tibia.i39150','oteryn:item.tibia.i39160','oteryn:item.tibia.i45644');
        IF FOUND THEN owner_character:=v_character; depth:=v_hops; RETURN; END IF;
        SELECT p.character_id INTO v_character FROM game_item_character_inbox_locations p
        JOIN game_item_instances i USING(item_instance_id)
        WHERE p.item_instance_id=v_item AND i.definition_production_key='oteryn:item.tibia.i23721';
        IF FOUND THEN owner_character:=v_character; depth:=v_hops; RETURN; END IF;
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
CREATE OR REPLACE FUNCTION game_item_loop_line_proven() RETURNS trigger
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
    IF NEW.to_kind = 4 THEN
        IF NOT EXISTS (SELECT 1 FROM game_item_character_depot_locations d
             WHERE d.item_instance_id = NEW.item_instance_id AND d.world_id = r.world_id
               AND d.character_id = r.character_id AND d.town_id = r.depot_town
               AND d.box_number = r.depot_box)
           OR NEW.from_kind NOT IN (1, 2) THEN
            RAISE EXCEPTION 'depot deposit differs from receipt' USING ERRCODE = '23514';
        END IF;
        RETURN NULL;
    END IF;
    IF NEW.to_kind = 1 THEN
        SELECT * INTO e FROM game_item_container_entries WHERE item_instance_id = NEW.item_instance_id;
        IF NOT FOUND OR e.character_id <> r.character_id OR e.world_id <> r.world_id
           OR e.parent_item_instance_id <> NEW.to_parent_item_instance_id
           OR e.placed_transaction_id <> NEW.transaction_id
           OR NOT EXISTS (SELECT 1 FROM game_item_container_slots s WHERE s.character_id = r.character_id
                AND s.item_instance_id = NEW.to_parent_item_instance_id)
           OR (SELECT count(*) FROM game_item_container_entries
                WHERE parent_item_instance_id = NEW.to_parent_item_instance_id) > NEW.to_capacity AND NEW.to_capacity <> 0 THEN
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
           OR v_owner IS DISTINCT FROM r.character_id OR v_depth < 0
           OR (SELECT count(*) FROM game_item_nested_entries
                WHERE parent_item_instance_id = NEW.to_parent_item_instance_id) > NEW.to_capacity AND NEW.to_capacity <> 0 THEN
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
DO $$ DECLARE n TEXT; BEGIN
    FOREACH n IN ARRAY ARRAY['game_item_loop_line_admit','game_item_loop_line_proven',
        'game_item_corpse_entry_removal_proven'] LOOP
        EXECUTE format('ALTER FUNCTION %I() SET search_path=pg_catalog,%I,pg_temp',n,current_schema());
    END LOOP;
    EXECUTE format('ALTER FUNCTION game_item_loop_owner(UUID) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_item_loop_line_admit(),game_item_loop_line_proven(),
    game_item_corpse_entry_removal_proven() FROM PUBLIC;

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
        + (SELECT count(*) FROM game_item_nested_entries WHERE item_instance_id = p_item)
        + (SELECT count(*) FROM game_item_character_depot_locations WHERE item_instance_id = p_item);
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
       AND NOT EXISTS(SELECT 1 FROM game_item_instances WHERE item_instance_id=p_item
            AND definition_production_key='oteryn:item.tibia.i23721')
       AND (EXISTS (SELECT 1 FROM game_item_container_entries
                     WHERE parent_item_instance_id = p_item)
            OR EXISTS (SELECT 1 FROM game_item_corpse_container_entries
                        WHERE parent_item_instance_id = p_item)
            OR EXISTS (SELECT 1 FROM game_item_nested_entries
                        WHERE parent_item_instance_id = p_item)) THEN
        RAISE EXCEPTION 'CharacterInbox item cannot have contents' USING ERRCODE = '23514';
    END IF;
    IF EXISTS (SELECT 1 FROM game_item_character_depot_locations WHERE item_instance_id = p_item)
       AND (EXISTS (SELECT 1 FROM game_item_container_entries
                     WHERE parent_item_instance_id = p_item)
            OR EXISTS (SELECT 1 FROM game_item_corpse_container_entries
                        WHERE parent_item_instance_id = p_item)
            OR EXISTS (SELECT 1 FROM game_item_nested_entries
                        WHERE parent_item_instance_id = p_item)) THEN
        RAISE EXCEPTION 'CharacterDepot item cannot have contents' USING ERRCODE = '23514';
    END IF;
END;
$$;
DO $$ BEGIN
    EXECUTE format('ALTER FUNCTION game_item_location_exclusive(UUID) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;

CREATE FUNCTION game_quiver_entry(p_character BYTEA,p_parent UUID,p_item UUID,p_ordinal NUMERIC,p_world UUID)
RETURNS BOOLEAN LANGUAGE sql STABLE SECURITY DEFINER AS $$
 SELECT EXISTS(SELECT 1 FROM game_item_nested_entries n JOIN game_character_equipment_slots e ON e.item_instance_id=n.parent_item_instance_id JOIN game_item_instances q ON q.item_instance_id=e.item_instance_id
 WHERE n.item_instance_id=p_item AND n.parent_item_instance_id=p_parent AND n.placement_ordinal=p_ordinal
 AND n.world_id=p_world AND e.world_id=p_world AND e.character_id=encode(p_character,'hex')::uuid AND e.slot IN(6,10)
 AND q.lifecycle=1 AND q.definition_production_key IN('oteryn:item.tibia.i35562','oteryn:item.tibia.i35848','oteryn:item.tibia.i35849','oteryn:item.tibia.i35524','oteryn:item.tibia.i36666','oteryn:item.tibia.i39150','oteryn:item.tibia.i39160','oteryn:item.tibia.i45644'))
$$;
CREATE OR REPLACE FUNCTION game_spell_item_capture_custody() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE l game_spell_item_lines%ROWTYPE; r game_spell_item_receipts%ROWTYPE;
BEGIN
 SELECT * INTO l FROM game_spell_item_lines WHERE transaction_id=NEW.last_transaction_id AND item_instance_id=NEW.item_instance_id AND operation_kind IN(5,6) AND created_xact_id=pg_current_xact_id();
 IF NOT FOUND THEN RETURN NEW; END IF;
 SELECT * INTO r FROM game_spell_item_receipts WHERE transaction_id=l.transaction_id AND created_xact_id=pg_current_xact_id();
 IF NOT FOUND OR OLD.lifecycle<>1 OR OLD.world_id<>r.world_id OR OLD.state_revision<>l.state_revision_before
   OR OLD.quantity<>l.quantity_before OR NEW.quantity<>l.quantity_after OR (NEW.definition_family,NEW.definition_production_key,NEW.definition_revision_ref)<>(l.definition_family,l.definition_production_key,l.definition_revision) THEN
  RAISE EXCEPTION 'spell consumption captures exact actual OLD item state only' USING ERRCODE='23514';
 END IF;
 IF l.operation_kind=5 THEN
  IF l.source_equipment_slot IS NOT NULL THEN
   IF NOT EXISTS(SELECT 1 FROM game_character_equipment_slots s JOIN game_character_equipment_state es USING(character_id)
     WHERE s.item_instance_id=OLD.item_instance_id AND s.character_id=r.character_id AND s.world_id=r.world_id
       AND s.slot=l.source_equipment_slot AND es.revision=l.source_equipment_revision) THEN
    RAISE EXCEPTION 'spell consumption source equipment custody changed' USING ERRCODE='23514';
   END IF;
  ELSIF NOT EXISTS(SELECT 1 FROM game_item_container_entries e WHERE e.item_instance_id=OLD.item_instance_id AND e.character_id=r.character_id AND e.world_id=r.world_id AND e.parent_item_instance_id=l.source_parent_item_instance_id AND e.placement_ordinal=l.source_placement_ordinal) AND NOT game_quiver_entry(decode(replace(r.character_id::text,'-',''),'hex'),l.source_parent_item_instance_id,OLD.item_instance_id,l.source_placement_ordinal,r.world_id) THEN
   RAISE EXCEPTION 'spell consumption source inventory custody changed' USING ERRCODE='23514';
  END IF;
 ELSE
  IF NOT ((l.source_entry_is_corpse AND EXISTS(SELECT 1 FROM game_item_corpse_container_entries e WHERE e.item_instance_id=OLD.item_instance_id AND e.world_id=r.world_id AND e.parent_item_instance_id=l.source_parent_item_instance_id AND e.placement_ordinal=l.source_placement_ordinal))
     OR (NOT l.source_entry_is_corpse AND EXISTS(SELECT 1 FROM game_item_container_entries e WHERE e.item_instance_id=OLD.item_instance_id AND e.world_id=r.world_id AND e.parent_item_instance_id=l.source_parent_item_instance_id AND e.placement_ordinal=l.source_placement_ordinal))) THEN
   RAISE EXCEPTION 'contained spell retirement source custody changed' USING ERRCODE='23514';
  END IF;
  IF NOT EXISTS(SELECT 1 FROM game_item_ground_locations g JOIN game_item_instances i USING(item_instance_id) JOIN game_item_mint_receipts m ON m.item_instance_id=i.item_instance_id
      WHERE g.item_instance_id=l.source_corpse_root AND i.lifecycle=1 AND m.loot_purpose_key='CORPSE_MATERIALIZATION'
       AND (g.world_id,g.channel_id,g.runtime_scope_ownership_generation)=(r.world_id,r.channel_id,r.ownership_generation)
       AND (g.spatial_position,g.map_revision,g.content_revision,g.native_room_placement_context)=(l.spatial_position,l.map_revision,l.content_revision,l.placement_context)) THEN
   RAISE EXCEPTION 'contained retirement root lacks actual current corpse placement' USING ERRCODE='23514';
  END IF;
 END IF;
 INSERT INTO game_spell_item_source_custody(transaction_id,item_instance_id,quantity_before,state_revision_before,source_parent,source_ordinal,source_slot)
 VALUES(l.transaction_id,OLD.item_instance_id,OLD.quantity,OLD.state_revision,l.source_parent_item_instance_id,l.source_placement_ordinal,l.source_equipment_slot);
 RETURN NEW;
END $$;

CREATE OR REPLACE FUNCTION game_spell_item_line_proven() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.operation_kind IN(5,6) THEN
      IF NOT EXISTS(SELECT 1 FROM game_spell_item_source_custody c WHERE c.transaction_id=NEW.transaction_id AND c.item_instance_id=NEW.item_instance_id
        AND c.quantity_before=NEW.quantity_before AND c.state_revision_before=NEW.state_revision_before
        AND c.source_parent IS NOT DISTINCT FROM NEW.source_parent_item_instance_id AND c.source_ordinal IS NOT DISTINCT FROM NEW.source_placement_ordinal
        AND c.source_slot IS NOT DISTINCT FROM NEW.source_equipment_slot AND c.created_xact_id=pg_current_xact_id()) THEN
        RAISE EXCEPTION 'spell source line lacks actual OLD custody capture' USING ERRCODE='23514';
      END IF;
      IF NOT EXISTS(SELECT 1 FROM game_spell_item_receipts r JOIN game_item_instances i ON i.item_instance_id=NEW.item_instance_id
        WHERE r.transaction_id=NEW.transaction_id AND r.created_xact_id=pg_current_xact_id() AND NEW.created_xact_id=pg_current_xact_id()
         AND NEW.ordinal<=r.operation_count AND (NEW.world_id,NEW.channel_id)=(r.world_id,r.channel_id)
         AND (i.world_id,i.definition_family,i.definition_production_key,i.definition_revision_ref)=(NEW.world_id,NEW.definition_family,NEW.definition_production_key,NEW.definition_revision)
         AND i.last_transaction_id=NEW.transaction_id AND i.state_revision=NEW.state_revision_before+1 AND i.quantity=NEW.quantity_after
         AND i.lifecycle=CASE WHEN NEW.quantity_after=0 THEN 2 ELSE 1 END
         AND EXISTS(SELECT 1 FROM game_item_transfer_quantity_evidence q WHERE q.item_instance_id=i.item_instance_id AND q.transaction_id=NEW.transaction_id AND q.quantity_before=NEW.quantity_before)
         AND NOT EXISTS(SELECT 1 FROM game_item_ground_locations g WHERE g.item_instance_id=i.item_instance_id)
         AND ((NEW.quantity_after=0 AND NOT EXISTS(SELECT 1 FROM game_item_container_entries e WHERE e.item_instance_id=i.item_instance_id OR e.parent_item_instance_id=i.item_instance_id)
             AND NOT EXISTS(SELECT 1 FROM game_item_corpse_container_entries e WHERE e.item_instance_id=i.item_instance_id OR e.parent_item_instance_id=i.item_instance_id)
             AND NOT EXISTS(SELECT 1 FROM game_character_equipment_slots e WHERE e.item_instance_id=i.item_instance_id) AND NOT EXISTS(SELECT 1 FROM game_item_nested_entries e WHERE e.item_instance_id=i.item_instance_id))
           OR (NEW.quantity_after>0 AND ((NEW.source_equipment_slot IS NOT NULL AND EXISTS(SELECT 1 FROM game_character_equipment_slots e WHERE e.character_id=r.character_id AND e.slot=NEW.source_equipment_slot AND e.item_instance_id=i.item_instance_id AND e.world_id=r.world_id))
             OR (NEW.source_equipment_slot IS NULL AND game_quiver_entry(decode(replace(r.character_id::text,'-',''),'hex'),NEW.source_parent_item_instance_id,i.item_instance_id,NEW.source_placement_ordinal,r.world_id))
             OR (NEW.source_equipment_slot IS NULL AND EXISTS(SELECT 1 FROM game_item_container_entries e WHERE e.character_id=r.character_id AND e.item_instance_id=i.item_instance_id AND e.parent_item_instance_id=NEW.source_parent_item_instance_id AND e.placement_ordinal=NEW.source_placement_ordinal AND e.world_id=r.world_id)))))) THEN
        RAISE EXCEPTION 'inventory spell source line lacks exact current item successor/custody' USING ERRCODE='23514';
      END IF;
      IF NEW.operation_kind=5 AND NEW.source_equipment_slot IS NOT NULL AND NOT EXISTS(
         SELECT 1 FROM game_spell_equipment_consumptions e WHERE e.transaction_id=NEW.transaction_id AND e.revision_before=NEW.source_equipment_revision AND e.created_xact_id=pg_current_xact_id()) THEN
        RAISE EXCEPTION 'equipped consumption requires receipt-linked equipment epoch' USING ERRCODE='23514';
      END IF;
      IF NEW.operation_kind=6 AND NOT EXISTS(
         WITH RECURSIVE chain(item,parent,path) AS (
           SELECT NEW.item_instance_id,NEW.source_parent_item_instance_id,ARRAY[NEW.item_instance_id]
           UNION ALL SELECT p.item_instance_id,p.source_parent_item_instance_id,c.path||p.item_instance_id
             FROM chain c JOIN game_spell_item_lines p ON p.transaction_id=NEW.transaction_id AND p.item_instance_id=c.parent AND p.operation_kind=6
             WHERE NOT p.item_instance_id=ANY(c.path) AND cardinality(c.path)<500 AND p.created_xact_id=pg_current_xact_id()
         ) SELECT 1 FROM chain WHERE parent=NEW.source_corpse_root) THEN
        RAISE EXCEPTION 'contained retirement must reach the actual root without cycles' USING ERRCODE='23514';
      END IF;
      IF NEW.operation_kind=6 AND NOT EXISTS(
         SELECT 1 FROM game_spell_item_lines root JOIN game_item_mint_receipts m ON m.item_instance_id=root.item_instance_id
          WHERE root.transaction_id=NEW.transaction_id AND root.item_instance_id=NEW.source_corpse_root AND root.operation_kind IN(2,3)
           AND root.created_xact_id=pg_current_xact_id() AND m.loot_purpose_key='CORPSE_MATERIALIZATION'
           AND (root.world_id,root.channel_id,root.spatial_position,root.map_revision,root.content_revision,root.placement_context)
             =(NEW.world_id,NEW.channel_id,NEW.spatial_position,NEW.map_revision,NEW.content_revision,NEW.placement_context)
           AND (NEW.source_parent_item_instance_id=root.item_instance_id OR EXISTS(SELECT 1 FROM game_spell_item_lines parent WHERE parent.transaction_id=NEW.transaction_id AND parent.operation_kind=6 AND parent.item_instance_id=NEW.source_parent_item_instance_id AND parent.source_corpse_root=NEW.source_corpse_root AND parent.created_xact_id=pg_current_xact_id()))) THEN
        RAISE EXCEPTION 'contained retirement requires its same-cast real corpse root/ancestor chain' USING ERRCODE='23514';
      END IF;
      RETURN NULL;
    END IF;

    IF NOT EXISTS(SELECT 1 FROM game_spell_item_receipts r JOIN game_item_instances i ON i.item_instance_id=NEW.item_instance_id
        WHERE r.transaction_id=NEW.transaction_id AND r.created_xact_id=pg_current_xact_id()
          AND NEW.created_xact_id=pg_current_xact_id() AND NEW.ordinal<=r.operation_count
          AND (NEW.world_id,NEW.channel_id)=(r.world_id,r.channel_id)
          AND (i.world_id,i.definition_family,i.definition_production_key,i.definition_revision_ref)=(NEW.world_id,NEW.definition_family,NEW.definition_production_key,NEW.definition_revision)
          AND ((NEW.operation_kind=1 AND i.lifecycle=1 AND i.quantity=NEW.quantity_after
                AND i.minted_transaction_id=NEW.transaction_id AND i.state_revision=1
                AND ((NEW.destination_parent_item_instance_id IS NULL AND EXISTS(SELECT 1 FROM game_item_ground_locations g WHERE g.item_instance_id=i.item_instance_id
                    AND (g.world_id,g.channel_id,g.spatial_position,g.map_revision,g.content_revision,g.native_room_placement_context)
                      =(NEW.world_id,NEW.channel_id,NEW.spatial_position,NEW.map_revision,NEW.content_revision,NEW.placement_context)
                    AND g.runtime_scope_ownership_generation=r.ownership_generation AND g.stack_ordinal IS NOT NULL))
                  OR (NEW.destination_parent_item_instance_id IS NOT NULL AND EXISTS(SELECT 1 FROM game_item_container_entries e WHERE e.item_instance_id=i.item_instance_id AND e.character_id=r.character_id AND e.parent_item_instance_id=NEW.destination_parent_item_instance_id AND e.placement_ordinal=NEW.destination_ordinal AND e.placed_transaction_id=NEW.transaction_id)))
                AND NOT (EXISTS(SELECT 1 FROM game_item_ground_locations g WHERE g.item_instance_id=i.item_instance_id) AND EXISTS(SELECT 1 FROM game_item_container_entries e WHERE e.item_instance_id=i.item_instance_id)))
            OR (NEW.operation_kind=4 AND i.lifecycle=1 AND i.last_transaction_id=NEW.transaction_id
                AND i.quantity=NEW.quantity_after AND i.state_revision=NEW.state_revision_before+1
                AND EXISTS(SELECT 1 FROM game_item_transfer_quantity_evidence q WHERE q.item_instance_id=i.item_instance_id AND q.transaction_id=NEW.transaction_id AND q.quantity_before=NEW.quantity_before)
                AND EXISTS(SELECT 1 FROM game_item_container_entries e WHERE e.item_instance_id=i.item_instance_id AND e.character_id=r.character_id AND e.parent_item_instance_id=NEW.destination_parent_item_instance_id AND e.placement_ordinal=NEW.destination_ordinal)
                AND NOT EXISTS(SELECT 1 FROM game_item_ground_locations g WHERE g.item_instance_id=i.item_instance_id))
            OR (NEW.operation_kind IN(2,3) AND i.lifecycle=2 AND i.last_transaction_id=NEW.transaction_id
                AND i.quantity=0 AND i.state_revision=NEW.state_revision_before+1
                AND EXISTS(SELECT 1 FROM game_item_transfer_quantity_evidence q WHERE q.item_instance_id=i.item_instance_id AND q.transaction_id=NEW.transaction_id AND q.quantity_before=NEW.quantity_before)
                AND NOT EXISTS(SELECT 1 FROM game_item_ground_locations g WHERE g.item_instance_id=i.item_instance_id)
                AND NOT EXISTS(SELECT 1 FROM game_item_corpse_container_entries e WHERE e.parent_item_instance_id=i.item_instance_id)))) THEN
        RAISE EXCEPTION 'spell item line does not prove its actual item/custody successor' USING ERRCODE='23514';
    END IF;
    IF NEW.operation_kind=3 AND NOT EXISTS(SELECT 1 FROM game_item_mint_receipts m
       WHERE m.item_instance_id=NEW.item_instance_id AND m.loot_purpose_key='CORPSE_MATERIALIZATION') THEN
        RAISE EXCEPTION 'Animate Dead consumes only an actual materialized corpse' USING ERRCODE='23514';
    END IF;
    IF NEW.operation_kind=3 AND NOT EXISTS(SELECT 1 FROM game_spell_companion_acquisition_receipts c WHERE c.transaction_id=NEW.transaction_id AND c.corpse_item_instance_id=NEW.item_instance_id AND c.corpse_state_revision=NEW.state_revision_before AND c.created_xact_id=pg_current_xact_id()) THEN RAISE EXCEPTION 'corpse consumption requires its prepared actual companion acquisition receipt' USING ERRCODE='23514'; END IF;
    RETURN NULL;
END $$;

CREATE OR REPLACE FUNCTION game_item_nested_entry_proven() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
    IF TG_OP='DELETE' AND EXISTS(SELECT 1 FROM game_spell_item_lines l
        JOIN game_spell_item_receipts r USING(transaction_id) JOIN game_item_instances i ON i.item_instance_id=l.item_instance_id
        JOIN game_spell_item_source_custody c ON c.transaction_id=l.transaction_id AND c.item_instance_id=i.item_instance_id
        WHERE l.operation_kind=5 AND l.item_instance_id=OLD.item_instance_id AND l.quantity_after=0
          AND l.source_parent_item_instance_id=OLD.parent_item_instance_id AND l.source_placement_ordinal=OLD.placement_ordinal
          AND c.source_parent=OLD.parent_item_instance_id AND c.source_ordinal=OLD.placement_ordinal
          AND i.lifecycle=2 AND i.quantity=0 AND i.last_transaction_id=l.transaction_id AND i.state_revision=l.state_revision_before+1
          AND l.created_xact_id=pg_current_xact_id() AND r.created_xact_id=pg_current_xact_id() AND c.created_xact_id=pg_current_xact_id()) THEN RETURN NULL; END IF;
    IF TG_OP = 'INSERT' AND game_item_loop_line_current(NEW.item_instance_id) THEN
        RETURN NULL;
    END IF;
    IF TG_OP = 'DELETE' AND game_item_loop_line_current(OLD.item_instance_id) THEN
        RETURN NULL;
    END IF;
    -- BAGS-USE-1: the source entry spent to zero, in this same physical transaction.
    IF TG_OP = 'DELETE' AND EXISTS (
        SELECT 1 FROM game_item_use_records u
          JOIN game_item_instances i ON i.item_instance_id = u.source_item_instance_id
          CROSS JOIN LATERAL game_item_loop_owner(OLD.parent_item_instance_id) o
         WHERE u.source_item_instance_id = OLD.item_instance_id
           AND u.world_id = OLD.world_id
           AND u.source_quantity_after = 0
           AND u.source_parent_item_instance_id = OLD.parent_item_instance_id
           AND u.source_placement_ordinal = OLD.placement_ordinal
           AND o.owner_character = u.character_id AND o.depth BETWEEN 1 AND 7
           AND i.last_transaction_id = u.transaction_id AND i.lifecycle = 2 AND i.quantity = 0
           AND u.created_xact_id = pg_current_xact_id()) THEN
        RETURN NULL;
    END IF;
    RAISE EXCEPTION 'nested container custody changes only by an item loop move' USING ERRCODE = '23514';
END;
$$;

DO $$ DECLARE n TEXT; BEGIN
    FOREACH n IN ARRAY ARRAY['game_spell_item_capture_custody','game_spell_item_line_proven','game_item_nested_entry_proven'] LOOP
        EXECUTE format('ALTER FUNCTION %I() SET search_path=pg_catalog,%I,pg_temp',n,current_schema());
    END LOOP;
    EXECUTE format('ALTER FUNCTION game_quiver_entry(BYTEA,UUID,UUID,NUMERIC,UUID) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_quiver_entry(BYTEA,UUID,UUID,NUMERIC,UUID),game_spell_item_capture_custody(),game_spell_item_line_proven(),game_item_nested_entry_proven() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_quiver_entry(BYTEA,UUID,UUID,NUMERIC,UUID) TO oteryn_game_runtime;

-- Store pouch custody stays in Inbox even while empty. All ordinary Inbox paths remain intact.
CREATE FUNCTION game_quick_loot_keep_pouch() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
    IF EXISTS(SELECT 1 FROM game_item_instances WHERE item_instance_id=OLD.item_instance_id
        AND definition_production_key='oteryn:item.tibia.i23721') THEN
        RAISE EXCEPTION 'Gold Pouch remains in Store Inbox' USING ERRCODE='23514';
    END IF;
    RETURN OLD;
END $$;
CREATE TRIGGER game_quick_loot_keep_pouch BEFORE DELETE ON game_item_character_inbox_locations
    FOR EACH ROW EXECUTE FUNCTION game_quick_loot_keep_pouch();
DO $$ BEGIN EXECUTE format('ALTER FUNCTION game_quick_loot_keep_pouch() SET search_path=pg_catalog,%I,pg_temp',current_schema()); END $$;
REVOKE ALL ON FUNCTION game_quick_loot_keep_pouch() FROM PUBLIC;

-- A quiver shot retains the captured exact source custody, even for the last arrow.
CREATE OR REPLACE FUNCTION game_ranged_attack_receipt_proven() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NEW.swing_ordinal IS NOT NULL AND (
   NEW.operation_count<>1 OR NEW.caster_lease_generation IS NULL
   OR NOT EXISTS (SELECT 1 FROM game_spell_item_lines l
       WHERE l.transaction_id=NEW.transaction_id AND l.operation_kind=5
       AND (l.source_equipment_slot IN(5,6,10) OR (l.source_equipment_slot IS NULL AND EXISTS (
           SELECT 1 FROM game_spell_item_source_custody c
           JOIN game_character_equipment_slots e ON e.item_instance_id=c.source_parent
           JOIN game_item_instances q ON q.item_instance_id=e.item_instance_id
           WHERE c.transaction_id=l.transaction_id AND c.item_instance_id=l.item_instance_id
             AND c.source_parent=l.source_parent_item_instance_id AND c.source_ordinal=l.source_placement_ordinal
             AND c.created_xact_id=NEW.created_xact_id AND e.character_id=NEW.character_id
             AND e.world_id=NEW.world_id AND e.slot IN(6,10) AND q.lifecycle=1
             AND q.definition_production_key IN('oteryn:item.tibia.i35562','oteryn:item.tibia.i35848',
               'oteryn:item.tibia.i35849','oteryn:item.tibia.i35524','oteryn:item.tibia.i36666',
               'oteryn:item.tibia.i39150','oteryn:item.tibia.i39160','oteryn:item.tibia.i45644'))))
       AND l.quantity_before=l.quantity_after+1)
   OR (convert_from(NEW.cost,'UTF8')::jsonb->>'mana_before')::numeric
        IS DISTINCT FROM (convert_from(NEW.cost,'UTF8')::jsonb->>'mana_after')::numeric
   OR (convert_from(NEW.cost,'UTF8')::jsonb->>'soul_before')::numeric
        IS DISTINCT FROM (convert_from(NEW.cost,'UTF8')::jsonb->>'soul_after')::numeric
   OR convert_from(NEW.cost,'UTF8')::jsonb->'cooldowns_before'
        IS DISTINCT FROM convert_from(NEW.cost,'UTF8')::jsonb->'cooldowns_after') THEN
  RAISE EXCEPTION 'ranged shot requires one equipped projectile and no spell payment' USING ERRCODE='23514';
 END IF;
 RETURN NULL;
END $$;

-- A child ordinal is never an independent bypass of command replay or batch custody.
CREATE FUNCTION game_quick_loot_child_proven() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
    IF NEW.quick_loot_ordinal>0 AND NOT EXISTS (
        SELECT 1 FROM game_quick_loot_receipts q
        WHERE q.game_session_id=NEW.game_session_id AND q.command_id=NEW.command_id
          AND q.character_id=NEW.character_id AND q.created_xact_id=NEW.created_xact_id) THEN
        RAISE EXCEPTION 'quick loot child requires the terminal batch receipt' USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_quick_loot_child_proven AFTER INSERT ON game_item_loop_move_receipts
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_quick_loot_child_proven();
DO $$ BEGIN
    EXECUTE format('ALTER FUNCTION game_quick_loot_child_proven() SET search_path=pg_catalog,%I,pg_temp',current_schema());
    EXECUTE format('ALTER FUNCTION game_ranged_attack_receipt_proven() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_quick_loot_child_proven() FROM PUBLIC;

ALTER TABLE game_character_quick_loot_settings ADD COLUMN accept_only BOOLEAN NOT NULL DEFAULT FALSE;
CREATE TABLE game_character_quick_loot_filters (
    character_id UUID NOT NULL REFERENCES game_character_quick_loot_settings(character_id),
    accepted BOOLEAN NOT NULL,
    item_production_key TEXT NOT NULL CHECK(item_production_key ~ '^oteryn:item\.tibia\.i[1-9][0-9]{0,9}$'),
    PRIMARY KEY(character_id,accepted,item_production_key)
);
REVOKE ALL ON game_character_quick_loot_filters FROM PUBLIC;
GRANT SELECT,INSERT,DELETE ON game_character_quick_loot_filters TO oteryn_game_runtime;
