-- NPC contribution redemption uses a distinct MINT identity for each priced
-- request. Preserve cooking and the existing once-only claim identities.
CREATE FUNCTION game_quest_npc_redemption_claim_key(
    plan JSONB, transition_key TEXT, cause_id UUID, cause_ordinal NUMERIC
) RETURNS TEXT LANGUAGE plpgsql IMMUTABLE AS $$
DECLARE
    redemption JSONB := plan->'redemption';
    ledger_change JSONB;
    item_count BIGINT;
    quantity BIGINT;
    cost BIGINT;
BEGIN
    IF redemption IS NULL THEN
        RETURN game_quest_npc_reward_claim_key(plan, transition_key);
    END IF;

    SELECT change INTO STRICT ledger_change FROM jsonb_array_elements(plan->'tracks') change
      WHERE change->>'key'=redemption->>'track';
    quantity := (redemption->>'quantity')::BIGINT;
    cost := (redemption->>'unit_cost')::BIGINT;
    SELECT (plan #>> '{give,count}')::BIGINT
         + coalesce(sum((entry->>'count')::BIGINT), 0) INTO item_count
      FROM jsonb_array_elements(coalesce(plan #> '{give,additional}', '[]'::JSONB)) entry;
    IF NOT ((quantity BETWEEN 1 AND 2000
        AND cost BETWEEN 1 AND 4294967295
        AND (ledger_change->>'after')::NUMERIC >= 0
        AND (ledger_change->>'before')::NUMERIC - (ledger_change->>'after')::NUMERIC
             = quantity * cost
        AND item_count = quantity
        AND NOT EXISTS(
            SELECT 1 FROM jsonb_array_elements(coalesce(plan #> '{give,additional}', '[]'::JSONB)) entry
             WHERE entry->>'item' IS DISTINCT FROM plan #>> '{give,item}'
        )) IS TRUE) THEN
        RAISE EXCEPTION 'invalid NPC contribution redemption' USING ERRCODE='23514';
    END IF;
    RETURN 'oteryn:reward-claim.npc-redemption.' || encode(
        sha256(convert_to(transition_key || '/' || encode(uuid_send(cause_id), 'hex')
            || '/' || cause_ordinal::TEXT, 'UTF8')), 'hex');
END $$;

DO $$
DECLARE
    body TEXT := pg_get_functiondef('game_quest_npc_item_complete()'::regprocedure);
    old_key CONSTANT TEXT := 'game_quest_npc_reward_claim_key(NEW.plan,NEW.transition_key)';
    new_key CONSTANT TEXT := 'game_quest_npc_redemption_claim_key(NEW.plan, NEW.transition_key, NEW.cause_id, NEW.cause_ordinal)';
    mint_guard CONSTANT TEXT := 'IF (NEW.plan->''give''=''null''::jsonb)';
BEGIN
    IF strpos(body, old_key) = 0 OR strpos(body, mint_guard) = 0 THEN
        RAISE EXCEPTION 'missing canonical NPC item MINT guard';
    END IF;
    body := replace(body, old_key, new_key);
    body := replace(body, mint_guard,
        'IF NEW.plan ? ''redemption'' AND NEW.mint_transaction_id IS NULL THEN
             RAISE EXCEPTION ''NPC contribution redemption lacks a MINT'' USING ERRCODE=''23514'';
         END IF;
         ' || mint_guard);
    EXECUTE body;
    EXECUTE format('ALTER FUNCTION game_quest_npc_redemption_claim_key(JSONB,TEXT,UUID,NUMERIC) SET search_path=pg_catalog,%I,pg_temp', current_schema());
END $$;

REVOKE ALL ON FUNCTION game_quest_npc_redemption_claim_key(JSONB,TEXT,UUID,NUMERIC) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_quest_npc_redemption_claim_key(JSONB,TEXT,UUID,NUMERIC) TO oteryn_game_runtime;

-- Crystal 201189f data/items/items.xml:13001-13016,16567-16572.
-- Scarab's death MINT uses the existing ground corpse scheduler. Its mature
-- stage is the exact item Tonar accepts (data-global/npc/tonar.lua:73-81).
DO $scarab_corpse_chain$
DECLARE
    body TEXT := pg_get_functiondef('game_summer_corpse_chain(text)'::regprocedure);
    anchor CONSTANT TEXT := 'WHEN ''oteryn:item.tibia.i54678'' THEN';
    root CONSTANT TEXT := 'oteryn:item.tibia.i6024';
BEGIN
    IF game_summer_corpse_chain(root) IS NOT NULL
        OR (length(body)-length(replace(body,anchor,'')))/length(anchor) <> 1 THEN
        RAISE EXCEPTION 'Scarab corpse registry ownership changed';
    END IF;
    EXECUTE replace(body, anchor,
        'WHEN ''oteryn:item.tibia.i6024'' THEN ARRAY[6024,4194,4195,4196] ' || anchor);
    IF game_summer_corpse_chain(root) IS DISTINCT FROM ARRAY[6024,4194,4195,4196]
        OR game_source_corpse_durations_v1(root) IS DISTINCT FROM ARRAY[10000,300000,300000,60000]::BIGINT[] THEN
        RAISE EXCEPTION 'Scarab corpse stages differ from the source';
    END IF;
END $scarab_corpse_chain$;

-- The final minute also applies when a mature shell has left native Ground
-- custody. Keep the original Ground corpse owner responsible for its loot tree.
DO $scarab_carried_deadline$
DECLARE
    body TEXT;
    target_name TEXT;
    needle CONSTANT TEXT := 'array_length(chain,1)-1';
    stage_max CONSTANT TEXT := 'array_length(chain,1)-CASE WHEN chain[1]=6024 THEN 0 ELSE 1 END';
    last_ground_transform CONSTANT TEXT := 'final.source_ordinal=0)';
    scheduled_target CONSTANT TEXT := $target$'Item','oteryn:item.tibia.i'||chain[stage+1]::text,'definition-r1',sha256$target$;
    qualified_target CONSTANT TEXT := $target$NEW.target_family IS DISTINCT FROM 'Item' OR NEW.target_revision IS DISTINCT FROM 'definition-r1'$target$;
BEGIN
    body := pg_get_functiondef('game_summer_corpse_schedule()'::regprocedure);
    IF (length(body)-length(replace(body,needle,'')))/length(needle) <> 1
       OR (length(body)-length(replace(body,scheduled_target,'')))/length(scheduled_target) <> 1 THEN
        RAISE EXCEPTION 'Scarab deadline scheduler predecessor changed';
    END IF;
    EXECUTE replace(replace(body,needle,stage_max),scheduled_target,$target$
        CASE WHEN chain[stage+1] IS NOT NULL THEN 'Item' END,
        'oteryn:item.tibia.i'||chain[stage+1]::text,
        CASE WHEN chain[stage+1] IS NOT NULL THEN 'definition-r1' END,sha256$target$);

    body := pg_get_functiondef('game_spell_field_temporal_schedule_proven()'::regprocedure);
    IF (length(body)-length(replace(body,needle,'')))/length(needle) <> 1
       OR (length(body)-length(replace(body,qualified_target,'')))/length(qualified_target) <> 1 THEN
        RAISE EXCEPTION 'Scarab deadline provenance predecessor changed';
    END IF;
    EXECUTE replace(replace(body,needle,stage_max),qualified_target,$target$
        NEW.target_family IS DISTINCT FROM CASE WHEN chain[NEW.source_stage+1] IS NOT NULL THEN 'Item' END
        OR NEW.target_revision IS DISTINCT FROM CASE WHEN chain[NEW.source_stage+1] IS NOT NULL THEN 'definition-r1' END$target$);

    FOREACH target_name IN ARRAY ARRAY['game_corpse_terminal_stage_ready(uuid)','game_item_decay_retire_consistency_guard()'] LOOP
        body := pg_get_functiondef(target_name::regprocedure);
        IF (length(body)-length(replace(body,last_ground_transform,'')))/length(last_ground_transform) <> 1 THEN
            RAISE EXCEPTION 'Scarab Ground retirement predecessor changed: %', target_name;
        END IF;
        EXECUTE replace(body,last_ground_transform,'final.source_ordinal=0 AND final.target_family IS NOT NULL)');
    END LOOP;
END $scarab_carried_deadline$;

-- Tonar's shell is an exact successor of a native Scarab corpse. The loot
-- container belongs to the corpse owner until its children have been removed.
-- Once portable, the committed transform or an ordinary custody handoff proves
-- that the same Item still has the original absolute deadline.
CREATE FUNCTION game_item_portable_scarab_shell(p_item UUID) RETURNS BOOLEAN
LANGUAGE sql STABLE STRICT SECURITY DEFINER AS $$
    SELECT EXISTS (
        SELECT 1 FROM game_item_instances i
        JOIN game_item_mint_receipts m ON m.item_instance_id=i.item_instance_id
            AND m.transaction_id=i.minted_transaction_id
        JOIN game_spell_field_temporal_schedules p ON p.item_instance_id=i.item_instance_id
            AND p.source_transaction_id=m.transaction_id AND p.source_ordinal=0
        JOIN game_spell_field_temporal_receipts prev ON prev.item_instance_id=i.item_instance_id
            AND prev.source_stage=p.source_stage-1
        WHERE i.item_instance_id=p_item AND i.lifecycle=1 AND i.quantity=1
          AND i.world_id=m.death_world_id AND i.definition_family='Item'
          AND i.definition_revision_ref='definition-r1'
          AND (p.source_stage,i.definition_production_key) IN
              ((3,'oteryn:item.tibia.i4195'),(4,'oteryn:item.tibia.i4196'))
          AND (p.definition_family,p.definition_production_key,p.definition_revision)=
              (i.definition_family,i.definition_production_key,i.definition_revision_ref)
          AND m.loot_purpose_key='CORPSE_MATERIALIZATION' AND m.draw_ordinal=0
          AND m.loot_table_family='Item' AND m.loot_table_production_key='oteryn:item.tibia.i6024'
          AND m.loot_table_revision_ref='definition-r1' AND m.materialized_at IS NOT NULL
          AND p.expires_at_unix_ms > floor(extract(epoch FROM statement_timestamp())*1000)::bigint
          AND ((prev.transaction_id=i.last_transaction_id AND prev.state_revision_before+1=i.state_revision)
            OR EXISTS (SELECT 1 FROM game_item_decay_handoffs h
                WHERE h.item_instance_id=i.item_instance_id AND h.result_revision=i.state_revision
                  AND h.custody_transaction_id=i.last_transaction_id
                  AND h.source_stage=prev.source_stage AND h.stage_transaction_id=prev.transaction_id))
          AND NOT EXISTS (SELECT 1 FROM game_item_corpse_container_entries loot
                          WHERE loot.parent_item_instance_id=i.item_instance_id)
    )
$$;

DO $scarab_pickup$
DECLARE
    body TEXT;
    anchor TEXT;
BEGIN
    EXECUTE format('ALTER FUNCTION game_item_portable_scarab_shell(UUID) SET search_path=pg_catalog,%I,pg_temp',current_schema());

    body := pg_get_functiondef('game_item_plain_corpse_source(uuid)'::regprocedure);
    anchor := 'AND NOT game_item_loop_drop_ref(g.corpse_ref))';
    IF (length(body)-length(replace(body,anchor,'')))/length(anchor) <> 1 THEN
        RAISE EXCEPTION 'Scarab plain corpse predecessor changed';
    END IF;
    EXECUTE replace(body,anchor,anchor||$arm$
        OR EXISTS (SELECT 1 FROM game_item_ground_locations g
            WHERE g.item_instance_id=p_item AND g.scope_kind=1
              AND game_corpse_stage_ground_current(p_item,g.world_id,g.channel_id,
                  g.runtime_scope_ownership_generation,g.stack_ordinal)
              AND game_item_portable_scarab_shell(p_item))$arm$);

    body := pg_get_functiondef('game_item_loop_line_admit()'::regprocedure);
    anchor := 'AND NOT COALESCE((NEW.to_kind IN(1,2) AND NEW.quantity_before=1 AND NEW.quantity_after=1';
    IF (length(body)-length(replace(body,anchor,'')))/length(anchor) <> 1 THEN
        RAISE EXCEPTION 'Scarab Ground pickup predecessor changed';
    END IF;
    EXECUTE replace(body,anchor,$arm$
        AND NOT (NEW.to_kind IN(1,2) AND NEW.quantity_before=1 AND NEW.quantity_after=1
            AND NEW.definition_family='Item' AND NEW.definition_revision_ref='definition-r1'
            AND NEW.definition_production_key IN ('oteryn:item.tibia.i4195','oteryn:item.tibia.i4196')
            AND game_item_portable_scarab_shell(NEW.item_instance_id)
            AND (game_item_plain_corpse_source(NEW.item_instance_id)
                 OR EXISTS (SELECT 1 FROM game_item_ground_locations dropped
                            WHERE dropped.item_instance_id=NEW.item_instance_id
                              AND game_item_loop_drop_ref(dropped.corpse_ref))))
        $arm$||anchor);
END $scarab_pickup$;

REVOKE ALL ON FUNCTION game_item_portable_scarab_shell(UUID) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_item_portable_scarab_shell(UUID) TO oteryn_game_runtime;


-- Crystal data-global/npc/marvin.lua:77-85 grants credit even when the
-- participant holds no votes by confirmation time.
DO $participant_debit$
DECLARE
    signature CONSTANT TEXT := 'game_quest_reward_effects_valid(jsonb,text[],bigint[],bigint[])';
    anchor CONSTANT TEXT := 'WHEN ''set_now'' THEN';
    body TEXT := pg_get_functiondef(signature::regprocedure);
BEGIN
    IF (length(body)-length(replace(body,anchor,'')))/length(anchor) <> 1 THEN
        RAISE EXCEPTION 'missing or ambiguous Quest reward effect branch';
    END IF;
    EXECUTE replace(body, anchor, $effect$
            WHEN 'subtract_if_at_least' THEN
                IF EXISTS(SELECT 1 FROM jsonb_object_keys(v_effect->'effect') key
                          WHERE key NOT IN ('kind','amount'))
                    OR jsonb_typeof(v_effect #> '{effect,amount}') IS DISTINCT FROM 'number'
                    OR NOT coalesce((v_effect #>> '{effect,amount}') ~ '^[1-9][0-9]{0,9}$', false)
                    THEN RETURN false; END IF;
                v_expected := (v_effect #>> '{effect,amount}')::numeric;
                IF v_expected > 4294967295 THEN RETURN false; END IF;
                v_expected := CASE WHEN v_before[v_index] >= v_expected
                    THEN v_before[v_index] - v_expected ELSE v_before[v_index] END;
            WHEN 'set_now' THEN$effect$);
END $participant_debit$;
