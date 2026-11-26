-- World-owned Bank Robbery. States: eligible, robber alive, bags outstanding,
-- returned (reserved), closed. Physical loot commits with the transition to 2.
CREATE TABLE game_bank_robbery_occurrences (
    world_id UUID NOT NULL,
    reset_epoch NUMERIC(20,0) NOT NULL,
    boot_ms BIGINT NOT NULL CHECK (boot_ms >= 0),
    last_checked_hour BIGINT NOT NULL CHECK (last_checked_hour >= 0),
    state SMALLINT NOT NULL DEFAULT 0 CHECK (state BETWEEN 0 AND 4),
    city TEXT,
    robber TEXT,
    firing_id UUID UNIQUE CHECK (firing_id IS NULL OR game_node_is_uuid_v7(firing_id)),
    channel_id UUID CHECK (channel_id IS NULL OR (game_node_is_uuid_v7(channel_id) AND channel_id <> world_id)),
    PRIMARY KEY (world_id, reset_epoch),
    FOREIGN KEY (world_id, reset_epoch) REFERENCES game_world_reset_epochs,
    CHECK (num_nonnulls(city, robber, firing_id) IN (0, 3)),
    CHECK (state <> 0 OR firing_id IS NULL),
    CHECK (state NOT BETWEEN 1 AND 3 OR firing_id IS NOT NULL),
    CHECK (channel_id IS NULL OR firing_id IS NOT NULL),
    CHECK (state NOT IN (2, 3) OR channel_id IS NOT NULL)
);
REVOKE ALL ON game_bank_robbery_occurrences FROM PUBLIC;
GRANT SELECT, INSERT, UPDATE ON game_bank_robbery_occurrences TO oteryn_game_runtime;
GRANT SELECT ON game_bank_robbery_occurrences TO oteryn_game_control;

CREATE TABLE game_bank_robbery_bags (
    world_id UUID NOT NULL,
    reset_epoch NUMERIC(20,0) NOT NULL,
    ordinal SMALLINT NOT NULL CHECK (ordinal BETWEEN 1 AND 48),
    item_instance_id UUID NOT NULL UNIQUE REFERENCES game_item_instances,
    retired_transaction_id UUID UNIQUE,
    retired_xid XID8,
    PRIMARY KEY (world_id, reset_epoch, ordinal),
    FOREIGN KEY (world_id, reset_epoch) REFERENCES game_bank_robbery_occurrences,
    CHECK ((retired_transaction_id IS NULL) = (retired_xid IS NULL))
);
REVOKE ALL ON game_bank_robbery_bags FROM PUBLIC;
GRANT SELECT ON game_bank_robbery_bags TO oteryn_game_runtime;
GRANT INSERT (world_id, reset_epoch, ordinal, item_instance_id) ON game_bank_robbery_bags TO oteryn_game_runtime;
GRANT SELECT ON game_bank_robbery_bags TO oteryn_game_control;

-- A binding must be written with the exact selected robber's native corpse loot.
CREATE FUNCTION game_bank_robbery_bind_mint() RETURNS TRIGGER
LANGUAGE plpgsql AS $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM game_bank_robbery_occurrences o
        JOIN game_world_reset_active USING (world_id, reset_epoch)
        JOIN game_item_instances i ON i.item_instance_id = NEW.item_instance_id
        JOIN game_item_mint_receipts r ON r.item_instance_id = i.item_instance_id
        JOIN game_item_audit_outbox event ON event.event_id = r.event_id
        JOIN game_item_corpse_container_entries e ON e.item_instance_id = i.item_instance_id
        WHERE (o.world_id, o.reset_epoch) = (NEW.world_id, NEW.reset_epoch)
          AND o.state = 2 AND i.world_id = NEW.world_id
          AND i.definition_production_key = 'oteryn:item.tibia.i13429'
          AND i.lifecycle = 1 AND i.quantity = 1
          AND (r.death_world_id, r.death_channel_id) = (o.world_id, o.channel_id)
          AND r.loot_table_production_key = replace(o.robber, 'oteryn:creature.', 'oteryn:loot.creature.')
          AND event.created_xact_id = pg_current_xact_id()
          AND e.placement_ordinal = NEW.ordinal
    ) THEN
        RAISE EXCEPTION 'Bank Robbery bag requires the current robber MINT' USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END
$$;
CREATE TRIGGER game_bank_robbery_bind_mint BEFORE INSERT ON game_bank_robbery_bags
FOR EACH ROW EXECUTE FUNCTION game_bank_robbery_bind_mint();

-- Retirement is authorized only by a committed epoch change, and only for the
-- exact occurrence mints. These proofs preserve every other custody guard.
CREATE FUNCTION game_bank_robbery_retiring(p_item UUID, p_transaction UUID) RETURNS BOOLEAN
LANGUAGE sql VOLATILE AS $$
    SELECT EXISTS (
        SELECT 1 FROM game_bank_robbery_bags
        WHERE item_instance_id = p_item
          AND retired_transaction_id = p_transaction
          AND retired_xid = pg_current_xact_id()
    )
$$;

CREATE FUNCTION game_bank_robbery_close() RETURNS TRIGGER
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE
    bag UUID;
    retirement UUID;
BEGIN
    IF NEW.reset_epoch <= OLD.reset_epoch THEN
        RETURN NULL;
    END IF;

    UPDATE game_bank_robbery_occurrences SET state = 4
      WHERE world_id = OLD.world_id AND reset_epoch = OLD.reset_epoch;

    -- Depot and Inbox remove hooks take the Character root before custody.
    PERFORM 1 FROM game_character_roots r WHERE r.character_id IN (
        SELECT d.character_id FROM game_bank_robbery_bags b
          JOIN game_item_character_depot_locations d USING (item_instance_id)
          WHERE b.world_id = OLD.world_id AND b.reset_epoch = OLD.reset_epoch
        UNION
        SELECT n.character_id FROM game_bank_robbery_bags b
          JOIN game_item_character_inbox_locations n USING (item_instance_id)
          WHERE b.world_id = OLD.world_id AND b.reset_epoch = OLD.reset_epoch
    ) ORDER BY r.character_id FOR UPDATE;

    FOR bag IN
        SELECT i.item_instance_id
        FROM game_bank_robbery_bags b JOIN game_item_instances i USING (item_instance_id)
        WHERE b.world_id = OLD.world_id AND b.reset_epoch = OLD.reset_epoch
          AND i.world_id = OLD.world_id AND i.lifecycle = 1
        ORDER BY i.item_instance_id FOR UPDATE OF i
    LOOP
        retirement := uuidv7();
        UPDATE game_bank_robbery_bags
          SET retired_transaction_id = retirement, retired_xid = pg_current_xact_id()
          WHERE item_instance_id = bag;
        UPDATE game_item_instances
          SET quantity = 0, lifecycle = 2, state_revision = state_revision + 1,
              last_transaction_id = retirement
          WHERE item_instance_id = bag;
        DELETE FROM game_item_character_depot_locations WHERE item_instance_id = bag;
        DELETE FROM game_item_character_inbox_locations WHERE item_instance_id = bag;
        DELETE FROM game_item_container_entries WHERE item_instance_id = bag;
        DELETE FROM game_item_corpse_container_entries WHERE item_instance_id = bag;
        DELETE FROM game_item_ground_locations WHERE item_instance_id = bag;
        DELETE FROM game_item_house_interior_locations WHERE item_instance_id = bag;
        DELETE FROM game_item_house_reclaim_provenance WHERE item_instance_id = bag;
        DELETE FROM game_item_nested_entries WHERE item_instance_id = bag;
    END LOOP;
    RETURN NULL;
END
$$;

CREATE TRIGGER game_bank_robbery_close AFTER UPDATE OF reset_epoch ON game_world_reset_active
FOR EACH ROW EXECUTE FUNCTION game_bank_robbery_close();

DO $migration$
DECLARE
    guard TEXT;
    definition TEXT;
    arm TEXT;
    entry_at INTEGER;
BEGIN
    FOREACH guard IN ARRAY ARRAY['game_item_instance_guard', 'game_item_instance_change_proven'] LOOP
        definition := pg_get_functiondef((guard || '()')::regprocedure);
        entry_at := regexp_instr(definition, E'\\mBEGIN\\M', 1, 1, 0, 'i');
        IF entry_at = 0 THEN
            RAISE EXCEPTION 'missing item instance guard %', guard;
        END IF;
        arm := $arm$
            IF TG_OP = 'UPDATE' AND OLD.lifecycle = 1 AND OLD.quantity = 1
              AND NEW.lifecycle = 2 AND NEW.quantity = 0
              AND NEW.state_revision = OLD.state_revision + 1
              AND (to_jsonb(NEW) - ARRAY['lifecycle','quantity','state_revision','last_transaction_id'])
                  = (to_jsonb(OLD) - ARRAY['lifecycle','quantity','state_revision','last_transaction_id'])
              AND game_bank_robbery_retiring(NEW.item_instance_id, NEW.last_transaction_id) THEN
        $arm$ || CASE guard WHEN 'game_item_instance_guard' THEN ' RETURN NEW; ' ELSE ' RETURN NULL; ' END || ' END IF; ';
        EXECUTE overlay(definition placing E'BEGIN\n' || arm FROM entry_at FOR 5);
    END LOOP;

    FOREACH guard IN ARRAY ARRAY[
        'game_item_container_entry_removal_proven', 'game_item_corpse_entry_removal_proven',
        'game_item_ground_removal_proven', 'game_item_nested_entry_proven'
    ] LOOP
        definition := pg_get_functiondef((guard || '()')::regprocedure);
        entry_at := regexp_instr(definition, E'\\mBEGIN\\M', 1, 1, 0, 'i');
        IF entry_at = 0 THEN
            RAISE EXCEPTION 'missing item removal guard %', guard;
        END IF;
        arm := $arm$
            IF TG_OP = 'DELETE' AND EXISTS (
                SELECT 1 FROM game_item_instances
                WHERE item_instance_id = OLD.item_instance_id AND lifecycle = 2
                  AND game_bank_robbery_retiring(item_instance_id, last_transaction_id)
            ) THEN RETURN NULL; END IF;
        $arm$;
        EXECUTE overlay(definition placing E'BEGIN\n' || arm FROM entry_at FOR 5);
    END LOOP;

    FOR guard, arm IN VALUES
        ('game_character_depot_remove', $depot$
            IF EXISTS (
                SELECT 1 FROM game_item_instances
                WHERE item_instance_id = OLD.item_instance_id AND lifecycle = 2
                  AND game_bank_robbery_retiring(item_instance_id, last_transaction_id)
            ) THEN
                UPDATE game_character_depot_box_counters SET committed = committed - 1
                  WHERE character_id = OLD.character_id AND town_id = OLD.town_id
                    AND box_number = OLD.box_number;
                RETURN OLD;
            END IF;
        $depot$),
        ('game_character_inbox_remove', $inbox$
            IF EXISTS (
                SELECT 1 FROM game_item_instances
                WHERE item_instance_id = OLD.item_instance_id AND lifecycle = 2
                  AND game_bank_robbery_retiring(item_instance_id, last_transaction_id)
            ) THEN RETURN OLD; END IF;
        $inbox$)
    LOOP
        definition := pg_get_functiondef((guard || '()')::regprocedure);
        entry_at := regexp_instr(definition, E'\\mBEGIN\\M', 1, 1, 0, 'i');
        IF entry_at = 0 THEN
            RAISE EXCEPTION 'missing item custody guard %', guard;
        END IF;
        EXECUTE overlay(definition placing E'BEGIN\n' || arm FROM entry_at FOR 5);
    END LOOP;

    EXECUTE format('ALTER FUNCTION game_bank_robbery_bind_mint() SET search_path = pg_catalog,%I,pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_bank_robbery_close() SET search_path = pg_catalog,%I,pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_bank_robbery_retiring(UUID,UUID) SET search_path = pg_catalog,%I,pg_temp', current_schema());
END
$migration$;
REVOKE ALL ON FUNCTION game_bank_robbery_bind_mint() FROM PUBLIC;
REVOKE ALL ON FUNCTION game_bank_robbery_close() FROM PUBLIC;
REVOKE ALL ON FUNCTION game_bank_robbery_retiring(UUID,UUID) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_bank_robbery_retiring(UUID,UUID) TO oteryn_game_runtime;
