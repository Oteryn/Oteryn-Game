-- Closed NPC473/474 physical cash. Original Gem proofs/SQL are unchanged.
CREATE FUNCTION game_npc_cash_context_current(p_fee UUID,p_character UUID,p_world UUID)RETURNS BOOLEAN LANGUAGE plpgsql STABLE SECURITY DEFINER SET search_path=public,pg_temp AS $$
BEGIN IF game_horse_rental_fee_current(p_fee,p_character,p_world) THEN RETURN TRUE;END IF;RETURN game_npc_source_fee_current(p_fee,p_character,p_world);END $$;
CREATE FUNCTION game_npc_cash_source_complete(p_fee UUID,p_character UUID,p_world UUID)RETURNS BOOLEAN LANGUAGE plpgsql STABLE SECURITY DEFINER SET search_path=public,pg_temp AS $$
BEGIN IF game_horse_rental_fee_complete(p_fee,p_character,p_world) THEN RETURN TRUE;END IF;RETURN game_npc_source_fee_complete(p_fee,p_character,p_world);END $$;
CREATE FUNCTION game_npc_cash_minimum(p_fee UUID,p_character UUID,p_world UUID) RETURNS BIGINT LANGUAGE plpgsql STABLE SECURITY DEFINER SET search_path=public,pg_temp AS $$
BEGIN IF game_horse_rental_fee_current(p_fee,p_character,p_world) THEN RETURN 0;END IF;
RETURN game_npc_source_fee_cash_minimum(p_fee,p_character,p_world);END $$;
CREATE TABLE IF NOT EXISTS game_npc_fee_cash_records (
 transaction_id UUID PRIMARY KEY REFERENCES game_item_fee_burns(transaction_id),
 character_id UUID NOT NULL REFERENCES game_character_roots,
 world_id UUID NOT NULL,
 backpack_item_instance_id UUID REFERENCES game_item_instances,
 backpack_capacity SMALLINT NOT NULL CHECK(backpack_capacity BETWEEN 0 AND 20),
 backpack_ordinals JSONB NOT NULL CHECK(jsonb_typeof(backpack_ordinals)='array' AND jsonb_array_length(backpack_ordinals)<=20),
 inputs JSONB NOT NULL CHECK(jsonb_typeof(inputs)='array' AND jsonb_array_length(inputs)<=508),
 equipment_cost_transaction_id UUID REFERENCES game_srv7_item_costs(transaction_id),
 equipment_before JSONB CHECK(equipment_before IS NULL OR jsonb_typeof(equipment_before)='object'),
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
 CHECK((backpack_item_instance_id IS NULL)=(backpack_capacity=0))
);
-- FK lookups and deferred current-transaction evidence must not scan historical payments.
CREATE INDEX IF NOT EXISTS game_npc_fee_cash_character_xact
 ON game_npc_fee_cash_records(character_id,created_xact_id);
CREATE INDEX IF NOT EXISTS game_npc_fee_cash_current_world
 ON game_npc_fee_cash_records(created_xact_id,world_id);
CREATE INDEX IF NOT EXISTS game_npc_fee_cash_backpack
 ON game_npc_fee_cash_records(backpack_item_instance_id) WHERE backpack_item_instance_id IS NOT NULL;

ALTER TABLE game_npc_fee_cash_records ADD CHECK(equipment_cost_transaction_id IS NULL OR equipment_cost_transaction_id<>transaction_id);
CREATE INDEX game_npc_fee_cash_equipment_cost ON game_npc_fee_cash_records(equipment_cost_transaction_id) WHERE equipment_cost_transaction_id IS NOT NULL;
ALTER TABLE game_npc_fee_cash_records ADD CHECK(equipment_before IS NOT NULL OR equipment_cost_transaction_id IS NULL);

CREATE OR REPLACE FUNCTION game_npc_cash_admit() RETURNS trigger LANGUAGE plpgsql SET search_path=public,pg_temp AS $$
DECLARE line JSONB; item game_item_instances%ROWTYPE; current_equipment JSONB; material_equipment JSONB; current_sources JSONB; prior_cost UUID;
BEGIN
 SELECT coalesce(jsonb_agg(to_jsonb(c)),'[]'::jsonb) INTO current_sources FROM game_atelier_carried_coin_sources(NEW.character_id,NEW.world_id) c;
 IF game_npc_cash_context_current(NEW.transaction_id,NEW.character_id,NEW.world_id) IS DISTINCT FROM TRUE
 OR NOT EXISTS(SELECT 1 FROM game_item_fee_burns f WHERE f.transaction_id=NEW.transaction_id AND f.created_xact_id=pg_current_xact_id()
   AND f.cause_kind IN (473,474) AND f.character_id=NEW.character_id AND f.world_id=NEW.world_id
   AND f.backpack_item_instance_id IS NOT DISTINCT FROM NEW.backpack_item_instance_id)
  OR NEW.backpack_item_instance_id IS DISTINCT FROM (SELECT s.item_instance_id FROM game_item_container_slots s WHERE s.character_id=NEW.character_id AND s.world_id=NEW.world_id)
  OR NEW.backpack_ordinals IS DISTINCT FROM coalesce((SELECT jsonb_agg(e.placement_ordinal ORDER BY e.placement_ordinal DESC) FROM game_item_container_entries e WHERE e.parent_item_instance_id=NEW.backpack_item_instance_id),'[]'::jsonb)
  OR jsonb_array_length(NEW.inputs)<>jsonb_array_length(current_sources)
  OR (SELECT count(DISTINCT l->>'item') FROM jsonb_array_elements(NEW.inputs) l)<>jsonb_array_length(NEW.inputs) THEN
  RAISE EXCEPTION 'NPC source coin proof requires the current complete carried sources' USING ERRCODE='23514';
 END IF;
 FOR line IN SELECT value FROM jsonb_array_elements(NEW.inputs) LOOP
  SELECT * INTO item FROM game_item_instances WHERE item_instance_id=(line->>'item')::uuid FOR UPDATE;
  IF item.item_instance_id IS NULL OR to_jsonb(item) IS DISTINCT FROM line->'before' OR item.world_id<>NEW.world_id OR item.lifecycle<>1
   OR item.definition_family<>'Item' OR game_item_coin_worth(item.definition_production_key) IS NULL OR item.quantity NOT BETWEEN 1 AND 100
   OR (line->>'burned')::int NOT BETWEEN 0 AND item.quantity
   OR EXISTS(SELECT 1 FROM game_item_keys WHERE item_instance_id=item.item_instance_id)
   OR EXISTS(SELECT 1 FROM game_item_timed_states WHERE item_instance_id=item.item_instance_id)
   OR EXISTS(SELECT 1 FROM game_item_texts WHERE item_instance_id=item.item_instance_id)
   OR EXISTS(SELECT 1 FROM game_item_imbuements WHERE item_instance_id=item.item_instance_id)
   OR NOT game_atelier_coin_source_present(NEW.character_id,NEW.world_id,line)
   OR NOT EXISTS(SELECT 1 FROM jsonb_array_elements(current_sources) c WHERE (c->>'item_instance_id')::uuid=item.item_instance_id
    AND c->'selection_order'=line->'selection_order' AND c->'surface'=line->'surface' AND c->'slot'=line->'slot'
    AND c->>'parent_item_instance_id' IS NOT DISTINCT FROM line->>'parent' AND (c->>'placement_ordinal')::numeric=(line->>'ordinal')::numeric) THEN
   RAISE EXCEPTION 'NPC source coin input is not its unchanged carried leaf' USING ERRCODE='23514';
  END IF;
 END LOOP;
 IF (NEW.equipment_before IS NOT NULL) IS DISTINCT FROM EXISTS(SELECT 1 FROM jsonb_array_elements(NEW.inputs) l WHERE (l->>'surface')::int=3 AND (l->>'burned')::int>0) THEN
  RAISE EXCEPTION 'NPC source coin equipment proof shape mismatch' USING ERRCODE='23514';
 END IF;
 IF NEW.equipment_before IS NOT NULL THEN
  SELECT to_jsonb(s) INTO current_equipment FROM game_character_equipment_state s WHERE s.character_id=NEW.character_id FOR UPDATE;
  SELECT a.equipment_before,a.cost_transaction_id INTO material_equipment,prior_cost FROM game_npc_cash_prior_equipment(NEW.transaction_id,NEW.character_id,NEW.world_id) a;
  IF (material_equipment IS NULL AND(NEW.equipment_before IS DISTINCT FROM current_equipment OR NEW.equipment_cost_transaction_id IS NOT NULL))
    OR (material_equipment IS NOT NULL AND (NEW.equipment_before IS DISTINCT FROM material_equipment OR NEW.equipment_cost_transaction_id IS DISTINCT FROM prior_cost
      OR (current_equipment->>'revision')::numeric<>(material_equipment->>'revision')::numeric+1
      OR (current_equipment->>'last_transaction_id')::uuid IS DISTINCT FROM prior_cost)) THEN
   RAISE EXCEPTION 'NPC source coin equipment requires the original shared epoch' USING ERRCODE='23514';
  END IF;
 END IF;
 NEW.created_xact_id:=pg_current_xact_id(); RETURN NEW;
END $$;

-- A successful full physical proof is shared by deferred custody callbacks in
-- the same SQL statement. Later statements and every footprint mutation require
-- the complete proof again, including SET CONSTRAINTS followed by more writes.
CREATE TABLE game_npc_cash_verified (
 backend_pid INTEGER NOT NULL,
 transaction_xid XID8 NOT NULL,
 fee_transaction_id UUID NOT NULL,
 statement_started TIMESTAMPTZ NOT NULL,
 PRIMARY KEY(backend_pid,fee_transaction_id)
);
REVOKE ALL ON game_npc_cash_verified FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
CREATE FUNCTION game_npc_cash_invalidate() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
BEGIN
 DELETE FROM game_npc_cash_verified WHERE backend_pid=pg_backend_pid();
 RETURN NULL;
END $$;
REVOKE ALL ON FUNCTION game_npc_cash_invalidate() FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
DO $cash_invalidation$ DECLARE relation TEXT;BEGIN
 -- Complete physical proof read-set, including the custody-presence helper.
 FOREACH relation IN ARRAY ARRAY[
 'game_npc_fee_cash_records','game_item_fee_burns','game_item_fee_burn_lines',
 'game_item_instances','game_character_roots','game_account_bank_entries',
 'game_item_container_entries','game_item_nested_entries','game_item_container_slots',
 'game_character_equipment_slots','game_character_equipment_state',
 'game_item_ground_locations','game_item_corpse_container_entries',
 'game_item_transfer_quantity_evidence','game_item_audit_outbox',
 'game_item_keys','game_item_timed_states','game_item_texts','game_item_imbuements',
 'game_horse_rental_receipts','game_horse_station_context','game_npc_source_context',
 'game_npc_cosmetic_receipts','game_srv7_item_costs',
 'game_runtime_scope_assignments','game_durability_reconnect_sessions',
 'game_durability_admission_character_guards','game_durability_admission_account_guards',
 'game_durability_admission_runtime_guards'] LOOP
  EXECUTE format('CREATE TRIGGER game_npc_cash_invalidate AFTER INSERT OR UPDATE OR DELETE OR TRUNCATE ON %I FOR EACH STATEMENT EXECUTE FUNCTION game_npc_cash_invalidate()',relation);
 END LOOP;
END $cash_invalidation$;

CREATE OR REPLACE FUNCTION game_npc_cash_complete(p_transaction UUID) RETURNS void LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE r game_npc_fee_cash_records%ROWTYPE; f game_item_fee_burns%ROWTYPE; line JSONB; item game_item_instances%ROWTYPE;
 burn game_item_fee_burn_lines%ROWTYPE; position INT:=0; remaining BIGINT; worth BIGINT; before_quantity BIGINT; amount BIGINT;
 first_item UUID; outputs INT; payer UUID; equipment JSONB; last_worth BIGINT; minimum_carried BIGINT;
BEGIN
 SELECT * INTO r FROM game_npc_fee_cash_records WHERE transaction_id=p_transaction;
 SELECT * INTO f FROM game_item_fee_burns WHERE transaction_id=p_transaction;
 minimum_carried:=game_npc_cash_minimum(p_transaction,r.character_id,r.world_id);
 IF r.transaction_id IS NULL OR f.transaction_id IS NULL OR r.created_xact_id<>pg_current_xact_id() OR f.created_xact_id<>pg_current_xact_id()
  OR minimum_carried IS NULL OR minimum_carried<0 OR minimum_carried>f.fee_gold_units OR f.fee_gold_units-f.bank_debit_gold_units<minimum_carried
  OR f.cause_kind NOT IN (473,474) OR f.character_id<>r.character_id OR f.world_id<>r.world_id OR f.backpack_item_instance_id IS DISTINCT FROM r.backpack_item_instance_id
  OR f.backpack_item_instance_id IS DISTINCT FROM (SELECT s.item_instance_id FROM game_item_container_slots s WHERE s.character_id=f.character_id AND s.world_id=f.world_id)
  OR NOT EXISTS(SELECT 1 FROM game_character_roots cr WHERE cr.character_id=f.character_id AND cr.world_id=f.world_id AND cr.character_revision=f.committed_character_revision AND cr.xmin=pg_current_xact_id()::xid)
  OR game_npc_cash_source_complete(f.transaction_id,f.character_id,f.world_id) IS DISTINCT FROM TRUE THEN
  RAISE EXCEPTION 'NPC source coin payment requires its fresh Character receipt' USING ERRCODE='23514';
 END IF;
 -- Authority and immutable parent completion above are deliberately rechecked
 -- even when another custody callback already proved this physical batch.
 IF EXISTS(SELECT 1 FROM game_npc_cash_verified v
  WHERE v.backend_pid=pg_backend_pid() AND v.transaction_xid=pg_current_xact_id()
   AND v.fee_transaction_id=p_transaction AND v.statement_started=statement_timestamp()) THEN
  RETURN;
 END IF;
 remaining:=f.fee_gold_units-f.bank_debit_gold_units;
 FOR line IN SELECT l FROM jsonb_array_elements(r.inputs) l ORDER BY game_item_coin_worth(l->'before'->>'definition_production_key'),(l->'before'->>'quantity')::bigint,(l->>'selection_order')::bigint LOOP
  worth:=game_item_coin_worth(line->'before'->>'definition_production_key'); before_quantity:=(line->'before'->>'quantity')::bigint;
  amount:=least(before_quantity,(remaining+worth-1)/worth);
  IF amount IS DISTINCT FROM (line->>'burned')::bigint THEN RAISE EXCEPTION 'NPC source coins must follow donor denomination and stack-worth order' USING ERRCODE='23514'; END IF;
  SELECT * INTO STRICT item FROM game_item_instances WHERE item_instance_id=(line->>'item')::uuid;
  IF amount=0 THEN
   IF to_jsonb(item) IS DISTINCT FROM line->'before' OR NOT game_atelier_coin_source_present(r.character_id,r.world_id,line) THEN
    RAISE EXCEPTION 'unselected NPC source coins must remain unchanged' USING ERRCODE='23514';
   END IF;
   CONTINUE;
  END IF;
  position:=position+1; last_worth:=worth;
  SELECT * INTO burn FROM game_item_fee_burn_lines WHERE transaction_id=p_transaction AND line_ordinal=position;
  IF position=1 THEN first_item:=item.item_instance_id; END IF;
  IF burn.item_instance_id IS DISTINCT FROM item.item_instance_id OR burn.created_xact_id<>pg_current_xact_id()
   OR burn.coin_worth<>worth OR burn.quantity_before<>before_quantity OR burn.quantity_after<>before_quantity-amount OR burn.placement_ordinal<>(line->>'ordinal')::numeric
   OR item.quantity<>burn.quantity_after OR item.lifecycle<>(CASE WHEN burn.quantity_after=0 THEN 2 ELSE 1 END)
   OR item.last_transaction_id IS DISTINCT FROM p_transaction OR item.state_revision<>(line->'before'->>'state_revision')::numeric+1
   OR (to_jsonb(item)-ARRAY['quantity','lifecycle','last_transaction_id','state_revision']) IS DISTINCT FROM ((line->'before')-ARRAY['quantity','lifecycle','last_transaction_id','state_revision'])
   OR NOT EXISTS(SELECT 1 FROM game_item_transfer_quantity_evidence e WHERE e.item_instance_id=item.item_instance_id AND e.transaction_id=p_transaction AND e.quantity_before=before_quantity)
   OR (burn.quantity_after>0 AND NOT game_atelier_coin_source_present(r.character_id,r.world_id,line))
   OR (burn.quantity_after=0 AND (EXISTS(SELECT 1 FROM game_item_container_entries e WHERE e.item_instance_id=item.item_instance_id)
     OR EXISTS(SELECT 1 FROM game_item_nested_entries e WHERE e.item_instance_id=item.item_instance_id)
     OR EXISTS(SELECT 1 FROM game_character_equipment_slots e WHERE e.item_instance_id=item.item_instance_id)
     OR EXISTS(SELECT 1 FROM game_item_container_slots e WHERE e.item_instance_id=item.item_instance_id)
     OR EXISTS(SELECT 1 FROM game_item_ground_locations e WHERE e.item_instance_id=item.item_instance_id)
     OR EXISTS(SELECT 1 FROM game_item_corpse_container_entries e WHERE e.item_instance_id=item.item_instance_id))) THEN
   RAISE EXCEPTION 'NPC source coin burn lacks exact before/after custody' USING ERRCODE='23514';
  END IF;
  remaining:=greatest(remaining-amount*worth,0);
 END LOOP;
 IF remaining<>0 OR position<>f.line_count OR (SELECT count(*) FROM game_item_fee_burn_lines WHERE transaction_id=p_transaction)<>position
  OR (SELECT coalesce(sum((quantity_before-quantity_after)*coin_worth),0) FROM game_item_fee_burn_lines WHERE transaction_id=p_transaction)<>f.burned_gold_units
  OR (position>0 AND f.change_gold_units>=last_worth)
  OR (f.bank_debit_gold_units>0 AND EXISTS(SELECT 1 FROM jsonb_array_elements(r.inputs) l WHERE (l->>'burned')::bigint<>(l->'before'->>'quantity')::bigint)) THEN
  RAISE EXCEPTION 'NPC source coin payment is not the exact coins-first plan' USING ERRCODE='23514';
 END IF;
 SELECT account_id INTO STRICT payer FROM game_character_roots WHERE character_id=f.character_id;
 IF (SELECT count(*) FROM game_account_bank_entries WHERE fee_transaction_id=p_transaction)<>(f.bank_debit_gold_units>0)::int
  OR (f.bank_debit_gold_units>0 AND NOT EXISTS(SELECT 1 FROM game_account_bank_entries e WHERE e.fee_transaction_id=p_transaction AND e.kind=5
   AND e.amount=f.bank_debit_gold_units AND e.account_id=payer AND e.world_id=f.world_id AND e.acting_character_id=f.character_id AND e.created_xact_id=pg_current_xact_id())) THEN
  RAISE EXCEPTION 'NPC source bank residual requires its one exact ledger debit' USING ERRCODE='23514';
 END IF;
 outputs:=(f.change_gold_units/100>0)::int+(f.change_gold_units%100>0)::int;
 IF (SELECT count(*) FROM game_item_instances WHERE minted_transaction_id=p_transaction)<>outputs
  OR (SELECT count(*) FROM game_item_instances WHERE last_transaction_id=p_transaction)<>position
  OR (SELECT count(*) FROM game_item_container_entries WHERE parent_item_instance_id=r.backpack_item_instance_id)>r.backpack_capacity
  OR (outputs>0 AND (r.backpack_item_instance_id IS NULL OR f.change_placement_ordinal<>1+coalesce((SELECT max(value::text::numeric) FROM jsonb_array_elements(r.backpack_ordinals)),0)))
  OR EXISTS(SELECT 1 FROM (VALUES (f.change_platinum_item_instance_id,'oteryn:item.tibia.i3035',f.change_gold_units/100,f.change_placement_ordinal),
    (f.change_gold_item_instance_id,'oteryn:item.tibia.i3031',f.change_gold_units%100,f.change_placement_ordinal+(f.change_gold_units/100>0)::int)) o(id,key,quantity,ordinal)
   WHERE o.quantity>0 AND NOT EXISTS(SELECT 1 FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id,world_id)
    WHERE i.item_instance_id=o.id AND i.world_id=f.world_id AND i.definition_family='Item' AND i.definition_production_key=o.key AND i.quantity=o.quantity AND i.lifecycle=1
     AND i.minted_transaction_id=p_transaction AND i.last_transaction_id IS NULL AND e.character_id=f.character_id AND e.parent_item_instance_id=r.backpack_item_instance_id
     AND e.placement_ordinal=o.ordinal AND e.placed_transaction_id=p_transaction)) THEN
  RAISE EXCEPTION 'NPC source change requires its bounded exact fresh main-backpack outputs' USING ERRCODE='23514';
 END IF;
 IF NOT EXISTS(SELECT 1 FROM game_item_audit_outbox a WHERE a.transaction_id=p_transaction AND a.event_id=f.event_id AND a.item_instance_id IS NOT DISTINCT FROM first_item
   AND a.occurred_at=f.occurred_at AND a.envelope_sha256=f.envelope_sha256 AND a.created_xact_id=pg_current_xact_id() AND a.publication_state=1 AND a.published_at IS NULL)
  OR (SELECT count(*) FROM game_item_audit_outbox WHERE transaction_id=p_transaction)<>1 THEN
  RAISE EXCEPTION 'NPC source coin payment requires one atomic audit event' USING ERRCODE='23514';
 END IF;
 IF r.equipment_before IS NOT NULL THEN
  SELECT to_jsonb(s) INTO equipment FROM game_character_equipment_state s WHERE s.character_id=r.character_id;
  IF equipment IS NULL OR (equipment->>'revision')::numeric<>(r.equipment_before->>'revision')::numeric+1 OR (equipment->>'last_transaction_id')::uuid IS DISTINCT FROM coalesce(r.equipment_cost_transaction_id,p_transaction)
   OR (equipment-ARRAY['revision','last_transaction_id']) IS DISTINCT FROM (r.equipment_before-ARRAY['revision','last_transaction_id']) THEN
   RAISE EXCEPTION 'NPC source coin equipment requires one shared successor' USING ERRCODE='23514';
  END IF;
 END IF;
 -- Only a successful, unchanged full proof may create this private memo.
 DELETE FROM game_npc_cash_verified v WHERE (v.backend_pid=pg_backend_pid()
  AND v.transaction_xid<>pg_current_xact_id())
  OR NOT EXISTS(SELECT 1 FROM pg_stat_activity a WHERE a.pid=v.backend_pid);
 INSERT INTO game_npc_cash_verified(backend_pid,transaction_xid,fee_transaction_id,statement_started)
 VALUES(pg_backend_pid(),pg_current_xact_id(),p_transaction,statement_timestamp())
 ON CONFLICT(backend_pid,fee_transaction_id) DO UPDATE
 SET transaction_xid=EXCLUDED.transaction_xid,statement_started=EXCLUDED.statement_started;
END $$;

CREATE OR REPLACE FUNCTION game_npc_cash_consistent() RETURNS trigger LANGUAGE plpgsql SET search_path=public,pg_temp AS $$
BEGIN PERFORM game_npc_cash_complete(NEW.transaction_id); RETURN NULL; END $$;
DROP TRIGGER IF EXISTS game_npc_cash_admit ON game_npc_fee_cash_records;
CREATE TRIGGER game_npc_cash_admit BEFORE INSERT ON game_npc_fee_cash_records FOR EACH ROW EXECUTE FUNCTION game_npc_cash_admit();
DROP TRIGGER IF EXISTS game_npc_cash_consistent ON game_npc_fee_cash_records;
CREATE CONSTRAINT TRIGGER game_npc_cash_consistent AFTER INSERT ON game_npc_fee_cash_records DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_npc_cash_consistent();
DROP TRIGGER IF EXISTS game_npc_cash_immutable ON game_npc_fee_cash_records;
CREATE TRIGGER game_npc_cash_immutable BEFORE UPDATE OR DELETE ON game_npc_fee_cash_records FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
DROP TRIGGER IF EXISTS game_npc_cash_no_truncate ON game_npc_fee_cash_records;
CREATE TRIGGER game_npc_cash_no_truncate BEFORE TRUNCATE ON game_npc_fee_cash_records EXECUTE FUNCTION game_item_reject_truncate();

-- Extend existing proof functions without replacing any other owner's guard chain.
DO $migration$
DECLARE n TEXT; body TEXT; arm TEXT; begin_at INT;
BEGIN
 FOREACH n IN ARRAY ARRAY['game_item_fee_burn_consistency_guard','game_item_nested_entry_proven','game_equipment_slot_proven','game_equipment_state_proven'] LOOP
  body:=pg_get_functiondef((n||'()')::regprocedure);
  IF position('game_npc_fee_cash_records' IN body)>0 THEN CONTINUE; END IF;
  IF n='game_item_fee_burn_consistency_guard' THEN
   arm:=$arm$
 IF NEW.cause_kind IN (473,474) THEN
  PERFORM game_npc_cash_complete(NEW.transaction_id); RETURN NULL;
 END IF;
$arm$;
  ELSIF n='game_item_nested_entry_proven' THEN
   arm:=$arm$
 IF TG_OP='DELETE' AND EXISTS(SELECT 1 FROM game_npc_fee_cash_records r,LATERAL jsonb_array_elements(r.inputs) l
  WHERE r.created_xact_id=pg_current_xact_id() AND r.world_id=OLD.world_id AND (l->>'surface')::int=2 AND (l->>'item')::uuid=OLD.item_instance_id
   AND (l->>'parent')::uuid=OLD.parent_item_instance_id AND (l->>'ordinal')::numeric=OLD.placement_ordinal AND (l->>'burned')::bigint=(l->'before'->>'quantity')::bigint) THEN
  PERFORM game_npc_cash_complete(r.transaction_id) FROM game_npc_fee_cash_records r,LATERAL jsonb_array_elements(r.inputs) l
   WHERE r.created_xact_id=pg_current_xact_id() AND (l->>'item')::uuid=OLD.item_instance_id; RETURN NULL;
 END IF;
$arm$;
  ELSIF n='game_equipment_slot_proven' THEN
   arm:=$arm$
 IF TG_OP='DELETE' AND EXISTS(SELECT 1 FROM game_npc_fee_cash_records r,LATERAL jsonb_array_elements(r.inputs) l
  WHERE r.created_xact_id=pg_current_xact_id() AND r.character_id=OLD.character_id AND r.world_id=OLD.world_id AND r.equipment_before IS NOT NULL
   AND (l->>'surface')::int=3 AND (l->>'item')::uuid=OLD.item_instance_id AND (l->>'slot')::smallint=OLD.slot AND (l->>'burned')::bigint=(l->'before'->>'quantity')::bigint) THEN
  PERFORM game_npc_cash_complete(r.transaction_id) FROM game_npc_fee_cash_records r WHERE r.created_xact_id=pg_current_xact_id() AND r.character_id=OLD.character_id; RETURN NULL;
 END IF;
$arm$;
  ELSE
   arm:=$arm$
 IF EXISTS(SELECT 1 FROM game_npc_fee_cash_records r WHERE r.transaction_id=NEW.last_transaction_id AND r.character_id=NEW.character_id
  AND r.created_xact_id=pg_current_xact_id() AND r.equipment_before=to_jsonb(OLD)) THEN
  PERFORM game_npc_cash_complete(NEW.last_transaction_id); RETURN NULL;
 END IF;
$arm$;
  END IF;
  begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
  IF begin_at=0 THEN RAISE EXCEPTION 'missing carried coin guard anchor: %',n; END IF;
  EXECUTE overlay(body placing E'BEGIN\n'||arm from begin_at for 5);
 END LOOP;
END $migration$;


DO $npc_cash_causes$ DECLARE old TEXT;BEGIN
 SELECT pg_get_expr(conbin,conrelid)INTO STRICT old FROM pg_constraint WHERE conrelid='game_item_fee_burns'::regclass AND conname='game_item_fee_burns_cause_kind_check';
 ALTER TABLE game_item_fee_burns DROP CONSTRAINT game_item_fee_burns_cause_kind_check;
 EXECUTE 'ALTER TABLE game_item_fee_burns ADD CONSTRAINT game_item_fee_burns_cause_kind_check CHECK(('||old||')OR cause_kind IN(473,474))';
END $npc_cash_causes$;
DO $npc_cash_checks$ DECLARE n TEXT;old TEXT;BEGIN
 FOREACH n IN ARRAY ARRAY['game_item_fee_burns_line_count_check','game_item_fee_burns_bank_only']LOOP
 SELECT pg_get_expr(conbin,conrelid)INTO STRICT old FROM pg_constraint WHERE conrelid='game_item_fee_burns'::regclass AND conname=n;
 EXECUTE format('ALTER TABLE game_item_fee_burns DROP CONSTRAINT %I,ADD CONSTRAINT %I CHECK((%s)OR(cause_kind IN(473,474)AND line_count BETWEEN 0 AND 508 AND((line_count=0)=(burned_gold_units=0))AND(line_count>0 OR bank_debit_gold_units=fee_gold_units)))',n,n,old);
 END LOOP;
END $npc_cash_checks$;
REVOKE ALL ON game_npc_fee_cash_records FROM PUBLIC;
GRANT SELECT,INSERT ON game_npc_fee_cash_records TO oteryn_game_runtime;

REVOKE ALL ON FUNCTION game_npc_cash_complete(UUID) FROM PUBLIC,oteryn_game_control;
GRANT EXECUTE ON FUNCTION game_npc_cash_complete(UUID) TO oteryn_game_runtime;

-- Incremental 0993 amendment; the full NPC cash template also owns these bytes.
-- The old generic, Gem, NPC trade and all other audit-shape ceilings are intact.
DO $npc_cash_audit_shape$
DECLARE definition TEXT; at INTEGER; arm TEXT;
BEGIN
 definition:=pg_get_functiondef('game_item_audit_envelope_size_proven()'::regprocedure);
 IF strpos(definition,'SRV7_CLOSED_NPC_CASH_AUDIT')>0 THEN RAISE EXCEPTION 'NPC cash audit shape already installed';END IF;
 arm:=$arm$
 -- SRV7_CLOSED_NPC_CASH_AUDIT: source-qualified 508-leaf fee proof only.
 IF NEW.created_xact_id=pg_current_xact_id()
  AND octet_length(NEW.envelope)<=262144
  AND EXISTS(SELECT 1 FROM game_item_fee_burns fee
   JOIN game_npc_fee_cash_records cash USING(transaction_id)
   WHERE fee.transaction_id=NEW.transaction_id AND fee.event_id=NEW.event_id
    AND fee.cause_kind IN(473,474) AND fee.created_xact_id=pg_current_xact_id()
    AND cash.created_xact_id=pg_current_xact_id() AND cash.character_id=fee.character_id
    AND cash.world_id=fee.world_id AND fee.envelope_sha256=NEW.envelope_sha256
    AND NEW.envelope_sha256=sha256(NEW.envelope)) THEN
  PERFORM game_npc_cash_complete(NEW.transaction_id);
  RETURN NULL;
 END IF;
$arm$;
 at:=regexp_instr(definition,E'\\mBEGIN\\M',1,1,0,'i');
 IF at=0 THEN RAISE EXCEPTION 'NPC cash audit shape boundary changed';END IF;
 EXECUTE overlay(definition placing E'BEGIN\n'||arm from at for 5);
END $npc_cash_audit_shape$;

-- New closed fee473/474 owns its exact current-XID physical coin mutation.
-- Every older Item/House/Gem proof arm remains byte-for-byte below this arm.
DO $npc_cash_item_change$
DECLARE definition TEXT;at INTEGER;arm TEXT;
BEGIN
 definition:=pg_get_functiondef('game_item_instance_change_proven()'::regprocedure);
 IF strpos(definition,'SRV7_CLOSED_NPC_COIN_CHANGE')>0 THEN RAISE EXCEPTION 'NPC coin change proof already installed';END IF;
 arm:=$arm$
 -- SRV7_CLOSED_NPC_COIN_CHANGE: choose by the closed cause, never by a
 -- nullable proof predicate that could fall through to another owner.
 IF EXISTS(SELECT 1 FROM game_item_fee_burns fee
  WHERE fee.transaction_id=NEW.last_transaction_id AND fee.cause_kind IN(473,474)
   AND fee.created_xact_id=pg_current_xact_id()) THEN
  PERFORM game_npc_cash_complete(NEW.last_transaction_id);
  IF NOT EXISTS(SELECT 1 FROM game_npc_fee_cash_records cash,
    LATERAL jsonb_array_elements(cash.inputs) line
   WHERE cash.transaction_id=NEW.last_transaction_id AND cash.created_xact_id=pg_current_xact_id()
    AND line->>'item'=OLD.item_instance_id::text AND line->'before'=to_jsonb(OLD)
    AND (line->>'burned')::bigint>0
    AND NEW.item_instance_id=OLD.item_instance_id AND NEW.world_id=OLD.world_id
    AND NEW.quantity=OLD.quantity-(line->>'burned')::bigint
    AND NEW.lifecycle=CASE WHEN NEW.quantity=0 THEN 2 ELSE 1 END
    AND NEW.state_revision=OLD.state_revision+1
    AND (to_jsonb(NEW)-ARRAY['quantity','lifecycle','last_transaction_id','state_revision'])
       =(to_jsonb(OLD)-ARRAY['quantity','lifecycle','last_transaction_id','state_revision'])
    AND EXISTS(SELECT 1 FROM game_item_instances actual
      WHERE actual.item_instance_id=NEW.item_instance_id AND to_jsonb(actual)=to_jsonb(NEW))) THEN
   RAISE EXCEPTION 'NPC coin update lacks its exact sealed before/final successor' USING ERRCODE='23514';
  END IF;
  RETURN NULL;
 END IF;
$arm$;
 at:=regexp_instr(definition,E'\\mBEGIN\\M',1,1,0,'i');
 IF at=0 THEN RAISE EXCEPTION 'NPC item change boundary changed';END IF;
 EXECUTE overlay(definition placing E'BEGIN\n'||arm from at for 5);
END $npc_cash_item_change$;

-- BEGIN npc-cash-indexed-custody.sql
-- Closed NPC coin custody indexes a successful full proof once per statement.
-- All29 existing footprint invalidations delete the parent memo and cascade
-- these tuples. These rows grant no authority without the fresh full-proof call.
CREATE TABLE game_npc_cash_verified_inputs(
 backend_pid INTEGER NOT NULL,fee_transaction_id UUID NOT NULL,item_instance_id UUID NOT NULL,
 transaction_xid XID8 NOT NULL,world_id UUID NOT NULL,parent_item_instance_id UUID,
 placement_ordinal NUMERIC NOT NULL,surface SMALLINT NOT NULL CHECK(surface IN(1,2,3)),
 slot SMALLINT NOT NULL,quantity_before BIGINT NOT NULL CHECK(quantity_before>0),
 burned BIGINT NOT NULL CHECK(burned>=0 AND burned<=quantity_before),
 coin_key TEXT NOT NULL,coin_worth BIGINT NOT NULL CHECK(coin_worth IN(1,100,10000)),
 item_before JSONB NOT NULL,item_after JSONB NOT NULL,
 PRIMARY KEY(backend_pid,fee_transaction_id,item_instance_id),
 FOREIGN KEY(backend_pid,fee_transaction_id) REFERENCES game_npc_cash_verified(backend_pid,fee_transaction_id) ON DELETE CASCADE
);
REVOKE ALL ON game_npc_cash_verified_inputs FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;

DO $indexed_custody_capture$ DECLARE definition TEXT;marker TEXT;capture TEXT;BEGIN
 definition:=pg_get_functiondef('game_npc_cash_complete(uuid)'::regprocedure);
 marker:='SET transaction_xid=EXCLUDED.transaction_xid,statement_started=EXCLUDED.statement_started;';
 IF strpos(definition,marker)=0 OR strpos(definition,'game_npc_cash_verified_inputs')>0 THEN
  RAISE EXCEPTION 'NPC cash proof indexed-capture boundary changed';END IF;
 capture:=$capture$
 -- Index exact sealed custody facts only after every physical and source proof.
 DELETE FROM game_npc_cash_verified_inputs WHERE backend_pid=pg_backend_pid() AND fee_transaction_id=p_transaction;
 INSERT INTO game_npc_cash_verified_inputs(backend_pid,fee_transaction_id,item_instance_id,transaction_xid,world_id,
  parent_item_instance_id,placement_ordinal,surface,slot,quantity_before,burned,coin_key,coin_worth,item_before,item_after)
 SELECT pg_backend_pid(),p_transaction,(input->>'item')::uuid,pg_current_xact_id(),r.world_id,
  (input->>'parent')::uuid,(input->>'ordinal')::numeric,(input->>'surface')::smallint,(input->>'slot')::smallint,
  (input->'before'->>'quantity')::bigint,(input->>'burned')::bigint,input->'before'->>'definition_production_key',
  game_item_coin_worth(input->'before'->>'definition_production_key'),input->'before',
  (SELECT to_jsonb(actual) FROM game_item_instances actual WHERE actual.item_instance_id=(input->>'item')::uuid)
 FROM jsonb_array_elements(r.inputs) input;
$capture$;
 EXECUTE replace(definition,marker,marker||capture);
END $indexed_custody_capture$;

CREATE FUNCTION game_npc_cash_nested_delete_proven(p_item UUID,p_world UUID,p_parent UUID,p_ordinal NUMERIC)
RETURNS VOID LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE fee UUID;
BEGIN
 -- Select by actual closed fee cause, before any nullable proof predicate.
 SELECT f.transaction_id INTO fee FROM game_item_instances i JOIN game_item_fee_burns f ON f.transaction_id=i.last_transaction_id
 WHERE i.item_instance_id=p_item AND f.cause_kind IN(473,474) AND f.created_xact_id=pg_current_xact_id();
 IF NOT FOUND THEN RAISE EXCEPTION 'NPC nested custody requires its exact closed current fee' USING ERRCODE='23514';END IF;
 -- This ALWAYS reproves the current source/parent/Root; a private memo only
 -- eliminates repeated physical scans after that independently fresh admission.
 PERFORM game_npc_cash_complete(fee);
 IF NOT EXISTS(SELECT 1 FROM game_npc_cash_verified_inputs proof
  JOIN game_npc_cash_verified memo USING(backend_pid,fee_transaction_id)
  JOIN game_item_instances actual ON actual.item_instance_id=proof.item_instance_id
  WHERE proof.backend_pid=pg_backend_pid() AND proof.fee_transaction_id=fee AND proof.item_instance_id=p_item
  AND proof.transaction_xid=pg_current_xact_id() AND memo.transaction_xid=pg_current_xact_id()
  AND memo.statement_started=statement_timestamp() AND proof.world_id=p_world AND actual.world_id=p_world
  AND proof.surface=2 AND proof.parent_item_instance_id=p_parent AND proof.placement_ordinal=p_ordinal
  AND proof.burned=proof.quantity_before AND proof.burned>0
  AND actual.definition_family='Item' AND actual.definition_production_key=proof.coin_key
  AND game_item_coin_worth(actual.definition_production_key)=proof.coin_worth
  AND actual.last_transaction_id=fee AND actual.quantity=0 AND actual.lifecycle=2) THEN
  RAISE EXCEPTION 'NPC nested delete lacks its exact sealed OLD custody, full burn and coin class' USING ERRCODE='23514';END IF;
END $$;
ALTER FUNCTION game_npc_cash_nested_delete_proven(uuid,uuid,uuid,numeric) SET search_path=pg_catalog,public,pg_temp;
REVOKE ALL ON FUNCTION game_npc_cash_nested_delete_proven(uuid,uuid,uuid,numeric) FROM PUBLIC,oteryn_game_control;
GRANT EXECUTE ON FUNCTION game_npc_cash_nested_delete_proven(uuid,uuid,uuid,numeric) TO oteryn_game_runtime;

DO $indexed_custody_guard$ DECLARE definition TEXT;old TEXT;replacement TEXT;BEGIN
 definition:=pg_get_functiondef('game_item_nested_entry_proven()'::regprocedure);
 old:=$old$
 IF TG_OP='DELETE' AND EXISTS(SELECT 1 FROM game_npc_fee_cash_records r,LATERAL jsonb_array_elements(r.inputs) l
  WHERE r.created_xact_id=pg_current_xact_id() AND r.world_id=OLD.world_id AND (l->>'surface')::int=2 AND (l->>'item')::uuid=OLD.item_instance_id
   AND (l->>'parent')::uuid=OLD.parent_item_instance_id AND (l->>'ordinal')::numeric=OLD.placement_ordinal AND (l->>'burned')::bigint=(l->'before'->>'quantity')::bigint) THEN
  PERFORM game_npc_cash_complete(r.transaction_id) FROM game_npc_fee_cash_records r,LATERAL jsonb_array_elements(r.inputs) l
   WHERE r.created_xact_id=pg_current_xact_id() AND (l->>'item')::uuid=OLD.item_instance_id; RETURN NULL;
 END IF;
$old$;
 replacement:=$replacement$
 IF TG_OP='DELETE' AND EXISTS(SELECT 1 FROM game_item_instances i JOIN game_item_fee_burns fee ON fee.transaction_id=i.last_transaction_id
  WHERE i.item_instance_id=OLD.item_instance_id AND fee.cause_kind IN(473,474) AND fee.created_xact_id=pg_current_xact_id()) THEN
  PERFORM game_npc_cash_nested_delete_proven(OLD.item_instance_id,OLD.world_id,OLD.parent_item_instance_id,OLD.placement_ordinal);
  RETURN NULL;
 END IF;
$replacement$;
 IF strpos(definition,old)=0 THEN RAISE EXCEPTION 'NPC nested exact custody arm changed';END IF;
 EXECUTE replace(definition,old,replacement);
END $indexed_custody_guard$;

CREATE FUNCTION game_npc_cash_item_change_proven(p_before JSONB,p_after JSONB)
RETURNS VOID LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE fee UUID;
BEGIN
 SELECT f.transaction_id INTO fee FROM game_item_fee_burns f
 WHERE f.transaction_id=(p_after->>'last_transaction_id')::uuid AND f.cause_kind IN(473,474)
 AND f.created_xact_id=pg_current_xact_id();
 IF NOT FOUND THEN RAISE EXCEPTION 'NPC item change requires its exact current closed fee' USING ERRCODE='23514';END IF;
 PERFORM game_npc_cash_complete(fee);
 IF NOT EXISTS(SELECT 1 FROM game_npc_cash_verified_inputs proof
  JOIN game_npc_cash_verified memo USING(backend_pid,fee_transaction_id)
  JOIN game_item_instances actual ON actual.item_instance_id=proof.item_instance_id
  WHERE proof.backend_pid=pg_backend_pid() AND proof.fee_transaction_id=fee
  AND proof.item_instance_id=(p_before->>'item_instance_id')::uuid
  AND proof.transaction_xid=pg_current_xact_id() AND memo.transaction_xid=pg_current_xact_id()
  AND memo.statement_started=statement_timestamp() AND proof.burned>0
  AND proof.item_before=p_before AND proof.item_after=p_after AND to_jsonb(actual)=p_after
  AND actual.world_id=proof.world_id AND actual.definition_family='Item'
  AND actual.definition_production_key=proof.coin_key AND game_item_coin_worth(actual.definition_production_key)=proof.coin_worth
  AND actual.last_transaction_id=fee) THEN
  RAISE EXCEPTION 'NPC item change lacks its exact complete sealed OLD/NEW/class/world snapshot' USING ERRCODE='23514';END IF;
END $$;
ALTER FUNCTION game_npc_cash_item_change_proven(jsonb,jsonb) SET search_path=pg_catalog,public,pg_temp;
REVOKE ALL ON FUNCTION game_npc_cash_item_change_proven(jsonb,jsonb) FROM PUBLIC,oteryn_game_control;
GRANT EXECUTE ON FUNCTION game_npc_cash_item_change_proven(jsonb,jsonb) TO oteryn_game_runtime;

-- Replace only the already closed NPC arm. All recovery/exclusivity and all
-- older mutation owners remain untouched; whole Row OLD/NEW equality is retained.
DO $indexed_item_change$ DECLARE definition TEXT;old TEXT;replacement TEXT;BEGIN
 definition:=pg_get_functiondef('game_item_instance_change_proven()'::regprocedure);
 old:=$old$
 IF EXISTS(SELECT 1 FROM game_item_fee_burns fee
  WHERE fee.transaction_id=NEW.last_transaction_id AND fee.cause_kind IN(473,474)
   AND fee.created_xact_id=pg_current_xact_id()) THEN
  PERFORM game_npc_cash_complete(NEW.last_transaction_id);
  IF NOT EXISTS(SELECT 1 FROM game_npc_fee_cash_records cash,
    LATERAL jsonb_array_elements(cash.inputs) line
   WHERE cash.transaction_id=NEW.last_transaction_id AND cash.created_xact_id=pg_current_xact_id()
    AND line->>'item'=OLD.item_instance_id::text AND line->'before'=to_jsonb(OLD)
    AND (line->>'burned')::bigint>0
    AND NEW.item_instance_id=OLD.item_instance_id AND NEW.world_id=OLD.world_id
    AND NEW.quantity=OLD.quantity-(line->>'burned')::bigint
    AND NEW.lifecycle=CASE WHEN NEW.quantity=0 THEN 2 ELSE 1 END
    AND NEW.state_revision=OLD.state_revision+1
    AND (to_jsonb(NEW)-ARRAY['quantity','lifecycle','last_transaction_id','state_revision'])
       =(to_jsonb(OLD)-ARRAY['quantity','lifecycle','last_transaction_id','state_revision'])
    AND EXISTS(SELECT 1 FROM game_item_instances actual
      WHERE actual.item_instance_id=NEW.item_instance_id AND to_jsonb(actual)=to_jsonb(NEW))) THEN
   RAISE EXCEPTION 'NPC coin update lacks its exact sealed before/final successor' USING ERRCODE='23514';
  END IF;
  RETURN NULL;
 END IF;
$old$;
 replacement:=$replacement$
 IF EXISTS(SELECT 1 FROM game_item_fee_burns fee
  WHERE fee.transaction_id=NEW.last_transaction_id AND fee.cause_kind IN(473,474)
   AND fee.created_xact_id=pg_current_xact_id()) THEN
  PERFORM game_npc_cash_item_change_proven(to_jsonb(OLD),to_jsonb(NEW));
  RETURN NULL;
 END IF;
$replacement$;
 IF strpos(definition,old)=0 THEN RAISE EXCEPTION 'NPC exact item before/after arm changed';END IF;
 EXECUTE replace(definition,old,replacement);
END $indexed_item_change$;

-- END npc-cash-indexed-custody.sql
