-- Crystal 201189f8 data-global/lib/quests/svargrond_arena.lua:207-246.
-- Arena retirement uses the ordinary Item retirement and audit ledgers, with
-- the exact live pit lease and original per-cell Ground observation.
CREATE TABLE game_svargrond_ground_retirements (
    item_instance_id UUID PRIMARY KEY REFERENCES game_item_instances(item_instance_id),
    world_id UUID NOT NULL,
    channel_id UUID NOT NULL,
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    game_session_id UUID NOT NULL,
    run_id UUID NOT NULL,
    pit SMALLINT NOT NULL CHECK (pit BETWEEN 1 AND 10),
    rank SMALLINT NOT NULL CHECK (rank BETWEEN 1 AND 3),
    scope_generation NUMERIC(20,0) NOT NULL CHECK (scope_generation > 0),
    expires_at_ms BIGINT NOT NULL CHECK (expires_at_ms > 0),
    spatial_position BYTEA NOT NULL CHECK (octet_length(spatial_position) = 10),
    source_stack_ordinal BIGINT NOT NULL CHECK (source_stack_ordinal > 0),
    created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id()
);

-- Both durable coordinate forms for the same pit floor share one observation.
CREATE UNIQUE INDEX game_svargrond_ground_retirement_cell
    ON game_svargrond_ground_retirements
    (world_id, channel_id, run_id, pit, substring(spatial_position FROM 1 FOR 8));

CREATE FUNCTION game_svargrond_ground_retirement_before() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE
    x0 INTEGER := 32205 + (ARRAY[0,-14,-28,-42,-35,-21,-7,-14,-28,-21])[NEW.pit];
    y0 INTEGER := 31094 + (ARRAY[0,0,0,0,-14,-14,-14,-28,-28,-42])[NEW.pit];
BEGIN
    IF NEW.created_xact_id <> pg_current_xact_id()
       OR octet_length(NEW.spatial_position) <> 10
       OR x0 IS NULL OR y0 IS NULL
       OR substring(NEW.spatial_position FROM 1 FOR 2) <> '\x0000'::bytea
       OR substring(NEW.spatial_position FROM 5 FOR 2) <> '\x0000'::bytea
       OR get_byte(NEW.spatial_position,2)*256+get_byte(NEW.spatial_position,3) NOT BETWEEN x0 AND x0+9
       OR get_byte(NEW.spatial_position,6)*256+get_byte(NEW.spatial_position,7) NOT BETWEEN y0 AND y0+9
       OR substring(NEW.spatial_position FROM 9 FOR 2) NOT IN ('\x0007'::bytea,'\xfff9'::bytea)
       OR NOT EXISTS (
           SELECT 1
           FROM game_svargrond_pit_leases l
           JOIN game_character_arena_matches a USING (character_id)
           JOIN game_item_ground_locations g ON g.item_instance_id = NEW.item_instance_id
           JOIN game_item_instances i USING (item_instance_id)
           WHERE (l.world_id,l.channel_id,l.character_id,l.game_session_id,l.run_id,l.pit,
                  l.scope_generation,l.expires_at_ms) =
                 (NEW.world_id,NEW.channel_id,NEW.character_id,NEW.game_session_id,NEW.run_id,
                  NEW.pit,NEW.scope_generation,NEW.expires_at_ms)
             AND l.expires_at_ms > floor(extract(epoch FROM clock_timestamp()) * 1000)::bigint
             AND (a.body->>'phase')::int = 1 AND (a.body->>'pit')::int = NEW.pit
             AND (a.body->>'rank')::int = NEW.rank
             AND (g.world_id,g.channel_id,g.scope_kind,g.runtime_scope_ownership_generation,
                  g.spatial_position,g.stack_ordinal) =
                 (NEW.world_id,NEW.channel_id,1,NEW.scope_generation,NEW.spatial_position,
                  NEW.source_stack_ordinal)
             AND i.lifecycle = 1
             AND NOT EXISTS (SELECT 1 FROM game_character_death_receipts d
                 WHERE d.character_id=NEW.character_id
                   AND d.committed_character_revision>(a.body->>'entry_revision')::numeric)
       ) THEN
        RAISE EXCEPTION 'Arena retirement requires the exact live pit and Ground observation'
            USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END $$;

CREATE TRIGGER game_svargrond_ground_retirement_before
    BEFORE INSERT ON game_svargrond_ground_retirements
    FOR EACH ROW EXECUTE FUNCTION game_svargrond_ground_retirement_before();
CREATE TRIGGER game_svargrond_ground_retirement_immutable
    BEFORE UPDATE OR DELETE ON game_svargrond_ground_retirements
    FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_svargrond_ground_retirement_no_truncate
    BEFORE TRUNCATE ON game_svargrond_ground_retirements
    FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();

DO $migration$
DECLARE relation TEXT; original TEXT;
BEGIN
    FOREACH relation IN ARRAY ARRAY['game_item_decay_retire_reservations','game_item_decay_retire_receipts'] LOOP
        SELECT pg_get_expr(conbin,conrelid) INTO STRICT original FROM pg_constraint
          WHERE conrelid=to_regclass(relation) AND conname='game_reset_retire_shape';
        EXECUTE format('ALTER TABLE %I DROP CONSTRAINT game_reset_retire_shape',relation);
        EXECUTE format('ALTER TABLE %I ADD CONSTRAINT game_reset_retire_shape CHECK ((%s) OR
            (cause_kind=''ARENA_RESET'' AND occurred_at>0 AND scope_kind=1
             AND corpse_item_instance_id IS NULL AND deadline IS NULL
             AND reset_epoch IS NULL AND retirement_binding IS NULL AND target_digest IS NULL
             AND activity_handoff_id IS NULL AND activity_ownership_generation IS NULL
             AND ground_root_item_instance_id IS NOT NULL
             AND (ground_root_item_instance_id=item_instance_id)=(placement_ordinal IS NULL)))',
             relation,original);
    END LOOP;
END $migration$;

CREATE FUNCTION game_svargrond_ground_retirement_proven(p_item UUID) RETURNS BOOLEAN
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE r game_item_decay_retire_receipts%ROWTYPE;
BEGIN
    SELECT * INTO r FROM game_item_decay_retire_receipts WHERE item_instance_id=p_item
      AND cause_kind='ARENA_RESET' AND created_xact_id=pg_current_xact_id();
    IF NOT FOUND THEN RETURN FALSE; END IF;
    IF NOT EXISTS (
           SELECT 1 FROM game_svargrond_ground_retirements e
           JOIN game_item_decay_retire_reservations v ON v.item_instance_id=r.item_instance_id
           JOIN game_runtime_scope_assignments a ON a.world_id=e.world_id AND a.channel_id=e.channel_id
           WHERE e.item_instance_id=r.ground_root_item_instance_id
             AND e.created_xact_id=pg_current_xact_id()
             AND r.occurred_at < e.expires_at_ms
             AND (e.world_id,e.channel_id,e.scope_generation) =
                 (r.world_id,r.channel_id,r.fence_scope_ownership_generation)
             AND (v.world_id,v.channel_id,v.fence_scope_ownership_generation,v.transaction_id,
                  v.event_id,v.quantity_before,v.occurred_at,v.cause_kind) =
                 (r.world_id,r.channel_id,r.fence_scope_ownership_generation,r.transaction_id,
                  r.event_id,r.quantity_before,r.occurred_at,r.cause_kind)
             AND sha256(v.envelope)=r.envelope_sha256
             AND v.ground_root_item_instance_id=r.ground_root_item_instance_id
             AND v.placement_ordinal IS NOT DISTINCT FROM r.placement_ordinal
             AND a.state=1 AND a.ownership_generation=e.scope_generation
             AND (a.holder_node_id,a.holder_registration_revision)=
                 (v.fence_holder_node_id,v.fence_holder_registration_revision)
       )
       OR NOT EXISTS (SELECT 1 FROM game_item_instances i WHERE i.item_instance_id=p_item
           AND i.world_id=r.world_id AND i.lifecycle=2 AND i.quantity=0
           AND i.last_transaction_id=r.transaction_id)
       OR NOT EXISTS (SELECT 1 FROM game_item_transfer_quantity_evidence q
           WHERE q.item_instance_id=p_item AND q.transaction_id=r.transaction_id
             AND q.quantity_before=r.quantity_before)
       OR NOT EXISTS (SELECT 1 FROM game_item_ground_removal_evidence e
           WHERE e.item_instance_id=r.ground_root_item_instance_id
             AND e.world_id=r.world_id AND e.channel_id=r.channel_id
             AND e.scope_kind=1 AND e.created_xact_id=pg_current_xact_id())
       OR (r.placement_ordinal IS NOT NULL AND (
           NOT EXISTS (SELECT 1 FROM game_item_corpse_entry_removal_evidence e
               WHERE e.item_instance_id=p_item AND e.parent_item_instance_id=r.ground_root_item_instance_id
                 AND e.placement_ordinal=r.placement_ordinal AND e.world_id=r.world_id
                 AND e.channel_id=r.channel_id AND e.scope_kind=1 AND e.created_xact_id=pg_current_xact_id())
           OR NOT EXISTS (SELECT 1 FROM game_item_decay_retire_receipts root
               WHERE root.item_instance_id=r.ground_root_item_instance_id
                 AND root.cause_kind='ARENA_RESET' AND root.placement_ordinal IS NULL
                 AND root.created_xact_id=pg_current_xact_id())))
       OR EXISTS (SELECT 1 FROM game_item_ground_locations WHERE item_instance_id=p_item)
       OR EXISTS (SELECT 1 FROM game_item_corpse_container_entries
                  WHERE item_instance_id=p_item OR parent_item_instance_id=p_item)
       OR EXISTS (SELECT 1 FROM game_item_container_entries
                  WHERE item_instance_id=p_item OR parent_item_instance_id=p_item)
       OR EXISTS (SELECT 1 FROM game_item_nested_entries
                  WHERE item_instance_id=p_item OR parent_item_instance_id=p_item)
       OR EXISTS (SELECT 1 FROM game_item_container_slots WHERE item_instance_id=p_item)
       OR EXISTS (SELECT 1 FROM game_item_house_interior_locations WHERE item_instance_id=p_item)
       OR NOT EXISTS (SELECT 1 FROM game_item_audit_outbox o
           WHERE o.item_instance_id=p_item AND o.event_id=r.event_id
             AND o.transaction_id=r.transaction_id AND o.occurred_at=r.occurred_at
             AND o.envelope_sha256=r.envelope_sha256
             AND o.created_xact_id=pg_current_xact_id()
             AND o.publication_state=1 AND o.published_at IS NULL) THEN
        RAISE EXCEPTION 'Arena retirement requires exact Ground removal, conservation and audit'
            USING ERRCODE = '23514';
    END IF;
    RETURN TRUE;
END $$;

CREATE FUNCTION game_svargrond_ground_retirement_consumed() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
    IF NOT game_svargrond_ground_retirement_proven(NEW.item_instance_id) THEN
        RAISE EXCEPTION 'Arena Ground observation must be consumed in its deciding transaction'
            USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END $$;

CREATE CONSTRAINT TRIGGER game_svargrond_ground_retirement_consumed
    AFTER INSERT ON game_svargrond_ground_retirements DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW EXECUTE FUNCTION game_svargrond_ground_retirement_consumed();

-- Preserve the existing proof arms for ordinary decay, transfers and resets.
DO $migration$
DECLARE f TEXT; body TEXT; item_ref TEXT;
BEGIN
    FOREACH f IN ARRAY ARRAY['game_item_corpse_entry_removal_proven','game_item_decay_retire_consistency_guard',
                             'game_item_ground_removal_proven','game_item_instance_change_proven'] LOOP
        SELECT pg_get_functiondef(to_regprocedure(f||'()')) INTO STRICT body;
        item_ref:=CASE WHEN f IN ('game_item_corpse_entry_removal_proven','game_item_ground_removal_proven')
            THEN 'OLD.item_instance_id' ELSE 'NEW.item_instance_id' END;
        body:=regexp_replace(body,'\mBEGIN\M',
            'BEGIN IF game_svargrond_ground_retirement_proven('||item_ref||') THEN RETURN NULL; END IF;');
        EXECUTE body;
        EXECUTE format('ALTER FUNCTION %I() SECURITY DEFINER SET search_path=%I,pg_temp',f,current_schema());
    END LOOP;
    EXECUTE format('ALTER FUNCTION game_svargrond_ground_retirement_before() SET search_path=%I,pg_temp',current_schema());
    EXECUTE format('ALTER FUNCTION game_svargrond_ground_retirement_proven(uuid) SET search_path=%I,pg_temp',current_schema());
    EXECUTE format('ALTER FUNCTION game_svargrond_ground_retirement_consumed() SET search_path=%I,pg_temp',current_schema());
END $migration$;

REVOKE ALL ON FUNCTION game_svargrond_ground_retirement_before() FROM PUBLIC;
REVOKE ALL ON FUNCTION game_svargrond_ground_retirement_proven(uuid) FROM PUBLIC;
REVOKE ALL ON FUNCTION game_svargrond_ground_retirement_consumed() FROM PUBLIC;
REVOKE ALL ON game_svargrond_ground_retirements FROM PUBLIC, oteryn_game_control;
GRANT SELECT, INSERT ON game_svargrond_ground_retirements TO oteryn_game_runtime;
