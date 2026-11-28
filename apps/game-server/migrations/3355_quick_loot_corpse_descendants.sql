-- Quick Loot uses the physical corpse root for rights, including for nested kill loot.
CREATE OR REPLACE FUNCTION game_corpse_loot_origin(p_item UUID, p_world UUID)
RETURNS TABLE(corpse UUID, depth INTEGER)
LANGUAGE sql STABLE STRICT AS $$
    WITH RECURSIVE ancestors(item, depth) AS (
        SELECT p_item, 1
        UNION ALL
        SELECT n.parent_item_instance_id, a.depth + 1
          FROM ancestors a
          JOIN game_item_nested_entries n ON n.item_instance_id = a.item AND n.world_id = p_world
          JOIN game_item_instances p ON p.item_instance_id = n.parent_item_instance_id
                                    AND p.world_id = n.world_id AND p.lifecycle = 1 AND p.quantity = 1
         WHERE a.depth < 8
    )
    SELECT e.parent_item_instance_id,a.depth
      FROM ancestors a
      JOIN game_item_corpse_container_entries e ON e.item_instance_id = a.item AND e.world_id = p_world
     LIMIT 1
$$;

DO $migration$
DECLARE
    body TEXT := pg_get_functiondef('game_item_loop_line_admit()'::regprocedure);
    old_source CONSTANT TEXT := $old$SELECT 1 FROM game_item_corpse_container_entries e
            JOIN game_item_ground_locations cg ON cg.item_instance_id=e.parent_item_instance_id
            JOIN game_item_instances ci ON ci.item_instance_id=cg.item_instance_id
            JOIN game_item_mint_receipts cr ON cr.item_instance_id=ci.item_instance_id
            WHERE e.item_instance_id=NEW.item_instance_id AND ci.lifecycle=1$old$;
    new_source CONSTANT TEXT := $new$SELECT 1 FROM game_item_ground_locations cg
            JOIN game_item_instances ci ON ci.item_instance_id=cg.item_instance_id
            JOIN game_item_mint_receipts cr ON cr.item_instance_id=ci.item_instance_id
            WHERE cg.item_instance_id=(SELECT corpse FROM game_corpse_loot_origin(NEW.item_instance_id,r.world_id)) AND ci.lifecycle=1
              AND NOT EXISTS (SELECT 1 FROM game_item_nested_entries WHERE parent_item_instance_id=NEW.item_instance_id)$new$;
BEGIN
    IF strpos(body,new_source)=0 THEN
        IF (length(body)-length(replace(body,old_source,'')))/length(old_source) <> 1 THEN
            RAISE EXCEPTION 'missing exact Quick Loot source guard';
        END IF;
        EXECUTE replace(body, old_source, new_source);
    END IF;
    EXECUTE format('ALTER FUNCTION game_corpse_loot_origin(uuid,uuid) SET search_path = pg_catalog, %I, pg_temp', current_schema());
END
$migration$;

REVOKE ALL ON FUNCTION game_corpse_loot_origin(UUID, UUID) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_corpse_loot_origin(UUID, UUID) TO oteryn_game_runtime;

-- Visit children before each container and preserve sibling placement order.
-- Read one indexed edge at a time and stop producing candidates at the caller's
-- budget. The two callers use the command and kill-loot-plan limits respectively.
CREATE OR REPLACE FUNCTION game_corpse_loot_postorder(p_root UUID, p_limit INTEGER)
RETURNS TABLE(item UUID, visit_ordinal INTEGER)
LANGUAGE plpgsql STABLE STRICT AS $walk$
DECLARE
    current_item UUID := p_root;
    after_ordinal NUMERIC := 0;
    parent_items UUID[] := ARRAY[]::uuid[];
    parent_ordinals NUMERIC[] := ARRAY[]::numeric[];
    child UUID;
    child_ordinal NUMERIC;
    depth INTEGER := 0;
BEGIN
    visit_ordinal := 0;
    WHILE visit_ordinal < least(p_limit,4097) LOOP
        IF depth=0 THEN
            SELECT e.item_instance_id,e.placement_ordinal INTO child,child_ordinal
            FROM game_item_corpse_container_entries e
            JOIN game_item_instances i USING(item_instance_id,world_id)
            WHERE e.parent_item_instance_id=current_item AND e.placement_ordinal>after_ordinal AND i.lifecycle=1
            ORDER BY e.placement_ordinal LIMIT 1;
        ELSIF depth<8 THEN
            SELECT e.item_instance_id,e.placement_ordinal INTO child,child_ordinal
            FROM game_item_nested_entries e
            JOIN game_item_instances i USING(item_instance_id,world_id)
            WHERE e.parent_item_instance_id=current_item AND e.placement_ordinal>after_ordinal AND i.lifecycle=1
            ORDER BY e.placement_ordinal LIMIT 1;
        ELSE
            child := NULL;
        END IF;
        IF child IS NOT NULL THEN
            depth := depth+1;
            parent_items[depth] := current_item;
            parent_ordinals[depth] := child_ordinal;
            current_item := child;
            after_ordinal := 0;
        ELSIF depth=0 THEN
            RETURN;
        ELSE
            item := current_item;
            visit_ordinal := visit_ordinal+1;
            RETURN NEXT;
            current_item := parent_items[depth];
            after_ordinal := parent_ordinals[depth];
            depth := depth-1;
        END IF;
    END LOOP;
END
$walk$;

-- Nested removal evidence records the actual edge and the same physical
-- transaction as the retirement. Direct corpse and other owners retain their
-- original evidence tables and proof paths.
CREATE TABLE IF NOT EXISTS game_corpse_nested_decay_edges (
    item_instance_id UUID PRIMARY KEY REFERENCES game_item_instances(item_instance_id),
    parent_item_instance_id UUID NOT NULL,
    corpse_item_instance_id UUID NOT NULL,
    placement_ordinal NUMERIC(20,0) NOT NULL CHECK (placement_ordinal BETWEEN 1 AND 18446744073709551615),
    transaction_id UUID NOT NULL UNIQUE,
    created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id()
);
REVOKE ALL ON game_corpse_nested_decay_edges FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;

CREATE OR REPLACE FUNCTION game_corpse_nested_decay_edge_proven()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER AS $proof$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM game_item_decay_retire_receipts r
        WHERE (r.item_instance_id,r.corpse_item_instance_id,r.placement_ordinal,r.transaction_id,r.created_xact_id)
            = (NEW.item_instance_id,NEW.corpse_item_instance_id,NEW.placement_ordinal,NEW.transaction_id,pg_current_xact_id())
          AND r.cause_kind='CORPSE_DECAY'
    ) THEN
        RAISE EXCEPTION 'nested corpse removal requires its retirement receipt' USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END
$proof$;
DROP TRIGGER IF EXISTS game_corpse_nested_decay_edge_proven ON game_corpse_nested_decay_edges;
CREATE CONSTRAINT TRIGGER game_corpse_nested_decay_edge_proven
    AFTER INSERT ON game_corpse_nested_decay_edges DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW EXECUTE FUNCTION game_corpse_nested_decay_edge_proven();
REVOKE ALL ON FUNCTION game_corpse_nested_decay_edge_proven() FROM PUBLIC;

CREATE OR REPLACE FUNCTION game_corpse_decay_remove_nested(p_item UUID, p_parent UUID, p_transaction UUID)
RETURNS BIGINT LANGUAGE sql VOLATILE STRICT SECURITY DEFINER AS $remove$
    WITH removed AS (
        DELETE FROM game_item_nested_entries e USING game_item_decay_retire_reservations r,
          game_item_ground_locations g, game_item_instances i
        WHERE e.item_instance_id=p_item AND e.parent_item_instance_id=p_parent
          AND r.item_instance_id=e.item_instance_id AND r.transaction_id=p_transaction
          AND r.cause_kind='CORPSE_DECAY' AND r.placement_ordinal=e.placement_ordinal AND r.world_id=e.world_id
          AND r.corpse_item_instance_id=g.item_instance_id AND g.world_id=r.world_id
          AND (g.scope_kind,g.channel_id,g.instance_id) IS NOT DISTINCT FROM (r.scope_kind,r.channel_id,r.instance_id)
          AND g.item_instance_id=i.item_instance_id AND i.lifecycle=1
          AND g.item_instance_id=(SELECT corpse FROM game_corpse_loot_origin(e.item_instance_id,e.world_id))
          AND r.deadline=game_corpse_retirement_deadline(g.item_instance_id)
          AND r.deadline<=floor(extract(epoch FROM clock_timestamp())*1000)::bigint
          AND game_corpse_terminal_stage_ready(g.item_instance_id)
        RETURNING e.item_instance_id,e.parent_item_instance_id,g.item_instance_id AS corpse,
                  e.placement_ordinal,r.transaction_id
    ), evidence AS (
        INSERT INTO game_corpse_nested_decay_edges
            (item_instance_id,parent_item_instance_id,corpse_item_instance_id,placement_ordinal,transaction_id)
        SELECT item_instance_id,parent_item_instance_id,corpse,placement_ordinal,transaction_id FROM removed
        RETURNING 1
    )
    SELECT count(*) FROM evidence
$remove$;

DO $migration$
DECLARE
    body TEXT;
    anchor CONSTANT TEXT := 'AND cre.created_xact_id = pg_current_xact_id())) THEN';
    proof CONSTANT TEXT := $proof$AND cre.created_xact_id = pg_current_xact_id()
                  AND cre.placement_ordinal<=48)
           AND NOT EXISTS (
               SELECT 1 FROM game_corpse_nested_decay_edges e
                WHERE (e.item_instance_id,e.corpse_item_instance_id,e.placement_ordinal,e.transaction_id,e.created_xact_id)
                    = (NEW.item_instance_id,NEW.corpse_item_instance_id,NEW.placement_ordinal,NEW.transaction_id,pg_current_xact_id())
           )) THEN$proof$;
    nested_arm CONSTANT TEXT := $arm$
    IF TG_OP='DELETE' AND EXISTS (
        SELECT 1 FROM game_corpse_nested_decay_edges e
        JOIN game_item_decay_retire_receipts r USING(item_instance_id,transaction_id)
        WHERE (e.item_instance_id,e.parent_item_instance_id,e.placement_ordinal,e.created_xact_id)
            = (OLD.item_instance_id,OLD.parent_item_instance_id,OLD.placement_ordinal,pg_current_xact_id())
          AND r.created_xact_id=e.created_xact_id AND r.cause_kind='CORPSE_DECAY'
          AND r.corpse_item_instance_id=e.corpse_item_instance_id AND r.placement_ordinal=e.placement_ordinal
          AND r.world_id=OLD.world_id
    ) THEN RETURN NULL; END IF;
$arm$;
    start_at INTEGER;
    table_name TEXT;
BEGIN
    body:=pg_get_functiondef('game_item_decay_retire_consistency_guard()'::regprocedure);
    IF strpos(body,proof)=0 THEN
        IF (length(body)-length(replace(body,anchor,'')))/length(anchor)<>1 THEN
            RAISE EXCEPTION 'missing exact corpse decay removal proof';
        END IF;
        EXECUTE replace(body,anchor,proof);
    END IF;

    body:=pg_get_functiondef('game_item_nested_entry_proven()'::regprocedure);
    IF strpos(body,nested_arm)=0 THEN
        start_at := regexp_instr(body,E'\\mBEGIN\\M',1,1,1,'i');
        IF start_at=0 THEN RAISE EXCEPTION 'missing nested item custody proof'; END IF;
        EXECUTE overlay(body placing nested_arm from start_at for 0);
    END IF;

    -- Direct receipt proofs still enforce the direct-entry ordinal bound.
    -- Nested entries use the exact u64 ordinal from their own parent.
    FOREACH table_name IN ARRAY ARRAY['game_item_decay_retire_reservations','game_item_decay_retire_receipts'] LOOP
        EXECUTE format('ALTER TABLE %I DROP CONSTRAINT IF EXISTS game_retire_placement_ordinal',table_name);
        EXECUTE format('ALTER TABLE %I ADD CONSTRAINT game_retire_placement_ordinal CHECK (placement_ordinal BETWEEN 1 AND 18446744073709551615)',table_name);
    END LOOP;

    EXECUTE format('ALTER FUNCTION game_corpse_nested_decay_edge_proven() SET search_path=pg_catalog,%I,pg_temp',current_schema());
    EXECUTE format('ALTER FUNCTION game_corpse_loot_postorder(uuid,integer) SET search_path=pg_catalog,%I,pg_temp',current_schema());
    EXECUTE format('ALTER FUNCTION game_corpse_decay_remove_nested(uuid,uuid,uuid) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END
$migration$;
REVOKE ALL ON FUNCTION game_corpse_loot_postorder(UUID,INTEGER),game_corpse_decay_remove_nested(UUID,UUID,UUID) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_corpse_loot_postorder(UUID,INTEGER),game_corpse_decay_remove_nested(UUID,UUID,UUID) TO oteryn_game_runtime;
