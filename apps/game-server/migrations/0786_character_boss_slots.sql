-- WB2/P02. Crystal io_bosstiary.cpp:225-232,311-315; protocolgame.cpp:10615-10639;
-- game.cpp:11062-11094; login.lua:179-181. Two personal slots, reset-epoch removal
-- prices and UI action cooldown. Existing daily boosted slot remains independent.
CREATE TABLE game_character_boss_slots (
 character_id UUID PRIMARY KEY REFERENCES game_character_roots(character_id),
 state JSONB NOT NULL CHECK(jsonb_typeof(state)='object' AND jsonb_array_length(state->'bosses')=2
  AND (state->>'removals')::int BETWEEN 1 AND 255 AND (state->>'ready')::numeric>=0)
);
CREATE TABLE game_character_boss_slot_operations (
 character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
 occurrence_id UUID NOT NULL CHECK(game_character_is_uuid_v7(occurrence_id)),
 ordinal BIGINT NOT NULL CHECK(ordinal>=1),
 request BYTEA NOT NULL CHECK(octet_length(request) BETWEEN 1 AND 130),
 reply BYTEA NOT NULL CHECK(octet_length(reply) BETWEEN 26 AND 284),
 state_before JSONB,
 state_after JSONB NOT NULL,
 fee_gold BIGINT NOT NULL CHECK(fee_gold>=0),
 fee_transaction_id UUID REFERENCES game_item_fee_burns(transaction_id) DEFERRABLE INITIALLY DEFERRED,
 occurred_at_ms BIGINT NOT NULL DEFAULT floor(extract(epoch FROM clock_timestamp())*1000)::bigint,
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
 PRIMARY KEY(character_id,occurrence_id), UNIQUE(character_id,ordinal),
 CHECK((fee_gold=0)=(fee_transaction_id IS NULL))
);
CREATE INDEX game_boss_slot_operations_fee_idx ON game_character_boss_slot_operations(fee_transaction_id);
REVOKE ALL ON game_character_boss_slots,game_character_boss_slot_operations FROM PUBLIC;
GRANT SELECT,INSERT,UPDATE ON game_character_boss_slots TO oteryn_game_runtime;
GRANT SELECT,INSERT ON game_character_boss_slot_operations TO oteryn_game_runtime;
GRANT SELECT ON game_character_boss_slots,game_character_boss_slot_operations TO oteryn_game_control;
CREATE TRIGGER game_boss_slot_operations_immutable BEFORE UPDATE OR DELETE ON game_character_boss_slot_operations
 FOR EACH ROW EXECUTE FUNCTION game_character_immutable();
CREATE TRIGGER game_boss_slot_operations_no_truncate BEFORE TRUNCATE ON game_character_boss_slot_operations
 EXECUTE FUNCTION game_character_reject_truncate();
CREATE TRIGGER game_boss_slots_no_truncate BEFORE TRUNCATE ON game_character_boss_slots
 EXECUTE FUNCTION game_character_reject_truncate();
CREATE FUNCTION game_boss_slots_proven() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE op game_character_boss_slot_operations%ROWTYPE; previous JSONB; counter INT; expected_fee BIGINT;
BEGIN
 SELECT * INTO op FROM game_character_boss_slot_operations WHERE character_id=NEW.character_id ORDER BY ordinal DESC LIMIT 1;
 IF op.character_id IS NULL OR op.created_xact_id<>pg_current_xact_id()
 OR (SELECT state FROM game_character_boss_slots WHERE character_id=NEW.character_id) IS DISTINCT FROM op.state_after THEN
  RAISE EXCEPTION 'Boss slots need their current operation' USING ERRCODE='23514'; END IF;
 IF TG_TABLE_NAME='game_character_boss_slot_operations' THEN
  SELECT state_after INTO previous FROM game_character_boss_slot_operations WHERE character_id=NEW.character_id AND ordinal=NEW.ordinal-1;
  counter:=CASE WHEN NEW.state_before->>'epoch'=NEW.state_after->>'epoch' THEN coalesce((NEW.state_before->>'removals')::int,1) ELSE 1 END;
  expected_fee:=CASE WHEN get_byte(NEW.request,0)=2 AND counter>=2 THEN 300000::bigint*counter-500000 ELSE 0 END;
  IF NEW.fee_gold<>expected_fee OR get_byte(NEW.request,0) NOT IN(1,2)
  OR (NEW.state_after->>'removals')::int<>LEAST(255,counter+CASE WHEN get_byte(NEW.request,0)=2 THEN 1 ELSE 0 END)
  OR NEW.state_before IS DISTINCT FROM previous OR NEW.created_xact_id<>pg_current_xact_id()
  OR (NEW.ordinal>1 AND previous IS NULL)
  OR (NEW.fee_gold>0 AND NOT EXISTS(SELECT 1 FROM game_item_fee_burns f WHERE f.transaction_id=NEW.fee_transaction_id
    AND f.character_id=NEW.character_id AND f.cause_kind=16 AND f.cause_occurrence_id=NEW.occurrence_id
    AND f.fee_gold_units=NEW.fee_gold AND f.created_xact_id=pg_current_xact_id())) THEN
   RAISE EXCEPTION 'Boss slot operation chain or fee mismatch' USING ERRCODE='23514'; END IF;
 END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_boss_slots_proven AFTER INSERT OR UPDATE ON game_character_boss_slots
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_boss_slots_proven();
CREATE CONSTRAINT TRIGGER game_boss_slot_operations_proven AFTER INSERT ON game_character_boss_slot_operations
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_boss_slots_proven();
REVOKE ALL ON FUNCTION game_boss_slots_proven() FROM PUBLIC;

-- Add a closed fee cause, retaining every existing source and the same audit envelope.
DO $$ DECLARE body TEXT; BEGIN
 SELECT regexp_replace(pg_get_constraintdef(oid),' NOT VALID$','') INTO STRICT body FROM pg_constraint WHERE conrelid='game_item_fee_burns'::regclass AND conname='game_item_fee_burns_cause_kind_check';
 ALTER TABLE game_item_fee_burns DROP CONSTRAINT game_item_fee_burns_cause_kind_check;
 EXECUTE 'ALTER TABLE game_item_fee_burns ADD CONSTRAINT game_item_fee_burns_cause_kind_check CHECK(('||substr(body,7)||') OR cause_kind=16)';
 SELECT regexp_replace(pg_get_constraintdef(oid),' NOT VALID$','') INTO STRICT body FROM pg_constraint WHERE conrelid='game_item_fee_burns'::regclass AND conname='game_item_fee_burns_purchase_cause';
 ALTER TABLE game_item_fee_burns DROP CONSTRAINT game_item_fee_burns_purchase_cause;
 EXECUTE 'ALTER TABLE game_item_fee_burns ADD CONSTRAINT game_item_fee_burns_purchase_cause CHECK(('||substr(body,7)||') OR (cause_kind=16 AND charm_key IS NULL AND purchase_service_key=''oteryn:bosstiary.slot_remove'' AND fee_gold_units BETWEEN 100000 AND 76000000)) NOT VALID';
 SELECT pg_get_constraintdef(oid) INTO STRICT body FROM pg_constraint WHERE conrelid='game_item_fee_burns'::regclass AND conname='game_item_fee_burns_committed_character_revision_check';
 ALTER TABLE game_item_fee_burns DROP CONSTRAINT game_item_fee_burns_committed_character_revision_check;
 EXECUTE 'ALTER TABLE game_item_fee_burns ADD CONSTRAINT game_item_fee_burns_committed_character_revision_check CHECK(('||substr(body,7)||') OR (cause_kind=16 AND committed_character_revision BETWEEN 1 AND 18446744073709551615))';
 body:=pg_get_functiondef('game_item_fee_burn_consistency_guard()'::regprocedure);
 IF position('(NEW.cause_kind=5 AND EXISTS' IN body)=0 THEN RAISE EXCEPTION 'missing item-only fee proof'; END IF;
 EXECUTE replace(body,'(NEW.cause_kind=5 AND EXISTS',
  '(NEW.cause_kind=16 AND EXISTS(SELECT 1 FROM game_character_boss_slot_operations p WHERE p.fee_transaction_id=NEW.transaction_id AND p.character_id=NEW.character_id AND p.occurrence_id=NEW.cause_occurrence_id AND p.fee_gold=NEW.fee_gold_units AND p.created_xact_id=pg_current_xact_id())) OR (NEW.cause_kind=5 AND EXISTS');
END $$;
ALTER TABLE game_character_bosstiary_kill_receipts ADD COLUMN kill_credit SMALLINT NOT NULL DEFAULT 1 CHECK(kill_credit IN(1,3));
