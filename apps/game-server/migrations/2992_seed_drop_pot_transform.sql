-- Crystal 201189f data/scripts/movements/seeds.lua: AddItem on an empty physical
-- flowerpot changes that Item in place. Its exact Ground row is the map CAS.
ALTER TABLE game_map_seed_drop_records
    ALTER COLUMN placement DROP NOT NULL,
    ADD COLUMN pot_item_instance_id UUID REFERENCES game_item_instances,
    ADD COLUMN pot_before JSONB,
    ADD COLUMN pot_ground JSONB,
    ADD COLUMN pot_first_stage INTEGER,
    ADD COLUMN pot_content_digest BYTEA,
    ADD CONSTRAINT game_seed_drop_target_shape CHECK ((
        (placement IS NOT NULL AND num_nonnulls(pot_item_instance_id,pot_before,
             pot_ground,pot_first_stage,pot_content_digest)=0)
        OR (placement IS NULL AND pot_item_instance_id IS NOT NULL
            AND pot_item_instance_id<>item_instance_id AND object_revision=0
            AND jsonb_typeof(pot_before)='object' AND jsonb_typeof(pot_ground)='object'
            AND pot_first_stage BETWEEN 1 AND 2147483645 AND octet_length(pot_content_digest)=32)
    ) IS TRUE);
CREATE INDEX game_seed_drop_pot_item ON game_map_seed_drop_records(pot_item_instance_id)
    WHERE pot_item_instance_id IS NOT NULL;

ALTER TABLE game_spell_field_temporal_schedules
    ADD COLUMN seed_drop_source_transaction_id UUID REFERENCES game_map_seed_drop_records,
    DROP CONSTRAINT game_field_temporal_one_source,
    DROP CONSTRAINT game_spell_field_temporal_schedules_source_stage_check,
    ADD CONSTRAINT game_field_temporal_one_source CHECK (num_nonnulls(source_transaction_id,
        event_source_transaction_id,map_source_transaction_id,seed_drop_source_transaction_id)=1),
    ADD CONSTRAINT game_field_temporal_schedule_stage CHECK (source_stage>=1
        AND (source_stage<=32 OR seed_drop_source_transaction_id IS NOT NULL));
CREATE INDEX game_seed_drop_pot_timer_source
    ON game_spell_field_temporal_schedules(seed_drop_source_transaction_id)
    WHERE seed_drop_source_transaction_id IS NOT NULL;
-- The schedule FK still controls admitted stage numbers. Two completed stages
-- stay in history when the same recovered empty pot starts another cycle.
ALTER TABLE game_spell_field_temporal_receipts
    DROP CONSTRAINT game_spell_field_temporal_receipts_source_stage_check,
    ADD CONSTRAINT game_field_temporal_receipt_stage CHECK(source_stage>=1);

CREATE FUNCTION game_seed_drop_pot_complete(r game_map_seed_drop_records)
RETURNS BOOLEAN LANGUAGE sql STABLE AS $$
    SELECT EXISTS(
        SELECT 1 FROM game_item_instances i JOIN game_item_ground_locations g USING(item_instance_id,world_id)
        WHERE i.item_instance_id=r.pot_item_instance_id AND i.last_transaction_id=r.transaction_id
          AND i.state_revision=(r.pot_before->>'state_revision')::numeric+1
          AND i.definition_production_key='oteryn:item.tibia.i'||r.after_item_id
          AND (to_jsonb(i)-ARRAY['definition_production_key','last_transaction_id','state_revision'])
              =(r.pot_before-ARRAY['definition_production_key','last_transaction_id','state_revision'])
          AND to_jsonb(g)=r.pot_ground
          AND (SELECT count(*) FROM game_spell_field_temporal_schedules p
               WHERE p.seed_drop_source_transaction_id=r.transaction_id
                 AND p.item_instance_id=i.item_instance_id AND p.created_xact_id=r.created_xact_id
                 AND p.source_stage IN(r.pot_first_stage,r.pot_first_stage+1))=2
    )
$$;

DO $migration$
DECLARE body TEXT; anchor TEXT;
BEGIN
    body:=pg_get_functiondef('game_map_seed_drop_admit()'::regprocedure);
    anchor:='AND g.spatial_position=int4send';
    IF (length(body)-length(replace(body,anchor,'')))/length(anchor)<>1 THEN
        RAISE EXCEPTION 'unexpected seed move cell proof';
    END IF;
    body:=replace(body,anchor,'AND (NEW.pot_item_instance_id IS NOT NULL OR g.spatial_position=int4send');
    anchor:='||int2send((-get_byte(NEW.placement,38))::smallint)';
    IF (length(body)-length(replace(body,anchor,'')))/length(anchor)<>1 THEN
        RAISE EXCEPTION 'unexpected seed move cell proof ending';
    END IF;
    body:=replace(body,anchor,anchor||')');
    anchor:='RETURN NEW;';
    IF (length(body)-length(replace(body,anchor,'')))/length(anchor)<>1 THEN
        RAISE EXCEPTION 'unexpected seed receipt admission ending';
    END IF;
    EXECUTE replace(body,anchor,$arm$
    IF NEW.pot_item_instance_id IS NOT NULL AND NOT EXISTS(
        SELECT 1 FROM game_item_instances i
        JOIN game_item_ground_locations g USING(item_instance_id,world_id)
        JOIN game_item_loop_move_receipts m ON m.transaction_id=NEW.move_transaction_id
        JOIN game_item_ground_locations seed ON seed.item_instance_id=NEW.item_instance_id
        WHERE i.item_instance_id=NEW.pot_item_instance_id AND i.world_id=NEW.world_id
          AND i.lifecycle=1 AND i.quantity=1 AND i.definition_family='Item'
          AND i.definition_production_key='oteryn:item.tibia.i306' AND i.definition_revision_ref='definition-r1'
          AND to_jsonb(i)=NEW.pot_before AND to_jsonb(g)=NEW.pot_ground
          AND g.scope_kind=1 AND g.channel_id=NEW.channel_id
          AND g.runtime_scope_ownership_generation=m.scope_generation
          AND g.stack_ordinal>0 AND g.spatial_position=seed.spatial_position
          AND g.map_revision=NEW.map_revision AND g.content_revision=seed.content_revision
          AND g.native_room_placement_context=seed.native_room_placement_context
          AND m.content_digest=NEW.pot_content_digest
          AND (game_item_loop_drop_ref(g.corpse_ref)
               OR game_item_loop_map_mint_source(i.item_instance_id)
               OR game_item_action_ground_source(i.item_instance_id))
          AND NEW.pot_first_stage=coalesce((SELECT max(p.source_stage) FROM game_spell_field_temporal_schedules p
                                            WHERE p.item_instance_id=i.item_instance_id),0)+1
          AND NOT EXISTS(SELECT 1 FROM game_spell_field_temporal_schedules p
                         WHERE p.item_instance_id=i.item_instance_id AND NOT EXISTS(
                             SELECT 1 FROM game_spell_field_temporal_receipts d
                             WHERE (d.item_instance_id,d.source_stage)=(p.item_instance_id,p.source_stage)))
    ) THEN
        RAISE EXCEPTION 'seed drop requires the exact empty Ground pot' USING ERRCODE='23514';
    END IF;
    RETURN NEW;
$arm$);

    body:=pg_get_functiondef('game_map_seed_drop_complete(uuid)'::regprocedure);
    anchor:='(SELECT count(*) FROM game_item_instances WHERE last_transaction_id=r.transaction_id)<>1';
    IF (length(body)-length(replace(body,anchor,'')))/length(anchor)<>1 THEN
        RAISE EXCEPTION 'unexpected seed receipt mutation count';
    END IF;
    -- Parentheses keep the CASE's THEN inside the surrounding PL/pgSQL IF expression.
    body:=replace(body,anchor,'(SELECT count(*) FROM game_item_instances WHERE last_transaction_id=r.transaction_id)'
        ||'<>(CASE WHEN r.pot_item_instance_id IS NULL THEN 1 ELSE 2 END)');
    anchor:='OR NOT EXISTS(SELECT 1 FROM game_terrain_tool_receipts t';
    IF (length(body)-length(replace(body,anchor,'')))/length(anchor)<>1 THEN
        RAISE EXCEPTION 'unexpected seed receipt terrain proof';
    END IF;
    body:=replace(body,anchor,'OR (r.pot_item_instance_id IS NOT NULL AND NOT game_seed_drop_pot_complete(r))'
        ||E'\n        OR (r.pot_item_instance_id IS NULL AND NOT EXISTS(SELECT 1 FROM game_terrain_tool_receipts t');
    anchor:='AND s.decay_delays=ARRAY[72000000,72000000]::bigint[])';
    IF (length(body)-length(replace(body,anchor,'')))/length(anchor)<>1 THEN
        RAISE EXCEPTION 'unexpected seed receipt terrain proof ending';
    END IF;
    EXECUTE replace(body,anchor,anchor||')');
END $migration$;

DO $migration$
DECLARE body TEXT;
    entry CONSTANT TEXT:=E'\nBEGIN\n';
BEGIN
    body:=pg_get_functiondef('game_item_instance_guard()'::regprocedure);
    IF (length(body)-length(replace(body,entry,'')))/length(entry)<>1 THEN
        RAISE EXCEPTION 'unexpected Item mutation guard entry';
    END IF;
    EXECUTE replace(body,entry,entry||$arm$
    IF TG_OP='UPDATE' AND EXISTS(
        SELECT 1 FROM game_map_seed_drop_records r
        WHERE r.transaction_id=NEW.last_transaction_id AND r.pot_item_instance_id=OLD.item_instance_id
          AND r.pot_before=to_jsonb(OLD) AND r.created_xact_id=pg_current_xact_id()
          AND NEW.definition_production_key='oteryn:item.tibia.i'||r.after_item_id
          AND (to_jsonb(NEW)-ARRAY['definition_production_key','last_transaction_id'])
              =(to_jsonb(OLD)-ARRAY['definition_production_key','last_transaction_id'])
    ) THEN
        NEW.state_revision:=OLD.state_revision+1;
        RETURN NEW;
    END IF;
$arm$);

    body:=pg_get_functiondef('game_item_instance_change_proven()'::regprocedure);
    IF (length(body)-length(replace(body,entry,'')))/length(entry)<>1 THEN
        RAISE EXCEPTION 'unexpected Item deferred mutation proof entry';
    END IF;
    EXECUTE replace(body,entry,entry||$arm$
    IF EXISTS(SELECT 1 FROM game_map_seed_drop_records r
              WHERE r.transaction_id=NEW.last_transaction_id AND r.pot_item_instance_id=NEW.item_instance_id
                AND r.pot_before=to_jsonb(OLD) AND r.created_xact_id=pg_current_xact_id()) THEN
        PERFORM game_map_seed_drop_complete(NEW.last_transaction_id);
        RETURN NULL;
    END IF;
$arm$);

    body:=pg_get_functiondef('game_spell_field_temporal_schedule_proven()'::regprocedure);
    IF (length(body)-length(replace(body,entry,'')))/length(entry)<>1 THEN
        RAISE EXCEPTION 'unexpected Item schedule proof entry';
    END IF;
    EXECUTE replace(body,entry,entry||$arm$
    IF NEW.seed_drop_source_transaction_id IS NOT NULL THEN
        IF NOT EXISTS(
            SELECT 1 FROM game_map_seed_drop_records r
            CROSS JOIN LATERAL (SELECT CASE r.after_item_id WHEN 324 THEN 330 ELSE 14032 END AS mature) plant
            WHERE r.transaction_id=NEW.seed_drop_source_transaction_id
              AND r.pot_item_instance_id=NEW.item_instance_id AND r.created_xact_id=pg_current_xact_id()
              AND NEW.created_xact_id=r.created_xact_id AND NEW.source_ordinal=1
              AND NEW.content_digest=r.pot_content_digest
              AND NEW.definition_revision='definition-r1' AND NEW.target_revision='definition-r1'
              AND NEW.target_family='Item'
              AND NOT (NEW.blocks_movement OR NEW.blocks_projectile OR NEW.immovable_block_solid)
              AND NEW.duration_millis=72000000
              AND ((NEW.source_stage,NEW.definition_production_key,NEW.target_production_key,NEW.expires_at_unix_ms)
                   = (r.pot_first_stage,'oteryn:item.tibia.i'||r.after_item_id,
                      'oteryn:item.tibia.i'||plant.mature,r.occurred_at+72000000)
                OR (NEW.source_stage,NEW.definition_production_key,NEW.target_production_key,NEW.expires_at_unix_ms)
                   = (r.pot_first_stage+1,'oteryn:item.tibia.i'||plant.mature,
                      'oteryn:item.tibia.i306',r.occurred_at+144000000))
        ) THEN
            RAISE EXCEPTION 'plant schedule lacks its exact seed drop stage' USING ERRCODE='23514';
        END IF;
        RETURN NULL;
    END IF;
$arm$);
END $migration$;

DO $migration$
DECLARE body TEXT; name TEXT;
    anchor CONSTANT TEXT:='p.event_source_transaction_id IS NOT NULL OR p.map_source_transaction_id IS NOT NULL';
    first_stage CONSTANT TEXT:='IF r.source_stage>1 AND NOT EXISTS';
    map_mint_tail CONSTANT TEXT:='AND g.native_room_placement_context=m.base_bundle_digest AND g.stack_ordinal>0)';
BEGIN
    -- Extend the existing move/decay custody policy to carry the same absolute dates.
    FOREACH name IN ARRAY ARRAY['game_item_loop_line_admit()','game_spell_field_temporal_capture()',
                                'game_event_item_timer_ground_current(uuid,uuid,uuid,numeric,bigint)'] LOOP
        body:=pg_get_functiondef(name::regprocedure);
        IF (length(body)-length(replace(body,anchor,'')))/length(anchor)<>1 THEN
            RAISE EXCEPTION 'unexpected hosted timer custody proof in %',name;
        END IF;
        body:=replace(body,anchor,anchor||' OR p.seed_drop_source_transaction_id IS NOT NULL');
        IF name='game_spell_field_temporal_capture()' THEN
            IF (length(body)-length(replace(body,first_stage,'')))/length(first_stage)<>1 THEN
                RAISE EXCEPTION 'unexpected Item schedule predecessor proof';
            END IF;
            body:=replace(body,first_stage,$arm$IF r.source_stage>1 AND NOT EXISTS(
                SELECT 1 FROM game_map_seed_drop_records s
                WHERE s.transaction_id=p.seed_drop_source_transaction_id AND s.pot_first_stage=r.source_stage
                  AND s.pot_item_instance_id=OLD.item_instance_id
                  AND OLD.state_revision>(s.pot_before->>'state_revision')::numeric
            ) AND NOT EXISTS$arm$);
        END IF;
        EXECUTE body;
    END LOOP;

    -- An emptied base placement is still taken. Only its original minted Item
    -- and exact unchanged Ground row can use this additional pickup arm.
    body:=pg_get_functiondef('game_item_loop_map_mint_source(uuid)'::regprocedure);
    IF (length(body)-length(replace(body,map_mint_tail,'')))/length(map_mint_tail)<>1 THEN
        RAISE EXCEPTION 'unexpected map-mint pickup proof ending';
    END IF;
    EXECUTE replace(body,map_mint_tail,map_mint_tail||$arm$
    OR EXISTS(
        SELECT 1 FROM game_map_item_mint_receipts m
        JOIN game_item_instances i USING(item_instance_id)
        JOIN game_item_ground_locations g ON g.item_instance_id=i.item_instance_id AND g.world_id=i.world_id
        JOIN game_map_seed_drop_records r ON r.pot_item_instance_id=i.item_instance_id
        WHERE i.item_instance_id=p_item AND i.lifecycle=1 AND i.quantity=1
          AND i.minted_transaction_id=m.transaction_id AND g.world_id=m.world_id AND g.channel_id=m.channel_id
          AND g.corpse_ref=convert_to('map-item-materialization-v1','UTF8')
          AND to_jsonb(g)=r.pot_ground
          AND ((i.last_transaction_id=r.transaction_id AND i.state_revision=(r.pot_before->>'state_revision')::numeric+1
                AND i.definition_production_key='oteryn:item.tibia.i'||r.after_item_id)
            OR EXISTS(SELECT 1 FROM game_spell_field_temporal_schedules p
                JOIN game_spell_field_temporal_receipts d USING(item_instance_id,source_stage)
                WHERE p.seed_drop_source_transaction_id=r.transaction_id AND p.item_instance_id=p_item
                  AND d.transaction_id=i.last_transaction_id AND i.state_revision=d.state_revision_before+1
                  AND (i.definition_family,i.definition_production_key,i.definition_revision_ref)
                     =(p.target_family,p.target_production_key,p.target_revision)))
    )
$arm$);

    EXECUTE format('ALTER FUNCTION game_seed_drop_pot_complete(game_map_seed_drop_records) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $migration$;
REVOKE ALL ON FUNCTION game_seed_drop_pot_complete(game_map_seed_drop_records) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_seed_drop_pot_complete(game_map_seed_drop_records) TO oteryn_game_runtime;
