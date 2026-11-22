-- The live corpse limit counts bodies with pending work. Permanent remains
-- still participate in Ground and corpse projections.
CREATE FUNCTION game_corpse_counts_toward_mint_cap(p_item UUID) RETURNS BOOLEAN
LANGUAGE sql STABLE STRICT AS $$
    SELECT game_corpse_retirement_deadline(p_item) IS NOT NULL
        OR NOT game_corpse_terminal_stage_ready(p_item)
$$;
REVOKE ALL ON FUNCTION game_corpse_counts_toward_mint_cap(UUID) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_corpse_counts_toward_mint_cap(UUID) TO oteryn_game_runtime;

-- Registered corpse stages whose active native profiles admit inventory and
-- have both movable and pickupable source facts, plus the preceding plain set.
CREATE OR REPLACE FUNCTION game_ordinary_corpse_portable_key(p_key TEXT) RETURNS BOOLEAN
LANGUAGE sql IMMUTABLE STRICT AS $$
    SELECT p_key = ANY(ARRAY(
        SELECT 'oteryn:item.tibia.i'||id FROM unnest(ARRAY[
        287,288,289,862,863,864,2983,3987,3990,3991,3992,3994,3995,3996,
        3998,3999,4000,4001,4002,4003,4004,4007,4008,4009,4011,4012,4016,4017,
        4018,4021,4022,4024,4029,4043,4044,4045,4046,4052,4053,4057,4058,4066,
        4067,4068,4069,4078,4079,4080,4081,4082,4083,4084,4085,4088,4094,4095,
        4096,4101,4102,4103,4104,4109,4110,4111,4116,4117,4118,4119,4120,4121,
        4122,4123,4125,4126,4127,4130,4131,4132,4137,4138,4139,4141,4142,4143,
        4145,4146,4148,4149,4156,4157,4158,4159,4162,4163,4164,4165,4168,4169,
        4179,4180,4181,4188,4189,4190,4191,4192,4193,4194,4195,4196,4209,4210,
        4211,4224,4225,4226,4227,4228,4229,4242,4246,4250,4281,4301,4302,4303,
        4317,4318,4319,4320,4330,4331,4332,4363,4364,4365,4378,4379,4380,4381,
        4991,4992,4993,5541,5564,5565,5566,5567,5625,5626,5934,5935,5936,6344,
        6345,6346,6354,8910,10330,10331,10332,10334,10335,10440,10441,10442,21092,21093,
        21094,21096,21097,21098,21476,22698,23729,31203,31204,31205,31206,33793,33891,50311,
        52118,52119,52120
        ]) AS id))
$$;

-- A corpse relocated by the Item loop retains the exact drop receipt in its
-- Ground row even after a later transform changes the Item's last transaction.
CREATE FUNCTION game_ordinary_corpse_loop_ground(p_item UUID) RETURNS BOOLEAN
LANGUAGE sql STABLE STRICT AS $$
    SELECT EXISTS (
        SELECT 1 FROM game_item_ground_locations g
        JOIN game_item_loop_move_lines l ON l.item_instance_id=g.item_instance_id
        JOIN game_item_loop_move_receipts r USING(transaction_id)
        WHERE g.item_instance_id=p_item
          AND game_item_loop_drop_ref(g.corpse_ref)
          AND r.transaction_id=CASE
              WHEN octet_length(g.corpse_ref)=octet_length('OTERYN_ITEM_LOOP_DROP/v1:')+16
              THEN right(encode(g.corpse_ref,'hex'),32)::uuid END
          AND (r.world_id,r.scope_kind,r.channel_id,r.instance_id)
              IS NOT DISTINCT FROM (g.world_id,g.scope_kind,g.channel_id,g.instance_id)
          AND r.scope_generation=g.runtime_scope_ownership_generation
          AND l.from_kind IN (1,2,3) AND l.to_kind=3
          AND l.quantity_before=1 AND l.quantity_after=1
          AND l.to_spatial_position=g.spatial_position
    )
$$;

-- Ground drag accepts an ordinary corpse's original MINT or exact relocated
-- Ground custody. Rust still qualifies movability from the pinned Item profile.
CREATE FUNCTION game_ordinary_corpse_drag_source(p_item UUID) RETURNS BOOLEAN
LANGUAGE sql STABLE STRICT AS $$
    SELECT EXISTS (
        SELECT 1 FROM game_item_instances i
        JOIN game_item_mint_receipts m ON m.item_instance_id=i.item_instance_id
            AND m.transaction_id=i.minted_transaction_id
        JOIN game_item_ground_locations g ON g.item_instance_id=i.item_instance_id
        WHERE i.item_instance_id=p_item AND i.lifecycle=1 AND i.quantity=1
          AND i.definition_family='Item' AND i.definition_revision_ref='definition-r1'
          AND m.loot_purpose_key='CORPSE_MATERIALIZATION' AND m.draw_ordinal=0
          AND m.materialized_at IS NOT NULL AND m.loot_table_family='Item'
          AND i.definition_production_key IN (
            SELECT 'oteryn:item.tibia.i'||id FROM unnest(game_summer_corpse_chain(m.loot_table_production_key)) id
          )
          AND (g.world_id,g.scope_kind,g.channel_id,g.instance_id)
              IS NOT DISTINCT FROM (m.death_world_id,m.death_scope_kind,m.death_channel_id,m.death_instance_id)
          AND (g.runtime_scope_ownership_generation=m.death_scope_ownership_generation
               OR game_ordinary_corpse_loop_ground(p_item))
    )
$$;

DO $ordinary_corpse_drag$
DECLARE
    body TEXT := pg_get_functiondef('game_item_loop_line_admit()'::regprocedure);
    source CONSTANT TEXT := 'OR game_item_plain_corpse_source(NEW.item_instance_id)';
    timed CONSTANT TEXT := 'NEW.from_kind=3 AND NEW.to_kind=3 AND game_item_loop_native_corpse_source(NEW.item_instance_id)';
    f TEXT;
    original CONSTANT TEXT := 'm.death_scope_ownership_generation=g.runtime_scope_ownership_generation';
BEGIN
    IF (length(body)-length(replace(body,source,'')))/length(source) <> 1
        OR (length(body)-length(replace(body,timed,'')))/length(timed) <> 1 THEN
        RAISE EXCEPTION 'Ordinary corpse move predecessor changed';
    END IF;
    body := replace(body, source, source||' OR (NEW.to_kind=3 AND NEW.quantity_before=1 AND NEW.quantity_after=1 AND game_ordinary_corpse_drag_source(NEW.item_instance_id))');
    EXECUTE replace(body, timed, 'NEW.from_kind=3 AND NEW.to_kind=3 AND
        (game_item_loop_native_corpse_source(NEW.item_instance_id) OR game_ordinary_corpse_drag_source(NEW.item_instance_id))');

    FOREACH f IN ARRAY ARRAY['game_corpse_stage_ground_current(uuid,uuid,uuid,numeric,bigint)',
                             'game_instance_corpse_stage_current(uuid,uuid,uuid,numeric,bigint)'] LOOP
        body := pg_get_functiondef(f::regprocedure);
        IF (length(body)-length(replace(body,original,'')))/length(original) <> 1 THEN
            RAISE EXCEPTION 'Corpse timer scope predecessor changed in %', f;
        END IF;
        EXECUTE replace(body,original,'('||original||' OR
            (g.runtime_scope_ownership_generation<=p_generation AND game_ordinary_corpse_loop_ground(p_item)))');
    END LOOP;

    FOREACH f IN ARRAY ARRAY['game_ordinary_corpse_loop_ground(uuid)','game_ordinary_corpse_drag_source(uuid)',
                             'game_corpse_counts_toward_mint_cap(uuid)'] LOOP
        EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',f,current_schema());
        EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC',f);
        EXECUTE format('GRANT EXECUTE ON FUNCTION %s TO oteryn_game_runtime,oteryn_game_control',f);
    END LOOP;
END $ordinary_corpse_drag$;

-- Only chains with a portable stage need an additional carried retirement
-- timer. Ground still retires through the corpse owner, including its loot.
CREATE FUNCTION game_ordinary_corpse_carried_terminal(chain INTEGER[], durations BIGINT[]) RETURNS INTEGER
LANGUAGE sql IMMUTABLE STRICT AS $$
    SELECT CASE WHEN durations[cardinality(chain)]>0 AND EXISTS (
        SELECT 1 FROM unnest(chain) id
        WHERE game_ordinary_corpse_portable_key('oteryn:item.tibia.i'||id)
    ) THEN cardinality(chain) ELSE cardinality(chain)-1 END
$$;
REVOKE ALL ON FUNCTION game_ordinary_corpse_carried_terminal(INTEGER[],BIGINT[]) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_ordinary_corpse_carried_terminal(INTEGER[],BIGINT[]) TO oteryn_game_runtime;

DO $carried_corpse_timer$
DECLARE
    body TEXT;
    f TEXT;
    old TEXT;
    stage_bound CONSTANT TEXT := 'array_length(chain,1)-1';
    final_stage CONSTANT TEXT := 'AND final.source_ordinal=0';
BEGIN
    f := 'game_summer_corpse_schedule()';
    body := pg_get_functiondef(f::regprocedure);
    old := $targets$'Item','oteryn:item.tibia.i'||chain[stage+1]::text,'definition-r1'$targets$;
    IF (length(body)-length(replace(body,stage_bound,'')))/length(stage_bound) <> 1
        OR (length(body)-length(replace(body,old,'')))/length(old) <> 1 THEN
        RAISE EXCEPTION 'Corpse schedule predecessor changed';
    END IF;
    body := replace(body,stage_bound,'game_ordinary_corpse_carried_terminal(chain,durations)');
    EXECUTE replace(body,old,$targets$CASE WHEN stage<cardinality(chain) THEN 'Item' END,
          'oteryn:item.tibia.i'||chain[stage+1]::text,
          CASE WHEN stage<cardinality(chain) THEN 'definition-r1' END$targets$);

    f := 'game_spell_field_temporal_schedule_proven()';
    body := pg_get_functiondef(f::regprocedure);
    old := $targets$OR NEW.target_family IS DISTINCT FROM 'Item' OR NEW.target_revision IS DISTINCT FROM 'definition-r1'$targets$;
    IF (length(body)-length(replace(body,stage_bound,'')))/length(stage_bound) <> 1
        OR (length(body)-length(replace(body,old,'')))/length(old) <> 1 THEN
        RAISE EXCEPTION 'Corpse schedule policy predecessor changed';
    END IF;
    body := replace(body,stage_bound,'game_ordinary_corpse_carried_terminal(chain,durations)');
    EXECUTE replace(body,old,$targets$OR NEW.target_family IS DISTINCT FROM (CASE WHEN NEW.source_stage<cardinality(chain) THEN 'Item' END)
          OR NEW.target_revision IS DISTINCT FROM (CASE WHEN NEW.source_stage<cardinality(chain) THEN 'definition-r1' END)$targets$);

    FOREACH f IN ARRAY ARRAY['game_corpse_terminal_stage_ready(uuid)','game_item_decay_retire_consistency_guard()'] LOOP
        body := pg_get_functiondef(f::regprocedure);
        IF (length(body)-length(replace(body,final_stage,'')))/length(final_stage) <> 1 THEN
            RAISE EXCEPTION 'Corpse terminal stage predecessor changed in %', f;
        END IF;
        body := replace(body,final_stage,final_stage||' AND final.target_production_key IS NOT NULL');
        old := 'p.source_ordinal=0 AND p.source_stage=1';
        IF (length(body)-length(replace(body,old,'')))/length(old) <> 1 THEN
            RAISE EXCEPTION 'Corpse transform presence predecessor changed in %', f;
        END IF;
        body := replace(body,old,old||' AND p.target_production_key IS NOT NULL');
        EXECUTE body;
    END LOOP;
    body := pg_get_functiondef('game_spell_field_temporal_receipt_proven()'::regprocedure);
    old := 'IF NEW.custody_kind<>1 AND NOT EXISTS(';
    IF (length(body)-length(replace(body,old,'')))/length(old) <> 1 THEN
        RAISE EXCEPTION 'Corpse temporal custody predecessor changed';
    END IF;
    EXECUTE replace(body,old,$custody$
      IF NEW.custody_kind=1 AND p.source_ordinal=0 AND p.target_production_key IS NULL
        AND EXISTS(SELECT 1 FROM game_item_mint_receipts m
                   WHERE m.transaction_id=p.source_transaction_id AND m.item_instance_id=p.item_instance_id
                     AND m.loot_purpose_key='CORPSE_MATERIALIZATION') THEN
        RAISE EXCEPTION 'Ground corpse retirement requires the corpse owner' USING ERRCODE='23514';
      END IF;
      IF NEW.custody_kind<>1 AND NOT EXISTS($custody$);
    EXECUTE format('ALTER FUNCTION game_ordinary_corpse_carried_terminal(INTEGER[], BIGINT[]) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $carried_corpse_timer$;
