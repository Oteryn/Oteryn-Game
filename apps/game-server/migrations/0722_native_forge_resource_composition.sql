-- P18 composes the existing Character dust, fee and supply-stash owners.
-- The denied SQL prototype in0320 deliberately remains denied and unused.
DO $$ DECLARE body TEXT; BEGIN
 SELECT regexp_replace(pg_get_constraintdef(oid),' NOT VALID$','') INTO STRICT body FROM pg_constraint
  WHERE conrelid='game_character_forge_dust_entries'::regclass AND conname='game_character_forge_dust_entry_cause_admitted';
 ALTER TABLE game_character_forge_dust_entries DROP CONSTRAINT game_character_forge_dust_entry_cause_admitted;
 EXECUTE 'ALTER TABLE game_character_forge_dust_entries ADD CONSTRAINT game_character_forge_dust_entry_cause_admitted CHECK (('
  ||substr(body,7)||') OR (cause=''forge'' AND kind IN(''SPEND'',''CONVERT'')))';
 SELECT regexp_replace(pg_get_constraintdef(oid),' NOT VALID$','') INTO STRICT body FROM pg_constraint
  WHERE conrelid='game_character_forge_dust_entries'::regclass AND conname='game_character_forge_dust_entry_arithmetic';
 ALTER TABLE game_character_forge_dust_entries DROP CONSTRAINT game_character_forge_dust_entry_arithmetic;
 EXECUTE 'ALTER TABLE game_character_forge_dust_entries ADD CONSTRAINT game_character_forge_dust_entry_arithmetic CHECK (('
  ||substr(body,7)||') OR (kind=''CONVERT'' AND cause=''forge'' AND amount=60 AND lost_amount=0 AND dust_limit_after=dust_limit_before AND balance_after=balance_before-amount))';
END $$;
CREATE FUNCTION game_forge_dust_source_proven() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NEW.cause='forge' THEN PERFORM game_forge_complete(NEW.transaction_id); END IF;
 IF NEW.kind='LIMIT_RAISE' AND EXISTS(SELECT 1 FROM game_forge_reservations r
   WHERE r.character_id=NEW.character_id AND r.occurrence_id=NEW.cause_occurrence_id
    AND (r.transaction_id<>NEW.transaction_id OR get_byte(r.intent,0)<>7
      OR r.expected_dust_upgrade_count IS DISTINCT FROM (NEW.dust_limit_before-100)/20
      OR r.work_units_used<>(SELECT work_units_used FROM game_forge_dust_limit_reservations l
        WHERE l.character_id=r.character_id AND l.occurrence_id=r.occurrence_id)
      OR EXISTS(SELECT 1 FROM game_forge_operations p WHERE p.character_id=r.character_id
        AND p.occurrence_id=r.occurrence_id AND NOT p.applied))) THEN
  RAISE EXCEPTION 'legacy DustLimit cannot change a conflicting or terminal Forge occurrence' USING ERRCODE='23514';
 END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_forge_dust_source_proven AFTER INSERT ON game_character_forge_dust_entries
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_forge_dust_source_proven();

DO $$ DECLARE body TEXT; BEGIN
 SELECT regexp_replace(pg_get_constraintdef(oid),' NOT VALID$','') INTO STRICT body FROM pg_constraint
  WHERE conrelid='game_item_fee_burns'::regclass AND conname='game_item_fee_burns_cause_kind_check';
 ALTER TABLE game_item_fee_burns DROP CONSTRAINT game_item_fee_burns_cause_kind_check;
 EXECUTE 'ALTER TABLE game_item_fee_burns ADD CONSTRAINT game_item_fee_burns_cause_kind_check CHECK (('||substr(body,7)||') OR cause_kind=14)';
 SELECT regexp_replace(pg_get_constraintdef(oid),' NOT VALID$','') INTO STRICT body FROM pg_constraint
  WHERE conrelid='game_item_fee_burns'::regclass AND conname='game_item_fee_burns_purchase_cause';
 ALTER TABLE game_item_fee_burns DROP CONSTRAINT game_item_fee_burns_purchase_cause;
 -- Keep the existing historical imbuement-price NOT VALID status.
 EXECUTE 'ALTER TABLE game_item_fee_burns ADD CONSTRAINT game_item_fee_burns_purchase_cause CHECK (('
  ||substr(body,7)||') OR (cause_kind=14 AND charm_key IS NULL AND purchase_service_key=''oteryn:service.forge'' AND fee_gold_units BETWEEN 1 AND 300000000000)) NOT VALID';
 SELECT regexp_replace(pg_get_constraintdef(oid),' NOT VALID$','') INTO STRICT body FROM pg_constraint
  WHERE conrelid='game_item_fee_burns'::regclass AND conname='game_item_fee_burns_committed_character_revision_check';
 ALTER TABLE game_item_fee_burns DROP CONSTRAINT game_item_fee_burns_committed_character_revision_check;
 EXECUTE 'ALTER TABLE game_item_fee_burns ADD CONSTRAINT game_item_fee_burns_committed_character_revision_check CHECK (('
  ||substr(body,7)||') OR (cause_kind=14 AND committed_character_revision BETWEEN 1 AND 18446744073709551615))';
 body:=pg_get_functiondef('game_item_fee_burn_consistency_guard()'::regprocedure);
 IF position('(NEW.cause_kind=5 AND EXISTS' IN body)=0 THEN RAISE EXCEPTION 'missing native item-only fee proof'; END IF;
 EXECUTE replace(body,'(NEW.cause_kind=5 AND EXISTS',
  '(NEW.cause_kind=14 AND EXISTS(SELECT 1 FROM game_forge_operations p WHERE p.transaction_id=NEW.transaction_id AND p.character_id=NEW.character_id AND p.occurrence_id=NEW.cause_occurrence_id AND p.applied AND p.gold_spent=NEW.fee_gold_units AND p.created_xact_id=pg_current_xact_id())) OR (NEW.cause_kind=5 AND EXISTS');
END $$;
CREATE FUNCTION game_forge_fee_source_proven() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NEW.cause_kind=14 THEN PERFORM game_forge_complete(NEW.transaction_id); END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_forge_fee_source_proven AFTER INSERT ON game_item_fee_burns
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_forge_fee_source_proven();

-- A Forge fee adds at most4096 bytes of bounded proof to the unchanged native
-- fee envelope ceiling25712; other fee shapes keep their old bound.
DO $migration$ DECLARE body TEXT; begin_at INT; BEGIN
 body:=pg_get_functiondef('game_item_audit_envelope_size_proven()'::regprocedure);
 begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
 IF begin_at=0 THEN RAISE EXCEPTION 'missing audit size proof'; END IF;
 EXECUTE overlay(body placing E'BEGIN\n'||$arm$
  IF octet_length(NEW.envelope)<=29808 AND EXISTS(SELECT 1 FROM game_forge_operations p
    JOIN game_item_fee_burns f USING(transaction_id,event_id)
    WHERE p.transaction_id=NEW.transaction_id AND p.event_id=NEW.event_id AND p.applied
      AND f.cause_kind=14 AND p.created_xact_id=pg_current_xact_id() AND f.created_xact_id=pg_current_xact_id()
      AND p.envelope_sha256=NEW.envelope_sha256 AND position(p.audit_proof IN NEW.envelope)>0)
  THEN PERFORM game_forge_complete(NEW.transaction_id); RETURN NULL; END IF;
$arm$ from begin_at for 5);
END $migration$;

-- Existing stash action4 is the one closed Forge material sink; no separate
-- balance or material outbox. The common Forge event covers all value lines.
ALTER TABLE game_character_stash_operations DROP CONSTRAINT game_character_stash_operations_action_check;
ALTER TABLE game_character_stash_operations ADD CONSTRAINT game_character_stash_operations_action_check CHECK(action IN(2,3,4));
DO $migration$ DECLARE body TEXT; begin_at INT; BEGIN
 body:=pg_get_functiondef('game_stash_admit()'::regprocedure);
 begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
 IF begin_at=0 THEN RAISE EXCEPTION 'missing stash admission proof'; END IF;
 EXECUTE overlay(body placing E'BEGIN\n'||$arm$
  IF TG_TABLE_NAME='game_character_stash_operations' THEN
   IF NEW.action=4 THEN
    IF NEW.outcome<>2 OR NOT EXISTS(SELECT 1 FROM game_forge_operations forge_operation JOIN game_forge_reservations forge_reservation USING(character_id,occurrence_id)
      WHERE forge_operation.transaction_id=NEW.transaction_id AND forge_operation.event_id=NEW.event_id AND forge_operation.character_id=NEW.character_id
        AND forge_reservation.world_id=NEW.world_id AND forge_operation.game_session_id=NEW.game_session_id AND forge_operation.command_id=NEW.command_id
        AND forge_operation.applied AND forge_operation.stash IS NOT NULL AND NEW.intent=sha256(forge_reservation.intent) AND forge_operation.created_xact_id=pg_current_xact_id())
    THEN RAISE EXCEPTION 'stash Forge sink needs its exact native action' USING ERRCODE='23514'; END IF;
    NEW.created_xact_id:=pg_current_xact_id(); RETURN NEW;
   END IF;
  ELSE
   IF EXISTS(SELECT 1 FROM game_character_stash_operations s WHERE s.transaction_id=NEW.transaction_id AND s.action=4) THEN
    IF TG_TABLE_NAME<>'game_character_stash_entries' THEN RAISE EXCEPTION 'Forge uses its own exact item proof, not stash item lines' USING ERRCODE='23514'; END IF;
    IF NOT EXISTS(SELECT 1 FROM game_forge_operations forge_operation JOIN game_forge_reservations forge_reservation USING(character_id,occurrence_id)
      WHERE forge_operation.transaction_id=NEW.transaction_id AND forge_operation.applied AND forge_operation.created_xact_id=pg_current_xact_id()
        AND forge_operation.character_id=NEW.character_id AND forge_reservation.world_id=NEW.world_id
        AND (forge_operation.stash->>'entry')::uuid=NEW.entry_id AND forge_operation.stash->>'key'=NEW.definition_key AND forge_operation.stash->>'revision'=NEW.definition_revision
        AND (forge_operation.stash->>'amount')::bigint=NEW.amount AND (forge_operation.stash->>'before')::bigint=NEW.quantity_before
        AND (forge_operation.stash->>'after')::bigint=NEW.quantity_after AND (forge_operation.stash->>'previous')::uuid=NEW.previous_entry_id)
    THEN RAISE EXCEPTION 'Forge stash entry does not match its native proof' USING ERRCODE='23514'; END IF;
    NEW.created_xact_id:=pg_current_xact_id(); RETURN NEW;
   END IF;
  END IF;
$arm$ from begin_at for 5);
 body:=pg_get_functiondef('game_stash_complete(uuid)'::regprocedure);
 begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
 IF begin_at=0 THEN RAISE EXCEPTION 'missing stash completion proof'; END IF;
 EXECUTE overlay(body placing E'BEGIN\n'||$arm$
  IF EXISTS(SELECT 1 FROM game_character_stash_operations s WHERE s.transaction_id=p_transaction AND s.action=4) THEN
   PERFORM game_forge_complete(p_transaction); RETURN;
  END IF;
$arm$ from begin_at for 5);
END $migration$;
DO $$ DECLARE n TEXT; BEGIN
 FOREACH n IN ARRAY ARRAY['game_forge_dust_source_proven()','game_forge_fee_source_proven()'] LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',n,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC',n);
 END LOOP;
END $$;
