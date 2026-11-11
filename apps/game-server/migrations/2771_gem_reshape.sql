-- Mod Reshaping (Winter Update 2026): one modifier of a revealed gem is redrawn for
-- 125,000 gold + 1 lesser fragment (position 1), 1,000,000 gold + 2 lesser fragments
-- (position 2) or 5,000,000 gold + 3 greater fragments (position 3).
-- The Atelier receipt kind RESHAPE pays through fee cause 483 and burns its fragments
-- through the existing Atelier item record, in the same transaction as the receipt.
-- Earlier migrations stay unchanged; every guard is extended forward from its live text.
-- Reapplication is a no-op.

ALTER TABLE game_character_atelier_receipts DROP CONSTRAINT IF EXISTS game_character_atelier_receipts_kind_check;
ALTER TABLE game_character_atelier_receipts ADD CONSTRAINT game_character_atelier_receipts_kind_check
 CHECK(kind IN ('GEM_INIT','SET_LOCK','REVEAL','SWITCH_DOMAIN','DISMANTLE','GRADE_UP','RESHAPE'));

-- Reshape burns carried fragments first; at a depot the Supply Stash pays the remainder.
ALTER TABLE game_atelier_item_records DROP CONSTRAINT IF EXISTS game_atelier_item_records_cause_check;
ALTER TABLE game_atelier_item_records ADD CONSTRAINT game_atelier_item_records_cause_check
 CHECK(cause IN ('REVEAL','DISMANTLE','GRADE_UP','RESHAPE'));
ALTER TABLE game_atelier_item_records DROP CONSTRAINT IF EXISTS game_atelier_material_shape;
ALTER TABLE game_atelier_item_records ADD CONSTRAINT game_atelier_material_shape CHECK(
  (cause='REVEAL' AND jsonb_array_length(inputs)=1 AND jsonb_array_length(outputs)=0 AND stash_input IS NULL)
  OR (cause='GRADE_UP' AND jsonb_array_length(outputs)=0 AND (jsonb_array_length(inputs)>0 OR stash_input IS NOT NULL))
  OR (cause='DISMANTLE' AND jsonb_array_length(inputs)=0 AND jsonb_array_length(outputs) BETWEEN 1 AND 10 AND stash_input IS NULL)
  OR (cause='RESHAPE' AND jsonb_array_length(inputs)<=3 AND jsonb_array_length(outputs)=0 AND (jsonb_array_length(inputs)>0 OR stash_input IS NOT NULL)));
ALTER TABLE game_atelier_item_records DROP CONSTRAINT IF EXISTS game_atelier_stash_shape;
ALTER TABLE game_atelier_item_records ADD CONSTRAINT game_atelier_stash_shape CHECK(stash_input IS NULL OR
  (cause IN ('GRADE_UP','RESHAPE') AND jsonb_typeof(stash_input)='object' AND (stash_input->>'count')::int BETWEEN 1 AND 30));

-- Fee ledger: cause 483 joins the Atelier carried-coin cause set and its closed price list.
DO $ledger$
DECLARE body TEXT; rule RECORD;
BEGIN
 FOR rule IN SELECT conname,pg_get_constraintdef(oid) AS definition FROM pg_constraint
  WHERE conrelid='game_item_fee_burns'::regclass AND contype='c' LOOP
  -- 0745 rebuilds these ceilings with causes 9-11 when it is reapplied; widen either form.
  IF rule.definition ~ 'ARRAY\[9, 10, 11(, 18)?\]' THEN
   EXECUTE format('ALTER TABLE game_item_fee_burns DROP CONSTRAINT %I',rule.conname);
   EXECUTE format('ALTER TABLE game_item_fee_burns ADD CONSTRAINT %I %s',rule.conname,
    regexp_replace(rule.definition,'ARRAY\[9, 10, 11(, 18)?\]','ARRAY[9, 10, 11\1, 483]','g'));
  END IF;
 END LOOP;
 SELECT pg_get_constraintdef(oid) INTO STRICT body FROM pg_constraint
  WHERE conrelid='game_item_fee_burns'::regclass AND conname='game_item_fee_burns_cause_kind_check';
 IF position('483' IN body)=0 THEN
  ALTER TABLE game_item_fee_burns DROP CONSTRAINT game_item_fee_burns_cause_kind_check;
  EXECUTE 'ALTER TABLE game_item_fee_burns ADD CONSTRAINT game_item_fee_burns_cause_kind_check CHECK (('
   ||substr(body,7)||') OR cause_kind=483)';
 END IF;
 SELECT regexp_replace(pg_get_constraintdef(oid),' NOT VALID$','') INTO STRICT body FROM pg_constraint
  WHERE conrelid='game_item_fee_burns'::regclass AND conname='game_item_fee_burns_purchase_cause';
 IF position('oteryn:atelier.reshape' IN body)=0 THEN
  ALTER TABLE game_item_fee_burns DROP CONSTRAINT game_item_fee_burns_purchase_cause;
  EXECUTE 'ALTER TABLE game_item_fee_burns ADD CONSTRAINT game_item_fee_burns_purchase_cause CHECK (('
   ||substr(body,7)||') OR (cause_kind=483 AND charm_key IS NULL AND purchase_service_key=''oteryn:atelier.reshape'' '
   ||'AND fee_gold_units IN (125000,1000000,5000000))) NOT VALID';
 END IF;
END $ledger$;

-- Same-transaction Atelier receipt proof and the carried-coin proof profile of causes 9-11.
-- Cause 483 is added beside the existing cause lists, never inside them: the reapplied
-- 0314 and 1007 keep finding their own anchors and stay no-ops.
DO $carried$
DECLARE body TEXT; target TEXT; site TEXT[];
BEGIN
 SELECT pg_get_functiondef('game_item_fee_burn_consistency_guard()'::regprocedure) INTO STRICT body;
 IF position('(NEW.cause_kind=483 AND EXISTS(SELECT 1 FROM game_character_atelier_receipts' IN body)=0 THEN
  IF position('(NEW.cause_kind=5 AND EXISTS' IN body)=0 THEN
   RAISE EXCEPTION 'reshape fee requires the Character proof anchor';
  END IF;
  EXECUTE replace(body,'(NEW.cause_kind=5 AND EXISTS',
   '(NEW.cause_kind=483 AND EXISTS(SELECT 1 FROM game_character_atelier_receipts r WHERE r.character_id=NEW.character_id AND r.occurrence=NEW.cause_occurrence_id AND r.committed_character_revision=NEW.committed_character_revision AND r.created_xact_id=pg_current_xact_id())) OR (NEW.cause_kind=5 AND EXISTS');
 END IF;
 FOREACH site SLICE 1 IN ARRAY ARRAY[
  ARRAY['game_atelier_coin_admit()','f.cause_kind IN (9,10,11,18)','(f.cause_kind IN (9,10,11,18) OR f.cause_kind=483)'],
  ARRAY['game_atelier_coin_complete(uuid)','f.cause_kind NOT IN (9,10,11,18)','(f.cause_kind NOT IN (9,10,11,18) AND f.cause_kind<>483)'],
  ARRAY['game_item_fee_burn_consistency_guard()','NEW.cause_kind IN (9,10,11,18) AND EXISTS(SELECT 1 FROM game_atelier_fee_coin_records',
   '(NEW.cause_kind IN (9,10,11,18) OR NEW.cause_kind=483) AND EXISTS(SELECT 1 FROM game_atelier_fee_coin_records'],
  ARRAY['game_item_audit_envelope_size_proven()','f.cause_kind IN (9,10,11)','(f.cause_kind IN (9,10,11) OR f.cause_kind=483)']] LOOP
  target:=site[1];
  SELECT pg_get_functiondef(target::regprocedure) INTO STRICT body;
  IF position(site[3] IN body)=0 THEN
   IF position(site[2] IN body)=0 THEN
    RAISE EXCEPTION 'reshape fee requires the carried-coin proof anchor: %',target;
   END IF;
   EXECUTE replace(body,site[2],site[3]);
  END IF;
 END LOOP;
END $carried$;

-- Atelier receipt, item record and fee bind into one transaction; RESHAPE needs all three.
CREATE OR REPLACE FUNCTION game_atelier_bind_assets() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 SELECT transaction_id INTO NEW.fee_transaction_id FROM game_item_fee_burns
  WHERE character_id=NEW.character_id AND cause_occurrence_id=NEW.occurrence AND cause_kind IN (9,10,11,483) AND created_xact_id=pg_current_xact_id();
 SELECT transaction_id INTO NEW.item_transaction_id FROM game_atelier_item_records
  WHERE character_id=NEW.character_id AND occurrence=NEW.occurrence AND created_xact_id=pg_current_xact_id();
 IF (NEW.kind IN ('REVEAL','SWITCH_DOMAIN','GRADE_UP','RESHAPE'))<>(NEW.fee_transaction_id IS NOT NULL)
   OR (NEW.kind IN ('REVEAL','DISMANTLE','GRADE_UP','RESHAPE'))<>(NEW.item_transaction_id IS NOT NULL)
   OR (NEW.kind IN ('REVEAL','GRADE_UP','RESHAPE') AND NEW.item_transaction_id IS DISTINCT FROM NEW.fee_transaction_id)
   OR NEW.outcome_json->'gold' IS DISTINCT FROM '0'::jsonb
   OR NEW.outcome_json->'fragments' IS DISTINCT FROM '{}'::jsonb
   OR NEW.outcome_json->'unrevealed' IS DISTINCT FROM '{}'::jsonb THEN
  RAISE EXCEPTION 'Atelier assets require real same-transaction custody' USING ERRCODE='23514';
 END IF;
 RETURN NEW;
END $$;

DO $guard$
DECLARE body TEXT; old_part TEXT; new_part TEXT;
BEGIN
 body:=pg_get_functiondef('game_atelier_item_complete(uuid)'::regprocedure);
 IF position('r.cause IN (''REVEAL'',''GRADE_UP'',''RESHAPE'')' IN body)=0 THEN
  old_part:='r.cause IN (''REVEAL'',''GRADE_UP'')';
  IF position(old_part IN body)=0 THEN RAISE EXCEPTION 'missing Atelier fee audit anchor'; END IF;
  EXECUTE replace(body,old_part,'r.cause IN (''REVEAL'',''GRADE_UP'',''RESHAPE'')');
 END IF;

 body:=pg_get_functiondef('game_atelier_assets_guard()'::regprocedure);
 IF position('a.kind=''RESHAPE''' IN body)=0 THEN
  old_part:='IF NEW.cause_kind NOT IN (9,10,11) THEN RETURN NULL; END IF;';
  IF position(old_part IN body)=0 THEN RAISE EXCEPTION 'missing Atelier fee cause anchor'; END IF;
  body:=replace(body,old_part,'IF NEW.cause_kind NOT IN (9,10,11,483) THEN RETURN NULL; END IF;');
  old_part:='count_needed INT; fee BIGINT;';
  IF position(old_part IN body)=0 THEN RAISE EXCEPTION 'missing Atelier guard declaration anchor'; END IF;
  body:=replace(body,old_part,'count_needed INT; fee BIGINT; reshape_at INT;');
  -- The fee names the position; at most one gem changes, and only in that modifier.
  -- A redraw of the same modifier leaves the gem unchanged and still pays.
  old_part:=' ELSIF a.kind=''GRADE_UP'' THEN';
  IF position(old_part IN body)=0 THEN RAISE EXCEPTION 'missing Atelier grade branch anchor'; END IF;
  new_part:=$arm$ ELSIF a.kind='RESHAPE' THEN
  reshape_at:=CASE f.fee_gold_units WHEN 125000 THEN 1 WHEN 1000000 THEN 2 WHEN 5000000 THEN 3 END;
  fee:=CASE reshape_at WHEN 1 THEN 125000 WHEN 2 THEN 1000000 WHEN 3 THEN 5000000 END;
  count_needed:=reshape_at;
  key:=CASE reshape_at WHEN 3 THEN 'oteryn:item.tibia.i46626' WHEN 1 THEN 'oteryn:item.tibia.i46625' WHEN 2 THEN 'oteryn:item.tibia.i46625' END;
  IF reshape_at IS NULL
    OR (SELECT array_agg(e.key ORDER BY e.key) FROM jsonb_each(a.outcome_json->'gems') e)
       IS DISTINCT FROM (SELECT array_agg(e.key ORDER BY e.key) FROM jsonb_each(before_state->'gems') e)
    OR (SELECT count(*) FROM jsonb_each(a.outcome_json->'gems') e WHERE e.value IS DISTINCT FROM before_state->'gems'->e.key)>1
    OR EXISTS(SELECT 1 FROM jsonb_each(a.outcome_json->'gems') e
       WHERE e.value IS DISTINCT FROM before_state->'gems'->e.key
        AND ((e.value-'mods') IS DISTINCT FROM (before_state->'gems'->e.key)-'mods'
         OR (before_state->'gems'->e.key->>'locked')::boolean
         OR jsonb_array_length(e.value->'mods')<>jsonb_array_length(before_state->'gems'->e.key->'mods')
         OR jsonb_array_length(e.value->'mods')<reshape_at
         OR EXISTS(SELECT 1 FROM generate_series(0,jsonb_array_length(e.value->'mods')-1) i
            WHERE i<>reshape_at-1 AND e.value->'mods'->i IS DISTINCT FROM before_state->'gems'->e.key->'mods'->i))) THEN
   RAISE EXCEPTION 'Atelier reshape must redraw one modifier of one unlocked gem' USING ERRCODE='23514';
  END IF;
 ELSIF a.kind='GRADE_UP' THEN$arm$;
  body:=replace(body,old_part,new_part);
  old_part:='CASE a.kind WHEN ''REVEAL'' THEN 9 WHEN ''SWITCH_DOMAIN'' THEN 10 ELSE 11 END';
  IF position(old_part IN body)=0 THEN RAISE EXCEPTION 'missing Atelier fee kind anchor'; END IF;
  body:=replace(body,old_part,'CASE a.kind WHEN ''REVEAL'' THEN 9 WHEN ''SWITCH_DOMAIN'' THEN 10 WHEN ''RESHAPE'' THEN 483 ELSE 11 END');
  old_part:='IF a.kind IN (''REVEAL'',''GRADE_UP'') AND (key IS NULL';
  IF position(old_part IN body)=0 THEN RAISE EXCEPTION 'missing Atelier material anchor'; END IF;
  EXECUTE replace(body,old_part,'IF a.kind IN (''REVEAL'',''GRADE_UP'',''RESHAPE'') AND (key IS NULL');
 END IF;
END $guard$;

-- The Supply Stash ledger accepts a RESHAPE debit exactly like a Grade Up debit.
DO $stash$
DECLARE site TEXT[]; body TEXT;
BEGIN
 FOREACH site SLICE 1 IN ARRAY ARRAY[
  ARRAY['game_atelier_supplement_complete(uuid)','r.cause<>''GRADE_UP''','r.cause NOT IN (''GRADE_UP'',''RESHAPE'')'],
  ARRAY['game_stash_admit()','a.cause=''GRADE_UP''','a.cause IN (''GRADE_UP'',''RESHAPE'')'],
  ARRAY['game_stash_balance_guard()','a.cause=''GRADE_UP''','a.cause IN (''GRADE_UP'',''RESHAPE'')']] LOOP
  SELECT pg_get_functiondef(site[1]::regprocedure) INTO STRICT body;
  IF position(site[3] IN body)=0 THEN
   IF position(site[2] IN body)=0 THEN
    RAISE EXCEPTION 'reshape stash debit requires the Atelier stash anchor: %',site[1];
   END IF;
   EXECUTE replace(body,site[2],site[3]);
  END IF;
 END LOOP;
END $stash$;

DO $search$
DECLARE n TEXT;
BEGIN
 FOREACH n IN ARRAY ARRAY['game_atelier_bind_assets()','game_atelier_assets_guard()','game_atelier_item_complete(uuid)',
  'game_atelier_coin_admit()','game_atelier_coin_complete(uuid)','game_item_fee_burn_consistency_guard()',
  'game_item_audit_envelope_size_proven()'] LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',n,current_schema());
 END LOOP;
END $search$;
REVOKE ALL ON FUNCTION game_atelier_bind_assets(),game_atelier_assets_guard() FROM PUBLIC;
