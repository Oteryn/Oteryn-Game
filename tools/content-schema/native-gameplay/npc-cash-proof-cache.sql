-- Incremental 0993 composition over the original cash block only. The full
-- npc-fee-carried-cash.sql already includes this proof cache; never concatenate
-- that full template with this incremental leaf when rebuilding a migration.
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


REVOKE ALL ON FUNCTION game_npc_cash_complete(UUID) FROM PUBLIC,oteryn_game_control;
GRANT EXECUTE ON FUNCTION game_npc_cash_complete(UUID) TO oteryn_game_runtime;
