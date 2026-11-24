-- W17: a pit visit performs one item reset, including across projection recovery
-- and a lost commit acknowledgement. Keep the existing one-item audit and receipt.
CREATE TABLE game_arena_pit_item_resets (
    world_id UUID NOT NULL,
    channel_id UUID NOT NULL,
    run_id UUID NOT NULL CHECK (game_character_is_uuid_v7(run_id)),
    pit SMALLINT NOT NULL CHECK (pit BETWEEN 1 AND 10),
    rank SMALLINT NOT NULL CHECK (rank BETWEEN 1 AND 3),
    character_id UUID NOT NULL,
    game_session_id UUID NOT NULL,
    scope_generation NUMERIC(20,0) NOT NULL CHECK (scope_generation BETWEEN 1 AND 18446744073709551615),
    expires_at_ms BIGINT NOT NULL CHECK (expires_at_ms > 0),
    binding BYTEA GENERATED ALWAYS AS (sha256(uuid_send(run_id) || int2send(pit))) STORED PRIMARY KEY,
    created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id()
);
REVOKE ALL ON game_arena_pit_item_resets FROM PUBLIC;
GRANT SELECT ON game_arena_pit_item_resets TO oteryn_game_control;
GRANT SELECT, INSERT (world_id, channel_id, run_id, pit, rank, character_id,
    game_session_id, scope_generation, expires_at_ms) ON game_arena_pit_item_resets TO oteryn_game_runtime;

-- Only the pre-DELETE guard can record that a cell actually lost its top.
CREATE TABLE game_arena_pit_item_removals (
    binding BYTEA NOT NULL REFERENCES game_arena_pit_item_resets(binding),
    cell BYTEA NOT NULL CHECK (octet_length(cell)=8),
    item_instance_id UUID NOT NULL UNIQUE,
    PRIMARY KEY (binding,cell)
);
REVOKE ALL ON game_arena_pit_item_removals FROM PUBLIC;
GRANT SELECT ON game_arena_pit_item_removals TO oteryn_game_control;
GRANT SELECT ON game_arena_pit_item_removals TO oteryn_game_runtime;

-- Record a child edge while its ancestors still have custody of it.
CREATE TABLE game_arena_pit_item_edges (
    item_instance_id UUID PRIMARY KEY,
    binding BYTEA NOT NULL REFERENCES game_arena_pit_item_resets(binding),
    ground_root_item_instance_id UUID NOT NULL,
    parent_item_instance_id UUID NOT NULL,
    placement_ordinal NUMERIC(20,0) NOT NULL CHECK (placement_ordinal BETWEEN 1 AND 18446744073709551615),
    entry_kind SMALLINT NOT NULL CHECK (entry_kind IN (1,2)),
    UNIQUE (ground_root_item_instance_id,parent_item_instance_id,placement_ordinal),
    CHECK (item_instance_id<>parent_item_instance_id AND item_instance_id<>ground_root_item_instance_id)
);
REVOKE ALL ON game_arena_pit_item_edges FROM PUBLIC;
GRANT SELECT ON game_arena_pit_item_edges TO oteryn_game_runtime,oteryn_game_control;

DO $migration$
DECLARE t TEXT; prior TEXT;
BEGIN
    FOREACH t IN ARRAY ARRAY['game_item_decay_retire_reservations','game_item_decay_retire_receipts'] LOOP
        SELECT pg_get_expr(conbin,conrelid) INTO STRICT prior FROM pg_constraint
        WHERE conrelid=t::regclass AND conname='game_reset_retire_shape';
        EXECUTE format('ALTER TABLE %I DROP CONSTRAINT game_reset_retire_shape',t);
        EXECUTE format($shape$
            ALTER TABLE %I ADD CONSTRAINT game_reset_retire_shape CHECK (
                CASE WHEN cause_kind='ARENA_RESET' THEN
                    scope_kind=1 AND occurred_at>0 AND corpse_item_instance_id IS NULL AND deadline IS NULL
                    AND reset_epoch IS NULL AND target_digest IS NULL
                    AND retirement_binding IS NOT NULL AND octet_length(retirement_binding)=32
                    AND ground_root_item_instance_id IS NOT NULL
                    AND (ground_root_item_instance_id=item_instance_id)=(placement_ordinal IS NULL)
                    AND activity_handoff_id IS NULL AND activity_ownership_generation IS NULL
                ELSE %s END)
        $shape$,t,prior);
    END LOOP;
END $migration$;

CREATE FUNCTION game_arena_pit_item_reset_guard() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
    PERFORM 1 FROM game_svargrond_pit_leases l
    JOIN game_character_arena_matches s USING (character_id)
    WHERE (l.world_id,l.channel_id,l.run_id,l.pit,l.character_id,l.game_session_id,l.scope_generation,l.expires_at_ms)
        =(NEW.world_id,NEW.channel_id,NEW.run_id,NEW.pit,NEW.character_id,NEW.game_session_id,NEW.scope_generation,NEW.expires_at_ms)
      AND (s.body->>'phase')='1' AND (s.body->>'pit')=NEW.pit::text AND (s.body->>'rank')=NEW.rank::text
      AND NEW.expires_at_ms>floor(extract(epoch FROM clock_timestamp())*1000)::bigint
      AND (s.body->>'entry_revision')::numeric>0
      AND NOT EXISTS(SELECT 1 FROM game_character_death_receipts d WHERE d.character_id=NEW.character_id
          AND d.committed_character_revision>(s.body->>'entry_revision')::numeric)
    FOR SHARE OF l,s;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'arena item reset requires the current fighting pit lease' USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END
$$;
CREATE CONSTRAINT TRIGGER game_arena_pit_item_reset_current AFTER INSERT ON game_arena_pit_item_resets
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_arena_pit_item_reset_guard();

-- Verify the pre-DELETE Ground row while it still exists. The set of durable
-- appends includes rows the native projection may be unable to render.
CREATE FUNCTION game_arena_reset_ground_top() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE visit game_arena_pit_item_resets%ROWTYPE; dx INTEGER; dy INTEGER;
BEGIN
    SELECT p.* INTO visit FROM game_item_decay_retire_reservations v
    JOIN game_arena_pit_item_resets p ON p.binding=v.retirement_binding
    WHERE v.item_instance_id=OLD.item_instance_id AND v.cause_kind='ARENA_RESET'
      AND p.created_xact_id=pg_current_xact_id();
    IF NOT FOUND THEN RETURN OLD; END IF;
    PERFORM pg_advisory_xact_lock(hashtextextended(encode(uuid_send(visit.world_id),'hex')
        || encode(uuid_send(visit.channel_id),'hex'),33));
    dx := (ARRAY[0,-14,-28,-42,-35,-21,-7,-14,-28,-21])[visit.pit];
    dy := (ARRAY[0,0,0,0,-14,-14,-14,-28,-28,-42])[visit.pit];
    IF OLD.scope_kind<>1 OR OLD.world_id<>visit.world_id OR OLD.channel_id<>visit.channel_id
       OR OLD.runtime_scope_ownership_generation<>visit.scope_generation
       OR OLD.stack_ordinal IS NULL OR octet_length(OLD.spatial_position)<>10
       OR substring(OLD.spatial_position FROM 9 FOR 2) NOT IN (int2send(-7::smallint),int2send(7::smallint))
       OR substring(OLD.spatial_position FROM 1 FOR 4) NOT BETWEEN int4send(32205+dx) AND int4send(32214+dx)
       OR substring(OLD.spatial_position FROM 5 FOR 4) NOT BETWEEN int4send(31094+dy) AND int4send(31103+dy)
       OR EXISTS (SELECT 1 FROM game_item_ground_locations g
           WHERE g.scope_kind=1 AND g.world_id=OLD.world_id AND g.channel_id=OLD.channel_id
             AND g.spatial_position IN (substring(OLD.spatial_position FOR 8)||int2send(-7::smallint),
                                        substring(OLD.spatial_position FOR 8)||int2send(7::smallint))
             AND g.item_instance_id<>OLD.item_instance_id
             AND (g.stack_ordinal IS NULL OR g.stack_ordinal>=OLD.stack_ordinal)) THEN
        RAISE EXCEPTION 'arena item reset requires the exact top Ground item in its leased pit' USING ERRCODE='23514';
    END IF;
    INSERT INTO game_arena_pit_item_removals(binding,cell,item_instance_id)
        VALUES(visit.binding,substring(OLD.spatial_position FOR 8),OLD.item_instance_id);
    RETURN OLD;
END
$$;
CREATE TRIGGER game_arena_reset_ground_top BEFORE DELETE ON game_item_ground_locations
    FOR EACH ROW EXECUTE FUNCTION game_arena_reset_ground_top();

CREATE FUNCTION game_arena_reset_capture_edge() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE v game_item_decay_retire_reservations%ROWTYPE;
        ancestor UUID; parents UUID[]; visited UUID[]; level INT; kind SMALLINT;
BEGIN
    SELECT r.* INTO v FROM game_item_decay_retire_reservations r
    JOIN game_arena_pit_item_resets p ON p.binding=r.retirement_binding
    WHERE r.item_instance_id=OLD.item_instance_id AND r.cause_kind='ARENA_RESET'
      AND r.placement_ordinal=OLD.placement_ordinal AND r.world_id=OLD.world_id
      AND p.created_xact_id=pg_current_xact_id();
    IF NOT FOUND THEN RETURN OLD; END IF;
    ancestor:=OLD.parent_item_instance_id;
    visited:=ARRAY[OLD.item_instance_id];
    kind:=CASE TG_TABLE_NAME WHEN 'game_item_container_entries' THEN 1 ELSE 2 END;
    FOR level IN 1..8 LOOP
        IF ancestor=ANY(visited) OR NOT EXISTS(SELECT 1 FROM game_item_instances
            WHERE item_instance_id=ancestor AND world_id=OLD.world_id AND lifecycle=1 AND quantity>0)
        THEN EXIT; END IF;
        visited:=visited||ancestor;
        IF ancestor=v.ground_root_item_instance_id THEN
            IF NOT EXISTS(SELECT 1 FROM game_item_ground_locations g WHERE g.item_instance_id=ancestor
                AND (g.world_id,g.channel_id,g.scope_kind,g.runtime_scope_ownership_generation)
                  =(v.world_id,v.channel_id,1,v.fence_scope_ownership_generation))
            THEN EXIT; END IF;
            INSERT INTO game_arena_pit_item_edges VALUES(OLD.item_instance_id,v.retirement_binding,
                ancestor,OLD.parent_item_instance_id,OLD.placement_ordinal,kind);
            RETURN OLD;
        END IF;
        SELECT array_agg(parent_item_instance_id) INTO parents FROM (
            SELECT parent_item_instance_id FROM game_item_container_entries
                WHERE item_instance_id=ancestor AND world_id=OLD.world_id
            UNION ALL SELECT parent_item_instance_id FROM game_item_nested_entries
                WHERE item_instance_id=ancestor AND world_id=OLD.world_id
        ) edges;
        IF cardinality(parents) IS DISTINCT FROM 1 THEN EXIT; END IF;
        ancestor:=parents[1];
    END LOOP;
    RAISE EXCEPTION 'arena retirement requires a bounded live path to the Ground root' USING ERRCODE='23514';
END
$$;
CREATE TRIGGER game_arena_reset_capture_edge BEFORE DELETE ON game_item_nested_entries
    FOR EACH ROW EXECUTE FUNCTION game_arena_reset_capture_edge();
CREATE TRIGGER game_arena_reset_capture_edge BEFORE DELETE ON game_item_container_entries
    FOR EACH ROW EXECUTE FUNCTION game_arena_reset_capture_edge();

CREATE FUNCTION game_arena_reset_item_proven(p_item UUID) RETURNS BOOLEAN
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE r game_item_decay_retire_receipts%ROWTYPE;
BEGIN
    SELECT * INTO r FROM game_item_decay_retire_receipts
    WHERE item_instance_id=p_item AND cause_kind='ARENA_RESET' AND created_xact_id=pg_current_xact_id();
    IF NOT FOUND THEN RETURN FALSE; END IF;
    IF NOT EXISTS (
        SELECT 1 FROM game_arena_pit_item_resets p
        JOIN game_item_decay_retire_reservations v ON v.retirement_binding=p.binding
        JOIN game_arena_pit_item_removals m ON m.binding=p.binding AND m.item_instance_id=r.ground_root_item_instance_id
        JOIN game_runtime_scope_assignments a ON a.world_id=p.world_id AND a.channel_id=p.channel_id
        JOIN game_item_audit_outbox o ON o.event_id=v.event_id AND o.transaction_id=v.transaction_id
        WHERE p.binding=r.retirement_binding AND p.created_xact_id=pg_current_xact_id()
          AND (p.world_id,p.channel_id,p.scope_generation)=(r.world_id,r.channel_id,r.fence_scope_ownership_generation)
          AND r.occurred_at<p.expires_at_ms
          AND (v.item_instance_id,v.world_id,v.channel_id,v.fence_scope_ownership_generation,v.transaction_id,
               v.event_id,v.quantity_before,v.occurred_at,v.cause_kind)
             =(p_item,r.world_id,r.channel_id,r.fence_scope_ownership_generation,r.transaction_id,
               r.event_id,r.quantity_before,r.occurred_at,r.cause_kind)
          AND (v.ground_root_item_instance_id,v.placement_ordinal)
              IS NOT DISTINCT FROM (r.ground_root_item_instance_id,r.placement_ordinal)
          AND (a.ownership_generation,a.holder_node_id,a.holder_registration_revision)
             =(p.scope_generation,v.fence_holder_node_id,v.fence_holder_registration_revision)
          AND o.item_instance_id=p_item AND o.occurred_at=r.occurred_at AND o.envelope=v.envelope
          AND o.envelope_sha256=r.envelope_sha256 AND sha256(v.envelope)=r.envelope_sha256
          AND o.created_xact_id=pg_current_xact_id() AND o.publication_state=1 AND o.published_at IS NULL)
       OR NOT EXISTS (SELECT 1 FROM game_item_instances i WHERE i.item_instance_id=p_item AND i.world_id=r.world_id
           AND i.lifecycle=2 AND i.quantity=0 AND i.last_transaction_id=r.transaction_id)
       OR NOT EXISTS (SELECT 1 FROM game_item_ground_removal_evidence e WHERE e.item_instance_id=r.ground_root_item_instance_id
           AND e.world_id=r.world_id AND e.channel_id=r.channel_id AND e.scope_kind=1 AND e.created_xact_id=pg_current_xact_id())
       OR (r.placement_ordinal IS NOT NULL AND NOT EXISTS (
           SELECT 1 FROM game_arena_pit_item_edges e
           JOIN game_item_decay_retire_receipts parent ON parent.item_instance_id=e.parent_item_instance_id
           JOIN game_item_decay_retire_receipts root ON root.item_instance_id=e.ground_root_item_instance_id
           WHERE e.item_instance_id=p_item AND e.binding=r.retirement_binding
             AND e.ground_root_item_instance_id=r.ground_root_item_instance_id
             AND e.placement_ordinal=r.placement_ordinal
             AND parent.created_xact_id=pg_current_xact_id() AND root.created_xact_id=pg_current_xact_id()
             AND parent.cause_kind='ARENA_RESET' AND root.cause_kind='ARENA_RESET'
             AND parent.retirement_binding=e.binding AND root.retirement_binding=e.binding
             AND parent.ground_root_item_instance_id=root.item_instance_id AND root.placement_ordinal IS NULL))
       OR (r.placement_ordinal IS NULL AND EXISTS (
           SELECT 1 FROM game_arena_pit_item_edges e WHERE e.ground_root_item_instance_id=p_item
           GROUP BY e.ground_root_item_instance_id
           HAVING count(*)>500 OR count(*) FILTER (WHERE NOT EXISTS (
               SELECT 1 FROM game_item_decay_retire_receipts child WHERE child.item_instance_id=e.item_instance_id
                 AND child.created_xact_id=pg_current_xact_id() AND child.retirement_binding=r.retirement_binding
                 AND child.cause_kind='ARENA_RESET'))>0))
       OR (r.placement_ordinal IS NULL AND EXISTS (
           SELECT 1 FROM game_arena_pit_item_edges e WHERE e.ground_root_item_instance_id=p_item
           GROUP BY e.parent_item_instance_id HAVING count(*)>20))
       OR NOT EXISTS (SELECT 1 FROM game_item_transfer_quantity_evidence e WHERE e.item_instance_id=p_item
           AND e.transaction_id=r.transaction_id AND e.quantity_before=r.quantity_before)
       OR EXISTS (SELECT 1 FROM game_item_ground_locations WHERE item_instance_id=p_item)
       OR EXISTS (SELECT 1 FROM game_item_container_slots WHERE item_instance_id=p_item)
       OR EXISTS (SELECT 1 FROM game_character_equipment_slots WHERE item_instance_id=p_item)
       OR EXISTS (SELECT 1 FROM game_item_corpse_container_entries WHERE item_instance_id=p_item OR parent_item_instance_id=p_item)
       OR EXISTS (SELECT 1 FROM game_item_container_entries WHERE item_instance_id=p_item OR parent_item_instance_id=p_item)
       OR EXISTS (SELECT 1 FROM game_item_nested_entries WHERE item_instance_id=p_item OR parent_item_instance_id=p_item) THEN
        RAISE EXCEPTION 'arena retirement requires exact Ground removal, conservation, reservation and audit' USING ERRCODE='23514';
    END IF;
    RETURN TRUE;
END
$$;

-- Keep all existing guard arms. Only admit the new cause through the same
-- terminal quantity-evidence arm and its own deferred same-transaction proof.
DO $migration$
DECLARE f TEXT; body TEXT; item_ref TEXT; begin_at INTEGER; gate TEXT; definition RECORD;
        anchor TEXT := 'r.cause_kind IN(''WORLD_RESET'',''ACTIVITY_RETIREMENT'')';
BEGIN
    SELECT pg_get_functiondef('game_item_instance_guard()'::regprocedure) INTO STRICT body;
    IF (length(body)-length(replace(body,anchor,'')))/length(anchor)<>1 THEN
        RAISE EXCEPTION 'unexpected terminal item retirement guard';
    END IF;
    EXECUTE replace(body,anchor,'r.cause_kind IN(''WORLD_RESET'',''ACTIVITY_RETIREMENT'',''ARENA_RESET'')');
    FOREACH f IN ARRAY ARRAY['game_item_decay_retire_consistency_guard','game_item_instance_change_proven',
        'game_item_ground_removal_proven'] LOOP
        SELECT pg_get_functiondef((f||'()')::regprocedure) INTO STRICT body;
        item_ref:=CASE WHEN f='game_item_ground_removal_proven' THEN 'OLD.item_instance_id' ELSE 'NEW.item_instance_id' END;
        begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
        IF begin_at=0 THEN RAISE EXCEPTION 'missing retirement guard %',f; END IF;
        EXECUTE overlay(body placing 'BEGIN IF game_arena_reset_item_proven('||item_ref||') THEN RETURN NULL; END IF;' FROM begin_at FOR 5);
    END LOOP;
    FOREACH f IN ARRAY ARRAY['game_item_container_entry_removal_proven','game_item_nested_entry_proven'] LOOP
        SELECT pg_get_functiondef((f||'()')::regprocedure) INTO STRICT body;
        begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
        IF begin_at=0 THEN RAISE EXCEPTION 'missing arena custody guard %',f; END IF;
        gate:=$gate$BEGIN
        IF TG_OP='DELETE' AND EXISTS (SELECT 1 FROM game_arena_pit_item_edges e
            WHERE e.item_instance_id=OLD.item_instance_id AND e.parent_item_instance_id=OLD.parent_item_instance_id
              AND e.placement_ordinal=OLD.placement_ordinal
              AND e.entry_kind=CASE TG_TABLE_NAME WHEN 'game_item_container_entries' THEN 1 ELSE 2 END)
            AND game_arena_reset_item_proven(OLD.item_instance_id) THEN RETURN NULL; END IF;
        $gate$;
        EXECUTE overlay(body placing gate FROM begin_at FOR 5);
    END LOOP;
    -- The existing bounded batch writes the same reservation, edge removal,
    -- terminal quantity, receipt and audit rows for each supported cause.
    FOR definition IN
        SELECT pg_get_functiondef(oid) AS sql FROM pg_proc
        WHERE pronamespace=current_schema()::regnamespace
          AND proname IN ('game_player_death_retire_entry','game_player_death_retire_entries')
    LOOP
        anchor:='IN(''WORLD_RESET'',''ACTIVITY_RETIREMENT'')';
        IF (length(definition.sql)-length(replace(definition.sql,anchor,'')))/length(anchor)<>1 THEN
            RAISE EXCEPTION 'unexpected bounded retirement writer';
        END IF;
        EXECUTE replace(definition.sql,anchor,'IN(''WORLD_RESET'',''ACTIVITY_RETIREMENT'',''ARENA_RESET'')');
    END LOOP;
    FOREACH f IN ARRAY ARRAY['game_arena_pit_item_reset_guard()','game_arena_reset_ground_top()',
        'game_arena_reset_item_proven(uuid)','game_arena_reset_capture_edge()'] LOOP
        EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',f,current_schema());
        EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC',f);
    END LOOP;
END $migration$;

-- The existing deferred Ground removal trigger runs with the writer's rights.
GRANT EXECUTE ON FUNCTION game_arena_reset_item_proven(UUID) TO oteryn_game_runtime;
