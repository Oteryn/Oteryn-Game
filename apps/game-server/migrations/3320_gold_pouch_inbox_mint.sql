-- Permit the canonical empty Gold Pouch in the existing frozen Inbox mint spec.
-- Keep the reservation signature and its existing execute privileges.
CREATE OR REPLACE FUNCTION game_reward_claim_inbox_mint_reserve(
    p_session UUID, p_command NUMERIC, p_character UUID, p_world UUID, p_channel UUID,
    p_transaction UUID, p_source_binding BYTEA, p_plan_binding BYTEA,
    p_items UUID[], p_spec BYTEA
) RETURNS TABLE(event_id UUID, occurred_at BIGINT, work_units_used SMALLINT)
LANGUAGE plpgsql SECURITY DEFINER STRICT AS $$
DECLARE
    spec JSONB;
    output JSONB;
    text_value TEXT;
    field TEXT;
    item UUID;
    item_bytes BYTEA := ''::BYTEA;
    command_bytes BYTEA;
    source_digest BYTEA;
    expected_binding BYTEA;
    expected_maximum NUMERIC;
    existing game_reward_claim_inbox_mint_reservations%ROWTYPE;
    index INTEGER;
BEGIN
    PERFORM game_reward_claim_inbox_mint_lock(p_session, p_command);
    IF NOT game_character_is_uuid_v7(p_character) OR NOT game_character_is_uuid_v7(p_world)
       OR NOT game_character_is_uuid_v7(p_channel) OR NOT game_character_is_uuid_v7(p_transaction)
       OR octet_length(p_source_binding) <> 32 OR octet_length(p_plan_binding) <> 32
       OR cardinality(p_items) NOT BETWEEN 1 AND 20 OR array_ndims(p_items) IS DISTINCT FROM 1
       OR array_lower(p_items, 1) IS DISTINCT FROM 1
       OR octet_length(p_spec) NOT BETWEEN 1 AND 65536 THEN
        RAISE EXCEPTION 'invalid bounded Inbox mint reservation' USING ERRCODE = '23514';
    END IF;
    FOREACH item IN ARRAY p_items LOOP
        IF item IS NULL OR NOT game_character_is_uuid_v7(item) THEN
            RAISE EXCEPTION 'invalid frozen Inbox item identity' USING ERRCODE = '23514';
        END IF;
        item_bytes := item_bytes || uuid_send(item);
    END LOOP;
    IF (SELECT count(DISTINCT value) FROM unnest(p_items) AS u(value)) <> cardinality(p_items) THEN
        RAISE EXCEPTION 'duplicate frozen Inbox item identity' USING ERRCODE = '23514';
    END IF;
    spec := convert_from(p_spec, 'UTF8')::JSONB;
    IF jsonb_typeof(spec) IS DISTINCT FROM 'object'
       OR (spec - ARRAY['version','claim','content_revision','ruleset_revision','sim_revision','outputs']) <> '{}'::JSONB
       OR spec->'version' IS DISTINCT FROM '1'::JSONB
       OR jsonb_typeof(spec->'claim') IS DISTINCT FROM 'object'
       OR ((spec->'claim') - ARRAY['family','key','revision']) <> '{}'::JSONB
       OR jsonb_typeof(spec->'outputs') IS DISTINCT FROM 'array'
       OR jsonb_array_length(spec->'outputs') <> cardinality(p_items) THEN
        RAISE EXCEPTION 'invalid Inbox mint spec shape' USING ERRCODE = '23514';
    END IF;
    FOREACH field IN ARRAY ARRAY['family','key','revision'] LOOP
        text_value := spec->'claim'->>field;
        IF jsonb_typeof(spec->'claim'->field) IS DISTINCT FROM 'string'
           OR octet_length(text_value) NOT BETWEEN 1 AND (CASE WHEN field = 'family' THEN 128 ELSE 512 END)
           OR text_value ~ '[[:cntrl:]]' THEN
            RAISE EXCEPTION 'invalid Inbox reward claim definition' USING ERRCODE = '23514';
        END IF;
    END LOOP;
    FOREACH field IN ARRAY ARRAY['content_revision','ruleset_revision','sim_revision'] LOOP
        text_value := spec->>field;
        IF jsonb_typeof(spec->field) IS DISTINCT FROM 'string'
           OR octet_length(text_value) NOT BETWEEN 1 AND 512 OR text_value ~ '[[:cntrl:]]' THEN
            RAISE EXCEPTION 'invalid Inbox mint interpretation' USING ERRCODE = '23514';
        END IF;
    END LOOP;
    FOR index IN 0..(cardinality(p_items)-1) LOOP
        output := spec->'outputs'->index;
        IF jsonb_typeof(output) IS DISTINCT FROM 'object'
           OR (output - ARRAY['source_line','family','key','revision','quantity','stack_kind',
                'stack_maximum','proven_stack_maximum','container_capacity',
                'container_slot_equip_pattern','key_number']) <> '{}'::JSONB
           OR (output->'container_capacity' IS DISTINCT FROM 'null'::JSONB
               AND NOT (output @> '{"family":"Item","key":"oteryn:item.tibia.i23721",
                   "revision":"definition-r1","container_capacity":20,"stack_kind":0,
                   "key_number":null}'::JSONB))
           OR output->'container_slot_equip_pattern' IS DISTINCT FROM 'false'::JSONB THEN
            RAISE EXCEPTION 'unsupported Inbox mint output shape' USING ERRCODE = '23514';
        END IF;
        FOREACH field IN ARRAY ARRAY['family','key','revision'] LOOP
            text_value := output->>field;
            IF jsonb_typeof(output->field) IS DISTINCT FROM 'string'
               OR octet_length(text_value) NOT BETWEEN 1 AND (CASE WHEN field = 'family' THEN 128 ELSE 512 END)
               OR text_value ~ '[[:cntrl:]]' THEN
                RAISE EXCEPTION 'invalid Inbox item definition' USING ERRCODE = '23514';
            END IF;
        END LOOP;
        FOREACH field IN ARRAY ARRAY['source_line','quantity','stack_kind','stack_maximum'] LOOP
            IF jsonb_typeof(output->field) IS DISTINCT FROM 'number'
               OR (output->>field) !~ '^(0|[1-9][0-9]*)$' THEN
                RAISE EXCEPTION 'invalid Inbox output number' USING ERRCODE = '23514';
            END IF;
        END LOOP;
        IF (output->>'source_line')::NUMERIC NOT BETWEEN 0 AND 4294967295
           OR output->>'stack_kind' NOT IN ('0','1')
           OR (output->>'quantity')::NUMERIC NOT BETWEEN 1 AND 100 THEN
            RAISE EXCEPTION 'Inbox source line, class or quantity exceeds its bound' USING ERRCODE = '23514';
        END IF;
        IF output->'proven_stack_maximum' IS DISTINCT FROM 'null'::JSONB
           AND (jsonb_typeof(output->'proven_stack_maximum') IS DISTINCT FROM 'number'
                OR (output->>'proven_stack_maximum') !~ '^[1-9][0-9]*$'
                OR (output->>'proven_stack_maximum')::NUMERIC NOT BETWEEN 1 AND 100) THEN
            RAISE EXCEPTION 'invalid proven Inbox stack maximum' USING ERRCODE = '23514';
        END IF;
        expected_maximum := CASE WHEN output->>'stack_kind' = '0' THEN 1
            ELSE coalesce((output->>'proven_stack_maximum')::NUMERIC, 100) END;
        IF (output->>'stack_maximum')::NUMERIC <> expected_maximum
           OR (output->>'quantity')::NUMERIC > expected_maximum
           OR (output->>'stack_kind' = '0' AND output->'proven_stack_maximum' IS DISTINCT FROM 'null'::JSONB) THEN
            RAISE EXCEPTION 'Inbox quantity disagrees with its definition stack bound' USING ERRCODE = '23514';
        END IF;
        IF output->'key_number' IS DISTINCT FROM 'null'::JSONB
           AND (jsonb_typeof(output->'key_number') IS DISTINCT FROM 'number'
                OR (output->>'key_number') !~ '^[1-9][0-9]*$'
                OR (output->>'key_number')::NUMERIC NOT BETWEEN 1 AND 65535
                OR output->>'stack_kind' <> '0' OR output->>'quantity' <> '1') THEN
            RAISE EXCEPTION 'Inbox key must be one nonstackable item' USING ERRCODE = '23514';
        END IF;
    END LOOP;
    command_bytes := int8send((CASE WHEN p_command > 9223372036854775807
        THEN p_command - 18446744073709551616 ELSE p_command END)::BIGINT);
    source_digest := sha256(convert_to('oteryn:reward_claim_mint:inbox-source:v1','UTF8') ||
        decode('00','hex') || p_source_binding || sha256(p_spec));
    expected_binding := sha256(convert_to('oteryn:reward_claim_mint:inbox-plan:v1','UTF8') ||
        decode('00','hex') || uuid_send(p_session) || command_bytes || uuid_send(p_character) ||
        uuid_send(p_world) || uuid_send(p_channel) || uuid_send(p_transaction) || source_digest ||
        int4send(cardinality(p_items)) || item_bytes);
    IF p_plan_binding <> expected_binding THEN
        RAISE EXCEPTION 'Inbox plan does not bind its complete frozen mint spec' USING ERRCODE = '23514';
    END IF;
    IF EXISTS (SELECT 1 FROM game_reward_claim_mint_reservations v
                WHERE (v.game_session_id, v.command_id) = (p_session, p_command)) THEN
        RAISE EXCEPTION 'reward-claim CommandRef is reserved for the legacy destination' USING ERRCODE = 'OTI05';
    END IF;
    SELECT * INTO existing FROM game_reward_claim_inbox_mint_reservations v
     WHERE (v.game_session_id, v.command_id) = (p_session, p_command) FOR UPDATE;
    IF FOUND THEN
        IF (existing.character_id, existing.world_id, existing.channel_id, existing.transaction_id,
            existing.source_binding, existing.plan_binding, existing.item_instance_ids, existing.mint_spec)
           IS DISTINCT FROM (p_character, p_world, p_channel, p_transaction,
            p_source_binding, p_plan_binding, p_items, p_spec) THEN
            RAISE EXCEPTION 'reward Inbox CommandRef has another frozen mint' USING ERRCODE = 'OTI05';
        END IF;
    ELSE
        INSERT INTO game_reward_claim_inbox_mint_reservations
            (game_session_id, command_id, character_id, world_id, channel_id, transaction_id,
             source_binding, plan_binding, item_instance_ids, mint_spec, mint_intent_sha256,
             event_id, occurred_at, work_units_used)
        VALUES (p_session, p_command, p_character, p_world, p_channel, p_transaction,
                p_source_binding, p_plan_binding, p_items, p_spec, sha256(p_spec),
                game_character_uuid_v7(), floor(extract(epoch FROM clock_timestamp()) * 1000)::BIGINT, 0)
        RETURNING * INTO existing;
    END IF;
    RETURN QUERY SELECT existing.event_id, existing.occurred_at, existing.work_units_used;
END;
$$;

-- Restore the SECURITY DEFINER search path set by 0101 after replacement.
DO $$ BEGIN
    EXECUTE format('ALTER FUNCTION game_reward_claim_inbox_mint_reserve(UUID, NUMERIC, UUID, UUID, UUID, UUID, BYTEA, BYTEA, UUID[], BYTEA) SET search_path = pg_catalog, %I, pg_temp', current_schema());
END $$;
