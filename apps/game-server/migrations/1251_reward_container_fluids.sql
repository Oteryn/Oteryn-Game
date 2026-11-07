-- Filled direct reward children reuse 0762 complete-plan MINT and 0560 fluids.
-- Legacy plans remain byte-identical. A fluid is part of reservation/receipt
-- intent, and its first state is proven only by the exact fresh child MINT.

CREATE OR REPLACE FUNCTION game_reward_claim_contents_plan_valid(p_plan JSONB, p_capacity SMALLINT)
RETURNS BOOLEAN LANGUAGE plpgsql IMMUTABLE AS $$
DECLARE entry JSONB; ordinal BIGINT := 0; ids UUID[] := ARRAY[]::UUID[]; ident UUID;
BEGIN
    IF p_plan IS NULL OR jsonb_typeof(p_plan) <> 'array'
       OR octet_length(p_plan::text) > 32768 THEN RETURN FALSE; END IF;
    IF p_capacity IS NULL THEN RETURN p_plan = '[]'::jsonb; END IF;
    IF p_capacity NOT BETWEEN 1 AND 20
       OR jsonb_array_length(p_plan) NOT BETWEEN 1 AND p_capacity THEN RETURN FALSE; END IF;
    FOR entry IN SELECT value FROM jsonb_array_elements(p_plan) LOOP
        ordinal := ordinal + 1;
        IF jsonb_typeof(entry) <> 'object' THEN RETURN FALSE; END IF;
        IF NOT entry ?& ARRAY['ordinal','transaction_id','event_id','item_instance_id',
               'family','production_key','revision_ref','quantity','key_number','text']
           OR (SELECT count(*) FROM jsonb_object_keys(entry)) <> (CASE WHEN entry ? 'fluid_subtype' THEN 11 ELSE 10 END)
           OR (entry ? 'fluid_subtype' AND (jsonb_typeof(entry->'fluid_subtype') <> 'number'
               OR (entry->>'fluid_subtype')::NUMERIC NOT BETWEEN 1 AND 255
               OR (entry->>'fluid_subtype')::NUMERIC <> trunc((entry->>'fluid_subtype')::NUMERIC)
               OR (entry->>'quantity')::NUMERIC <> 1
               OR (entry->>'key_number')::NUMERIC <> 0 OR entry->'text' <> 'null'::jsonb))
           OR jsonb_typeof(entry->'ordinal') <> 'number'
           OR (entry->>'ordinal')::NUMERIC <> ordinal
           OR jsonb_typeof(entry->'quantity') <> 'number'
           OR (entry->>'quantity')::NUMERIC NOT BETWEEN 1 AND 100
           OR (entry->>'quantity')::NUMERIC <> trunc((entry->>'quantity')::NUMERIC)
           OR jsonb_typeof(entry->'key_number') <> 'number'
           OR (entry->>'key_number')::NUMERIC NOT BETWEEN 0 AND 65535
           OR (entry->>'key_number')::NUMERIC <> trunc((entry->>'key_number')::NUMERIC)
           OR ((entry->>'key_number')::INTEGER <> 0 AND (entry->>'quantity')::BIGINT <> 1)
           OR (entry->'text' <> 'null'::jsonb AND
               (jsonb_typeof(entry->'text') <> 'string'
                OR octet_length(entry->>'text') NOT BETWEEN 1 AND 7996
                OR char_length(entry->>'text') > 4000
                OR (entry->>'quantity')::BIGINT <> 1))
           OR jsonb_typeof(entry->'family') <> 'string' OR entry->>'family' <> 'Item'
           OR jsonb_typeof(entry->'production_key') <> 'string'
           OR octet_length(entry->>'production_key') NOT BETWEEN 1 AND 512
           OR jsonb_typeof(entry->'revision_ref') <> 'string'
           OR octet_length(entry->>'revision_ref') NOT BETWEEN 1 AND 512
           OR jsonb_typeof(entry->'transaction_id') <> 'string'
           OR jsonb_typeof(entry->'event_id') <> 'string'
           OR jsonb_typeof(entry->'item_instance_id') <> 'string' THEN RETURN FALSE; END IF;
        FOREACH ident IN ARRAY ARRAY[(entry->>'transaction_id')::UUID,
            (entry->>'event_id')::UUID, (entry->>'item_instance_id')::UUID] LOOP
            IF NOT game_character_is_uuid_v7(ident) OR ident = ANY(ids) THEN RETURN FALSE; END IF;
            ids := array_append(ids, ident);
        END LOOP;
    END LOOP;
    RETURN TRUE;
EXCEPTION WHEN invalid_text_representation OR numeric_value_out_of_range THEN RETURN FALSE;
END;
$$;

CREATE TABLE game_item_fluid_reward_mints (
    item_instance_id UUID PRIMARY KEY REFERENCES game_item_instances(item_instance_id),
    transaction_id UUID NOT NULL UNIQUE CHECK (game_character_is_uuid_v7(transaction_id)),
    root_transaction_id UUID NOT NULL REFERENCES game_reward_claim_mint_receipts(transaction_id),
    fluid_subtype SMALLINT NOT NULL CHECK (fluid_subtype BETWEEN 1 AND 255),
    created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id()
);
CREATE FUNCTION game_reward_fluid_source_current(p_item UUID,p_transaction UUID,p_root UUID,p_fluid SMALLINT)
RETURNS BOOLEAN LANGUAGE SQL STABLE SECURITY DEFINER AS $$
    SELECT EXISTS (
        SELECT 1 FROM game_reward_claim_mint_receipts r
          JOIN game_reward_claim_mint_reservations v USING(game_session_id,command_id)
          JOIN game_item_instances i ON i.item_instance_id=p_item
         WHERE r.transaction_id=p_root AND r.created_xact_id=pg_current_xact_id()
           AND i.world_id=v.world_id AND i.minted_transaction_id=p_transaction
           AND i.lifecycle=1 AND i.state_revision=1 AND i.last_transaction_id IS NULL
           AND r.contents_plan @> jsonb_build_array(jsonb_build_object(
               'item_instance_id',p_item::text,'transaction_id',p_transaction::text,
               'fluid_subtype',p_fluid::INTEGER,'quantity',1,'key_number',0,'text',NULL))
    );
$$;
CREATE FUNCTION game_reward_fluid_mint_guard() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
    IF NOT game_reward_fluid_source_current(NEW.item_instance_id,NEW.transaction_id,
        NEW.root_transaction_id,NEW.fluid_subtype) THEN
        RAISE EXCEPTION 'REWARD_FLUID_UNPROVEN' USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END $$;
CREATE TRIGGER game_item_fluid_reward_mints_stamp BEFORE INSERT ON game_item_fluid_reward_mints
    FOR EACH ROW EXECUTE FUNCTION game_item_fluid_transition_stamp();
CREATE TRIGGER game_item_fluid_reward_mints_proof BEFORE INSERT ON game_item_fluid_reward_mints
    FOR EACH ROW EXECUTE FUNCTION game_reward_fluid_mint_guard();
CREATE TRIGGER game_item_fluid_reward_mints_immutable BEFORE UPDATE OR DELETE ON game_item_fluid_reward_mints
    FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_item_fluid_reward_mints_no_truncate BEFORE TRUNCATE ON game_item_fluid_reward_mints
    EXECUTE FUNCTION game_item_immutable();

CREATE OR REPLACE FUNCTION game_item_fluid_state_proven() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE
    v_item UUID := CASE WHEN TG_OP = 'DELETE' THEN OLD.item_instance_id ELSE NEW.item_instance_id END;
    v_transaction UUID := CASE WHEN TG_OP = 'DELETE' THEN NULL ELSE NEW.last_transaction_id END;
    v_after SMALLINT := CASE WHEN TG_OP = 'DELETE' THEN 0 ELSE NEW.fluid_subtype END;
BEGIN
    IF TG_OP = 'UPDATE' AND (NEW.item_instance_id, NEW.world_id) IS DISTINCT FROM
                            (OLD.item_instance_id, OLD.world_id) THEN
        RAISE EXCEPTION 'ITEM_FLUID_IDENTITY' USING ERRCODE = '23514';
    END IF;
    IF TG_OP = 'INSERT' AND EXISTS (
        SELECT 1 FROM game_item_fluid_reward_mints m
          JOIN game_item_instances i USING(item_instance_id)
         WHERE m.item_instance_id = v_item AND m.transaction_id = v_transaction
           AND m.fluid_subtype = v_after AND i.world_id = NEW.world_id
           AND m.created_xact_id = pg_current_xact_id()
           AND game_reward_fluid_source_current(m.item_instance_id,m.transaction_id,
               m.root_transaction_id,m.fluid_subtype)
    ) THEN RETURN NEW; END IF;
    IF TG_OP = 'INSERT' AND EXISTS (
        SELECT 1 FROM game_item_fluid_loot_mints m
          JOIN game_item_mint_receipts r
            ON r.item_instance_id = m.item_instance_id
           AND r.transaction_id = m.transaction_id
          JOIN game_item_instances i
            ON i.item_instance_id = m.item_instance_id
           AND i.world_id = NEW.world_id
           AND i.minted_transaction_id = m.transaction_id
         WHERE m.item_instance_id = v_item
           AND m.transaction_id = v_transaction
           AND m.fluid_subtype = v_after
           AND m.created_xact_id = pg_current_xact_id()
    ) THEN
        RETURN NEW;
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM game_item_fluid_transitions t
         WHERE t.item_instance_id = v_item
           AND (v_transaction IS NULL OR t.transaction_id = v_transaction)
           AND t.fluid_after = v_after
           AND t.created_xact_id = pg_current_xact_id()
    ) THEN
        RAISE EXCEPTION 'ITEM_FLUID_UNPROVEN' USING ERRCODE = '23514';
    END IF;
    RETURN CASE WHEN TG_OP = 'DELETE' THEN OLD ELSE NEW END;
END $$;

CREATE FUNCTION game_reward_fluid_container_guard() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE entry JSONB; child UUID;
BEGIN
    FOR entry IN SELECT value FROM jsonb_array_elements(NEW.contents_plan) LOOP
        child := (entry->>'item_instance_id')::UUID;
        IF entry ? 'fluid_subtype' THEN
            IF NOT EXISTS (
                SELECT 1 FROM game_item_fluid_reward_mints m
                  JOIN game_item_fluid_states f USING(item_instance_id)
                 WHERE m.item_instance_id=child AND m.root_transaction_id=NEW.transaction_id
                   AND m.transaction_id=(entry->>'transaction_id')::UUID
                   AND m.fluid_subtype=(entry->>'fluid_subtype')::SMALLINT
                   AND f.fluid_subtype=m.fluid_subtype AND f.last_transaction_id=m.transaction_id
                   AND m.created_xact_id=pg_current_xact_id()
                   AND game_reward_fluid_source_current(child,m.transaction_id,
                       NEW.transaction_id,m.fluid_subtype)
            ) THEN RAISE EXCEPTION 'REWARD_FLUID_INCOMPLETE' USING ERRCODE='23514'; END IF;
        ELSIF EXISTS (SELECT 1 FROM game_item_fluid_states WHERE item_instance_id=child) THEN
            RAISE EXCEPTION 'REWARD_FLUID_UNEXPECTED' USING ERRCODE='23514';
        END IF;
    END LOOP;
    RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_reward_fluid_container_complete AFTER INSERT ON game_reward_claim_mint_receipts
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_reward_fluid_container_guard();

DO $$ DECLARE signature TEXT; BEGIN
    FOREACH signature IN ARRAY ARRAY[
        'game_reward_claim_contents_plan_valid(JSONB,SMALLINT)',
        'game_reward_fluid_source_current(UUID,UUID,UUID,SMALLINT)',
        'game_reward_fluid_mint_guard()','game_item_fluid_state_proven()',
        'game_reward_fluid_container_guard()'] LOOP
        EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',signature,current_schema());
        EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC',signature);
    END LOOP;
END $$;
REVOKE ALL ON game_item_fluid_reward_mints FROM PUBLIC;
GRANT SELECT,INSERT ON game_item_fluid_reward_mints TO oteryn_game_runtime;
GRANT SELECT ON game_item_fluid_reward_mints TO oteryn_game_control;
GRANT EXECUTE ON FUNCTION game_reward_fluid_source_current(UUID,UUID,UUID,SMALLINT)
    TO oteryn_game_runtime,oteryn_game_control;
