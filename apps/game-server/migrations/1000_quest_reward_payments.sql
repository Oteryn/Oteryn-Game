-- Exact Quest fees compose existing fee, Quest receipt and item/appearance owners.
CREATE TABLE game_quest_reward_fees (
 character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
 fee_occurrence UUID NOT NULL UNIQUE,
 cause_id UUID NOT NULL,
 cause_ordinal NUMERIC(20,0) NOT NULL,
 transition_key TEXT NOT NULL,
 fee_gold_units BIGINT NOT NULL CHECK(fee_gold_units BETWEEN 1 AND 1000000000000),
 include_bank BOOLEAN NOT NULL,
 committed_character_revision NUMERIC(20,0) NOT NULL,
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
 PRIMARY KEY(character_id,cause_id,cause_ordinal,transition_key),
 FOREIGN KEY(character_id,cause_id,cause_ordinal,transition_key)
  REFERENCES game_quest_reward_sources(character_id,cause_id,cause_ordinal,transition_key)
  DEFERRABLE INITIALLY DEFERRED
);
CREATE FUNCTION game_quest_reward_fee_complete() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NEW.created_xact_id<>pg_current_xact_id() OR NOT EXISTS(
  SELECT 1 FROM game_quest_reward_sources s JOIN game_item_fee_burns f
   ON f.cause_kind=18 AND f.cause_occurrence_id=NEW.fee_occurrence
    AND f.character_id=NEW.character_id
  WHERE s.character_id=NEW.character_id AND s.cause_id=NEW.cause_id
   AND s.cause_ordinal=NEW.cause_ordinal AND s.transition_key=NEW.transition_key
   AND s.created_xact_id=pg_current_xact_id() AND f.created_xact_id=pg_current_xact_id()
   AND f.purchase_service_key=NEW.transition_key AND f.fee_gold_units=NEW.fee_gold_units
   AND f.committed_character_revision=NEW.committed_character_revision
   AND (NEW.include_bank OR f.bank_debit_gold_units=0)
   AND s.plan->'fee'->>'gold'=NEW.fee_gold_units::text
   AND s.plan->'fee'->>'include_bank'=NEW.include_bank::text
 ) THEN RAISE EXCEPTION 'quest payment lacks its exact same-transaction source and fee' USING ERRCODE='23514'; END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_quest_reward_fee_complete AFTER INSERT ON game_quest_reward_fees
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_quest_reward_fee_complete();
CREATE TRIGGER game_quest_reward_fees_immutable BEFORE UPDATE OR DELETE ON game_quest_reward_fees
 FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_quest_reward_fees_no_truncate BEFORE TRUNCATE ON game_quest_reward_fees
 EXECUTE FUNCTION game_item_reject_truncate();
DO $migration$ DECLARE body TEXT; BEGIN
 SELECT pg_get_constraintdef(oid) INTO STRICT body FROM pg_constraint
 WHERE conrelid='game_item_fee_burns'::regclass AND conname='game_item_fee_burns_cause_kind_check';
 ALTER TABLE game_item_fee_burns DROP CONSTRAINT game_item_fee_burns_cause_kind_check;
 EXECUTE 'ALTER TABLE game_item_fee_burns ADD CONSTRAINT game_item_fee_burns_cause_kind_check CHECK (('||substr(body,7)||') OR cause_kind=18)';
 SELECT regexp_replace(pg_get_constraintdef(oid),' NOT VALID$','') INTO STRICT body FROM pg_constraint
 WHERE conrelid='game_item_fee_burns'::regclass AND conname='game_item_fee_burns_purchase_cause';
 ALTER TABLE game_item_fee_burns DROP CONSTRAINT game_item_fee_burns_purchase_cause;
 EXECUTE 'ALTER TABLE game_item_fee_burns ADD CONSTRAINT game_item_fee_burns_purchase_cause CHECK (('
  ||substr(body,7)||') OR (cause_kind=18 AND charm_key IS NULL AND purchase_service_key LIKE ''oteryn:quest-reward/%'' '
  ||'AND fee_gold_units BETWEEN 1 AND 1000000000000)) NOT VALID';
 SELECT pg_get_functiondef('game_item_fee_burn_consistency_guard()'::regprocedure) INTO STRICT body;
 IF position('(NEW.cause_kind=5 AND EXISTS' IN body)=0 THEN RAISE EXCEPTION 'quest payment requires fee source-proof anchor'; END IF;
 EXECUTE replace(body,'(NEW.cause_kind=5 AND EXISTS',
  '(NEW.cause_kind=18 AND EXISTS(SELECT 1 FROM game_quest_reward_fees q WHERE q.character_id=NEW.character_id '
  ||'AND q.fee_occurrence=NEW.cause_occurrence_id AND q.transition_key=NEW.purchase_service_key '
  ||'AND q.fee_gold_units=NEW.fee_gold_units AND q.committed_character_revision=NEW.committed_character_revision '
  ||'AND (q.include_bank OR NEW.bank_debit_gold_units=0) AND q.created_xact_id=pg_current_xact_id())) OR (NEW.cause_kind=5 AND EXISTS');
 SELECT pg_get_functiondef('game_quest_reward_complete()'::regprocedure) INTO STRICT body;
 IF position('RETURN NULL;' IN body)=0 THEN RAISE EXCEPTION 'quest payment requires source completion anchor'; END IF;
 EXECUTE replace(body,'RETURN NULL;', $proof$
  IF NEW.plan ? 'fee' AND NOT EXISTS(SELECT 1 FROM game_quest_reward_fees q
   WHERE q.character_id=NEW.character_id AND q.cause_id=NEW.cause_id
    AND q.cause_ordinal=NEW.cause_ordinal AND q.transition_key=NEW.transition_key
    AND q.created_xact_id=pg_current_xact_id()) THEN
   RAISE EXCEPTION 'paid quest reward lacks its atomic fee' USING ERRCODE='23514';
  END IF;
  RETURN NULL;
 $proof$);
END $migration$;
REVOKE ALL ON game_quest_reward_fees FROM PUBLIC;
GRANT SELECT,INSERT ON game_quest_reward_fees TO oteryn_game_runtime;
GRANT SELECT ON game_quest_reward_fees TO oteryn_game_control;
-- Reuse the complete carried-coin proof profile for the new source-bound cause.
-- Only the cause set changes; all custody/ordering/conservation/epoch checks stay intact.
DO $carried$ DECLARE body TEXT; target TEXT; rule RECORD; BEGIN
 FOREACH target IN ARRAY ARRAY['game_atelier_coin_admit()','game_atelier_coin_complete(uuid)','game_item_fee_burn_consistency_guard()'] LOOP
  SELECT pg_get_functiondef(target::regprocedure) INTO STRICT body;
  IF position('IN (9,10,11)' IN body)=0 THEN RAISE EXCEPTION 'quest payment requires complete carried proof anchor: %',target; END IF;
  EXECUTE replace(body,'IN (9,10,11)','IN (9,10,11,18)');
 END LOOP;
 FOR rule IN SELECT conname,pg_get_constraintdef(oid) AS definition FROM pg_constraint
  WHERE conrelid='game_item_fee_burns'::regclass AND contype='c' LOOP
  IF position('ARRAY[9, 10, 11]' IN rule.definition)>0 THEN
   EXECUTE format('ALTER TABLE game_item_fee_burns DROP CONSTRAINT %I',rule.conname);
   EXECUTE format('ALTER TABLE game_item_fee_burns ADD CONSTRAINT %I %s',rule.conname,
    replace(rule.definition,'ARRAY[9, 10, 11]','ARRAY[9, 10, 11, 18]'));
  END IF;
 END LOOP;
END $carried$;
-- A Quest payment must prove both complete physical custody and its source receipt.
-- The legacy carried wrapper returns early; require the Quest source before that return.
DO $quest_source$ DECLARE body TEXT; BEGIN
 SELECT pg_get_functiondef('game_item_fee_burn_consistency_guard()'::regprocedure) INTO STRICT body;
 IF position('BEGIN' IN body)=0 THEN RAISE EXCEPTION 'quest payment requires consistency body anchor'; END IF;
 EXECUTE regexp_replace(body,'BEGIN',$guard$BEGIN
 IF NEW.cause_kind=18 THEN
  IF NOT EXISTS(SELECT 1 FROM game_quest_reward_fees q WHERE q.character_id=NEW.character_id
   AND q.fee_occurrence=NEW.cause_occurrence_id AND q.transition_key=NEW.purchase_service_key
   AND q.fee_gold_units=NEW.fee_gold_units AND q.committed_character_revision=NEW.committed_character_revision
   AND (q.include_bank OR NEW.bank_debit_gold_units=0) AND q.created_xact_id=pg_current_xact_id()) THEN
   RAISE EXCEPTION 'quest payment lacks its atomic source proof' USING ERRCODE='23514';
  END IF;
  PERFORM game_atelier_coin_complete(NEW.transaction_id);
  RETURN NULL;
 END IF;
$guard$);
END $quest_source$;
-- Preserve the Atelier receipt proof; add only the matching Quest receipt alternative.
DO $receipt$ DECLARE body TEXT; original TEXT := $old$OR NOT EXISTS(SELECT 1 FROM game_character_atelier_receipts a WHERE a.character_id=f.character_id AND a.occurrence=f.cause_occurrence_id
   AND a.committed_character_revision=f.committed_character_revision AND a.fee_transaction_id=f.transaction_id AND a.created_xact_id=pg_current_xact_id())$old$;
BEGIN
 SELECT pg_get_functiondef('game_atelier_coin_complete(uuid)'::regprocedure) INTO STRICT body;
 IF position(original IN body)=0 THEN RAISE EXCEPTION 'quest payment requires original Character receipt anchor'; END IF;
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
END $receipt$;
