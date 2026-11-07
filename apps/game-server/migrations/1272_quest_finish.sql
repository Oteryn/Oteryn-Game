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

-- NPC programme response storage is owned by canonical main migration1310.

-- Complete Falcon section for root-owned migration0969; not a separate migration.
CREATE TABLE game_character_falcon_chest_cycles (
 character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
 chest_key TEXT NOT NULL CHECK (chest_key IN (
 'oteryn:reward-claim.quest.u11_80.the_secret_library.falcon_bastion.chests_timer.coast',
 'oteryn:reward-claim.quest.u11_80.the_secret_library.falcon_bastion.chests_timer.island',
 'oteryn:reward-claim.quest.u11_80.the_secret_library.falcon_bastion.chests_timer.throne_hall',
 'oteryn:reward-claim.quest.u11_80.the_secret_library.falcon_bastion.chests_timer.shortcut',
 'oteryn:reward-claim.quest.u11_80.the_secret_library.falcon_bastion.chests_timer.lower_bastion',
 'oteryn:reward-claim.quest.u11_80.the_secret_library.falcon_bastion.chests_timer.underground_bastion')),
 cycle BIGINT NOT NULL CHECK(cycle>0),
 entropy BYTEA NOT NULL CHECK(octet_length(entropy)=32),
 selected_index SMALLINT NOT NULL CHECK(selected_index BETWEEN 0 AND 6),
 drawn_at BIGINT NOT NULL,
 claimed_at BIGINT NULL,
 transaction_id UUID NULL REFERENCES game_reward_claim_mint_receipts(transaction_id),
 CHECK((claimed_at IS NULL) = (transaction_id IS NULL)),
 CHECK(claimed_at IS NULL OR claimed_at>=drawn_at),
 PRIMARY KEY(character_id,chest_key,cycle)
);
CREATE UNIQUE INDEX game_falcon_cycle_pending ON game_character_falcon_chest_cycles(character_id,chest_key) WHERE claimed_at IS NULL;
CREATE UNIQUE INDEX game_falcon_cycle_transaction ON game_character_falcon_chest_cycles(transaction_id) WHERE transaction_id IS NOT NULL;
REVOKE ALL ON game_character_falcon_chest_cycles FROM PUBLIC;
GRANT SELECT,INSERT,UPDATE ON game_character_falcon_chest_cycles TO oteryn_game_runtime;
GRANT SELECT ON game_character_falcon_chest_cycles TO oteryn_game_control;
CREATE FUNCTION game_falcon_now_ms() RETURNS BIGINT
LANGUAGE SQL VOLATILE AS $$
    SELECT floor(extract(epoch FROM statement_timestamp())*1000)::bigint
$$;

CREATE FUNCTION game_falcon_draw_index(p_entropy BYTEA) RETURNS SMALLINT
LANGUAGE plpgsql IMMUTABLE STRICT AS $$
DECLARE seed_block BYTEA := p_entropy; ordinal INTEGER; sample_byte INTEGER;
BEGIN
    IF octet_length(seed_block) <> 32 THEN
        RAISE EXCEPTION 'Falcon entropy must be 32 bytes' USING ERRCODE='23514';
    END IF;
    LOOP
        FOR ordinal IN 0..31 LOOP
            sample_byte := get_byte(seed_block,ordinal);
            IF sample_byte < 252 THEN RETURN (sample_byte % 7)::smallint; END IF;
        END LOOP;
        seed_block := sha256(seed_block);
    END LOOP;
END;
$$;

CREATE FUNCTION game_falcon_cycle_guard() RETURNS TRIGGER
LANGUAGE plpgsql AS $$
DECLARE previous_cycle BIGINT; previous_claimed BIGINT;
BEGIN
    IF TG_OP='DELETE' THEN
        RAISE EXCEPTION 'Falcon cycle receipts are retained' USING ERRCODE='23514';
    END IF;
    -- Serialize the six per-character cycle owners even for direct SQL calls.
    PERFORM 1 FROM game_character_roots WHERE character_id=NEW.character_id FOR UPDATE;
    IF NEW.selected_index <> game_falcon_draw_index(NEW.entropy) THEN
        RAISE EXCEPTION 'Falcon draw differs from frozen entropy' USING ERRCODE='23514';
    END IF;
    IF TG_OP='INSERT' THEN
        SELECT cycle,claimed_at INTO previous_cycle,previous_claimed
          FROM game_character_falcon_chest_cycles
         WHERE character_id=NEW.character_id AND chest_key=NEW.chest_key
         ORDER BY cycle DESC LIMIT 1;
        IF NEW.claimed_at IS NOT NULL OR NEW.transaction_id IS NOT NULL
           OR NEW.cycle <> COALESCE(previous_cycle,0)+1
           OR NEW.drawn_at NOT BETWEEN game_falcon_now_ms()-10000 AND game_falcon_now_ms()
           OR (previous_cycle IS NOT NULL AND
               (previous_claimed IS NULL OR
                NEW.drawn_at::numeric < previous_claimed::numeric+86400000)) THEN
            RAISE EXCEPTION 'Falcon cycle must follow successful 24h eligibility' USING ERRCODE='23514';
        END IF;
    ELSIF TG_OP='UPDATE' THEN
        IF OLD.claimed_at IS NOT NULL OR NEW.claimed_at IS NULL OR NEW.transaction_id IS NULL
           OR ROW(NEW.character_id,NEW.chest_key,NEW.cycle,NEW.entropy,NEW.selected_index,NEW.drawn_at)
              IS DISTINCT FROM ROW(OLD.character_id,OLD.chest_key,OLD.cycle,OLD.entropy,OLD.selected_index,OLD.drawn_at)
           OR NOT EXISTS (
                SELECT 1 FROM game_reward_claim_mint_receipts r
                 WHERE r.transaction_id=NEW.transaction_id
                   AND r.character_id=NEW.character_id
                   AND r.claim_family='RewardClaim'
                   AND r.claim_production_key=NEW.chest_key||':cycle:'||NEW.cycle::text
                   AND r.occurred_at=NEW.claimed_at
                   AND r.created_xact_id=pg_current_xact_id()) THEN
            RAISE EXCEPTION 'Falcon cycle may only acknowledge its atomic mint' USING ERRCODE='23514';
        END IF;
    END IF;
    RETURN NEW;
END;
$$;
CREATE TRIGGER game_falcon_cycle_guard BEFORE INSERT OR UPDATE OR DELETE
ON game_character_falcon_chest_cycles FOR EACH ROW EXECUTE FUNCTION game_falcon_cycle_guard();
CREATE TRIGGER game_falcon_cycles_no_truncate BEFORE TRUNCATE
ON game_character_falcon_chest_cycles FOR EACH STATEMENT EXECUTE FUNCTION game_item_immutable();

CREATE FUNCTION game_falcon_receipt_guard() RETURNS TRIGGER
LANGUAGE plpgsql AS $$
BEGIN
    -- Internal operation identities remain subject to every existing generic
    -- mint proof. This additional deferred proof closes direct once-owner calls.
    IF NOT starts_with(NEW.claim_production_key,'oteryn:reward-claim.quest.u11_80.the_secret_library.falcon_bastion.chests_timer.') THEN
        RETURN NULL;
    END IF;
    IF NEW.claim_production_key !~ '^oteryn:reward-claim[.]quest[.]u11_80[.]the_secret_library[.]falcon_bastion[.]chests_timer[.](coast|island|throne_hall|shortcut|lower_bastion|underground_bastion):cycle:[1-9][0-9]*$' THEN
        RAISE EXCEPTION 'Falcon public claims require the repeat owner' USING ERRCODE='23514';
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM game_character_falcon_chest_cycles c
        JOIN game_item_instances i ON i.item_instance_id=NEW.item_instance_id
        WHERE c.character_id=NEW.character_id AND c.transaction_id=NEW.transaction_id
          AND NEW.claim_family='RewardClaim'
          AND NEW.claim_production_key=c.chest_key||':cycle:'||c.cycle::text
          AND c.claimed_at=NEW.occurred_at
          AND NEW.quantity=(ARRAY[1,1,1,1,1,1,10])[c.selected_index+1]
          AND i.definition_family='Item'
          AND i.definition_production_key='oteryn:item.tibia.i'||(ARRAY[9081,28821,28823,9058,6299,3052,3035])[c.selected_index+1]::text
          AND c.selected_index=game_falcon_draw_index(c.entropy)) THEN
        RAISE EXCEPTION 'Falcon mint requires its exact cycle and selected pool item' USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END;
$$;
CREATE CONSTRAINT TRIGGER game_falcon_receipt_guard AFTER INSERT
ON game_reward_claim_mint_receipts DEFERRABLE INITIALLY DEFERRED
FOR EACH ROW EXECUTE FUNCTION game_falcon_receipt_guard();

DO $$
DECLARE signature TEXT;
BEGIN
    FOREACH signature IN ARRAY ARRAY['game_falcon_now_ms()','game_falcon_draw_index(BYTEA)',
        'game_falcon_cycle_guard()','game_falcon_receipt_guard()'] LOOP
        EXECUTE format('ALTER FUNCTION %s SET search_path = %I, pg_temp',signature,current_schema());
    END LOOP;
END;
$$;
REVOKE ALL ON FUNCTION game_falcon_now_ms(),game_falcon_draw_index(BYTEA),game_falcon_cycle_guard(),game_falcon_receipt_guard() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_falcon_now_ms(),game_falcon_draw_index(BYTEA),game_falcon_cycle_guard(),game_falcon_receipt_guard() TO oteryn_game_runtime;
GRANT EXECUTE ON FUNCTION game_falcon_now_ms(),game_falcon_draw_index(BYTEA) TO oteryn_game_control;
