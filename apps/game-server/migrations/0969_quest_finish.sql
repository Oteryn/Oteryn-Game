-- QUEST-FINISH: source-resolved fluid rewards use the existing fluid state
-- owner and once-claim transaction, never a synthetic item or a later fill.
-- Crystal201189f data-global/startup/tables/chest.lua:1499-1506 (milk vial),
-- :1448-1455 (jug/cup), :2131-2155 (Dreamer's Challenge wine bags).
-- Those tuples alone do not establish a subtype: the content resolver must
-- supply a separately evidenced subtype; NULL remains an empty container.
CREATE FUNCTION game_reward_claim_fluid_plan_valid(p_plan JSONB, p_contents JSONB)
RETURNS BOOLEAN LANGUAGE plpgsql IMMUTABLE AS $$
DECLARE entry JSONB; previous INTEGER := -1; ordinal INTEGER; subtype INTEGER;
BEGIN
    IF p_plan IS NULL OR jsonb_typeof(p_plan) <> 'array'
       OR jsonb_array_length(p_plan) > 21 THEN RETURN FALSE; END IF;
    FOR entry IN SELECT value FROM jsonb_array_elements(p_plan) LOOP
        IF jsonb_typeof(entry) <> 'object'
           OR NOT entry ?& ARRAY['ordinal','subtype']
           OR (SELECT count(*) FROM jsonb_object_keys(entry)) <> 2
           OR jsonb_typeof(entry->'ordinal') <> 'number'
           OR jsonb_typeof(entry->'subtype') <> 'number' THEN RETURN FALSE; END IF;
        ordinal := (entry->>'ordinal')::INTEGER;
        subtype := (entry->>'subtype')::INTEGER;
        IF (entry->>'ordinal')::NUMERIC <> ordinal
           OR (entry->>'subtype')::NUMERIC <> subtype
           OR ordinal <= previous OR ordinal > jsonb_array_length(p_contents)
           OR subtype NOT BETWEEN 1 AND 255 THEN RETURN FALSE; END IF;
        IF ordinal > 0 AND ((p_contents->(ordinal-1)->>'quantity')::BIGINT <> 1
           OR (p_contents->(ordinal-1)->>'key_number')::INTEGER <> 0
           OR p_contents->(ordinal-1)->'text' <> 'null'::jsonb) THEN RETURN FALSE; END IF;
        previous := ordinal;
    END LOOP;
    RETURN TRUE;
EXCEPTION WHEN invalid_text_representation OR numeric_value_out_of_range THEN RETURN FALSE;
END $$;

ALTER TABLE game_reward_claim_mint_reservations ADD COLUMN fluid_plan JSONB NOT NULL DEFAULT '[]'::jsonb,
    ADD CONSTRAINT game_reward_claim_reserved_fluid_shape CHECK (
        game_reward_claim_fluid_plan_valid(fluid_plan,contents_plan)
        AND (NOT fluid_plan @> '[{"ordinal":0}]'::jsonb OR (root_text IS NULL AND contents_capacity IS NULL)));
ALTER TABLE game_reward_claim_mint_receipts ADD COLUMN fluid_plan JSONB NOT NULL DEFAULT '[]'::jsonb,
    ADD CONSTRAINT game_reward_claim_committed_fluid_shape CHECK (
        game_reward_claim_fluid_plan_valid(fluid_plan,contents_plan)
        AND (NOT fluid_plan @> '[{"ordinal":0}]'::jsonb OR (quantity=1 AND root_text IS NULL AND contents_capacity IS NULL)));

-- The BEFORE state guard accepts initialization only for the exact fresh
-- item, subtype, minted transaction and world in this transaction's receipt.
-- Subsequent fills/pours continue through the existing transition proof.
CREATE FUNCTION game_reward_claim_fluid_initialization_proven(p_item UUID, p_world UUID, p_transaction UUID, p_subtype SMALLINT)
RETURNS BOOLEAN LANGUAGE sql SECURITY DEFINER AS $$
    SELECT EXISTS (
        SELECT 1 FROM (
            SELECT * FROM game_reward_claim_mint_receipts WHERE transaction_id=p_transaction
            UNION ALL
            SELECT * FROM game_reward_claim_mint_receipts
             WHERE contents_capacity IS NOT NULL AND contents_plan @> jsonb_build_array(
                 jsonb_build_object('item_instance_id',p_item::TEXT,'transaction_id',p_transaction::TEXT))
        ) r
        JOIN game_reward_claim_mint_reservations v USING(game_session_id,command_id)
        JOIN game_item_instances i ON i.item_instance_id=p_item
        CROSS JOIN LATERAL jsonb_array_elements(r.fluid_plan) f
        WHERE r.created_xact_id=pg_current_xact_id() AND r.fluid_plan=v.fluid_plan
          AND r.contents_plan=v.contents_plan AND r.intent_binding=v.intent_binding
          AND r.transaction_id=v.transaction_id AND r.item_instance_id=v.item_instance_id
          AND i.item_instance_id=p_item AND i.world_id=p_world AND i.world_id=v.world_id
          AND i.minted_transaction_id=p_transaction AND i.lifecycle=1
          AND i.quantity=1 AND i.state_revision=1 AND i.last_transaction_id IS NULL
          AND (f->>'subtype')::SMALLINT=p_subtype
          AND CASE WHEN (f->>'ordinal')::INTEGER=0 THEN i.item_instance_id=r.item_instance_id AND p_transaction=r.transaction_id
              ELSE i.item_instance_id=(r.contents_plan->((f->>'ordinal')::INTEGER-1)->>'item_instance_id')::UUID
                AND p_transaction=(r.contents_plan->((f->>'ordinal')::INTEGER-1)->>'transaction_id')::UUID END
    );
$$;

-- Deferred receipt proof detects a dropped initialization as well as a
-- substituted subtype; the root and every child roll back together.
CREATE FUNCTION game_reward_claim_fluids_complete(p_transaction UUID)
RETURNS BOOLEAN LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE r game_reward_claim_mint_receipts%ROWTYPE; v game_reward_claim_mint_reservations%ROWTYPE;
        item UUID; minted UUID; entry JSONB; ordinal INTEGER; expected SMALLINT; actual SMALLINT; actual_transaction UUID;
BEGIN
    SELECT * INTO r FROM game_reward_claim_mint_receipts
     WHERE transaction_id=p_transaction AND created_xact_id=pg_current_xact_id();
    IF NOT FOUND THEN RETURN FALSE; END IF;
    SELECT * INTO v FROM game_reward_claim_mint_reservations
     WHERE game_session_id=r.game_session_id AND command_id=r.command_id;
    IF NOT FOUND OR r.fluid_plan IS DISTINCT FROM v.fluid_plan THEN RETURN FALSE; END IF;
    FOR ordinal IN 0..jsonb_array_length(r.contents_plan) LOOP
        IF ordinal=0 THEN item:=r.item_instance_id; minted:=r.transaction_id;
        ELSE entry:=r.contents_plan->(ordinal-1);
            item:=(entry->>'item_instance_id')::UUID; minted:=(entry->>'transaction_id')::UUID;
        END IF;
        SELECT (f->>'subtype')::SMALLINT INTO expected
          FROM jsonb_array_elements(r.fluid_plan) f WHERE (f->>'ordinal')::INTEGER=ordinal;
        SELECT fluid_subtype,last_transaction_id INTO actual,actual_transaction
          FROM game_item_fluid_states WHERE item_instance_id=item;
        IF actual IS DISTINCT FROM expected OR (expected IS NOT NULL AND actual_transaction IS DISTINCT FROM minted)
           OR (expected IS NOT NULL AND EXISTS (SELECT 1 FROM game_item_keys WHERE item_instance_id=item))
        THEN RETURN FALSE; END IF;
    END LOOP;
    RETURN TRUE;
END $$;

-- Extend installed guard bodies in place, preserving every owner arm.
DO $migration$
DECLARE name TEXT; body TEXT; arm TEXT; insert_at INTEGER;
BEGIN
    FOREACH name IN ARRAY ARRAY['game_item_fluid_state_proven','game_reward_claim_mint_consistency_guard'] LOOP
        body:=pg_get_functiondef((name||'()')::regprocedure);
        insert_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,1,'i');
        IF insert_at=0 THEN RAISE EXCEPTION 'unexpected QUEST-FINISH guard: %',name; END IF;
        IF name='game_item_fluid_state_proven' THEN arm:=$arm$
    IF TG_OP='INSERT' AND game_reward_claim_fluid_initialization_proven(
        NEW.item_instance_id,NEW.world_id,NEW.last_transaction_id,NEW.fluid_subtype)
    THEN RETURN NEW; END IF;
$arm$;
        ELSE arm:=$arm$
    IF NOT game_reward_claim_fluids_complete(NEW.transaction_id) THEN
        RAISE EXCEPTION 'reward claim must initialize its complete frozen fluid plan' USING ERRCODE='23514';
    END IF;
$arm$;
        END IF;
        EXECUTE overlay(body placing arm from insert_at for 0);
    END LOOP;
END $migration$;
DO $$ DECLARE signature TEXT; BEGIN
    FOREACH signature IN ARRAY ARRAY['game_reward_claim_fluid_plan_valid(JSONB,JSONB)',
        'game_reward_claim_fluid_initialization_proven(UUID,UUID,UUID,SMALLINT)',
        'game_reward_claim_fluids_complete(UUID)'] LOOP
        EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',signature,current_schema());
        EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC',signature);
    END LOOP;
END $$;
GRANT EXECUTE ON FUNCTION game_reward_claim_fluid_plan_valid(JSONB,JSONB),
    game_reward_claim_fluids_complete(UUID)
    TO oteryn_game_runtime,oteryn_game_control;
