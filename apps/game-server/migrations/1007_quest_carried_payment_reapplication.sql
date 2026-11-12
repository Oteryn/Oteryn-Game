-- Preserve the Atelier owner when its migrations are reapplied before the
-- later Quest carried-payment extension. Keep the Atelier-only receipt arm
-- limited to its own causes; Quest cause 18 has its independent source proof.
DO $migration$
DECLARE body TEXT; target TEXT;
 original TEXT := $old$OR NOT EXISTS(SELECT 1 FROM game_character_atelier_receipts a WHERE a.character_id=f.character_id AND a.occurrence=f.cause_occurrence_id
   AND a.committed_character_revision=f.committed_character_revision AND a.fee_transaction_id=f.transaction_id AND a.created_xact_id=pg_current_xact_id())$old$;
BEGIN
 SELECT pg_get_functiondef('game_item_fee_burn_consistency_guard()'::regprocedure) INTO STRICT body;
 IF position('game_quest_reward_fees' IN body)=0 THEN
  RAISE EXCEPTION 'Quest carried payment requires its existing source proof';
 END IF;
 EXECUTE replace(body,
  'NEW.cause_kind IN (9,10,11,18) AND EXISTS(SELECT 1 FROM game_character_atelier_receipts',
  'NEW.cause_kind IN (9,10,11) AND EXISTS(SELECT 1 FROM game_character_atelier_receipts');
 FOREACH target IN ARRAY ARRAY['game_atelier_coin_admit()','game_atelier_coin_complete(uuid)'] LOOP
  SELECT pg_get_functiondef(target::regprocedure) INTO STRICT body;
  IF position('IN (9,10,11,18)' IN body)=0 THEN
   IF position('IN (9,10,11)' IN body)=0 THEN
    RAISE EXCEPTION 'Quest carried payment requires complete coin proof: %',target;
   END IF;
   EXECUTE replace(body,'IN (9,10,11)','IN (9,10,11,18)');
  END IF;
 END LOOP;
 SELECT pg_get_functiondef('game_atelier_coin_complete(uuid)'::regprocedure) INTO STRICT body;
 IF position('game_quest_reward_fees' IN body)=0 THEN
  IF position(original IN body)=0 THEN
   RAISE EXCEPTION 'Quest carried payment requires original Atelier receipt proof';
  END IF;
  EXECUTE replace(body,original,$new$OR NOT (
  EXISTS(SELECT 1 FROM game_character_atelier_receipts a WHERE a.character_id=f.character_id AND a.occurrence=f.cause_occurrence_id
   AND a.committed_character_revision=f.committed_character_revision AND a.fee_transaction_id=f.transaction_id AND a.created_xact_id=pg_current_xact_id())
  OR (f.cause_kind=18 AND EXISTS(SELECT 1 FROM game_quest_reward_fees q
   JOIN game_quest_reward_sources s USING(character_id,cause_id,cause_ordinal,transition_key)
   WHERE q.character_id=f.character_id AND q.fee_occurrence=f.cause_occurrence_id
    AND q.committed_character_revision=f.committed_character_revision AND q.transition_key=f.purchase_service_key
    AND q.fee_gold_units=f.fee_gold_units AND (q.include_bank OR f.bank_debit_gold_units=0)
    AND q.created_xact_id=pg_current_xact_id() AND s.created_xact_id=pg_current_xact_id()))
 )$new$);
 END IF;
END $migration$;
