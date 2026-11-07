-- Retain the atomic payment proof after later reward-completion replacements.
-- Clock effects reuse the original Quest receipt and database transaction time.
CREATE FUNCTION game_quest_reward_clocks_proven(p_character UUID,p_cause UUID,p_ordinal NUMERIC,p_transition TEXT,p_plan JSONB)
RETURNS void LANGUAGE plpgsql AS $$
DECLARE clock JSONB; receipt game_character_quest_receipts%ROWTYPE; slot INTEGER; seconds BIGINT; set_now BOOLEAN;
BEGIN
 SELECT * INTO STRICT receipt FROM game_character_quest_receipts r
  WHERE r.character_id=p_character AND r.cause_id=p_cause AND r.cause_ordinal=p_ordinal
   AND r.transition_key=p_transition AND r.created_xact_id=pg_current_xact_id();
 FOR clock IN SELECT value FROM jsonb_array_elements(coalesce(p_plan->'clocks','[]'::jsonb))
  UNION ALL SELECT jsonb_build_object('track',p_plan#>>'{cooldown,track}',
   'elapsed_seconds',p_plan#>'{cooldown,seconds}','set_now',true) WHERE p_plan ? 'cooldown'
 LOOP
  slot:=array_position(receipt.track_keys,clock->>'track');
  seconds:=(clock->>'elapsed_seconds')::bigint;
  set_now:=(clock->>'set_now')::boolean;
  IF slot IS NULL OR set_now IS NULL
   OR (seconds IS NOT NULL AND (seconds NOT BETWEEN 1 AND 31536000
    OR floor(extract(epoch FROM now()))-receipt.values_before[slot]::numeric<seconds))
   OR (set_now AND receipt.values_after[slot] IS DISTINCT FROM floor(extract(epoch FROM now()))::bigint)
   OR (NOT set_now AND (seconds IS NULL OR receipt.values_after[slot] IS DISTINCT FROM receipt.values_before[slot])) THEN
   RAISE EXCEPTION 'quest clock lacks exact database-time receipt effect' USING ERRCODE='23514';
  END IF;
 END LOOP;
END $$;
REVOKE ALL ON FUNCTION game_quest_reward_clocks_proven(UUID,UUID,NUMERIC,TEXT,JSONB) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_quest_reward_clocks_proven(UUID,UUID,NUMERIC,TEXT,JSONB) TO oteryn_game_runtime;

DO $migration$
DECLARE body TEXT; original TEXT;
BEGIN
 body:=pg_get_functiondef('game_quest_reward_complete()'::regprocedure);
 original:='jsonb_array_length(NEW.plan->''take'')=0';
 IF position(original IN body)=0 OR regexp_count(body,'RETURN NULL;')<>1 THEN
  RAISE EXCEPTION 'missing bounded repeat/completion predecessor';
 END IF;
 body:=replace(body,original,
  '(jsonb_array_length(NEW.plan->''take'')=0 AND NEW.plan->''fee'' IS NULL AND NEW.plan->''cooldown'' IS NULL)');
 EXECUTE replace(body,'RETURN NULL;',$proof$
 IF NEW.plan ? 'fee' AND NOT EXISTS(SELECT 1 FROM game_quest_reward_fees q
  WHERE q.character_id=NEW.character_id AND q.cause_id=NEW.cause_id
   AND q.cause_ordinal=NEW.cause_ordinal AND q.transition_key=NEW.transition_key
   AND q.created_xact_id=pg_current_xact_id()) THEN
  RAISE EXCEPTION 'paid quest reward lacks its atomic fee' USING ERRCODE='23514';
 END IF;
 PERFORM game_quest_reward_clocks_proven(NEW.character_id,NEW.cause_id,NEW.cause_ordinal,NEW.transition_key,NEW.plan);
 RETURN NULL;
 $proof$);
END $migration$;
