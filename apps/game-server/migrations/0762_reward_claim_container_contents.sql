-- W15b: source-authored reward containers use the existing once-claim owner.
-- The root reservation freezes one bounded, ordered child plan. The root,
-- every child, its nested custody, optional key and one-item audit commit in
-- the same physical transaction. Legacy empty plans keep their exact receipts.

CREATE FUNCTION game_reward_claim_contents_plan_valid(p_plan JSONB, p_capacity SMALLINT)
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
           OR (SELECT count(*) FROM jsonb_object_keys(entry)) <> 10
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

ALTER TABLE game_reward_claim_mint_reservations
    ADD COLUMN contents_capacity SMALLINT,
    ADD COLUMN contents_plan JSONB NOT NULL DEFAULT '[]'::jsonb,
    ADD COLUMN root_text TEXT,
    ADD CONSTRAINT game_reward_claim_reserved_root_text_shape
        CHECK (root_text IS NULL OR (octet_length(root_text) BETWEEN 1 AND 7996
            AND char_length(root_text) <= 4000
            AND contents_capacity IS NULL)),
    ADD CONSTRAINT game_reward_claim_reserved_contents_shape
        CHECK (game_reward_claim_contents_plan_valid(contents_plan, contents_capacity));
ALTER TABLE game_reward_claim_mint_receipts
    ADD COLUMN contents_capacity SMALLINT,
    ADD COLUMN contents_plan JSONB NOT NULL DEFAULT '[]'::jsonb,
    ADD COLUMN root_text TEXT,
    ADD CONSTRAINT game_reward_claim_committed_root_text_shape
        CHECK (root_text IS NULL OR (octet_length(root_text) BETWEEN 1 AND 7996
            AND char_length(root_text) <= 4000
            AND contents_capacity IS NULL AND quantity = 1)),
    ADD CONSTRAINT game_reward_claim_committed_contents_shape
        CHECK (game_reward_claim_contents_plan_valid(contents_plan, contents_capacity));
CREATE INDEX game_reward_claim_contents_items
    ON game_reward_claim_mint_receipts USING GIN(contents_plan jsonb_path_ops)
    WHERE contents_capacity IS NOT NULL;

-- Proof of the entire plan, never merely the particular child whose trigger
-- happens to run first. The root's existing proof still checks its fence-bound
-- reservation, main-backpack placement, audit, claim and quest obligation.
CREATE FUNCTION game_reward_claim_container_complete(p_transaction UUID) RETURNS BOOLEAN
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE r game_reward_claim_mint_receipts%ROWTYPE;
        v game_reward_claim_mint_reservations%ROWTYPE;
        root_item game_item_instances%ROWTYPE;
        entry JSONB; child UUID; minted UUID; event UUID;
BEGIN
    SELECT * INTO r FROM game_reward_claim_mint_receipts
     WHERE transaction_id = p_transaction AND created_xact_id = pg_current_xact_id();
    IF NOT FOUND THEN RETURN FALSE; END IF;
    SELECT * INTO v FROM game_reward_claim_mint_reservations
     WHERE game_session_id = r.game_session_id AND command_id = r.command_id;
    IF NOT FOUND OR v.contents_plan IS DISTINCT FROM r.contents_plan
       OR v.contents_capacity IS DISTINCT FROM r.contents_capacity
       OR v.root_text IS DISTINCT FROM r.root_text
       OR (v.transaction_id,v.event_id,v.item_instance_id,v.intent_binding,v.character_id)
          IS DISTINCT FROM (r.transaction_id,r.event_id,r.item_instance_id,r.intent_binding,r.character_id)
       OR NOT game_reward_claim_contents_plan_valid(r.contents_plan,r.contents_capacity)
       OR (SELECT count(*) FROM game_item_nested_entries
            WHERE parent_item_instance_id = r.item_instance_id) <> jsonb_array_length(r.contents_plan)
    THEN RETURN FALSE; END IF;
    IF r.root_text IS NULL AND (
        EXISTS (SELECT 1 FROM game_item_texts WHERE item_instance_id = r.item_instance_id)
        OR EXISTS (SELECT 1 FROM game_item_text_revisions WHERE item_instance_id = r.item_instance_id))
       OR r.root_text IS NOT NULL AND NOT EXISTS (
        SELECT 1 FROM game_item_texts t JOIN game_item_text_revisions x USING(item_instance_id)
         WHERE t.item_instance_id = r.item_instance_id AND t.text = r.root_text
           AND t.writer_character_id IS NULL AND t.text_revision = 1 AND x.last_revision = 1
           AND t.written_at = to_timestamp(r.occurred_at::double precision/1000.0))
    THEN RETURN FALSE; END IF;
    IF r.contents_capacity IS NULL AND r.root_text IS NULL THEN RETURN TRUE; END IF;
    SELECT * INTO root_item FROM game_item_instances WHERE item_instance_id = r.item_instance_id;
    IF NOT FOUND OR root_item.quantity <> 1 OR r.quantity <> 1
       OR root_item.world_id <> v.world_id OR root_item.lifecycle <> 1
       OR root_item.state_revision <> 1 OR root_item.last_transaction_id IS NOT NULL
       OR root_item.minted_transaction_id <> r.transaction_id THEN RETURN FALSE; END IF;
    FOR entry IN SELECT value FROM jsonb_array_elements(r.contents_plan) LOOP
        child := (entry->>'item_instance_id')::UUID;
        minted := (entry->>'transaction_id')::UUID;
        event := (entry->>'event_id')::UUID;
        IF child IN (r.item_instance_id,r.transaction_id,r.event_id)
           OR minted IN (r.item_instance_id,r.transaction_id,r.event_id)
           OR event IN (r.item_instance_id,r.transaction_id,r.event_id)
           OR NOT EXISTS (
               SELECT 1 FROM game_item_instances i
                 JOIN game_item_nested_entries e USING(item_instance_id)
                 JOIN game_item_audit_outbox a ON a.item_instance_id = i.item_instance_id
                WHERE i.item_instance_id = child AND i.minted_transaction_id = minted
                  AND i.world_id = v.world_id AND i.lifecycle = 1 AND i.state_revision = 1
                  AND i.last_transaction_id IS NULL
                  AND (i.definition_family,i.definition_production_key,i.definition_revision_ref,i.quantity)
                      = (entry->>'family',entry->>'production_key',entry->>'revision_ref',
                         (entry->>'quantity')::BIGINT)
                  AND e.world_id = v.world_id AND e.parent_item_instance_id = r.item_instance_id
                  AND e.placement_ordinal = (entry->>'ordinal')::NUMERIC
                  AND e.placed_transaction_id = minted
                  AND a.event_id = event AND a.transaction_id = minted
                  AND a.transaction_ordinal = 1 AND a.transaction_count = 1
                  AND a.event_type_id = 2 AND a.occurred_at = r.occurred_at
                  AND a.envelope_sha256 = sha256(a.envelope)
                  AND a.created_xact_id = pg_current_xact_id()
                  AND a.publication_state = 1 AND a.published_at IS NULL)
           OR EXISTS (SELECT 1 FROM game_item_nested_entries WHERE parent_item_instance_id = child)
           OR ((entry->>'key_number')::INTEGER = 0
               AND EXISTS (SELECT 1 FROM game_item_keys WHERE item_instance_id = child))
           OR ((entry->>'key_number')::INTEGER <> 0 AND NOT EXISTS (
               SELECT 1 FROM game_item_keys k WHERE k.item_instance_id = child
                 AND k.key_number = (entry->>'key_number')::INTEGER
                 AND k.minted_transaction_id = minted AND k.created_xact_id = pg_current_xact_id()))
           OR (entry->'text' = 'null'::jsonb AND (
               EXISTS (SELECT 1 FROM game_item_texts WHERE item_instance_id = child)
               OR EXISTS (SELECT 1 FROM game_item_text_revisions WHERE item_instance_id = child)))
           OR (entry->'text' <> 'null'::jsonb AND NOT EXISTS (
               SELECT 1 FROM game_item_texts t JOIN game_item_text_revisions x USING(item_instance_id)
                WHERE t.item_instance_id = child AND t.text = entry->>'text'
                  AND t.writer_character_id IS NULL AND t.text_revision = 1 AND x.last_revision = 1
                  AND t.written_at = to_timestamp(r.occurred_at::double precision/1000.0)))
        THEN RETURN FALSE; END IF;
        PERFORM game_item_location_exclusive(child);
    END LOOP;
    RETURN TRUE;
END;
$$;

CREATE FUNCTION game_reward_claim_container_item_proven(p_item UUID, p_transaction UUID)
RETURNS BOOLEAN LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE root_transaction UUID;
BEGIN
    SELECT r.transaction_id INTO root_transaction FROM game_reward_claim_mint_receipts r
     WHERE r.contents_capacity IS NOT NULL AND r.created_xact_id = pg_current_xact_id()
       AND r.contents_plan @> jsonb_build_array(jsonb_build_object(
           'item_instance_id',p_item::TEXT,'transaction_id',p_transaction::TEXT));
    RETURN root_transaction IS NOT NULL AND game_reward_claim_container_complete(root_transaction);
END;
$$;

-- Preserve every installed owner arm, including nested kill loot, Grizzly,
-- craft, market, callbacks, hotkeys and pouch custody. Never copy an old body.
DO $migration$
DECLARE name TEXT; body TEXT; arm TEXT; insert_at INTEGER;
BEGIN
    FOREACH name IN ARRAY ARRAY['game_reward_claim_mint_consistency_guard',
        'game_item_mint_consistency_guard','game_item_nested_entry_proven','game_item_key_proven'] LOOP
        body := pg_get_functiondef((name || '()')::regprocedure);
        insert_at := regexp_instr(body,E'\\mBEGIN\\M',1,1,1,'i');
        IF insert_at = 0 THEN RAISE EXCEPTION 'unexpected reward container guard: %',name; END IF;
        IF name = 'game_reward_claim_mint_consistency_guard' THEN arm := $arm$
    IF NOT game_reward_claim_container_complete(NEW.transaction_id) THEN
        RAISE EXCEPTION 'reward container must commit its complete reserved child plan'
            USING ERRCODE = '23514';
    END IF;
$arm$;
        ELSIF name = 'game_item_nested_entry_proven' THEN arm := $arm$
    IF TG_OP = 'INSERT' AND game_reward_claim_container_item_proven(
        NEW.item_instance_id,NEW.placed_transaction_id) THEN RETURN NULL; END IF;
$arm$;
        ELSE arm := $arm$
    IF TG_OP = 'INSERT' AND game_reward_claim_container_item_proven(
        NEW.item_instance_id,NEW.minted_transaction_id) THEN RETURN NULL; END IF;
$arm$;
        END IF;
        EXECUTE overlay(body placing arm from insert_at for 0);
    END LOOP;
END
$migration$;

DO $$ DECLARE name TEXT; signature TEXT; BEGIN
    FOREACH signature IN ARRAY ARRAY['game_reward_claim_contents_plan_valid(JSONB,SMALLINT)',
        'game_reward_claim_container_complete(UUID)','game_reward_claim_container_item_proven(UUID,UUID)',
        'game_reward_claim_mint_consistency_guard()','game_item_mint_consistency_guard()',
        'game_item_nested_entry_proven()','game_item_key_proven()'] LOOP
        EXECUTE format('ALTER FUNCTION %s SET search_path = pg_catalog,%I,pg_temp',signature,current_schema());
    END LOOP;
END $$;
REVOKE ALL ON FUNCTION game_reward_claim_contents_plan_valid(JSONB,SMALLINT),
    game_reward_claim_container_complete(UUID),game_reward_claim_container_item_proven(UUID,UUID) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_reward_claim_contents_plan_valid(JSONB,SMALLINT),
    game_reward_claim_container_complete(UUID),game_reward_claim_container_item_proven(UUID,UUID)
    TO oteryn_game_runtime, oteryn_game_control;
