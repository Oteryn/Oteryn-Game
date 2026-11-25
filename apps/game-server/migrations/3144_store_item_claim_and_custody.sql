-- Store Inbox claims bind a complete item UUID. Depot searches still bind a
-- four-byte definition ID and a one-byte retrieval mode.
ALTER TABLE game_depot_search_retrieve_receipts
    DROP CONSTRAINT game_depot_search_retrieve_receipts_request_check,
    ADD CONSTRAINT game_depot_search_retrieve_receipts_request_check
        CHECK (octet_length(request) IN (5, 16));

-- A Store Inbox item retains the purchaser's custody through the existing item binding proofs.
ALTER TABLE game_item_bindings
    DROP CONSTRAINT game_item_bindings_cause_check,
    ADD CONSTRAINT game_item_bindings_cause_check
        CHECK (cause IN ('DAILY_REWARD','STORE_PURCHASE'));

-- Backfill retained ordinary mints that are still in the purchaser's custody.
-- Earlier transfers cannot establish present custody for the original purchaser.
WITH RECURSIVE ancestry(item,world,character,ancestor,depth) AS (
    SELECT r.item_instance_id,r.world_id,r.character_id,r.item_instance_id,0
    FROM store_delivery_receipts r JOIN game_item_instances i USING(item_instance_id,world_id)
    WHERE i.lifecycle=1 AND i.definition_production_key<>'oteryn:item.tibia.i29432'
    UNION ALL
    SELECT a.item,a.world,a.character,n.parent_item_instance_id,a.depth+1
    FROM ancestry a JOIN game_item_nested_entries n
      ON n.item_instance_id=a.ancestor AND n.world_id=a.world
    WHERE a.depth<8
)
INSERT INTO game_item_bindings(item_instance_id,world_id,character_id,cause)
SELECT item,world,character,'STORE_PURCHASE' FROM ancestry a
WHERE EXISTS (
    SELECT 1 FROM game_item_character_inbox_locations x
    WHERE x.item_instance_id=a.ancestor AND x.character_id=a.character AND x.world_id=a.world
    UNION ALL
    SELECT 1 FROM game_item_character_depot_locations d
    WHERE d.item_instance_id=a.ancestor AND d.character_id=a.character AND d.world_id=a.world
    UNION ALL
    SELECT 1 FROM game_character_equipment_slots s
    WHERE s.item_instance_id=a.ancestor AND s.character_id=a.character AND s.world_id=a.world
    UNION ALL
    SELECT 1 FROM game_item_container_entries e
    WHERE e.item_instance_id=a.ancestor AND e.character_id=a.character AND e.world_id=a.world
    UNION ALL
    SELECT 1 FROM game_item_container_slots s
    WHERE s.item_instance_id=a.ancestor AND s.character_id=a.character
)
ON CONFLICT(item_instance_id) DO NOTHING;

-- A death drop moves a carried root with its entire subtree. Check both
-- carried edge types using the same depth and size bounds as the custody guard.
CREATE FUNCTION game_player_death_item_custody_protected(p_item UUID, p_character UUID, p_world UUID)
RETURNS BOOLEAN
LANGUAGE sql STABLE SECURITY DEFINER SET search_path=pg_catalog,public AS $$
    WITH RECURSIVE tree(item,depth) AS (
        SELECT i.item_instance_id,0
        FROM public.game_item_instances i
        WHERE i.item_instance_id=p_item AND i.world_id=p_world AND i.lifecycle=1
          AND EXISTS (
            SELECT 1 FROM public.game_item_container_slots s
            WHERE s.item_instance_id=p_item AND s.character_id=p_character
            UNION ALL
            SELECT 1 FROM public.game_character_equipment_slots s
            WHERE s.item_instance_id=p_item AND s.character_id=p_character AND s.world_id=p_world
            UNION ALL
            SELECT 1 FROM public.game_item_container_entries e
            WHERE e.item_instance_id=p_item AND e.character_id=p_character AND e.world_id=p_world
          )
        UNION ALL
        SELECT child.item_instance_id,t.depth+1
        FROM tree t CROSS JOIN LATERAL (
            SELECT e.item_instance_id FROM public.game_item_container_entries e
            WHERE e.parent_item_instance_id=t.item AND e.world_id=p_world
            UNION ALL
            SELECT n.item_instance_id FROM public.game_item_nested_entries n
            WHERE n.parent_item_instance_id=t.item AND n.world_id=p_world
        ) child
        WHERE t.depth<8
    )
    SELECT EXISTS (
        SELECT 1 FROM (SELECT item FROM tree LIMIT 501) affected
        JOIN public.game_item_bindings b ON b.item_instance_id=affected.item
        JOIN public.game_item_instances i USING(item_instance_id,world_id)
        WHERE b.character_id=p_character AND b.world_id=p_world AND i.lifecycle=1
    )
$$;
REVOKE ALL ON FUNCTION game_player_death_item_custody_protected(UUID, UUID, UUID) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_player_death_item_custody_protected(UUID, UUID, UUID)
    TO oteryn_game_runtime, oteryn_game_control;

-- A pre-binding selection stays in its immutable receipt. Pending respawn may
-- finish if its selected item is still in the Character's protected custody.
CREATE OR REPLACE FUNCTION game_player_death_items_complete(occurrence UUID) RETURNS BOOLEAN
LANGUAGE sql STABLE AS $$
    SELECT NOT EXISTS (
        SELECT 1 FROM game_character_death_receipts d,
        LATERAL unnest(d.lost_item_ids || CASE WHEN d.amulet_of_loss_item_id IS NULL
            THEN ARRAY[]::uuid[] ELSE ARRAY[d.amulet_of_loss_item_id] END) selected(item)
        WHERE d.death_occurrence_id=occurrence
          AND NOT EXISTS (
            SELECT 1 FROM game_player_death_item_records r
            WHERE r.death_occurrence_id=d.death_occurrence_id AND r.item_instance_id=selected.item
              AND r.character_id=d.character_id
              AND r.operation=CASE WHEN d.amulet_of_loss_item_id=selected.item THEN 2 ELSE 1 END
          )
          AND NOT game_player_death_item_custody_protected(selected.item,d.character_id,d.death_world_id)
    )
$$;
DO $$ BEGIN
    EXECUTE format('ALTER FUNCTION game_player_death_items_complete(UUID) SET search_path = %I, pg_temp', current_schema());
END $$;

-- Platform allocates the CLAIM identity independently of the delivery identity.
CREATE TABLE game_store_claim_operations (
    delivery_operation_id UUID PRIMARY KEY REFERENCES store_delivery_receipts(operation_id),
    claim_operation_id UUID NOT NULL UNIQUE CHECK (game_character_is_uuid_v7(claim_operation_id)),
    CHECK (delivery_operation_id<>claim_operation_id)
);
CREATE TRIGGER store_claim_operation_immutable BEFORE UPDATE OR DELETE ON game_store_claim_operations
    FOR EACH ROW EXECUTE FUNCTION game_character_immutable();
CREATE TRIGGER store_claim_operations_no_truncate BEFORE TRUNCATE ON game_store_claim_operations
    EXECUTE FUNCTION game_character_reject_truncate();
REVOKE ALL ON game_store_claim_operations FROM PUBLIC;
GRANT SELECT, INSERT ON game_store_claim_operations TO oteryn_game_runtime;
GRANT SELECT ON game_store_claim_operations TO oteryn_game_control;

CREATE INDEX store_delivery_item_entitlement
    ON store_delivery_receipts(account_id,character_id,world_id,fulfilment_unit_id,operation_id)
    WHERE item_instance_id IS NOT NULL;
CREATE INDEX store_claim_inbox_move ON game_item_loop_move_lines(item_instance_id,transaction_id)
    WHERE from_kind=6 AND to_kind=1;

-- An operator can resolve a reopened operation whose Game decision was lost.
-- A retained conflict keeps gameplay from applying it while Platform reviews it.
CREATE TABLE game_store_restore_conflicts (
    operation_id UUID PRIMARY KEY CHECK (game_character_is_uuid_v7(operation_id))
);
REVOKE ALL ON game_store_restore_conflicts FROM PUBLIC;
GRANT SELECT ON game_store_restore_conflicts TO oteryn_game_runtime;
GRANT SELECT, INSERT ON game_store_restore_conflicts TO oteryn_game_control;
