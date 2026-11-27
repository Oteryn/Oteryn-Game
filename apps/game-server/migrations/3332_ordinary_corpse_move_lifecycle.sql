-- MINT and carried drops count all live Ground corpse roots, including
-- permanent remains, while holding the same scope cap lock.
CREATE FUNCTION game_corpse_ground_cap_recount(p_world UUID,p_channel UUID,p_instance UUID)
RETURNS BIGINT LANGUAGE sql VOLATILE AS $$
    SELECT count(*) FROM game_item_ground_locations g
    JOIN game_item_mint_receipts m USING(item_instance_id)
    JOIN game_item_instances i USING(item_instance_id)
    WHERE g.world_id=p_world AND g.channel_id IS NOT DISTINCT FROM p_channel
      AND g.instance_id IS NOT DISTINCT FROM p_instance
      AND m.loot_purpose_key='CORPSE_MATERIALIZATION' AND m.draw_ordinal=0
      AND i.lifecycle=1 AND m.transaction_id=i.minted_transaction_id
$$;

-- Registered corpse stages whose active native profiles admit inventory and
-- have both movable and pickupable source facts, plus the preceding plain set.
CREATE OR REPLACE FUNCTION game_ordinary_corpse_portable_key(p_key TEXT) RETURNS BOOLEAN
LANGUAGE sql IMMUTABLE STRICT AS $$
    SELECT p_key = ANY(ARRAY(
        SELECT 'oteryn:item.tibia.i'||id FROM unnest(ARRAY[
        -- Begin generated portable stages
        866,4013,4034,4035,4036,4041,4042,4047,4048,4086,4087,4105,4106,4107,4128,4147,4160,4161,4166,4167,4176,4177,4178,4200,4201,4202,4230,4231,4232,5013,5629,5727,5728,5729,6348,6349,6350,21100,21101,21102,
        -- End generated portable stages
        287,288,289,862,863,864,2983,3987,3990,3991,3992,3994,3995,3996,
        3998,3999,4000,4001,4002,4003,4004,4007,4008,4009,4011,4012,4016,4017,
        4018,4021,4022,4024,4029,4043,4044,4045,4046,4052,4053,4057,4058,4066,
        4067,4068,4069,4078,4079,4080,4081,4082,4083,4084,4085,4088,4094,4095,
        4096,4101,4102,4103,4104,4109,4110,4111,4116,4117,4118,4119,4120,4121,
        4122,4123,4125,4126,4127,4130,4131,4132,4137,4138,4139,4141,4142,4143,
        4145,4146,4148,4149,4156,4157,4158,4159,4164,4165,4168,4169,
        4179,4180,4181,4188,4189,4190,4191,4192,4193,4194,4195,4196,4209,4210,
        4211,4224,4225,4226,4227,4228,4229,4242,4246,4250,4281,4301,4302,4303,
        4317,4318,4319,4320,4330,4331,4332,4363,4364,4365,4379,4380,4381,
        5541,5564,5565,5566,5567,5625,5626,5934,5935,5936,6344,
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
          AND g.world_id=m.death_world_id
          AND (((g.scope_kind,g.channel_id,g.instance_id,g.runtime_scope_ownership_generation)
                 IS NOT DISTINCT FROM (m.death_scope_kind,m.death_channel_id,m.death_instance_id,m.death_scope_ownership_generation))
                OR game_ordinary_corpse_loop_ground(p_item))
    )
$$;

DO $ordinary_corpse_drag$
DECLARE
    body TEXT := pg_get_functiondef('game_item_loop_line_admit()'::regprocedure);
    source CONSTANT TEXT := 'OR game_item_plain_corpse_source(NEW.item_instance_id)';
    timed CONSTANT TEXT := 'NEW.from_kind=3 AND NEW.to_kind=3 AND game_item_loop_native_corpse_source(NEW.item_instance_id)';
    f TEXT;
    original TEXT;
    house CONSTANT TEXT := 'IF game_house_container_line_admit(NEW) THEN';
BEGIN
    IF (length(body)-length(replace(body,source,'')))/length(source) <> 1
        OR (length(body)-length(replace(body,timed,'')))/length(timed) <> 1 THEN
        RAISE EXCEPTION 'Ordinary corpse move predecessor changed';
    END IF;
    body := replace(body, source, source||' OR (NEW.to_kind=3 AND NEW.quantity_before=1 AND NEW.quantity_after=1 AND game_ordinary_corpse_drag_source(NEW.item_instance_id))');
    body := replace(body, timed, 'NEW.from_kind=3 AND NEW.to_kind=3 AND
        (game_item_loop_native_corpse_source(NEW.item_instance_id) OR game_ordinary_corpse_drag_source(NEW.item_instance_id))');
    original := 'IF NEW.from_kind=3 AND NEW.to_kind<>3 AND EXISTS';
    IF (length(body)-length(replace(body,original,'')))/length(original) <> 1
        OR (length(body)-length(replace(body,house,'')))/length(house) <> 1 THEN
        RAISE EXCEPTION 'Ordinary corpse custody predecessor changed';
    END IF;
    body := replace(body, original, $guard$
    -- Every ordinary retirement freezes the corpse's Ground, including a
    -- child step. Serialize that read with relocation on the root Item row.
    IF NEW.from_kind=3 THEN
        SELECT loc.* INTO g FROM game_item_ground_locations loc
        JOIN game_item_instances corpse USING(item_instance_id,world_id)
        JOIN game_item_mint_receipts m ON m.item_instance_id=corpse.item_instance_id
            AND m.transaction_id=corpse.minted_transaction_id
        WHERE loc.item_instance_id=NEW.item_instance_id AND corpse.lifecycle=1
          AND m.loot_purpose_key='CORPSE_MATERIALIZATION' AND m.draw_ordinal=0
        FOR UPDATE OF corpse;
        IF FOUND AND EXISTS (
            SELECT 1 FROM game_item_decay_retire_reservations v
            WHERE v.item_instance_id=NEW.item_instance_id AND v.corpse_item_instance_id=NEW.item_instance_id
              AND v.cause_kind='CORPSE_DECAY'
              AND (v.world_id,v.scope_kind,v.channel_id,v.instance_id)
                  IS NOT DISTINCT FROM (g.world_id,g.scope_kind,g.channel_id,g.instance_id)
            UNION ALL
            SELECT 1 FROM game_item_corpse_container_entries e
            JOIN game_item_decay_retire_reservations v ON v.item_instance_id=e.item_instance_id
                AND v.corpse_item_instance_id=e.parent_item_instance_id
                AND v.placement_ordinal=e.placement_ordinal
            WHERE e.parent_item_instance_id=NEW.item_instance_id AND v.cause_kind='CORPSE_DECAY'
              AND (v.world_id,v.scope_kind,v.channel_id,v.instance_id)
                  IS NOT DISTINCT FROM (g.world_id,g.scope_kind,g.channel_id,g.instance_id)
        ) THEN
            RAISE EXCEPTION 'Ground corpse retirement is reserved'
                USING ERRCODE='23514', CONSTRAINT='game_corpse_ground_retirement_reserved';
        END IF;
    END IF;
    IF NEW.from_kind=3 AND NEW.to_kind<>3 AND EXISTS (
        SELECT 1 FROM game_item_corpse_container_entries e
        WHERE e.parent_item_instance_id=NEW.item_instance_id
    ) THEN
        RAISE EXCEPTION 'Empty the corpse before picking it up' USING ERRCODE='23514';
    END IF;
    IF NEW.to_kind=2 AND game_corpse_retirement_deadline(NEW.to_parent_item_instance_id) IS NOT NULL THEN
        RAISE EXCEPTION 'A decaying corpse cannot receive inventory items' USING ERRCODE='23514';
    END IF;
    $guard$ || original);
    body := replace(body, house, $cap$
    IF NEW.from_kind IN (1,2) AND NEW.to_kind=3 AND EXISTS (
        SELECT 1 FROM game_item_mint_receipts m
        WHERE m.item_instance_id=NEW.item_instance_id AND m.loot_purpose_key='CORPSE_MATERIALIZATION'
          AND m.draw_ordinal=0
    ) THEN
        SELECT * INTO r FROM game_item_loop_move_receipts
        WHERE transaction_id=NEW.transaction_id AND created_xact_id=pg_current_xact_id();
        IF NOT FOUND THEN
            RAISE EXCEPTION 'Corpse drop requires its receipt of this transaction' USING ERRCODE='23514';
        END IF;
        PERFORM pg_advisory_xact_lock(hashtextextended('oteryn:corpse-cap:' ||
            encode(CASE WHEN r.scope_kind=1 THEN decode('01','hex')||uuid_send(r.world_id)||uuid_send(r.channel_id)
                        ELSE game_activity_scope_key(r.world_id,r.instance_id) END,'hex'),0));
        IF game_corpse_ground_cap_recount(r.world_id,r.channel_id,r.instance_id)>=64 THEN
            RAISE EXCEPTION 'The destination scope has too many live Ground corpses'
                USING ERRCODE='23514', CONSTRAINT='game_corpse_ground_cap';
        END IF;
    END IF;
    $cap$ || house);
    original := 'cg.runtime_scope_ownership_generation=r.scope_generation';
    IF (length(body)-length(replace(body,original,'')))/length(original) <> 1 THEN
        RAISE EXCEPTION 'Ordinary corpse Quick Loot predecessor changed';
    END IF;
    body := replace(body,original,'(('||original||') OR
        (cg.runtime_scope_ownership_generation<r.scope_generation
         AND (game_ordinary_corpse_loop_ground(cg.item_instance_id)
              OR game_ordinary_corpse_drag_source(cg.item_instance_id))))');
    EXECUTE body;

    -- Try the exact relocated receipt independently of the original death scope.
    -- Each recovery predicate retains its current assignment and stack checks.
    f := 'game_corpse_stage_ground_current(uuid,uuid,uuid,numeric,bigint)';
    body := pg_get_functiondef(f::regprocedure);
    IF to_regprocedure('game_dragged_rat_ground_current(uuid,uuid,uuid,numeric,bigint)') IS NULL THEN
        original := 'm.death_channel_id=g.channel_id
   AND m.death_scope_ownership_generation=g.runtime_scope_ownership_generation';
    ELSE
        original := 'm.death_channel_id=g.channel_id
   AND (m.death_scope_ownership_generation=g.runtime_scope_ownership_generation OR game_dragged_rat_ground_current(p_item,p_world,p_channel,p_generation,p_stack))';
    END IF;
    IF (length(body)-length(replace(body,original,'')))/length(original) <> 1 THEN
        RAISE EXCEPTION 'Corpse timer scope predecessor changed in %', f;
    END IF;
    EXECUTE replace(body,original,'(('||original||') OR
        (g.runtime_scope_ownership_generation<=p_generation AND game_ordinary_corpse_loop_ground(p_item)))');

    f := 'game_instance_corpse_stage_current(uuid,uuid,uuid,numeric,bigint)';
    body := pg_get_functiondef(f::regprocedure);
    original := 'm.death_scope_kind=2 AND m.death_world_id=p_world AND m.death_instance_id=p_instance
   AND m.death_scope_ownership_generation=g.runtime_scope_ownership_generation';
    IF (length(body)-length(replace(body,original,'')))/length(original) <> 1
        OR position('a.scope_key=m.death_scope_key' IN body)=0 THEN
        RAISE EXCEPTION 'Corpse timer scope predecessor changed in %', f;
    END IF;
    body := replace(body,'a.scope_key=m.death_scope_key','a.scope_key=game_activity_scope_key(g.world_id,g.instance_id)');
    EXECUTE replace(body,original,'(m.death_world_id=g.world_id AND (('||original||') OR
        (g.runtime_scope_ownership_generation<=p_generation AND game_ordinary_corpse_loop_ground(p_item))))');

    FOREACH f IN ARRAY ARRAY['game_ordinary_corpse_loop_ground(uuid)','game_ordinary_corpse_drag_source(uuid)',
                             'game_corpse_ground_cap_recount(uuid,uuid,uuid)'] LOOP
        EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',f,current_schema());
        EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC',f);
        EXECUTE format('GRANT EXECUTE ON FUNCTION %s TO oteryn_game_runtime,oteryn_game_control',f);
    END LOOP;
END $ordinary_corpse_drag$;

-- Only chains with a portable stage need an additional carried retirement
-- timer. Ground still retires through the corpse owner, including its loot.
CREATE FUNCTION game_ordinary_corpse_carried_terminal(chain INTEGER[], durations BIGINT[]) RETURNS INTEGER
LANGUAGE sql IMMUTABLE STRICT AS $$
    -- The rabbit skinning owner already seals the three transform rows;
    -- its terminal deadline remains with the original corpse owner.
    SELECT CASE WHEN chain[1]<>6017 AND durations[cardinality(chain)]>0 AND EXISTS (
        SELECT 1 FROM unnest(chain) id
        WHERE game_ordinary_corpse_portable_key('oteryn:item.tibia.i'||id)
    ) THEN cardinality(chain) ELSE cardinality(chain)-1 END
$$;
REVOKE ALL ON FUNCTION game_ordinary_corpse_carried_terminal(INTEGER[],BIGINT[]) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_ordinary_corpse_carried_terminal(INTEGER[],BIGINT[]) TO oteryn_game_runtime;

-- The rabbit's existing ground proof owns exactly three transform schedules.
-- Publish its separate carried retirement only with a real whole-item pickup.
CREATE FUNCTION game_rabbit_carried_corpse_schedule(p_item UUID)
RETURNS TABLE(mint_transaction UUID, deadline BIGINT, content_digest BYTEA)
LANGUAGE sql STABLE STRICT AS $$
    SELECT m.transaction_id,p.expires_at_unix_ms+60000,p.content_digest
    FROM game_item_loop_move_lines l
    JOIN game_item_loop_move_receipts r USING(transaction_id)
    JOIN game_item_mint_receipts m ON m.item_instance_id=l.item_instance_id
    JOIN game_spell_field_temporal_schedules p
      ON p.item_instance_id=m.item_instance_id AND p.source_transaction_id=m.transaction_id
    WHERE l.item_instance_id=p_item AND r.operation=5 AND r.world_id=m.death_world_id
      AND l.created_xact_id=pg_current_xact_id() AND r.created_xact_id=l.created_xact_id
      AND l.from_kind=3 AND l.to_kind IN (1,2)
      AND l.quantity_before=1 AND l.quantity_after=1
      AND l.definition_family='Item' AND l.definition_revision_ref='definition-r1'
      AND l.definition_production_key IN ('oteryn:item.tibia.i4301','oteryn:item.tibia.i4302','oteryn:item.tibia.i4303')
      AND m.loot_purpose_key='CORPSE_MATERIALIZATION' AND m.draw_ordinal=0
      AND (m.loot_table_family,m.loot_table_production_key,m.loot_table_revision_ref)=('Item','oteryn:item.tibia.i6017','definition-r1')
      AND p.source_ordinal=0 AND p.source_stage=3
      AND (p.target_family,p.target_production_key,p.target_revision)=('Item','oteryn:item.tibia.i4303','definition-r1')
      AND p.expires_at_unix_ms=m.materialized_at+610000
$$;

CREATE FUNCTION game_record_rabbit_carried_corpse_schedule() RETURNS TRIGGER
LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
    INSERT INTO game_spell_field_temporal_schedules(
        item_instance_id,source_stage,source_transaction_id,source_ordinal,duration_millis,expires_at_unix_ms,
        definition_family,definition_production_key,definition_revision,content_digest,
        blocks_movement,blocks_projectile,immovable_block_solid
    )
    SELECT NEW.item_instance_id,4,s.mint_transaction,0,60000,s.deadline,
           'Item','oteryn:item.tibia.i4303','definition-r1',s.content_digest,false,false,false
    FROM game_rabbit_carried_corpse_schedule(NEW.item_instance_id) s
    ON CONFLICT (item_instance_id,source_stage) DO NOTHING;
    RETURN NULL;
END
$$;

CREATE TRIGGER game_record_rabbit_carried_corpse_schedule AFTER INSERT ON game_item_loop_move_lines
FOR EACH ROW WHEN (
    NEW.from_kind=3 AND NEW.to_kind IN (1,2)
    AND NEW.definition_production_key IN ('oteryn:item.tibia.i4301','oteryn:item.tibia.i4302','oteryn:item.tibia.i4303')
) EXECUTE FUNCTION game_record_rabbit_carried_corpse_schedule();
REVOKE ALL ON FUNCTION game_record_rabbit_carried_corpse_schedule(),game_rabbit_carried_corpse_schedule(UUID) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_rabbit_carried_corpse_schedule(UUID) TO oteryn_game_runtime,oteryn_game_control;

DO $carried_corpse_timer$
DECLARE
    body TEXT;
    f TEXT;
    old TEXT;
    stage_bound CONSTANT TEXT := 'array_length(chain,1)-1';
    at INTEGER;
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
    body := replace(body,old,$targets$OR NEW.target_family IS DISTINCT FROM (CASE WHEN NEW.source_stage<cardinality(chain) THEN 'Item' END)
          OR NEW.target_revision IS DISTINCT FROM (CASE WHEN NEW.source_stage<cardinality(chain) THEN 'definition-r1' END)$targets$);
    at := regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
    IF at=0 THEN
        RAISE EXCEPTION 'Rabbit carried retirement proof predecessor changed';
    END IF;
    EXECUTE overlay(body placing 'BEGIN'||$rabbit$
      IF NEW.source_stage=4 AND NEW.source_ordinal=0 AND NEW.created_xact_id=pg_current_xact_id()
         AND (NEW.definition_family,NEW.definition_production_key,NEW.definition_revision)=('Item','oteryn:item.tibia.i4303','definition-r1')
         AND NEW.duration_millis=60000 AND NEW.target_production_key IS NULL
         AND NOT (NEW.blocks_movement OR NEW.blocks_projectile OR NEW.immovable_block_solid)
         AND EXISTS (SELECT 1 FROM game_rabbit_carried_corpse_schedule(NEW.item_instance_id) s
             WHERE (s.mint_transaction,s.deadline,s.content_digest)=
                   (NEW.source_transaction_id,NEW.expires_at_unix_ms,NEW.content_digest))
      THEN RETURN NULL; END IF;
    $rabbit$ FROM at FOR 5);

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
    EXECUTE format('ALTER FUNCTION game_rabbit_carried_corpse_schedule(UUID) SET search_path=pg_catalog,%I,pg_temp',current_schema());
    EXECUTE format('ALTER FUNCTION game_record_rabbit_carried_corpse_schedule() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $carried_corpse_timer$;

-- Timed corpse actions must still use the donor's exact old-to-spent rule.
-- Keep these typed IDs in sync with items::fishing::corpse_skinning_rule.
CREATE FUNCTION game_corpse_skinning_rule(p_before TEXT,p_spent TEXT,p_tool TEXT DEFAULT NULL)
RETURNS BOOLEAN LANGUAGE sql IMMUTABLE AS $$
    SELECT EXISTS (
        SELECT 1 FROM (VALUES
            (5908,52677,ARRAY[52678,52679,52680]),
            (5908,4012,ARRAY[4272,5969]),
            (5908,4053,ARRAY[4052,5982]),
            (5908,4048,ARRAY[4047,5981]),
            (5908,4058,ARRAY[4057,5983]),
            (5908,21093,ARRAY[21091,21092]),
            (5908,21097,ARRAY[21095,21096]),
            (5908,21101,ARRAY[21099,21100]),
            (5908,21002,ARRAY[21000,21001]),
            (5908,21006,ARRAY[21004,21005]),
            (5908,20998,ARRAY[20996,20997]),
            (5908,4325,ARRAY[4324,6040]),
            (5908,4328,ARRAY[4327,6041]),
            (5908,4322,ARRAY[4239,4321]),
            (5908,10369,ARRAY[10368,10371]),
            (5908,10361,ARRAY[10360,10363]),
            (5908,10353,ARRAY[10352,10355]),
            (5908,10365,ARRAY[10364,10367]),
            (5908,4287,ARRAY[4286,5973]),
            (5908,4063,ARRAY[4062,5984]),
            (5908,4113,ARRAY[4112,5999]),
            (5908,4213,ARRAY[4212,6030]),
            (5908,22744,ARRAY[22742,22743]),
            (5942,4098,ARRAY[4097,5995]),
            (5942,4138,ARRAY[4137,6006]),
            (5942,8742,ARRAY[8738,8744]),
            (5942,18959,ARRAY[18958,18961]),
            (3469,4302,ARRAY[4301]),
            (5908,4302,ARRAY[4301]),
            (9594,4302,ARRAY[4301]),
            (9598,4302,ARRAY[4301])
        ) rule(tool,spent,original)
        WHERE p_spent='oteryn:item.tibia.i'||rule.spent
          AND (p_tool IS NULL OR p_tool='oteryn:item.tibia.i'||rule.tool)
          AND p_before=ANY(ARRAY(SELECT 'oteryn:item.tibia.i'||id FROM unnest(rule.original) id))
    )
$$;

-- The existing temporal handoff has foreign keys to an actual decay receipt.
-- A fresh mint or a skinning action supplies its own original receipt instead.
CREATE TABLE game_corpse_skinning_handoffs (
    item_instance_id UUID NOT NULL REFERENCES game_item_instances(item_instance_id),
    result_revision NUMERIC(20,0) NOT NULL CHECK(result_revision BETWEEN 2 AND 18446744073709551615),
    custody_transaction_id UUID NOT NULL CHECK(game_character_is_uuid_v7(custody_transaction_id)),
    mint_transaction_id UUID REFERENCES game_item_mint_receipts(transaction_id),
    skinning_transaction_id UUID REFERENCES game_corpse_skinning_records(transaction_id),
    PRIMARY KEY(item_instance_id,result_revision),
    CHECK (num_nonnulls(mint_transaction_id,skinning_transaction_id)=1)
);
CREATE INDEX game_corpse_skinning_handoffs_mint ON game_corpse_skinning_handoffs(mint_transaction_id);
CREATE INDEX game_corpse_skinning_handoffs_skinning ON game_corpse_skinning_handoffs(skinning_transaction_id);
REVOKE ALL ON game_corpse_skinning_handoffs FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
GRANT SELECT ON game_corpse_skinning_handoffs TO oteryn_game_runtime,oteryn_game_control;
CREATE TRIGGER game_corpse_skinning_handoffs_immutable BEFORE UPDATE OR DELETE ON game_corpse_skinning_handoffs
FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_corpse_skinning_handoffs_no_truncate BEFORE TRUNCATE ON game_corpse_skinning_handoffs
EXECUTE FUNCTION game_item_reject_truncate();

CREATE FUNCTION game_corpse_skinning_handoff_capture() RETURNS TRIGGER
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE
    mint_origin UUID;
    skinning_origin UUID;
BEGIN
    -- Independent Item mutation and custody guards must prove the update.
    IF NEW.last_transaction_id IS NULL OR NEW.last_transaction_id IS NOT DISTINCT FROM OLD.last_transaction_id
       OR NEW.state_revision<>OLD.state_revision+1 OR NEW.quantity<>1 OR NEW.lifecycle<>1
       OR (to_jsonb(NEW)-ARRAY['last_transaction_id','state_revision']) IS DISTINCT FROM
          (to_jsonb(OLD)-ARRAY['last_transaction_id','state_revision']) THEN RETURN NEW; END IF;

    SELECT h.mint_transaction_id,h.skinning_transaction_id INTO mint_origin,skinning_origin
    FROM game_corpse_skinning_handoffs h
    WHERE h.item_instance_id=OLD.item_instance_id AND h.result_revision=OLD.state_revision
      AND h.custody_transaction_id=OLD.last_transaction_id;
    IF NOT FOUND THEN
        IF OLD.state_revision=1 AND OLD.last_transaction_id IS NULL THEN
            SELECT m.transaction_id INTO mint_origin
            FROM game_item_mint_receipts m
            JOIN game_spell_field_temporal_schedules p ON p.item_instance_id=m.item_instance_id
                AND p.source_transaction_id=m.transaction_id AND p.source_ordinal=0 AND p.source_stage=1
            WHERE m.item_instance_id=OLD.item_instance_id AND m.transaction_id=OLD.minted_transaction_id
              AND m.loot_purpose_key='CORPSE_MATERIALIZATION' AND m.draw_ordinal=0 AND m.materialized_at IS NOT NULL
              AND (p.definition_family,p.definition_production_key,p.definition_revision)=
                  (OLD.definition_family,OLD.definition_production_key,OLD.definition_revision_ref)
              AND (m.loot_table_family,m.loot_table_production_key,m.loot_table_revision_ref)=
                  (p.definition_family,p.definition_production_key,p.definition_revision)
              AND EXISTS (
                  SELECT 1 FROM game_spell_field_temporal_schedules spent
                  WHERE spent.item_instance_id=p.item_instance_id AND spent.source_transaction_id=m.transaction_id
                    AND spent.source_ordinal=0 AND game_corpse_skinning_rule(p.definition_production_key,spent.target_production_key)
              );
        ELSE
            SELECT s.transaction_id INTO skinning_origin
            FROM game_corpse_skinning_records s
            WHERE s.corpse_item_instance_id=OLD.item_instance_id AND s.transaction_id=OLD.last_transaction_id
              AND (s.before_item->>'state_revision')::numeric+1=OLD.state_revision
              AND (OLD.definition_family,OLD.definition_production_key,OLD.definition_revision_ref)=
                  ('Item',s.spent_key,s.spent_revision)
              AND (to_jsonb(OLD)-ARRAY['definition_production_key','definition_revision_ref','state_revision','last_transaction_id'])=
                  (s.before_item-ARRAY['definition_production_key','definition_revision_ref','state_revision','last_transaction_id'])
              AND EXISTS (
                  SELECT 1 FROM game_spell_field_temporal_schedules p
                  WHERE p.item_instance_id=OLD.item_instance_id AND p.source_transaction_id=OLD.minted_transaction_id
                    AND p.source_ordinal=0 AND (p.target_family,p.target_production_key,p.target_revision)=
                        ('Item',s.spent_key,s.spent_revision)
              );
        END IF;
    END IF;
    IF mint_origin IS NOT NULL OR skinning_origin IS NOT NULL THEN
        INSERT INTO game_corpse_skinning_handoffs(
            item_instance_id,result_revision,custody_transaction_id,mint_transaction_id,skinning_transaction_id
        ) VALUES(NEW.item_instance_id,NEW.state_revision,NEW.last_transaction_id,mint_origin,skinning_origin);
    END IF;
    RETURN NEW;
END
$$;
CREATE TRIGGER game_corpse_skinning_handoff_capture AFTER UPDATE ON game_item_instances
FOR EACH ROW EXECUTE FUNCTION game_corpse_skinning_handoff_capture();

CREATE OR REPLACE FUNCTION game_corpse_skinning_temporal_allowed(p_item UUID,p_spent TEXT,p_revision TEXT)
RETURNS BOOLEAN LANGUAGE sql AS $$
    SELECT (p_spent<>'oteryn:item.tibia.i4302' AND NOT EXISTS (
        SELECT 1 FROM game_spell_field_temporal_schedules t WHERE t.item_instance_id=p_item
    )) OR EXISTS (
        SELECT 1 FROM game_item_instances i
        JOIN game_item_mint_receipts m ON m.item_instance_id=i.item_instance_id AND m.transaction_id=i.minted_transaction_id
        JOIN game_corpse_source_races race ON race.item_instance_id=i.item_instance_id AND race.mint_transaction_id=m.transaction_id
        JOIN game_spell_field_temporal_schedules p
          ON p.item_instance_id=i.item_instance_id AND p.source_transaction_id=m.transaction_id AND p.source_ordinal=0
        JOIN game_spell_field_temporal_schedules spent
          ON spent.item_instance_id=i.item_instance_id AND spent.source_transaction_id=m.transaction_id AND spent.source_ordinal=0
        WHERE i.item_instance_id=p_item AND i.lifecycle=1 AND i.quantity=1
          AND m.loot_purpose_key='CORPSE_MATERIALIZATION' AND m.draw_ordinal=0
          AND (m.loot_table_family,m.loot_table_revision_ref)=('Item','definition-r1')
          AND m.materialized_at IS NOT NULL AND m.death_world_id=i.world_id
          AND (p.definition_family,p.definition_production_key,p.definition_revision)=
              (i.definition_family,i.definition_production_key,i.definition_revision_ref)
          AND (spent.target_family,spent.target_production_key,spent.target_revision)=('Item',p_spent,p_revision)
          AND spent.source_stage>=p.source_stage
          AND game_corpse_skinning_rule(i.definition_production_key,p_spent)
          AND p.expires_at_unix_ms>floor(extract(epoch FROM clock_timestamp())*1000)::bigint
          AND (m.loot_table_production_key<>'oteryn:item.tibia.i5995'
               OR (race.race_key,race.race_definition_revision)=('oteryn:creature.demon','definition-r1'))
          AND (p_spent<>'oteryn:item.tibia.i4302' OR (
               m.loot_table_production_key='oteryn:item.tibia.i6017'
               AND race.race_key IN ('oteryn:creature.rabbit','oteryn:creature.killer_rabbit')
               AND race.race_definition_revision='definition-r1'))
          AND (
               (p.source_stage=1 AND (m.loot_table_family,m.loot_table_production_key,m.loot_table_revision_ref)=
                    (p.definition_family,p.definition_production_key,p.definition_revision)
                AND ((i.state_revision=1 AND i.last_transaction_id IS NULL)
                     OR EXISTS (
                         SELECT 1 FROM game_corpse_skinning_handoffs h
                         WHERE h.item_instance_id=i.item_instance_id AND h.result_revision=i.state_revision
                           AND h.custody_transaction_id=i.last_transaction_id AND h.mint_transaction_id=m.transaction_id
                     )))
               OR EXISTS (
                   SELECT 1 FROM game_spell_field_temporal_receipts r
                   WHERE r.item_instance_id=i.item_instance_id AND r.source_stage=p.source_stage-1
                     AND ((r.transaction_id=i.last_transaction_id AND r.state_revision_before+1=i.state_revision)
                          OR EXISTS (
                              SELECT 1 FROM game_item_decay_handoffs h
                              WHERE h.item_instance_id=r.item_instance_id AND h.source_stage=r.source_stage
                                AND h.stage_transaction_id=r.transaction_id AND h.result_revision=i.state_revision
                                AND h.custody_transaction_id=i.last_transaction_id
                          ))
               )
           )
    )
$$;

-- The spent item can stand where its original schedule would arrive after
-- the skipped transform. Bind that substitute to the exact craft receipt.
CREATE FUNCTION game_ordinary_corpse_skinning_reached(
    p_item UUID,p_stage INTEGER,p_key TEXT,p_revision TEXT,p_state NUMERIC,p_transaction UUID
) RETURNS BOOLEAN LANGUAGE sql STABLE STRICT AS $$
    SELECT EXISTS (
        SELECT 1 FROM game_corpse_skinning_records s
        JOIN game_item_craft_records c USING(transaction_id)
        JOIN game_spell_field_temporal_schedules stage
          ON stage.item_instance_id=s.corpse_item_instance_id
         AND stage.source_transaction_id=(s.before_item->>'minted_transaction_id')::uuid
        WHERE s.corpse_item_instance_id=p_item AND (s.spent_key,s.spent_revision)=(p_key,p_revision)
          AND c.created_xact_id=s.created_xact_id AND c.recipe='oteryn:skinning.crystal' AND NOT c.consume_source
          AND game_corpse_skinning_rule(s.before_item->>'definition_production_key',s.spent_key,c.before_item->>'definition_production_key')
          AND stage.source_ordinal=0 AND stage.source_stage=p_stage
          AND (stage.target_family,stage.target_production_key,stage.target_revision)=('Item',p_key,p_revision)
          AND ((s.transaction_id=p_transaction AND (s.before_item->>'state_revision')::numeric+1=p_state)
               OR EXISTS (
                   SELECT 1 FROM game_corpse_skinning_handoffs h
                   WHERE h.item_instance_id=p_item AND h.result_revision=p_state
                     AND h.custody_transaction_id=p_transaction AND h.skinning_transaction_id=s.transaction_id
               ))
    )
$$;
REVOKE ALL ON FUNCTION game_corpse_skinning_rule(TEXT,TEXT,TEXT),game_corpse_skinning_handoff_capture(),
    game_ordinary_corpse_skinning_reached(UUID,INTEGER,TEXT,TEXT,NUMERIC,UUID) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_corpse_skinning_rule(TEXT,TEXT,TEXT),
    game_ordinary_corpse_skinning_reached(UUID,INTEGER,TEXT,TEXT,NUMERIC,UUID)
    TO oteryn_game_runtime,oteryn_game_control;

DO $ordinary_corpse_skinning$
DECLARE
    body TEXT;
    anchor TEXT;
    f TEXT;
    final_stage CONSTANT TEXT := '(SELECT max(final.source_stage) FROM game_spell_field_temporal_schedules final
        WHERE final.item_instance_id=i.item_instance_id AND final.source_ordinal=0 AND final.target_production_key IS NOT NULL)';
BEGIN
    FOREACH f IN ARRAY ARRAY['game_corpse_skinning_rule(text,text,text)','game_corpse_skinning_handoff_capture()',
        'game_ordinary_corpse_skinning_reached(uuid,integer,text,text,numeric,uuid)',
        'game_corpse_skinning_temporal_allowed(uuid,text,text)'] LOOP
        EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',f,current_schema());
    END LOOP;

    -- Preserve the special rabbit yield guard. Every other timed attempt
    -- uses the exact tool for its old-to-spent transition.
    body := pg_get_functiondef('game_corpse_skinning_complete()'::regprocedure);
    anchor := $tool$OR (c.before_item->>'definition_production_key'='oteryn:item.tibia.i5942'
      AND NEW.before_item->>'definition_production_key'<>'oteryn:item.tibia.i4301')$tool$;
    IF (length(body)-length(replace(body,anchor,'')))/length(anchor) <> 1 THEN
        RAISE EXCEPTION 'Existing corpse skinning completion proof changed';
    END IF;
    EXECUTE replace(body,anchor,$tool$OR (NEW.before_item->>'definition_production_key'<>'oteryn:item.tibia.i4301'
      AND game_corpse_skinning_rule(NEW.before_item->>'definition_production_key',NEW.spent_key,c.before_item->>'definition_production_key'))$tool$);

    body := pg_get_functiondef('game_spell_field_temporal_capture()'::regprocedure);
    anchor := ' THEN RAISE EXCEPTION ''field stage requires exact preceding receipt''';
    IF (length(body)-length(replace(body,anchor,'')))/length(anchor) <> 1 THEN
        RAISE EXCEPTION 'Existing corpse stage predecessor proof changed';
    END IF;
    EXECUTE replace(body,anchor,$capture$ AND NOT coalesce(game_ordinary_corpse_skinning_reached(
        OLD.item_instance_id,r.source_stage-1,OLD.definition_production_key,OLD.definition_revision_ref,
        OLD.state_revision,OLD.last_transaction_id),false)$capture$||anchor);

    body := pg_get_functiondef('game_corpse_terminal_stage_ready(uuid)'::regprocedure);
    anchor := '(p.target_family,p.target_production_key,p.target_revision))';
    IF (length(body)-length(replace(body,anchor,'')))/length(anchor) <> 1 THEN
        RAISE EXCEPTION 'Existing corpse terminal-read proof changed';
    END IF;
    EXECUTE replace(body,anchor,anchor||$ready$
      OR EXISTS (SELECT 1 FROM game_item_instances i WHERE i.item_instance_id=p_item AND i.lifecycle=1 AND i.quantity=1
        AND game_ordinary_corpse_skinning_reached(i.item_instance_id,$ready$||final_stage||$ready$,
            i.definition_production_key,i.definition_revision_ref,i.state_revision,i.last_transaction_id))
    $ready$);

    body := pg_get_functiondef('game_item_decay_retire_consistency_guard()'::regprocedure);
    anchor := ' THEN' || chr(10) || '   RAISE EXCEPTION ''corpse retirement requires committed final transform''';
    IF (length(body)-length(replace(body,anchor,'')))/length(anchor) <> 1 THEN
        RAISE EXCEPTION 'Existing corpse terminal-write proof changed';
    END IF;
    EXECUTE replace(body,anchor,$retire$
      AND NOT EXISTS (
        SELECT 1 FROM game_item_instances i
        JOIN game_corpse_skinning_records s ON s.corpse_item_instance_id=i.item_instance_id
        LEFT JOIN game_corpse_skinning_handoffs h ON h.item_instance_id=i.item_instance_id
          AND h.result_revision=i.state_revision-1 AND h.skinning_transaction_id=s.transaction_id
        WHERE i.item_instance_id=NEW.corpse_item_instance_id
          AND (
              (NEW.item_instance_id<>i.item_instance_id AND i.lifecycle=1 AND i.quantity=1
               AND game_ordinary_corpse_skinning_reached(i.item_instance_id,$retire$||final_stage||$retire$,
                   i.definition_production_key,i.definition_revision_ref,i.state_revision,i.last_transaction_id))
              OR (NEW.item_instance_id=i.item_instance_id AND i.lifecycle=2 AND i.quantity=0
                  AND i.last_transaction_id=NEW.transaction_id
                  AND game_ordinary_corpse_skinning_reached(i.item_instance_id,$retire$||final_stage||$retire$,
                      i.definition_production_key,i.definition_revision_ref,i.state_revision-1,
                      coalesce(h.custody_transaction_id,s.transaction_id)))
          )
      )
    $retire$||anchor);
END $ordinary_corpse_skinning$;
