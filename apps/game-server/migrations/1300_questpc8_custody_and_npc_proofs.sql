-- questpc8 prerequisites absent from main 1ae39c7f.
-- Main 0999-1008 remain canonical and unchanged. These distinct custody,
-- source NPC and atomic Father proofs have no second SQL definition on main.

-- Source prerequisite: quest_finish.sql
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

-- NPC storage-only responses share the exact physical quest receipt transaction.
CREATE TABLE game_quest_npc_program_receipts (
    character_id uuid NOT NULL REFERENCES game_character_roots(character_id),
    game_session_id uuid NOT NULL,
    command_id numeric(20,0) NOT NULL CHECK(command_id>=1),
    transition_key text NOT NULL CHECK(transition_key LIKE 'oteryn:quest-npc/%'),
    reply text[] NOT NULL,
    next_topic integer NOT NULL CHECK(next_topic>=0),
    PRIMARY KEY(character_id,game_session_id,command_id,transition_key),
    FOREIGN KEY(character_id,game_session_id,command_id,transition_key)
        REFERENCES game_character_quest_receipts(character_id,cause_id,cause_ordinal,transition_key)
);
REVOKE ALL ON game_quest_npc_program_receipts FROM PUBLIC;
GRANT SELECT,INSERT ON game_quest_npc_program_receipts TO oteryn_game_runtime;
GRANT SELECT ON game_quest_npc_program_receipts TO oteryn_game_control;

CREATE FUNCTION game_quest_npc_program_response_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF cardinality(NEW.reply)>32 OR EXISTS (
        SELECT 1 FROM unnest(NEW.reply) line
        WHERE line IS NULL OR octet_length(line)>2048
    ) OR NOT EXISTS (
        SELECT 1 FROM game_character_quest_receipts r
        WHERE r.character_id=NEW.character_id AND r.cause_id=NEW.game_session_id
          AND r.cause_ordinal=NEW.command_id AND r.transition_key=NEW.transition_key
          AND r.cause_kind='command' AND r.created_xact_id=pg_current_xact_id()
    ) THEN
        RAISE EXCEPTION 'NPC response requires bounded lines and its same-transaction quest receipt'
            USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END $$;
DO $$ BEGIN
    EXECUTE format('ALTER FUNCTION game_quest_npc_program_response_guard() SET search_path = %I, pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_quest_npc_program_response_guard() FROM PUBLIC;
CREATE TRIGGER game_quest_npc_program_response_proven
BEFORE INSERT ON game_quest_npc_program_receipts
FOR EACH ROW EXECUTE FUNCTION game_quest_npc_program_response_guard();

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


-- Source prerequisite: corruptor_lifecycle.sql
-- QUESTPC: Crystal201189f8 items.xml:87366-84. The approved native30
-- projection preserves the donor's source48 fact in the authoring overlay.
-- Reuse the original MINT, temporal-chain and corpse-retirement owners.
-- Preserve their current guards; extend only this closed source policy.
DO $$
DECLARE body TEXT; old TEXT; replacement TEXT;
BEGIN
 -- The DO block is atomic; an installed chain marks the complete policy.
 IF game_summer_corpse_chain('oteryn:item.tibia.i52113')=ARRAY[52113,52114,52115,52116] THEN
  RETURN;
 END IF;
 SELECT pg_get_functiondef('game_summer_corpse_chain(text)'::regprocedure) INTO STRICT body;
 old := 'WHEN ''oteryn:item.tibia.i54678'' THEN ARRAY[54678,54679,54680,54681]';
 IF position(old IN body)=0 THEN RAISE EXCEPTION 'corpse chain changed; review migration'; END IF;
 EXECUTE replace(body,old,'WHEN ''oteryn:item.tibia.i52113'' THEN ARRAY[52113,52114,52115,52116] '||old);

 SELECT pg_get_functiondef('game_summer_corpse_schedule()'::regprocedure) INTO STRICT body;
 old := 'CASE WHEN stage=1 THEN 10000 ELSE 300000 END';
 replacement := 'CASE WHEN chain[1]=52113 THEN CASE WHEN stage=3 THEN 10000 ELSE 600000 END ELSE '||old||' END';
 IF position(old IN body)=0 THEN RAISE EXCEPTION 'corpse schedule duration changed; review migration'; END IF;
 body := replace(body,old,replacement);
 old := 'CASE stage WHEN 1 THEN 10000 WHEN 2 THEN 310000 ELSE 610000 END';
 replacement := 'CASE WHEN chain[1]=52113 THEN CASE stage WHEN 1 THEN 600000 WHEN 2 THEN 1200000 ELSE 1210000 END ELSE '||old||' END';
 IF position(old IN body)=0 THEN RAISE EXCEPTION 'corpse schedule deadline changed; review migration'; END IF;
 EXECUTE replace(body,old,replacement);

 SELECT pg_get_functiondef('game_spell_field_temporal_schedule_proven()'::regprocedure) INTO STRICT body;
 old := 'CASE WHEN NEW.source_stage=1 THEN 10000 ELSE 300000 END';
 replacement := 'CASE WHEN chain[1]=52113 THEN CASE WHEN NEW.source_stage=3 THEN 10000 ELSE 600000 END ELSE '||old||' END';
 IF position(old IN body)=0 THEN RAISE EXCEPTION 'corpse proof duration changed; review migration'; END IF;
 body := replace(body,old,replacement);
 old := 'CASE NEW.source_stage WHEN 1 THEN 10000 WHEN 2 THEN 310000 ELSE 610000 END';
 replacement := 'CASE WHEN chain[1]=52113 THEN CASE NEW.source_stage WHEN 1 THEN 600000 WHEN 2 THEN 1200000 ELSE 1210000 END ELSE '||old||' END';
 IF position(old IN body)=0 THEN RAISE EXCEPTION 'corpse proof deadline changed; review migration'; END IF;
 EXECUTE replace(body,old,replacement);

 SELECT pg_get_functiondef('game_corpse_retirement_deadline(uuid)'::regprocedure) INTO STRICT body;
 old := 'ELSE 670000 END';
 replacement := 'WHEN EXISTS(SELECT 1 FROM game_spell_field_temporal_schedules p WHERE p.item_instance_id=p_item AND p.source_ordinal=0 AND p.source_stage=1 AND p.definition_production_key=''oteryn:item.tibia.i52113'') THEN 1810000 ELSE 670000 END';
 IF position(old IN body)=0 THEN RAISE EXCEPTION 'corpse retirement policy changed; review migration'; END IF;
 EXECUTE replace(body,old,replacement);
END $$;


-- Source prerequisite: questpc_npc_revision.sql
-- QUESTPC / NPC-LEGACY-1: read-compatible definition activation, no durable repinning.
-- Existing states/receipts/track rows are immutable. The catalogue retains each
-- exact pre-activation hash and absent-track default; only those hashes are read
-- compatibly by the fenced quest owner. Unknown hashes still require migration.
-- Father acceptance is one source operation: eleven tracks, one Character
-- revision and receipt. Every other receipt keeps the existing eight-track cap.
-- Rollback retains this additive constraint and the compatibility reader; an old
-- artifact unable to read the new definition must be rejected before activation.
ALTER TABLE game_character_quest_receipts
    DROP CONSTRAINT game_character_quest_receipt_effects_shape;
ALTER TABLE game_character_quest_receipts
    ADD CONSTRAINT game_character_quest_receipt_effects_shape CHECK (
        (cardinality(track_keys) BETWEEN 0 AND 8
             OR (cause_kind = 'command'
                 AND NOT completes
                 AND quest_key = 'oteryn:quest.a_father_s_burden_quest'
                 AND transition_key = 'oteryn:quest-npc/a_father_s_burden_quest/tereban_functions/mission-accept'
                 AND track_keys = ARRAY['oteryn:quest-progress/quest/u8_6/afathers_burden/cloth','oteryn:quest-progress/quest/u8_6/afathers_burden/crystal','oteryn:quest-progress/quest/u8_6/afathers_burden/iron','oteryn:quest-progress/quest/u8_6/afathers_burden/progress','oteryn:quest-progress/quest/u8_6/afathers_burden/quest_log','oteryn:quest-progress/quest/u8_6/afathers_burden/root','oteryn:quest-progress/quest/u8_6/afathers_burden/scale','oteryn:quest-progress/quest/u8_6/afathers_burden/silk','oteryn:quest-progress/quest/u8_6/afathers_burden/sinew','oteryn:quest-progress/quest/u8_6/afathers_burden/status','oteryn:quest-progress/quest/u8_6/afathers_burden/wood']::text[]
                 AND values_after = ARRAY[1,1,1,0,1,1,1,1,1,1,1]::bigint[]))
        AND (cardinality(track_keys) > 0 OR completes)
        AND (cardinality(track_keys) = 0
             OR (array_ndims(track_keys) = 1 AND array_lower(track_keys, 1) = 1
                 AND array_ndims(values_before) = 1 AND array_lower(values_before, 1) = 1
                 AND array_ndims(values_after) = 1 AND array_lower(values_after, 1) = 1))
        AND cardinality(values_before) = cardinality(track_keys)
        AND cardinality(values_after) = cardinality(track_keys)
        AND array_position(track_keys, NULL) IS NULL
        AND array_position(values_before, NULL) IS NULL
        AND array_position(values_after, NULL) IS NULL);


-- Source prerequisite: questpc_viola.sql
-- Current TibiaWiki Lost Wisdom reward, retaining Crystal's mission/yes flow.
-- Forward rollback: disable Viola's adapter; retain earned state and receipts.
CREATE TABLE game_character_viola (
 character_id UUID PRIMARY KEY REFERENCES game_character_roots(character_id),
 stage SMALLINT NOT NULL CHECK(stage IN (1,2))
);
CREATE TABLE game_viola_receipts (
 character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
 game_session_id UUID NOT NULL,
 command_id NUMERIC(20,0) NOT NULL CHECK(command_id>0),
 action SMALLINT NOT NULL CHECK(action IN (1,2)),
 reply SMALLINT NOT NULL CHECK(reply BETWEEN 0 AND 6),
 craft_transaction_id UUID UNIQUE REFERENCES game_item_craft_records(transaction_id),
 xp_occurrence_id UUID UNIQUE REFERENCES game_character_xp_receipts(reward_occurrence_id),
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
 PRIMARY KEY(character_id,game_session_id,command_id),
 CHECK((reply=3)=(craft_transaction_id IS NOT NULL AND xp_occurrence_id IS NOT NULL)),
 CHECK(reply=3 OR (craft_transaction_id IS NULL AND xp_occurrence_id IS NULL))
);
CREATE UNIQUE INDEX game_viola_once ON game_viola_receipts(character_id) WHERE reply=3;
CREATE INDEX game_viola_receipts_session ON game_viola_receipts(game_session_id);
REVOKE ALL ON game_character_viola,game_viola_receipts FROM PUBLIC;
GRANT SELECT,INSERT,UPDATE ON game_character_viola TO oteryn_game_runtime;
GRANT SELECT,INSERT ON game_viola_receipts TO oteryn_game_runtime;
GRANT SELECT ON game_character_viola,game_viola_receipts TO oteryn_game_control;
CREATE FUNCTION game_viola_complete() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE r game_item_craft_records%ROWTYPE; x game_character_xp_receipts%ROWTYPE;
BEGIN
 IF NEW.reply=3 THEN
  SELECT * INTO r FROM game_item_craft_records WHERE transaction_id=NEW.craft_transaction_id;
  SELECT * INTO x FROM game_character_xp_receipts WHERE reward_occurrence_id=NEW.xp_occurrence_id;
  IF r.created_xact_id IS DISTINCT FROM pg_current_xact_id() OR NEW.created_xact_id IS DISTINCT FROM pg_current_xact_id()
    OR r.character_id IS DISTINCT FROM NEW.character_id
    OR r.game_session_id IS DISTINCT FROM NEW.game_session_id OR r.command_id IS DISTINCT FROM NEW.command_id
    OR r.recipe IS DISTINCT FROM 'oteryn:npc-handoff.viola'
    OR r.before_item->>'definition_production_key' IS DISTINCT FROM 'oteryn:item.tibia.i51977'
    OR jsonb_array_length(r.outputs) IS DISTINCT FROM 1
    OR r.outputs->0->>'key' IS DISTINCT FROM 'oteryn:item.tibia.i3572'
    OR (r.outputs->0->>'count')::int IS DISTINCT FROM 1
    OR NOT EXISTS(SELECT 1 FROM game_character_xp_receipts WHERE reward_occurrence_id=NEW.xp_occurrence_id AND xmin=pg_current_xact_id()::xid)
    OR x.character_id IS DISTINCT FROM NEW.character_id OR x.experience_awarded IS DISTINCT FROM 800
    OR NOT EXISTS(SELECT 1 FROM game_character_viola WHERE character_id=NEW.character_id AND stage=2)
  THEN RAISE EXCEPTION 'Viola requires atomic book hand-in, scarf and 800 XP' USING ERRCODE='23514'; END IF;
 END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_viola_complete AFTER INSERT ON game_viola_receipts DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_viola_complete();
CREATE TRIGGER game_viola_receipt_immutable BEFORE UPDATE OR DELETE ON game_viola_receipts FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_viola_receipt_no_truncate BEFORE TRUNCATE ON game_viola_receipts FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();

-- Do not let the general item exchange owner acquire Viola completion authority alone.
CREATE FUNCTION game_viola_handoff_proven() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NEW.recipe='oteryn:npc-handoff.viola' AND NOT EXISTS(
   SELECT 1 FROM game_viola_receipts WHERE craft_transaction_id=NEW.transaction_id
    AND reply=3 AND created_xact_id=pg_current_xact_id())
 THEN RAISE EXCEPTION 'Viola handoff lacks its atomic completion receipt' USING ERRCODE='23514'; END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_viola_handoff_proven AFTER INSERT ON game_item_craft_records DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_viola_handoff_proven();
CREATE FUNCTION game_viola_state_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF (TG_OP='INSERT' AND NEW.stage<>1) OR
    (TG_OP='UPDATE' AND (NEW.character_id IS DISTINCT FROM OLD.character_id OR OLD.stage<>1 OR NEW.stage<>2))
 THEN RAISE EXCEPTION 'invalid Viola quest successor' USING ERRCODE='23514'; END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER game_viola_state_guard BEFORE INSERT OR UPDATE ON game_character_viola FOR EACH ROW EXECUTE FUNCTION game_viola_state_guard();
CREATE FUNCTION game_viola_state_proven() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NOT EXISTS(SELECT 1 FROM game_viola_receipts WHERE character_id=NEW.character_id
   AND created_xact_id=pg_current_xact_id() AND reply=CASE NEW.stage WHEN 1 THEN 1 ELSE 3 END)
 THEN RAISE EXCEPTION 'Viola quest state requires its same-transaction receipt' USING ERRCODE='23514'; END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_viola_state_proven AFTER INSERT OR UPDATE ON game_character_viola DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_viola_state_proven();
CREATE TRIGGER game_viola_state_no_truncate BEFORE TRUNCATE ON game_character_viola FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();

-- Reuse the carried-tree proof of the consume/mint owner for a nested book.
-- Preserve its existing W19 constraint and add only this exact NPC handoff.
DO $migration$
DECLARE previous TEXT;
BEGIN
 SELECT pg_get_expr(conbin,conrelid) INTO STRICT previous FROM pg_constraint
  WHERE conrelid='game_item_craft_records'::regclass AND conname='game_item_craft_event_source_location_check';
 ALTER TABLE game_item_craft_records DROP CONSTRAINT game_item_craft_event_source_location_check;
 EXECUTE format('ALTER TABLE game_item_craft_records ADD CONSTRAINT game_item_craft_event_source_location_check CHECK ((%s) OR (recipe=''oteryn:npc-handoff.viola'' AND consume_source AND before_item->>''definition_production_key''=''oteryn:item.tibia.i51977'' AND jsonb_typeof(event_source_location)=''object'' AND octet_length(event_source_location::text) BETWEEN 2 AND 4096 AND event_source_location ?& ARRAY[''item_instance_id'',''parent_item_instance_id'',''world_id'',''placement_ordinal'',''placed_transaction_id'']))',previous);
END $migration$;

DO $$
DECLARE signature TEXT;
BEGIN
 FOREACH signature IN ARRAY ARRAY['game_viola_complete()','game_viola_handoff_proven()','game_viola_state_guard()','game_viola_state_proven()'] LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path = %I, pg_temp',signature,current_schema());
 END LOOP;
END $$;
REVOKE ALL ON FUNCTION game_viola_complete(),game_viola_handoff_proven(),game_viola_state_guard(),game_viola_state_proven() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_viola_complete(),game_viola_handoff_proven(),game_viola_state_guard(),game_viola_state_proven() TO oteryn_game_runtime;


-- Source prerequisite: questpc_viola_book.sql
-- Current TibiaWiki Book of Wisdom (51977): one per character per 300 seconds.
-- Forward rollback disables the literal USE adapter; earned items/receipts remain.
CREATE TABLE game_character_viola_book_cycles (
 character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
 cycle BIGINT NOT NULL CHECK(cycle>0),
 claimed_at BIGINT NOT NULL,
 transaction_id UUID NOT NULL UNIQUE REFERENCES game_reward_claim_mint_receipts(transaction_id),
 PRIMARY KEY(character_id,cycle)
);
REVOKE ALL ON game_character_viola_book_cycles FROM PUBLIC;
GRANT SELECT,INSERT ON game_character_viola_book_cycles TO oteryn_game_runtime;
GRANT SELECT ON game_character_viola_book_cycles TO oteryn_game_control;
CREATE FUNCTION game_viola_book_now_ms() RETURNS BIGINT LANGUAGE SQL VOLATILE AS $$
 SELECT floor(extract(epoch FROM statement_timestamp())*1000)::bigint
$$;
CREATE FUNCTION game_viola_book_cycle_guard() RETURNS TRIGGER LANGUAGE plpgsql AS $$
DECLARE previous_cycle BIGINT; previous_time BIGINT;
BEGIN
 PERFORM 1 FROM game_character_roots WHERE character_id=NEW.character_id FOR UPDATE;
 SELECT cycle,claimed_at INTO previous_cycle,previous_time
 FROM game_character_viola_book_cycles WHERE character_id=NEW.character_id ORDER BY cycle DESC LIMIT 1;
 IF NEW.cycle<>COALESCE(previous_cycle,0)+1
    OR NEW.claimed_at NOT BETWEEN game_viola_book_now_ms()-10000 AND game_viola_book_now_ms()
    OR (previous_time IS NOT NULL AND NEW.claimed_at::numeric<previous_time::numeric+300000)
    OR NOT EXISTS(SELECT 1 FROM game_reward_claim_mint_receipts r
      JOIN game_item_instances i ON i.item_instance_id=r.item_instance_id
      WHERE r.transaction_id=NEW.transaction_id AND r.character_id=NEW.character_id
       AND r.claim_family='RewardClaim'
       AND r.claim_production_key='oteryn:reward-claim.tutorial.viola-book:cycle:'||NEW.cycle::text
       AND r.claim_revision_ref='reward-claim-r1' AND r.quantity=1
       AND i.definition_family='Item' AND i.definition_production_key='oteryn:item.tibia.i51977'
       AND i.definition_revision_ref='definition-r1'
       AND r.occurred_at=NEW.claimed_at AND r.created_xact_id=pg_current_xact_id())
 THEN RAISE EXCEPTION 'Book of Wisdom requires its atomic repeat-eligible mint' USING ERRCODE='23514'; END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER game_viola_book_cycle_guard BEFORE INSERT ON game_character_viola_book_cycles
 FOR EACH ROW EXECUTE FUNCTION game_viola_book_cycle_guard();
CREATE TRIGGER game_viola_book_cycle_immutable BEFORE UPDATE OR DELETE ON game_character_viola_book_cycles
 FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_viola_book_cycle_no_truncate BEFORE TRUNCATE ON game_character_viola_book_cycles
 FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();
CREATE FUNCTION game_viola_book_receipt_guard() RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
 IF starts_with(NEW.claim_production_key,'oteryn:reward-claim.tutorial.viola-book') AND NOT EXISTS(
   SELECT 1 FROM game_character_viola_book_cycles c WHERE c.transaction_id=NEW.transaction_id
    AND c.character_id=NEW.character_id AND c.claimed_at=NEW.occurred_at
    AND NEW.claim_family='RewardClaim'
    AND NEW.claim_production_key='oteryn:reward-claim.tutorial.viola-book:cycle:'||c.cycle::text)
 THEN RAISE EXCEPTION 'Book of Wisdom mint lacks its repeat receipt' USING ERRCODE='23514'; END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_viola_book_receipt_guard AFTER INSERT ON game_reward_claim_mint_receipts
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_viola_book_receipt_guard();
DO $$
DECLARE signature TEXT;
BEGIN
 FOREACH signature IN ARRAY ARRAY['game_viola_book_now_ms()','game_viola_book_cycle_guard()','game_viola_book_receipt_guard()'] LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path = %I, pg_temp',signature,current_schema());
 END LOOP;
END $$;
REVOKE ALL ON FUNCTION game_viola_book_now_ms(),game_viola_book_cycle_guard(),game_viola_book_receipt_guard() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_viola_book_now_ms(),game_viola_book_cycle_guard(),game_viola_book_receipt_guard() TO oteryn_game_runtime;
GRANT EXECUTE ON FUNCTION game_viola_book_now_ms() TO oteryn_game_control;


-- Source prerequisite: fathers_burden_items.sql
-- Tereban's eight material debits and final cape are bound to the existing
-- source NPC quest receipt and recoverable quest-XP obligation in one transaction.
-- Rollback disables these NPC branches; immutable earned receipts remain.
DO $migration$
DECLARE previous TEXT;
BEGIN
 SELECT pg_get_expr(conbin,conrelid) INTO STRICT previous FROM pg_constraint
 WHERE conrelid='game_item_craft_records'::regclass AND conname='game_item_craft_records_outputs_check';
 ALTER TABLE game_item_craft_records DROP CONSTRAINT game_item_craft_records_outputs_check;
 EXECUTE format('ALTER TABLE game_item_craft_records ADD CONSTRAINT game_item_craft_records_outputs_check CHECK ((%s) OR (consume_source AND recipe=''oteryn:npc-handoff.father-material'' AND jsonb_typeof(outputs)=''array'' AND jsonb_array_length(outputs)=0 AND before_item->>''definition_production_key'' IN (''oteryn:item.tibia.i11545'',''oteryn:item.tibia.i11546'',''oteryn:item.tibia.i11547'',''oteryn:item.tibia.i11548'',''oteryn:item.tibia.i11549'',''oteryn:item.tibia.i11550'',''oteryn:item.tibia.i11551'',''oteryn:item.tibia.i11552'')))',previous);
 SELECT pg_get_expr(conbin,conrelid) INTO STRICT previous FROM pg_constraint
 WHERE conrelid='game_item_craft_records'::regclass AND conname='game_item_craft_event_source_location_check';
 ALTER TABLE game_item_craft_records DROP CONSTRAINT game_item_craft_event_source_location_check;
 EXECUTE format('ALTER TABLE game_item_craft_records ADD CONSTRAINT game_item_craft_event_source_location_check CHECK ((%s) OR (consume_source AND recipe=''oteryn:npc-handoff.father-material'' AND before_item->>''definition_production_key'' IN (''oteryn:item.tibia.i11545'',''oteryn:item.tibia.i11546'',''oteryn:item.tibia.i11547'',''oteryn:item.tibia.i11548'',''oteryn:item.tibia.i11549'',''oteryn:item.tibia.i11550'',''oteryn:item.tibia.i11551'',''oteryn:item.tibia.i11552'') AND jsonb_typeof(event_source_location)=''object'' AND octet_length(event_source_location::text) BETWEEN 2 AND 4096 AND event_source_location ?& ARRAY[''item_instance_id'',''parent_item_instance_id'',''world_id'',''placement_ordinal'',''placed_transaction_id'']))',previous);
END $migration$;

CREATE FUNCTION game_father_values_proven() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE q game_character_quest_receipts%ROWTYPE; craft game_item_craft_records%ROWTYPE;
        cape game_reward_claim_mint_receipts%ROWTYPE; material TEXT; expected_track TEXT; expected_rule TEXT; amount BIGINT;
BEGIN
 IF TG_TABLE_NAME='game_character_quest_receipts' THEN q:=NEW;
 ELSIF TG_TABLE_NAME='game_item_craft_records' THEN
  IF NEW.recipe<>'oteryn:npc-handoff.father-material' THEN RETURN NULL; END IF;
  SELECT * INTO q FROM game_character_quest_receipts WHERE character_id=NEW.character_id
   AND cause_id=NEW.game_session_id AND cause_ordinal=NEW.command_id AND quest_key='oteryn:quest.a_father_s_burden_quest';
 ELSE
  SELECT * INTO q FROM game_character_quest_receipts WHERE character_id=NEW.character_id
   AND cause_id=NEW.game_session_id AND cause_ordinal=NEW.command_id AND transition_key='oteryn:quest-npc/a_father_s_burden_quest/tereban_functions/mission-complete';
 END IF;
 IF q.quest_key IS DISTINCT FROM 'oteryn:quest.a_father_s_burden_quest'
  OR q.cause_kind IS DISTINCT FROM 'command' OR q.created_xact_id IS DISTINCT FROM pg_current_xact_id()
 THEN RAISE EXCEPTION 'Father values require their atomic NPC receipt' USING ERRCODE='23514'; END IF;
 IF q.transition_key='oteryn:quest-npc/a_father_s_burden_quest/tereban_functions/mission-complete' THEN
  SELECT * INTO cape FROM game_reward_claim_mint_receipts WHERE character_id=q.character_id
   AND claim_family='RewardClaim' AND claim_production_key='oteryn:reward-claim.npc.tereban.old_cape'
   AND game_session_id=q.cause_id AND command_id=q.cause_ordinal;
  IF cape.created_xact_id IS DISTINCT FROM pg_current_xact_id()
   OR q.track_keys IS DISTINCT FROM ARRAY['oteryn:quest-progress/quest/u8_6/afathers_burden/status']::text[]
   OR q.values_before IS DISTINCT FROM ARRAY[1]::bigint[] OR q.values_after IS DISTINCT FROM ARRAY[2]::bigint[]
   OR NOT EXISTS(SELECT 1 FROM game_character_quest_tracks WHERE character_id=q.character_id
    AND track_key='oteryn:quest-progress/quest/u8_6/afathers_burden/progress' AND value=8)
  THEN RAISE EXCEPTION 'Father completion requires eight hand-ins and one cape' USING ERRCODE='23514'; END IF;
  IF NOT EXISTS(SELECT 1 FROM game_item_instances WHERE item_instance_id=cape.item_instance_id AND definition_family='Item' AND definition_production_key='oteryn:item.tibia.i11701' AND quantity=1 AND lifecycle=1 AND minted_transaction_id=cape.transaction_id) THEN RAISE EXCEPTION 'Father requires the exact fresh cape' USING ERRCODE='23514'; END IF;
  amount:=8000;
 ELSE
  SELECT * INTO craft FROM game_item_craft_records WHERE character_id=q.character_id AND game_session_id=q.cause_id
   AND command_id=q.cause_ordinal AND recipe='oteryn:npc-handoff.father-material';
  material:=craft.before_item->>'definition_production_key';
  SELECT track,rule INTO expected_track,expected_rule FROM (VALUES
   ('oteryn:item.tibia.i11548','sinew','strong-sinew-deliver'),('oteryn:item.tibia.i11547','wood','exquisite-wood-deliver'),
   ('oteryn:item.tibia.i11546','cloth','spectral-cloth-deliver'),('oteryn:item.tibia.i11545','silk','exquisite-silk-deliver'),
   ('oteryn:item.tibia.i11552','crystal','magic-crystal-deliver'),('oteryn:item.tibia.i11551','root','mystic-root-deliver'),
   ('oteryn:item.tibia.i11549','iron','old-iron-deliver'),('oteryn:item.tibia.i11550','scale','flexible-dragon-scale-deliver')) v(item,track,rule)
   WHERE item=material;
  IF craft.created_xact_id IS DISTINCT FROM pg_current_xact_id() OR expected_track IS NULL
   OR q.transition_key IS DISTINCT FROM 'oteryn:quest-npc/a_father_s_burden_quest/tereban_functions/'||expected_rule
   OR cardinality(q.track_keys)<>2 OR NOT craft.consume_source OR craft.outputs IS DISTINCT FROM '[]'::jsonb
   OR array_position(q.track_keys,'oteryn:quest-progress/quest/u8_6/afathers_burden/'||expected_track) IS NULL
   OR array_position(q.track_keys,'oteryn:quest-progress/quest/u8_6/afathers_burden/progress') IS NULL
   OR q.values_before[array_position(q.track_keys,'oteryn:quest-progress/quest/u8_6/afathers_burden/'||expected_track)] IS DISTINCT FROM 1
   OR q.values_after[array_position(q.track_keys,'oteryn:quest-progress/quest/u8_6/afathers_burden/'||expected_track)] IS DISTINCT FROM 2
   OR q.values_before[array_position(q.track_keys,'oteryn:quest-progress/quest/u8_6/afathers_burden/progress')] NOT BETWEEN 0 AND 7
   OR q.values_after[array_position(q.track_keys,'oteryn:quest-progress/quest/u8_6/afathers_burden/progress')]
    IS DISTINCT FROM q.values_before[array_position(q.track_keys,'oteryn:quest-progress/quest/u8_6/afathers_burden/progress')]+1
  THEN RAISE EXCEPTION 'Father hand-in requires its exact material and progress' USING ERRCODE='23514'; END IF;
  PERFORM game_item_craft_complete(craft.transaction_id);
  amount:=2500;
 END IF;
 IF NOT EXISTS(SELECT 1 FROM game_character_quest_xp_obligations x WHERE x.character_id=q.character_id
  AND x.cause_id=q.cause_id AND x.cause_ordinal=q.cause_ordinal AND x.transition_key=q.transition_key
  AND x.amount=amount AND x.xmin=pg_current_xact_id()::xid)
 THEN RAISE EXCEPTION 'Father values require their atomic quest XP obligation' USING ERRCODE='23514'; END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_father_npc_values AFTER INSERT ON game_character_quest_receipts
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW WHEN (NEW.quest_key='oteryn:quest.a_father_s_burden_quest'
 AND (NEW.transition_key LIKE '%-deliver' OR NEW.transition_key='oteryn:quest-npc/a_father_s_burden_quest/tereban_functions/mission-complete'))
 EXECUTE FUNCTION game_father_values_proven();
CREATE CONSTRAINT TRIGGER game_father_material_values AFTER INSERT ON game_item_craft_records
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_father_values_proven();
CREATE CONSTRAINT TRIGGER game_father_cape_values AFTER INSERT ON game_reward_claim_mint_receipts
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW WHEN (NEW.claim_family='RewardClaim' AND NEW.claim_production_key='oteryn:reward-claim.npc.tereban.old_cape')
 EXECUTE FUNCTION game_father_values_proven();
DO $$ BEGIN EXECUTE format('ALTER FUNCTION game_father_values_proven() SET search_path=pg_catalog,%I,pg_temp',current_schema()); END $$;
REVOKE ALL ON FUNCTION game_father_values_proven() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_father_values_proven() TO oteryn_game_runtime;
