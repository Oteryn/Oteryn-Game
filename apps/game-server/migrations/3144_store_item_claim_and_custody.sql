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

-- Only a bound item and its ancestors stay. Each unbound branch below them
-- can use the ordinary one-item death writer, with its subtree intact.
CREATE FUNCTION game_player_death_custody_split(p_item UUID, p_character UUID, p_world UUID)
RETURNS TABLE(retained_item UUID, next_item UUID)
LANGUAGE sql STABLE SECURITY DEFINER AS $$
    WITH RECURSIVE tree AS MATERIALIZED (
        SELECT t.* FROM game_item_instances i
        CROSS JOIN LATERAL game_item_loop_subtree_edges(i.item_instance_id) t
        WHERE i.item_instance_id=p_item AND i.world_id=p_world AND i.lifecycle=1
          AND EXISTS (
            SELECT 1 FROM game_item_container_slots s
            WHERE s.item_instance_id=p_item AND s.character_id=p_character AND s.world_id=p_world
            UNION ALL
            SELECT 1 FROM game_character_equipment_slots s
            WHERE s.item_instance_id=p_item AND s.character_id=p_character AND s.world_id=p_world
            UNION ALL
            SELECT 1 FROM game_item_container_entries e JOIN game_item_container_slots s
              ON s.item_instance_id=e.parent_item_instance_id AND s.world_id=e.world_id
            WHERE e.item_instance_id=p_item AND s.character_id=p_character AND e.world_id=p_world
          )
    ), retained(item,parent) AS (
        SELECT t.item_instance_id,t.parent_item_instance_id FROM tree t
        JOIN game_item_bindings b ON b.item_instance_id=t.item_instance_id
        JOIN game_item_instances i ON i.item_instance_id=b.item_instance_id AND i.world_id=b.world_id
        WHERE b.character_id=p_character AND b.world_id=p_world AND i.lifecycle=1
        UNION
        SELECT t.item_instance_id,t.parent_item_instance_id FROM tree t
        JOIN retained r ON r.parent=t.item_instance_id
    )
    SELECT p_item, (
        SELECT t.item_instance_id FROM tree t JOIN retained r ON r.item=t.parent_item_instance_id
        WHERE NOT EXISTS (SELECT 1 FROM retained keep WHERE keep.item=t.item_instance_id)
        ORDER BY t.item_instance_id LIMIT 1
    )
    WHERE EXISTS (SELECT 1 FROM retained WHERE item=p_item)
      AND (SELECT count(*)<=501 AND count(*)=count(DISTINCT item_instance_id)
                  AND bool_and(depth<=8 AND NOT cyclic AND (depth=0 OR world_id=p_world))
           FROM tree)
$$;
DO $$ BEGIN
    EXECUTE format('ALTER FUNCTION game_player_death_custody_split(UUID, UUID, UUID) SET search_path = pg_catalog, %I, pg_temp', current_schema());
END $$;
REVOKE ALL ON FUNCTION game_player_death_custody_split(UUID, UUID, UUID) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_player_death_custody_split(UUID, UUID, UUID)
    TO oteryn_game_runtime, oteryn_game_control;

-- A branch can use a deeper carried edge only while the immutable death
-- selection still requires it. Capture the actual deleted edge in the same
-- transaction, as for the original direct backpack entries.
DO $migration$
DECLARE body TEXT; amended TEXT; begin_at INT;
BEGIN
    body:=pg_get_functiondef('game_player_death_item_admit()'::regprocedure);
    amended:=replace(body,'AND e.placed_transaction_id=NEW.source_placement))',$branch$
     AND e.placed_transaction_id=NEW.source_placement)
     AND NOT EXISTS (
        SELECT 1 FROM game_character_death_receipts d
        CROSS JOIN LATERAL unnest(d.lost_item_ids) root(item)
        CROSS JOIN LATERAL game_player_death_custody_split(root.item,d.character_id,d.death_world_id) split
        WHERE d.death_occurrence_id=NEW.death_occurrence_id AND d.character_id=NEW.character_id
          AND d.death_world_id=NEW.world_id AND NEW.operation=1 AND split.next_item=NEW.item_instance_id
          AND EXISTS (
            SELECT 1 FROM (
                SELECT world_id,parent_item_instance_id,placement_ordinal,placed_transaction_id
                FROM game_item_nested_entries WHERE item_instance_id=NEW.item_instance_id
                UNION ALL
                SELECT world_id,parent_item_instance_id,placement_ordinal,placed_transaction_id
                FROM game_item_container_entries WHERE item_instance_id=NEW.item_instance_id
            ) e
            WHERE (e.world_id,e.parent_item_instance_id,e.placement_ordinal,e.placed_transaction_id)
                =(NEW.world_id,NEW.source_parent,NEW.source_ordinal,NEW.source_placement)
          )
     ))$branch$);
    IF amended=body THEN RAISE EXCEPTION 'missing death entry admission guard'; END IF;
    EXECUTE amended;

    body:=pg_get_functiondef('game_player_death_item_capture_removal()'::regprocedure);
    amended:=replace(body,$old$ELSIF TG_TABLE_NAME='game_item_container_entries' THEN
  INSERT INTO game_player_death_item_removals(transaction_id)
   SELECT r.transaction_id FROM game_player_death_item_records r
    WHERE r.item_instance_id=OLD.item_instance_id AND r.character_id=OLD.character_id AND r.world_id=OLD.world_id$old$,
    $new$ELSIF TG_TABLE_NAME IN ('game_item_container_entries','game_item_nested_entries') THEN
  INSERT INTO game_player_death_item_removals(transaction_id)
   SELECT r.transaction_id FROM game_player_death_item_records r
    WHERE r.item_instance_id=OLD.item_instance_id AND r.world_id=OLD.world_id$new$);
    IF amended=body THEN RAISE EXCEPTION 'missing death entry removal capture'; END IF;
    EXECUTE amended;

    body:=pg_get_functiondef('game_item_container_entry_removal_proven()'::regprocedure);
    amended:=replace(body,$old$r.character_id=OLD.character_id
     AND r.source_slot=0$old$,$new$r.world_id=OLD.world_id
     AND r.source_slot=0$new$);
    IF amended=body THEN RAISE EXCEPTION 'missing direct death entry removal proof'; END IF;
    EXECUTE amended;

    body:=pg_get_functiondef('game_item_nested_entry_proven()'::regprocedure);
    begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
    IF begin_at=0 THEN RAISE EXCEPTION 'missing nested entry proof'; END IF;
    EXECUTE overlay(body placing E'BEGIN\n'||$arm$
    IF TG_OP='DELETE' AND EXISTS(
        SELECT 1 FROM game_player_death_item_records r JOIN game_player_death_item_removals e USING(transaction_id)
        WHERE r.item_instance_id=OLD.item_instance_id AND r.world_id=OLD.world_id AND r.source_slot=0
          AND r.source_parent=OLD.parent_item_instance_id AND r.source_ordinal=OLD.placement_ordinal
          AND r.source_placement=OLD.placed_transaction_id
          AND r.created_xact_id=pg_current_xact_id() AND e.created_xact_id=pg_current_xact_id()
    ) THEN RETURN NULL; END IF;
$arm$ from begin_at for 5);
END $migration$;
CREATE TRIGGER game_player_death_item_capture_removal BEFORE DELETE ON game_item_nested_entries
    FOR EACH ROW EXECUTE FUNCTION game_player_death_item_capture_removal();

-- Keep historical selections immutable. A retained root is complete once
-- every unbound branch has left its carried tree through the death writer.
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
          AND NOT EXISTS (
            SELECT 1 FROM game_player_death_custody_split(selected.item,d.character_id,d.death_world_id) split
            WHERE split.next_item IS NULL OR selected.item=d.amulet_of_loss_item_id
          )
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
