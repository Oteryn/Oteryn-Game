-- Counted services reuse the same MINT once-owner with an exact command-scoped claim.
-- Earlier quest rewards retain their original claim identity and every old proof.
DO $migration$ DECLARE body TEXT; original TEXT; replacement TEXT; BEGIN
 -- Extend the adopted main helper; its paid-exchange hash and fixed claims remain exact.
 SELECT pg_get_functiondef('game_quest_reward_mint_claim_key(uuid,uuid,numeric,text,jsonb)'::regprocedure) INTO STRICT body;
 original:=$old$SELECT CASE WHEN jsonb_array_length(v_plan->'storage')=0$old$;
 replacement:=$new$SELECT CASE WHEN v_plan ? 'counter' THEN
     v_transition || '/occurrence/' || replace(v_cause::text,'-','') || '/' || v_ordinal::text
     WHEN jsonb_array_length(v_plan->'storage')=0$new$;
 IF (length(body)-length(replace(body,original,'')))/length(original)<>1 THEN
  RAISE EXCEPTION 'repeatable quest exchange requires the exact main claim-key helper';
 END IF;
 EXECUTE replace(body,original,replacement);
 SELECT pg_get_functiondef('game_quest_reward_complete()'::regprocedure) INTO STRICT body;
 EXECUTE replace(body,'RETURN NULL;', $proof$
  IF NEW.plan ? 'counter' AND NOT EXISTS(
   SELECT 1 FROM game_character_quest_receipts r
   CROSS JOIN LATERAL generate_subscripts(r.track_keys,1) counter_slot(slot_index)
   WHERE r.character_id=NEW.character_id AND r.cause_id=NEW.cause_id
    AND r.cause_ordinal=NEW.cause_ordinal AND r.transition_key=NEW.transition_key
    AND r.cause_kind='command' AND r.quest_key=NEW.plan->>'quest'
    AND r.created_xact_id=pg_current_xact_id() AND NOT r.completes
    AND r.track_keys[counter_slot.slot_index]=NEW.plan #>> '{counter,track}'
    AND r.values_before[counter_slot.slot_index]>=0
    AND r.values_after[counter_slot.slot_index]=r.values_before[counter_slot.slot_index]+1
    AND r.values_after[counter_slot.slot_index]<=(NEW.plan #>> '{counter,maximum}')::bigint
    AND (NEW.plan #>> '{counter,maximum}')::bigint BETWEEN 1 AND 1000000000000
  ) THEN RAISE EXCEPTION 'repeatable quest exchange lacks its exact bounded same-transaction counter receipt' USING ERRCODE='23514'; END IF;
  RETURN NULL;
 $proof$);
END $migration$;
