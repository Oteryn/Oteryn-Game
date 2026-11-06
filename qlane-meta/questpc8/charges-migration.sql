-- Exact source partial charges initialize the existing timed owner at mint.
-- The immutable claim receipt proves initialization; later timed writes retain
-- their existing revision, cause, custody and mutation proofs.
CREATE FUNCTION game_reward_claim_charges_plan_valid(p_plan JSONB, p_contents JSONB)
RETURNS BOOLEAN LANGUAGE plpgsql IMMUTABLE AS $$
DECLARE entry JSONB; previous INTEGER := -1; ordinal INTEGER;
BEGIN
    IF p_plan IS NULL OR jsonb_typeof(p_plan) <> 'array'
       OR jsonb_array_length(p_plan)>21 THEN RETURN FALSE; END IF;
    FOR entry IN SELECT value FROM jsonb_array_elements(p_plan) LOOP
        IF jsonb_typeof(entry)<>'object'
           OR NOT entry ?& ARRAY['ordinal','charges_remaining']
           OR (SELECT count(*) FROM jsonb_object_keys(entry))<>2
           OR jsonb_typeof(entry->'ordinal')<>'number'
           OR jsonb_typeof(entry->'charges_remaining')<>'number'
           OR (entry->>'charges_remaining')::NUMERIC<>1 THEN RETURN FALSE; END IF;
        ordinal:=(entry->>'ordinal')::INTEGER;
        IF (entry->>'ordinal')::NUMERIC<>ordinal OR ordinal<=previous
           OR ordinal>jsonb_array_length(p_contents) THEN RETURN FALSE; END IF;
        IF ordinal>0 AND ((p_contents->(ordinal-1)->>'quantity')::BIGINT<>1
           OR (p_contents->(ordinal-1)->>'key_number')::INTEGER<>0
           OR p_contents->(ordinal-1)->'text'<>'null'::jsonb) THEN RETURN FALSE; END IF;
        previous:=ordinal;
    END LOOP;
    RETURN TRUE;
EXCEPTION WHEN invalid_text_representation OR numeric_value_out_of_range THEN RETURN FALSE;
END $$;

ALTER TABLE game_reward_claim_mint_reservations ADD COLUMN charges_plan JSONB NOT NULL DEFAULT '[]'::jsonb,
    ADD CONSTRAINT game_reward_claim_reserved_charges_shape CHECK (
        game_reward_claim_charges_plan_valid(charges_plan,contents_plan)
        AND (NOT charges_plan @> '[{"ordinal":0}]'::jsonb OR (root_text IS NULL AND contents_capacity IS NULL)));
ALTER TABLE game_reward_claim_mint_receipts ADD COLUMN charges_plan JSONB NOT NULL DEFAULT '[]'::jsonb,
    ADD CONSTRAINT game_reward_claim_committed_charges_shape CHECK (
        game_reward_claim_charges_plan_valid(charges_plan,contents_plan)
        AND (NOT charges_plan @> '[{"ordinal":0}]'::jsonb OR (quantity=1 AND root_text IS NULL AND contents_capacity IS NULL)));

CREATE FUNCTION game_reward_claim_charge_initialization_proven(p_item UUID, p_charges INTEGER)
RETURNS BOOLEAN LANGUAGE sql SECURITY DEFINER AS $$
    SELECT EXISTS (
        SELECT 1 FROM game_reward_claim_mint_receipts r
        JOIN game_reward_claim_mint_reservations v USING(game_session_id,command_id)
        JOIN game_item_instances i ON i.item_instance_id=p_item
        CROSS JOIN LATERAL jsonb_array_elements(r.charges_plan) f
        WHERE r.created_xact_id=pg_current_xact_id() AND r.charges_plan=v.charges_plan
          AND r.contents_plan=v.contents_plan AND r.intent_binding=v.intent_binding
          AND r.transaction_id=v.transaction_id AND r.item_instance_id=v.item_instance_id
          AND r.character_id=v.character_id AND i.world_id=v.world_id
          AND i.lifecycle=1 AND i.quantity=1 AND i.state_revision=1 AND i.last_transaction_id IS NULL
          AND i.definition_family='Item' AND i.definition_revision_ref='definition-r1'
          AND i.definition_production_key IN ('oteryn:item.tibia.i3048','oteryn:item.tibia.i3054',
              'oteryn:item.tibia.i3081','oteryn:item.tibia.i3082','oteryn:item.tibia.i3083','oteryn:item.tibia.i3085')
          AND p_charges=1 AND (f->>'charges_remaining')::INTEGER=p_charges
          AND NOT EXISTS(SELECT 1 FROM game_item_keys WHERE item_instance_id=p_item)
          AND NOT EXISTS(SELECT 1 FROM game_item_texts WHERE item_instance_id=p_item)
          AND NOT EXISTS(SELECT 1 FROM game_item_fluid_states WHERE item_instance_id=p_item)
          AND CASE WHEN (f->>'ordinal')::INTEGER=0 THEN i.item_instance_id=r.item_instance_id
                AND i.minted_transaction_id=r.transaction_id
              ELSE i.item_instance_id=(r.contents_plan->((f->>'ordinal')::INTEGER-1)->>'item_instance_id')::UUID
                AND i.minted_transaction_id=(r.contents_plan->((f->>'ordinal')::INTEGER-1)->>'transaction_id')::UUID END
          AND game_reward_claim_container_complete(r.transaction_id)
    );
$$;

CREATE FUNCTION game_reward_claim_charges_complete(p_transaction UUID)
RETURNS BOOLEAN LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE r game_reward_claim_mint_receipts%ROWTYPE; v game_reward_claim_mint_reservations%ROWTYPE;
        item UUID; ordinal INTEGER; expected INTEGER; actual game_item_timed_states%ROWTYPE;
BEGIN
    SELECT * INTO r FROM game_reward_claim_mint_receipts
     WHERE transaction_id=p_transaction AND created_xact_id=pg_current_xact_id();
    IF NOT FOUND THEN RETURN FALSE; END IF;
    SELECT * INTO v FROM game_reward_claim_mint_reservations
     WHERE game_session_id=r.game_session_id AND command_id=r.command_id;
    IF NOT FOUND OR r.charges_plan IS DISTINCT FROM v.charges_plan THEN RETURN FALSE; END IF;
    FOR ordinal IN 0..jsonb_array_length(r.contents_plan) LOOP
        IF ordinal=0 THEN item:=r.item_instance_id;
        ELSE item:=(r.contents_plan->(ordinal-1)->>'item_instance_id')::UUID; END IF;
        SELECT (f->>'charges_remaining')::INTEGER INTO expected
          FROM jsonb_array_elements(r.charges_plan) f WHERE (f->>'ordinal')::INTEGER=ordinal;
        SELECT * INTO actual FROM game_item_timed_states WHERE item_instance_id=item;
        IF expected IS NULL THEN
            IF FOUND THEN RETURN FALSE; END IF;
        ELSIF NOT FOUND OR actual.charges IS DISTINCT FROM expected OR actual.state_revision<>1
           OR actual.remaining_ms IS NOT NULL OR actual.deadline_at IS NOT NULL
           OR NOT game_reward_claim_charge_initialization_proven(item,expected)
        THEN RETURN FALSE; END IF;
    END LOOP;
    RETURN TRUE;
END $$;

DO $migration$
DECLARE name TEXT; body TEXT; arm TEXT; insert_at INTEGER;
BEGIN
    FOREACH name IN ARRAY ARRAY['game_item_timed_state_row_guard','game_item_timed_state_row_proven',
                                'game_reward_claim_mint_consistency_guard'] LOOP
        body:=pg_get_functiondef((name||'()')::regprocedure);
        insert_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,1,'i');
        IF insert_at=0 THEN RAISE EXCEPTION 'unexpected charge initialization guard: %',name; END IF;
        IF name='game_reward_claim_mint_consistency_guard' THEN arm:=$arm$
    IF NOT game_reward_claim_charges_complete(NEW.transaction_id) THEN
        RAISE EXCEPTION 'reward claim must initialize its complete frozen charge plan' USING ERRCODE='23514';
    END IF;
$arm$;
        ELSE
            arm:=$arm$
    IF TG_OP='INSERT' AND NEW.state_revision=1 AND NEW.remaining_ms IS NULL AND NEW.deadline_at IS NULL
       AND game_reward_claim_charge_initialization_proven(NEW.item_instance_id,NEW.charges)
    THEN $arm$ || CASE WHEN name='game_item_timed_state_row_guard' THEN 'RETURN NEW;' ELSE 'RETURN NULL;' END || ' END IF;';
        END IF;
        EXECUTE overlay(body placing arm from insert_at for 0);
    END LOOP;
END $migration$;
DO $$ DECLARE signature TEXT; BEGIN
    FOREACH signature IN ARRAY ARRAY['game_reward_claim_charges_plan_valid(JSONB,JSONB)',
        'game_reward_claim_charge_initialization_proven(UUID,INTEGER)',
        'game_reward_claim_charges_complete(UUID)'] LOOP
        EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',signature,current_schema());
        EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC',signature);
    END LOOP;
END $$;
GRANT EXECUTE ON FUNCTION game_reward_claim_charges_plan_valid(JSONB,JSONB),
    game_reward_claim_charge_initialization_proven(UUID,INTEGER),game_reward_claim_charges_complete(UUID)
    TO oteryn_game_runtime,oteryn_game_control;
