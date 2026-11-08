-- QMECH/qready2: ordinary sibling Items share the fenced frozen quest claim.
-- Preserve the existing container/fluid owners and every installed guard arm.
CREATE FUNCTION game_reward_claim_siblings_valid(p_plan JSONB, p_capacity SMALLINT)
RETURNS BOOLEAN LANGUAGE plpgsql IMMUTABLE AS $$
DECLARE part JSONB;
BEGIN
    IF NOT game_reward_claim_contents_plan_valid(p_plan,p_capacity) THEN RETURN FALSE; END IF;
    IF p_capacity IS NULL THEN RETURN p_plan='[]'::jsonb; END IF;
    IF jsonb_array_length(p_plan)>=p_capacity THEN RETURN FALSE; END IF;
    FOR part IN SELECT value FROM jsonb_array_elements(p_plan) LOOP
        -- Variable-fluid, numbered-key and authored-text siblings need their
        -- own metadata participant. Never accept and silently omit one here.
        IF part ? 'fluid_subtype' OR part->'text'<>'null'::jsonb
           OR (part->>'key_number')::INTEGER<>0 THEN RETURN FALSE; END IF;
    END LOOP;
    RETURN TRUE;
END $$;

ALTER TABLE game_reward_claim_mint_reservations
 ADD COLUMN additional_capacity SMALLINT,
 ADD COLUMN additional_plan JSONB NOT NULL DEFAULT '[]'::jsonb,
 ADD CONSTRAINT game_reward_claim_reserved_siblings CHECK (
  game_reward_claim_siblings_valid(additional_plan,additional_capacity));
ALTER TABLE game_reward_claim_mint_receipts
 ADD COLUMN additional_capacity SMALLINT,
 ADD COLUMN additional_plan JSONB NOT NULL DEFAULT '[]'::jsonb,
 ADD CONSTRAINT game_reward_claim_committed_siblings CHECK (
  game_reward_claim_siblings_valid(additional_plan,additional_capacity));
CREATE INDEX game_reward_claim_sibling_lookup
 ON game_reward_claim_mint_receipts USING GIN(additional_plan jsonb_path_ops)
 WHERE additional_capacity IS NOT NULL;

CREATE FUNCTION game_reward_claim_additional_complete(p_transaction UUID)
RETURNS BOOLEAN LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE receipt game_reward_claim_mint_receipts%ROWTYPE;
        reserved game_reward_claim_mint_reservations%ROWTYPE;
        part JSONB; item UUID; minted UUID; event UUID; ident UUID;
        used UUID[];
BEGIN
    SELECT * INTO receipt FROM game_reward_claim_mint_receipts
     WHERE transaction_id=p_transaction AND created_xact_id=pg_current_xact_id();
    IF NOT FOUND THEN RETURN FALSE; END IF;
    SELECT * INTO reserved FROM game_reward_claim_mint_reservations
     WHERE game_session_id=receipt.game_session_id AND command_id=receipt.command_id;
    IF NOT FOUND OR reserved.additional_capacity IS DISTINCT FROM receipt.additional_capacity
       OR reserved.additional_plan IS DISTINCT FROM receipt.additional_plan
       OR NOT game_reward_claim_siblings_valid(receipt.additional_plan,receipt.additional_capacity)
       OR NOT game_reward_claim_container_complete(p_transaction) THEN RETURN FALSE; END IF;
    IF receipt.additional_capacity IS NULL THEN RETURN TRUE; END IF;
    IF NOT EXISTS (SELECT 1 FROM game_item_container_slots
                   WHERE character_id=receipt.character_id
                     AND item_instance_id=receipt.destination_parent_item_instance_id)
       OR (SELECT count(*) FROM game_item_container_entries
            WHERE parent_item_instance_id=receipt.destination_parent_item_instance_id)
          > receipt.additional_capacity THEN RETURN FALSE; END IF;
    used:=ARRAY[receipt.transaction_id,receipt.event_id,receipt.item_instance_id];
    FOR part IN SELECT value FROM jsonb_array_elements(receipt.contents_plan) LOOP
        used:=used || ARRAY[(part->>'transaction_id')::UUID,(part->>'event_id')::UUID,
                           (part->>'item_instance_id')::UUID];
    END LOOP;
    FOR part IN SELECT value FROM jsonb_array_elements(receipt.additional_plan) LOOP
        item:=(part->>'item_instance_id')::UUID;
        minted:=(part->>'transaction_id')::UUID;
        event:=(part->>'event_id')::UUID;
        FOREACH ident IN ARRAY ARRAY[item,minted,event] LOOP
            IF ident=ANY(used) THEN RETURN FALSE; END IF;
            used:=array_append(used,ident);
        END LOOP;
        IF NOT EXISTS (
            SELECT 1 FROM game_item_instances i
            JOIN game_item_container_entries c USING(item_instance_id)
            JOIN game_item_audit_outbox a USING(item_instance_id)
            WHERE i.item_instance_id=item AND i.world_id=reserved.world_id
              AND i.lifecycle=1 AND i.state_revision=1 AND i.last_transaction_id IS NULL
              AND i.minted_transaction_id=minted
              AND (i.definition_family,i.definition_production_key,i.definition_revision_ref,i.quantity)
                  =(part->>'family',part->>'production_key',part->>'revision_ref',(part->>'quantity')::BIGINT)
              AND c.world_id=reserved.world_id AND c.character_id=receipt.character_id
              AND c.parent_item_instance_id=receipt.destination_parent_item_instance_id
              AND c.placement_ordinal=receipt.destination_ordinal+(part->>'ordinal')::NUMERIC
              AND c.placed_transaction_id=minted
              AND a.event_id=event AND a.transaction_id=minted
              AND a.transaction_ordinal=1 AND a.transaction_count=1
              AND a.event_type_id=2 AND a.occurred_at=receipt.occurred_at
              AND a.created_xact_id=pg_current_xact_id()
              AND a.envelope_sha256=sha256(a.envelope)
              AND a.publication_state=1 AND a.published_at IS NULL)
           OR EXISTS (SELECT 1 FROM game_item_nested_entries WHERE parent_item_instance_id=item)
           OR EXISTS (SELECT 1 FROM game_item_keys WHERE item_instance_id=item)
           OR EXISTS (SELECT 1 FROM game_item_texts WHERE item_instance_id=item)
           OR EXISTS (SELECT 1 FROM game_item_text_revisions WHERE item_instance_id=item)
           OR EXISTS (SELECT 1 FROM game_item_fluid_states WHERE item_instance_id=item)
        THEN RETURN FALSE; END IF;
        PERFORM game_item_location_exclusive(item);
    END LOOP;
    RETURN TRUE;
END $$;

CREATE FUNCTION game_reward_claim_additional_item_proven(p_item UUID,p_transaction UUID)
RETURNS BOOLEAN LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE parent UUID;
BEGIN
    SELECT transaction_id INTO parent FROM game_reward_claim_mint_receipts
     WHERE additional_capacity IS NOT NULL AND created_xact_id=pg_current_xact_id()
       AND additional_plan @> jsonb_build_array(jsonb_build_object(
           'item_instance_id',p_item::TEXT,'transaction_id',p_transaction::TEXT));
    RETURN parent IS NOT NULL AND game_reward_claim_additional_complete(parent);
END $$;

DO $migration$
DECLARE target TEXT; definition TEXT; position INTEGER; participant TEXT;
BEGIN
    FOREACH target IN ARRAY ARRAY['game_reward_claim_mint_consistency_guard',
      'game_item_mint_consistency_guard','game_item_placement_proven','game_quest_reward_complete'] LOOP
        definition:=pg_get_functiondef((target||'()')::regprocedure);
        position:=regexp_instr(definition,E'\\mBEGIN\\M',1,1,1,'i');
        IF position=0 THEN RAISE EXCEPTION 'missing sibling guard insertion point: %',target; END IF;
        IF target='game_quest_reward_complete' THEN participant:=$guard$
    IF NEW.mint_transaction_id IS NOT NULL AND NOT EXISTS (
        SELECT 1 FROM game_reward_claim_mint_receipts r
         WHERE r.transaction_id=NEW.mint_transaction_id
           AND jsonb_array_length(r.additional_plan)=jsonb_array_length(
               coalesce(NEW.plan #> '{give,additional}','[]'::JSONB))
           AND NOT EXISTS (
               SELECT 1 FROM jsonb_array_elements(r.additional_plan) actual
                WHERE NOT EXISTS (
                    SELECT 1 FROM jsonb_array_elements(coalesce(
                        NEW.plan #> '{give,additional}','[]'::JSONB))
                        WITH ORDINALITY expected(item,ordinal)
                     WHERE expected.ordinal=(actual->>'ordinal')::BIGINT
                       AND expected.item->>'item'=actual->>'production_key'
                       AND (expected.item->>'count')::BIGINT=(actual->>'quantity')::BIGINT))
           AND game_reward_claim_additional_complete(r.transaction_id)) THEN
        RAISE EXCEPTION 'quest reward lacks exact independent outputs' USING ERRCODE='23514';
    END IF;
$guard$;
        ELSIF target='game_reward_claim_mint_consistency_guard' THEN participant:=$guard$
    IF NOT game_reward_claim_additional_complete(NEW.transaction_id) THEN
        RAISE EXCEPTION 'QUEST_REWARD_SIBLINGS_INCOMPLETE' USING ERRCODE='23514';
    END IF;
$guard$;
        ELSIF target='game_item_placement_proven' THEN participant:=$guard$
    IF TG_TABLE_NAME='game_item_container_entries' AND TG_OP='INSERT' THEN
        IF game_reward_claim_additional_item_proven(NEW.item_instance_id,NEW.placed_transaction_id)
        THEN RETURN NULL; END IF;
    END IF;
$guard$;
        ELSE participant:=$guard$
    IF TG_OP='INSERT' AND game_reward_claim_additional_item_proven(
       NEW.item_instance_id,NEW.minted_transaction_id) THEN RETURN NULL; END IF;
$guard$;
        END IF;
        EXECUTE overlay(definition placing participant from position for 0);
    END LOOP;
    FOREACH target IN ARRAY ARRAY['game_reward_claim_siblings_valid(JSONB,SMALLINT)',
      'game_reward_claim_additional_complete(UUID)',
      'game_reward_claim_additional_item_proven(UUID,UUID)',
      'game_reward_claim_mint_consistency_guard()','game_item_mint_consistency_guard()',
      'game_item_placement_proven()','game_quest_reward_complete()'] LOOP
        EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',target,current_schema());
    END LOOP;
END $migration$;
REVOKE ALL ON FUNCTION game_reward_claim_siblings_valid(JSONB,SMALLINT),
 game_reward_claim_additional_complete(UUID),game_reward_claim_additional_item_proven(UUID,UUID) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_reward_claim_siblings_valid(JSONB,SMALLINT),
 game_reward_claim_additional_complete(UUID),game_reward_claim_additional_item_proven(UUID,UUID)
 TO oteryn_game_runtime,oteryn_game_control;
