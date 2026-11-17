-- Additional top-level rewards share the original claim, frozen intent and physical commit.
-- The existing filled-container proof and every other installed owner remain unchanged.
ALTER TABLE game_reward_claim_mint_reservations
 ADD COLUMN additional_capacity SMALLINT,
 ADD COLUMN additional_plan JSONB NOT NULL DEFAULT '[]'::jsonb,
 ADD CONSTRAINT game_reward_claim_reserved_additional_shape CHECK (
  game_reward_claim_contents_plan_valid(additional_plan,additional_capacity)
  AND (additional_capacity IS NULL OR jsonb_array_length(additional_plan)<additional_capacity));
ALTER TABLE game_reward_claim_mint_receipts
 ADD COLUMN additional_capacity SMALLINT,
 ADD COLUMN additional_plan JSONB NOT NULL DEFAULT '[]'::jsonb,
 ADD CONSTRAINT game_reward_claim_committed_additional_shape CHECK (
  game_reward_claim_contents_plan_valid(additional_plan,additional_capacity)
  AND (additional_capacity IS NULL OR jsonb_array_length(additional_plan)<additional_capacity));
CREATE INDEX game_reward_claim_additional_items
 ON game_reward_claim_mint_receipts USING GIN(additional_plan jsonb_path_ops)
 WHERE additional_capacity IS NOT NULL;

CREATE FUNCTION game_reward_claim_additional_complete(p_transaction UUID) RETURNS BOOLEAN
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE r game_reward_claim_mint_receipts%ROWTYPE;
 v game_reward_claim_mint_reservations%ROWTYPE;
 entry JSONB; child UUID; minted UUID; event UUID;
 ids UUID[]; ident UUID;
BEGIN
 SELECT * INTO r FROM game_reward_claim_mint_receipts
  WHERE transaction_id=p_transaction AND created_xact_id=pg_current_xact_id();
 IF NOT FOUND THEN RETURN FALSE; END IF;
 SELECT * INTO v FROM game_reward_claim_mint_reservations
  WHERE game_session_id=r.game_session_id AND command_id=r.command_id;
 IF NOT FOUND OR v.additional_plan IS DISTINCT FROM r.additional_plan
  OR v.additional_capacity IS DISTINCT FROM r.additional_capacity
  OR NOT game_reward_claim_contents_plan_valid(r.additional_plan,r.additional_capacity)
  OR NOT game_reward_claim_container_complete(p_transaction)
 THEN RETURN FALSE; END IF;
 IF r.additional_capacity IS NULL THEN RETURN r.additional_plan='[]'::jsonb; END IF;
 IF jsonb_array_length(r.additional_plan)>=r.additional_capacity
  OR (SELECT count(*) FROM game_item_container_entries
       WHERE parent_item_instance_id=r.destination_parent_item_instance_id)>r.additional_capacity
  OR NOT EXISTS(SELECT 1 FROM game_item_container_slots
       WHERE character_id=r.character_id AND item_instance_id=r.destination_parent_item_instance_id)
 THEN RETURN FALSE; END IF;
 ids:=ARRAY[r.transaction_id,r.event_id,r.item_instance_id];
 FOR entry IN SELECT value FROM jsonb_array_elements(r.contents_plan) LOOP
  ids:=ids || ARRAY[(entry->>'transaction_id')::UUID,(entry->>'event_id')::UUID,
                   (entry->>'item_instance_id')::UUID];
 END LOOP;
 FOR entry IN SELECT value FROM jsonb_array_elements(r.additional_plan) LOOP
  child:=(entry->>'item_instance_id')::UUID;
  minted:=(entry->>'transaction_id')::UUID;
  event:=(entry->>'event_id')::UUID;
  FOREACH ident IN ARRAY ARRAY[child,minted,event] LOOP
   IF ident=ANY(ids) THEN RETURN FALSE; END IF;
   ids:=array_append(ids,ident);
  END LOOP;
  IF NOT EXISTS(
    SELECT 1 FROM game_item_instances i
     JOIN game_item_container_entries e USING(item_instance_id)
     JOIN game_item_audit_outbox a ON a.item_instance_id=i.item_instance_id
    WHERE i.item_instance_id=child AND i.minted_transaction_id=minted
     AND i.world_id=v.world_id AND i.lifecycle=1 AND i.state_revision=1
     AND i.last_transaction_id IS NULL
     AND (i.definition_family,i.definition_production_key,i.definition_revision_ref,i.quantity)=
      (entry->>'family',entry->>'production_key',entry->>'revision_ref',(entry->>'quantity')::BIGINT)
     AND e.world_id=v.world_id AND e.character_id=r.character_id
     AND e.parent_item_instance_id=r.destination_parent_item_instance_id
     AND e.placement_ordinal=r.destination_ordinal+(entry->>'ordinal')::NUMERIC
     AND e.placed_transaction_id=minted
     AND a.event_id=event AND a.transaction_id=minted
     AND a.transaction_ordinal=1 AND a.transaction_count=1
     AND a.event_type_id=2 AND a.occurred_at=r.occurred_at
     AND a.envelope_sha256=sha256(a.envelope)
     AND a.created_xact_id=pg_current_xact_id()
     AND a.publication_state=1 AND a.published_at IS NULL)
   OR EXISTS(SELECT 1 FROM game_item_nested_entries WHERE parent_item_instance_id=child)
   OR ((entry->>'key_number')::INTEGER=0
       AND EXISTS(SELECT 1 FROM game_item_keys WHERE item_instance_id=child))
   OR ((entry->>'key_number')::INTEGER<>0 AND NOT EXISTS(
       SELECT 1 FROM game_item_keys WHERE item_instance_id=child
        AND key_number=(entry->>'key_number')::INTEGER
        AND minted_transaction_id=minted AND created_xact_id=pg_current_xact_id()))
   OR (entry->'text'='null'::jsonb AND (
       EXISTS(SELECT 1 FROM game_item_texts WHERE item_instance_id=child)
       OR EXISTS(SELECT 1 FROM game_item_text_revisions WHERE item_instance_id=child)))
   OR (entry->'text'<>'null'::jsonb AND NOT EXISTS(
       SELECT 1 FROM game_item_texts t JOIN game_item_text_revisions x USING(item_instance_id)
       WHERE t.item_instance_id=child AND t.text=entry->>'text'
        AND t.writer_character_id IS NULL AND t.text_revision=1 AND x.last_revision=1
        AND t.written_at=to_timestamp(r.occurred_at::double precision/1000.0)))
  THEN RETURN FALSE; END IF;
  PERFORM game_item_location_exclusive(child);
 END LOOP;
 RETURN TRUE;
END $$;

CREATE FUNCTION game_reward_claim_additional_item_proven(p_item UUID,p_transaction UUID)
RETURNS BOOLEAN LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE root_transaction UUID;
BEGIN
 SELECT r.transaction_id INTO root_transaction FROM game_reward_claim_mint_receipts r
  WHERE r.additional_capacity IS NOT NULL AND r.created_xact_id=pg_current_xact_id()
   AND r.additional_plan @> jsonb_build_array(jsonb_build_object(
    'item_instance_id',p_item::TEXT,'transaction_id',p_transaction::TEXT));
 RETURN root_transaction IS NOT NULL AND game_reward_claim_additional_complete(root_transaction);
END $$;

-- Add only the new complete-plan arm; retain every existing owner body.
DO $migration$
DECLARE name TEXT; body TEXT; arm TEXT; insert_at INTEGER;
BEGIN
 FOREACH name IN ARRAY ARRAY['game_reward_claim_mint_consistency_guard',
  'game_item_mint_consistency_guard','game_item_placement_proven','game_item_key_proven'] LOOP
  body:=pg_get_functiondef((name||'()')::regprocedure);
  insert_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,1,'i');
  IF insert_at=0 THEN RAISE EXCEPTION 'unexpected additional reward guard: %',name; END IF;
  IF name='game_reward_claim_mint_consistency_guard' THEN arm:=$arm$
   IF NOT game_reward_claim_additional_complete(NEW.transaction_id) THEN
    RAISE EXCEPTION 'reward claim must commit every reserved top-level item' USING ERRCODE='23514';
   END IF;
$arm$;
  ELSIF name='game_item_placement_proven' THEN arm:=$arm$
   IF TG_TABLE_NAME='game_item_container_entries' THEN
    IF TG_OP='INSERT' AND game_reward_claim_additional_item_proven(
      NEW.item_instance_id,NEW.placed_transaction_id) THEN RETURN NULL; END IF;
   END IF;
$arm$;
  ELSE arm:=$arm$
   IF TG_OP='INSERT' AND game_reward_claim_additional_item_proven(
     NEW.item_instance_id,NEW.minted_transaction_id) THEN RETURN NULL; END IF;
$arm$;
  END IF;
  EXECUTE overlay(body placing arm from insert_at for 0);
 END LOOP;
END $migration$;
DO $$ DECLARE signature TEXT; BEGIN
 FOREACH signature IN ARRAY ARRAY['game_reward_claim_additional_complete(UUID)',
  'game_reward_claim_additional_item_proven(UUID,UUID)',
  'game_reward_claim_mint_consistency_guard()','game_item_mint_consistency_guard()',
  'game_item_placement_proven()','game_item_key_proven()'] LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',signature,current_schema());
 END LOOP;
END $$;
REVOKE ALL ON FUNCTION game_reward_claim_additional_complete(UUID),
 game_reward_claim_additional_item_proven(UUID,UUID) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_reward_claim_additional_complete(UUID),
 game_reward_claim_additional_item_proven(UUID,UUID) TO oteryn_game_runtime,oteryn_game_control;

-- Retain the native clock occurrence alongside main's existing paid/NPC proof.
CREATE OR REPLACE FUNCTION game_quest_reward_mint_claim_key(v_character UUID,v_cause UUID,v_ordinal NUMERIC,
    v_transition TEXT,v_plan JSONB) RETURNS TEXT LANGUAGE SQL IMMUTABLE AS $function$
    SELECT CASE WHEN v_plan->'per_occurrence_mint'='true'::jsonb OR (
        v_plan #>> '{world_callback,trigger}'='use'
        AND v_plan #>> '{world_callback,key}'=v_transition
        AND v_plan #> '{world_callback,state_operations}'=v_plan->'state_ops'
        AND EXISTS(SELECT 1 FROM jsonb_array_elements(coalesce(v_plan->'state_ops','[]'::jsonb)) operation
            WHERE operation #>> '{from,kind}'='elapsed_at_least'
                AND (operation #>> '{from,seconds}')::numeric BETWEEN 1 AND 31622400
                AND operation->'effect'='{"kind":"set_now"}'::jsonb)
    ) OR (
        jsonb_array_length(v_plan->'storage')=0
        AND jsonb_array_length(coalesce(v_plan->'state_ops','[]'::jsonb))=0
        AND jsonb_array_length(coalesce(v_plan->'participants','[]'::jsonb))=0
        AND jsonb_array_length(v_plan->'take')>0 AND jsonb_typeof(v_plan->'give')='object')
    THEN 'oteryn:quest-exchange/'||encode(sha256(
        convert_to('oteryn:quest-exchange:v1','UTF8')||decode('00','hex')
        ||uuid_send(v_character)||uuid_send(v_cause)
        ||convert_to(v_ordinal::text||':'||v_transition,'UTF8')),'hex')
    ELSE v_transition END
$function$;
DO $migration$
BEGIN
    EXECUTE format('ALTER FUNCTION game_quest_reward_mint_claim_key(UUID,UUID,NUMERIC,TEXT,JSONB) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $migration$;
REVOKE ALL ON FUNCTION game_quest_reward_mint_claim_key(UUID,UUID,NUMERIC,TEXT,JSONB) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_quest_reward_mint_claim_key(UUID,UUID,NUMERIC,TEXT,JSONB) TO oteryn_game_runtime;
