-- Successful native pair operations mint one Exaltation Chest and move their
-- surviving operands into it. The original item, fee, dust and audit checks
-- remain in charge of the same fenced physical transaction.


CREATE OR REPLACE FUNCTION game_forge_operation_admit() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE r game_forge_reservations%ROWTYPE; i game_item_instances%ROWTYPE; l JSONB; n INT:=0; key TEXT;
BEGIN
 SELECT * INTO STRICT r FROM game_forge_reservations WHERE character_id=NEW.character_id AND occurrence_id=NEW.occurrence_id FOR UPDATE;
 IF r.work_units_used=0 OR r.transaction_id<>NEW.transaction_id OR r.event_id<>NEW.event_id
    OR r.occurred_at<>NEW.occurred_at OR get_byte(r.intent,0)<>NEW.operation
    OR NOT EXISTS(SELECT 1 FROM game_character_roots WHERE character_id=NEW.character_id AND world_id=r.world_id AND lifecycle=1)
 THEN RAISE EXCEPTION 'Forge operation requires its current native reservation' USING ERRCODE='23514'; END IF;
 NEW.created_xact_id:=pg_current_xact_id();
 IF NOT NEW.applied THEN
  IF NEW.inputs<>'[]'::jsonb OR NEW.outputs<>'[]'::jsonb OR NEW.stash IS NOT NULL
    OR NEW.dust_spent<>0 OR NEW.gold_spent<>0 OR NEW.audit_proof IS NOT NULL OR NEW.envelope_sha256 IS NOT NULL
  THEN RAISE EXCEPTION 'terminal refusal cannot carry effects' USING ERRCODE='23514'; END IF;
  IF NEW.operation<>7 AND (NEW.reply->'applied' IS DISTINCT FROM 'false'::jsonb
     OR NEW.reply->'succeeded' IS DISTINCT FROM 'false'::jsonb OR NEW.reply->'result' IS DISTINCT FROM 'null'::jsonb
     OR jsonb_typeof(NEW.reply->'reason') IS DISTINCT FROM 'string') THEN
   RAISE EXCEPTION 'terminal refusal requires a closed refusal reply' USING ERRCODE='23514'; END IF;
  RETURN NEW;
 END IF;
 IF NEW.operation=7 THEN RAISE EXCEPTION 'DustLimit composes its existing receipt' USING ERRCODE='23514'; END IF;
 IF r.exhausted OR NEW.dust_before IS NULL OR NEW.dust_limit IS NULL OR NEW.audit_proof IS NULL
    OR NOT (NEW.reply ?& ARRAY['operation','succeeded','applied','reason','bonus','result'])
    OR NEW.reply->'applied' IS DISTINCT FROM 'true'::jsonb OR jsonb_typeof(NEW.reply->'succeeded') IS DISTINCT FROM 'boolean'
    OR jsonb_typeof(NEW.reply->'result') IS DISTINCT FROM 'object'
    OR NOT (NEW.reply->'result' ?& ARRAY['first_item','first_tier','second_item','second_tier','dust','dust_limit','gold_spent','cores_spent','slivers_spent','slivers_created','cores_created'])
    OR EXISTS(SELECT 1 FROM unnest(ARRAY['dust','dust_limit','gold_spent','cores_spent','slivers_spent','slivers_created','cores_created']) k
       WHERE jsonb_typeof(NEW.reply->'result'->k) IS DISTINCT FROM 'number')
    OR NEW.reply->'result'->>'gold_spent' IS DISTINCT FROM NEW.gold_spent::text
    OR (NEW.reply->'result'->>'dust')::bigint<>NEW.dust_before-NEW.dust_spent
    OR (NEW.reply->'result'->>'dust_limit')::smallint<>NEW.dust_limit
    OR NEW.dust_spent>NEW.dust_before
    OR NEW.dust_limit NOT BETWEEN 100 AND 4600 OR (NEW.dust_limit-100)%20<>0
    OR NEW.dust_before<>(SELECT coalesce((SELECT balance FROM game_character_forge_dust WHERE character_id=NEW.character_id),0))
    OR NEW.dust_limit<>(SELECT coalesce((SELECT dust_limit FROM game_character_forge_dust WHERE character_id=NEW.character_id),100))
    OR (SELECT count(DISTINCT value->>'item') FROM jsonb_array_elements(NEW.inputs))<>jsonb_array_length(NEW.inputs)
 THEN RAISE EXCEPTION 'invalid native Forge action proof' USING ERRCODE='23514'; END IF;
 FOR l IN SELECT value FROM jsonb_array_elements(NEW.inputs) LOOP
  n:=n+1;
  IF jsonb_typeof(l) IS DISTINCT FROM 'object' OR NOT(l ?& ARRAY['item','before','location','quantity_after','tier_before','tier_after'])
     OR jsonb_typeof(l->'item') IS DISTINCT FROM 'string' OR jsonb_typeof(l->'before') IS DISTINCT FROM 'object'
     OR jsonb_typeof(l->'location') IS DISTINCT FROM 'object'
     OR NOT(l->'location' ?& ARRAY['kind','parent','ordinal'])
     OR jsonb_typeof(l->'location'->'parent') IS DISTINCT FROM 'string' OR jsonb_typeof(l->'location'->'ordinal') IS DISTINCT FROM 'string'
     OR jsonb_typeof(l->'location'->'kind') IS DISTINCT FROM 'number'
     OR EXISTS(SELECT 1 FROM unnest(ARRAY['quantity_after','tier_before','tier_after']) k WHERE jsonb_typeof(l->k) IS DISTINCT FROM 'number')
  THEN RAISE EXCEPTION 'Forge input proof is incomplete' USING ERRCODE='23514'; END IF;
  SELECT * INTO i FROM game_item_instances WHERE item_instance_id=(l->>'item')::uuid FOR UPDATE;
  IF i.item_instance_id IS NULL OR to_jsonb(i) IS DISTINCT FROM l->'before' OR i.world_id<>r.world_id OR i.lifecycle<>1
    OR i.definition_family<>'Item' OR i.quantity<1
    OR NOT game_forge_carried(NEW.character_id,r.world_id,i.item_instance_id,l->'location')
    OR (l->>'tier_before')::int<>coalesce((SELECT tier FROM game_item_tiers WHERE item_instance_id=i.item_instance_id),0)
    OR (l->>'tier_after')::int NOT BETWEEN 0 AND 10 OR (l->>'quantity_after')::bigint NOT BETWEEN 0 AND 100
    OR ((l->>'quantity_after')::bigint=0 AND (l->>'tier_after')::int<>0)
    OR EXISTS(SELECT 1 FROM game_item_imbuements WHERE item_instance_id=i.item_instance_id)
  THEN RAISE EXCEPTION 'Forge input changed custody, state or tier' USING ERRCODE='23514'; END IF;
  IF NEW.operation BETWEEN 1 AND 4 AND n<=2 THEN
   IF i.quantity<>1 OR (l->>'quantity_after')::bigint NOT IN(0,1)
      OR (n=1 AND i.item_instance_id IS DISTINCT FROM r.first_item_instance_id)
      OR (n=2 AND i.item_instance_id IS DISTINCT FROM r.second_item_instance_id)
      OR EXISTS(SELECT 1 FROM game_item_nested_entries WHERE parent_item_instance_id=i.item_instance_id)
   THEN RAISE EXCEPTION 'Forge pair must match its distinct single-item intent' USING ERRCODE='23514'; END IF;
  ELSE
   key:=CASE WHEN NEW.operation=5 THEN 'oteryn:item.tibia.i37109'
             WHEN NEW.operation=6 AND (l->>'quantity_after')::bigint>i.quantity THEN 'oteryn:item.tibia.i37110'
             WHEN NEW.operation=6 THEN 'oteryn:item.tibia.i37109' ELSE 'oteryn:item.tibia.i37110' END;
   IF i.definition_production_key<>key OR (l->>'tier_before')::int<>0 OR (l->>'tier_after')::int<>0
      OR (NEW.operation BETWEEN 1 AND 4 AND (l->>'quantity_after')::bigint>=i.quantity)
      OR EXISTS(SELECT 1 FROM game_item_keys WHERE item_instance_id=i.item_instance_id)
      OR EXISTS(SELECT 1 FROM game_item_texts WHERE item_instance_id=i.item_instance_id)
      OR EXISTS(SELECT 1 FROM game_item_timed_states WHERE item_instance_id=i.item_instance_id)
      OR EXISTS(SELECT 1 FROM game_spell_item_temporal_schedules WHERE item_instance_id=i.item_instance_id)
      OR EXISTS(SELECT 1 FROM game_spell_field_temporal_schedules WHERE item_instance_id=i.item_instance_id)
      OR EXISTS(SELECT 1 FROM game_item_nested_entries WHERE parent_item_instance_id=i.item_instance_id)
   THEN RAISE EXCEPTION 'Forge material requires plain native resource units' USING ERRCODE='23514'; END IF;
  END IF;
 END LOOP;
 IF NEW.operation BETWEEN 1 AND 4 AND jsonb_array_length(NEW.inputs)<2 THEN
  RAISE EXCEPTION 'Forge pair missing' USING ERRCODE='23514'; END IF;
 IF NEW.operation BETWEEN 1 AND 4 AND jsonb_array_length(NEW.outputs)<>
      (CASE WHEN NEW.reply->'succeeded'='true'::jsonb THEN 1 ELSE 0 END) THEN
  RAISE EXCEPTION 'successful Forge pair requires exactly one chest' USING ERRCODE='23514'; END IF;
 FOR l IN SELECT value FROM jsonb_array_elements(NEW.outputs) LOOP
  IF jsonb_typeof(l) IS DISTINCT FROM 'object' OR NOT(l ?& ARRAY['item','key','revision','count','location'])
     OR jsonb_typeof(l->'item') IS DISTINCT FROM 'string' OR jsonb_typeof(l->'key') IS DISTINCT FROM 'string'
     OR jsonb_typeof(l->'revision') IS DISTINCT FROM 'string' OR jsonb_typeof(l->'count') IS DISTINCT FROM 'number'
     OR jsonb_typeof(l->'location') IS DISTINCT FROM 'object' OR NOT(l->'location' ?& ARRAY['kind','parent','ordinal'])
     OR jsonb_typeof(l->'location'->'parent') IS DISTINCT FROM 'string' OR jsonb_typeof(l->'location'->'ordinal') IS DISTINCT FROM 'string'
     OR jsonb_typeof(l->'location'->'kind') IS DISTINCT FROM 'number'
  THEN RAISE EXCEPTION 'Forge output proof is incomplete' USING ERRCODE='23514'; END IF;
  IF NEW.operation NOT BETWEEN 1 AND 6 OR NOT game_character_is_uuid_v7((l->>'item')::uuid)
    OR EXISTS(SELECT 1 FROM game_item_instances WHERE item_instance_id=(l->>'item')::uuid)
    OR l->>'key'<>(CASE WHEN NEW.operation BETWEEN 1 AND 4 THEN 'oteryn:item.tibia.i37561'
                        WHEN NEW.operation=5 THEN 'oteryn:item.tibia.i37109' ELSE 'oteryn:item.tibia.i37110' END)
    OR (NEW.operation BETWEEN 1 AND 4 AND l->>'revision'<>'definition-r1')
    OR (l->>'count')::int NOT BETWEEN 1 AND (CASE NEW.operation WHEN 5 THEN 3 ELSE 1 END)
    OR octet_length(l->>'revision') NOT BETWEEN 1 AND 512 OR (l->'location'->>'ordinal')::numeric<=0
    OR (l->'location'->>'kind')::int NOT IN(1,2)
  THEN RAISE EXCEPTION 'invalid fresh Forge resource output' USING ERRCODE='23514'; END IF;
 END LOOP;
 IF NEW.stash IS NOT NULL AND (NOT(NEW.stash ?& ARRAY['entry','key','revision','amount','before','after','previous'])
    OR EXISTS(SELECT 1 FROM unnest(ARRAY['entry','key','revision','previous']) k WHERE jsonb_typeof(NEW.stash->k) IS DISTINCT FROM 'string')
    OR EXISTS(SELECT 1 FROM unnest(ARRAY['amount','before','after']) k WHERE jsonb_typeof(NEW.stash->k) IS DISTINCT FROM 'number')
    OR (NEW.stash->>'amount')::bigint<=0 OR NEW.stash->>'key'<>(CASE WHEN NEW.operation=6 THEN 'oteryn:item.tibia.i37109' ELSE 'oteryn:item.tibia.i37110' END)
    OR NEW.operation=5) THEN RAISE EXCEPTION 'Forge stash proof is incomplete' USING ERRCODE='23514'; END IF;
 RETURN NEW;
END $$;

CREATE OR REPLACE FUNCTION game_forge_complete(p_transaction UUID) RETURNS void LANGUAGE plpgsql AS $$
DECLARE p game_forge_operations%ROWTYPE; r game_forge_reservations%ROWTYPE; i game_item_instances%ROWTYPE;
 l JSONB; f game_item_fee_burns%ROWTYPE; e game_character_stash_entries%ROWTYPE; expected_mints INT; chest UUID; n INT; expected_location JSONB;
BEGIN
 SELECT * INTO p FROM game_forge_operations WHERE transaction_id=p_transaction;
 SELECT * INTO r FROM game_forge_reservations WHERE character_id=p.character_id AND occurrence_id=p.occurrence_id;
 IF p.transaction_id IS NULL OR p.created_xact_id<>pg_current_xact_id() OR r.work_units_used=0 THEN
  RAISE EXCEPTION 'Forge proof must be this physical transaction' USING ERRCODE='23514'; END IF;
 IF NOT p.applied THEN
  IF EXISTS(SELECT 1 FROM game_item_instances WHERE last_transaction_id=p_transaction OR minted_transaction_id=p_transaction)
     OR EXISTS(SELECT 1 FROM game_character_forge_dust_entries WHERE transaction_id=p_transaction)
     OR EXISTS(SELECT 1 FROM game_item_fee_burns WHERE transaction_id=p_transaction)
     OR EXISTS(SELECT 1 FROM game_character_stash_entries WHERE transaction_id=p_transaction)
     OR EXISTS(SELECT 1 FROM game_item_audit_outbox WHERE transaction_id=p_transaction)
     OR EXISTS(SELECT 1 FROM game_forge_history WHERE character_id=p.character_id AND occurrence_id=p.occurrence_id)
  THEN RAISE EXCEPTION 'refused Forge command changed durable value' USING ERRCODE='23514'; END IF;
  RETURN;
 END IF;
 IF p.envelope_sha256 IS NULL OR (SELECT count(*) FROM game_forge_history WHERE character_id=p.character_id)>1000
    OR NOT EXISTS(SELECT 1 FROM game_forge_history h WHERE h.character_id=p.character_id
     AND h.occurrence_id=p.occurrence_id AND h.reply=p.reply AND h.created_xact_id=pg_current_xact_id())
    OR (SELECT count(*) FROM game_item_audit_outbox WHERE transaction_id=p_transaction)<>1
    OR NOT EXISTS(SELECT 1 FROM game_item_audit_outbox a WHERE a.event_id=p.event_id AND a.transaction_id=p_transaction
      AND a.envelope_sha256=p.envelope_sha256 AND sha256(a.envelope)=p.envelope_sha256
      AND a.occurred_at=p.occurred_at AND a.created_xact_id=pg_current_xact_id()
      AND a.transaction_ordinal=1 AND a.transaction_count=1 AND a.publication_state=1 AND a.published_at IS NULL)
 THEN RAISE EXCEPTION 'applied Forge action needs its exact event and history' USING ERRCODE='23514'; END IF;
 IF p.operation=7 THEN
  IF NOT EXISTS(SELECT 1 FROM game_forge_dust_limit_receipts d WHERE d.transaction_id=p_transaction
    AND d.character_id=p.character_id AND d.occurrence_id=p.occurrence_id AND d.outcome=1
    AND d.envelope_sha256=p.envelope_sha256 AND d.created_xact_id=pg_current_xact_id()
    AND p.reply=jsonb_build_object('operation','dust_limit','succeeded',true,
      'result',jsonb_build_object('dust',d.balance_after,'dust_limit',d.dust_limit_after)))
  THEN RAISE EXCEPTION 'DustLimit must reuse its admitted receipt' USING ERRCODE='23514'; END IF;
  RETURN;
 END IF;
 IF p.audit_proof IS NULL OR NOT EXISTS(SELECT 1 FROM game_item_audit_outbox a WHERE a.event_id=p.event_id
    AND position(p.audit_proof IN a.envelope)>0)
    OR p.reply->'result'->>'gold_spent' IS DISTINCT FROM p.gold_spent::text
 THEN RAISE EXCEPTION 'Forge event omitted its bounded native proof' USING ERRCODE='23514'; END IF;
 chest:=(p.outputs->0->>'item')::uuid;
 IF p.operation BETWEEN 1 AND 4 AND p.reply->'succeeded'='true'::jsonb THEN
  IF jsonb_array_length(p.outputs)<>1 OR p.outputs->0->>'key' IS DISTINCT FROM 'oteryn:item.tibia.i37561'
     OR (SELECT count(*) FROM game_item_nested_entries WHERE parent_item_instance_id=chest)
        <>(SELECT count(*) FROM jsonb_array_elements(p.inputs) WITH ORDINALITY e(value,ord)
             WHERE ord<=2 AND (value->>'quantity_after')::bigint=1)
     OR EXISTS(SELECT 1 FROM game_item_container_entries WHERE parent_item_instance_id=chest)
     OR EXISTS(SELECT 1 FROM game_item_corpse_container_entries WHERE parent_item_instance_id=chest)
  THEN RAISE EXCEPTION 'Forge chest must contain exactly its surviving operands' USING ERRCODE='23514'; END IF;
 ELSE
  chest:=NULL;
 END IF;
 FOR l,n IN SELECT value,ord FROM jsonb_array_elements(p.inputs) WITH ORDINALITY e(value,ord) LOOP
  expected_location:=l->'location';
  IF chest IS NOT NULL AND n<=2 AND (l->>'quantity_after')::bigint=1 THEN
   expected_location:=jsonb_build_object('kind',2,'parent',chest,'ordinal',n);
   IF NOT EXISTS(SELECT 1 FROM game_item_nested_entries c WHERE c.item_instance_id=(l->>'item')::uuid
       AND c.parent_item_instance_id=chest AND c.placement_ordinal=n AND c.placed_transaction_id=p_transaction)
   THEN RAISE EXCEPTION 'Forge chest must receive the same surviving ItemInstance' USING ERRCODE='23514'; END IF;
  END IF;
  SELECT * INTO i FROM game_item_instances WHERE item_instance_id=(l->>'item')::uuid;
  IF i.item_instance_id IS NULL OR i.quantity<>(l->>'quantity_after')::bigint
    OR i.lifecycle<>(CASE WHEN i.quantity=0 THEN 2 ELSE 1 END) OR i.last_transaction_id IS DISTINCT FROM p_transaction
    OR (to_jsonb(i)-ARRAY['quantity','lifecycle','last_transaction_id']) IS DISTINCT FROM ((l->'before')-ARRAY['quantity','lifecycle','last_transaction_id'])
    OR coalesce((SELECT tier FROM game_item_tiers WHERE item_instance_id=i.item_instance_id),0)<>(l->>'tier_after')::int
    OR (i.quantity=0 AND (EXISTS(SELECT 1 FROM game_item_container_entries WHERE item_instance_id=i.item_instance_id)
      OR EXISTS(SELECT 1 FROM game_item_nested_entries WHERE item_instance_id=i.item_instance_id)))
    OR (i.quantity>0 AND NOT game_forge_carried(p.character_id,r.world_id,i.item_instance_id,expected_location))
    OR ((l->>'tier_before')::int<>(l->>'tier_after')::int AND NOT EXISTS(
      SELECT 1 FROM game_item_tier_audit a WHERE a.item_instance_id=i.item_instance_id AND a.created_xact_id=pg_current_xact_id()
        AND coalesce(a.tier_before,0)=(l->>'tier_before')::int AND coalesce(a.tier_after,0)=(l->>'tier_after')::int))
  THEN RAISE EXCEPTION 'Forge input after-state, custody or tier differs' USING ERRCODE='23514'; END IF;
 END LOOP;
 FOR l IN SELECT value FROM jsonb_array_elements(p.outputs) LOOP
  SELECT * INTO i FROM game_item_instances WHERE item_instance_id=(l->>'item')::uuid;
  IF i.item_instance_id IS NULL OR i.world_id<>r.world_id OR i.definition_family<>'Item'
    OR i.definition_production_key<>l->>'key' OR i.definition_revision_ref<>l->>'revision'
    OR i.quantity<>(l->>'count')::bigint OR i.lifecycle<>1 OR i.minted_transaction_id IS DISTINCT FROM p_transaction
    OR i.last_transaction_id IS NOT NULL OR EXISTS(SELECT 1 FROM game_item_tiers WHERE item_instance_id=i.item_instance_id)
    OR NOT game_forge_carried(p.character_id,r.world_id,i.item_instance_id,l->'location')
    OR NOT (EXISTS(SELECT 1 FROM game_item_container_entries mint_custody WHERE mint_custody.item_instance_id=i.item_instance_id AND mint_custody.placed_transaction_id=p_transaction)
         OR EXISTS(SELECT 1 FROM game_item_nested_entries nested_mint_custody WHERE nested_mint_custody.item_instance_id=i.item_instance_id AND nested_mint_custody.placed_transaction_id=p_transaction))
  THEN RAISE EXCEPTION 'Forge resource mint or custody differs' USING ERRCODE='23514'; END IF;
  -- Both Forge placement arms return through this completion proof. Retain the
  -- existing item-loop per-container20 and carried-tree500 ceilings (0128),
  -- even when a runtime writer supplies a complete, internally valid receipt.
  -- Conversions keep their pre-consumption slot check. Crystal's Forge inserts
  -- the chest after removing the operands and resources.
  IF ((l->'location'->>'kind')::int=1 AND (SELECT count(*) FROM game_item_container_entries
        WHERE parent_item_instance_id=(l->'location'->>'parent')::uuid)
        +(SELECT count(*) FROM jsonb_array_elements(p.inputs) c WHERE chest IS NULL AND (c->>'quantity_after')::bigint=0
          AND (c->'location'->>'parent')::uuid=(l->'location'->>'parent')::uuid)>20)
     OR ((l->'location'->>'kind')::int=2 AND (SELECT count(*) FROM game_item_nested_entries
        WHERE parent_item_instance_id=(l->'location'->>'parent')::uuid)
        +(SELECT count(*) FROM jsonb_array_elements(p.inputs) c WHERE chest IS NULL AND (c->>'quantity_after')::bigint=0
          AND (c->'location'->>'parent')::uuid=(l->'location'->>'parent')::uuid)>20)
     OR game_item_loop_tree_size(p.character_id)>500
  THEN RAISE EXCEPTION 'Forge resource destination exceeds container or carried-tree ceilings' USING ERRCODE='23514'; END IF;
 END LOOP;
 IF game_forge_resource_delta(p,'oteryn:item.tibia.i37109') IS DISTINCT FROM
     (p.reply->'result'->>'slivers_created')::bigint-(p.reply->'result'->>'slivers_spent')::bigint
    OR game_forge_resource_delta(p,'oteryn:item.tibia.i37110') IS DISTINCT FROM
     (p.reply->'result'->>'cores_created')::bigint-(p.reply->'result'->>'cores_spent')::bigint
    OR (p.operation=5 AND (p.dust_spent<>60 OR (p.reply->'result'->>'slivers_created')::bigint<>3
        OR (p.reply->'result'->>'slivers_spent')::bigint<>0 OR (p.reply->'result'->>'cores_created')::bigint<>0 OR p.gold_spent<>0))
    OR (p.operation=6 AND (p.dust_spent<>0 OR (p.reply->'result'->>'slivers_spent')::bigint<>50
        OR (p.reply->'result'->>'cores_created')::bigint<>1 OR (p.reply->'result'->>'slivers_created')::bigint<>0 OR p.gold_spent<>0))
 THEN RAISE EXCEPTION 'Forge material conversion does not conserve native units' USING ERRCODE='23514'; END IF;
 IF (SELECT count(*) FROM game_character_forge_dust_entries WHERE transaction_id=p_transaction)<>(p.dust_spent>0)::int
    OR (p.dust_spent>0 AND NOT EXISTS(SELECT 1 FROM game_character_forge_dust_entries d
      JOIN game_character_forge_dust b USING(character_id) WHERE d.transaction_id=p_transaction AND d.entry_id=r.entry_id
      AND d.character_id=p.character_id AND d.cause='forge' AND d.cause_occurrence_id=p.occurrence_id
      AND d.kind=(CASE WHEN p.operation=5 THEN 'CONVERT' ELSE 'SPEND' END) AND d.amount=p.dust_spent AND d.lost_amount=0
      AND d.balance_before=p.dust_before AND d.balance_after=p.dust_before-p.dust_spent
      AND d.dust_limit_before=p.dust_limit AND d.dust_limit_after=p.dust_limit
      AND b.last_entry_id=d.entry_id AND b.balance=d.balance_after AND b.dust_limit=d.dust_limit_after
      AND d.created_xact_id=pg_current_xact_id()))
 THEN RAISE EXCEPTION 'Forge dust debit must use the existing ledger exactly once' USING ERRCODE='23514'; END IF;
 SELECT * INTO f FROM game_item_fee_burns WHERE transaction_id=p_transaction;
 IF (f.transaction_id IS NOT NULL)<>(p.gold_spent>0)
    OR (p.gold_spent>0 AND (f.cause_kind<>14 OR f.cause_occurrence_id<>p.occurrence_id OR f.character_id<>p.character_id
       OR f.world_id<>r.world_id OR f.channel_id<>r.channel_id OR f.fee_gold_units<>p.gold_spent
       OR f.event_id<>p.event_id OR f.envelope_sha256<>p.envelope_sha256 OR f.created_xact_id<>pg_current_xact_id()))
 THEN RAISE EXCEPTION 'Forge gold must use its native fee cause exactly once' USING ERRCODE='23514'; END IF;
 expected_mints:=jsonb_array_length(p.outputs)+coalesce((f.change_gold_units/100>0)::int+(f.change_gold_units%100>0)::int,0);
 IF (SELECT count(*) FROM game_item_instances WHERE minted_transaction_id=p_transaction)<>expected_mints
    OR (SELECT count(*) FROM game_item_instances WHERE last_transaction_id=p_transaction)<>
      jsonb_array_length(p.inputs)+(SELECT count(*) FROM game_item_fee_burn_lines WHERE transaction_id=p_transaction)
    OR EXISTS(SELECT 1 FROM game_item_fee_burn_lines fee_line,jsonb_array_elements(p.inputs) forge_input(value)
      WHERE fee_line.transaction_id=p_transaction AND fee_line.item_instance_id=(forge_input.value->>'item')::uuid)
 THEN RAISE EXCEPTION 'Forge operation borrowed or omitted an item effect' USING ERRCODE='23514'; END IF;
 SELECT * INTO e FROM game_character_stash_entries WHERE transaction_id=p_transaction;
 IF (e.entry_id IS NOT NULL)<>(p.stash IS NOT NULL) THEN RAISE EXCEPTION 'Forge stash effect mismatch' USING ERRCODE='23514'; END IF;
 IF p.stash IS NOT NULL THEN
  IF e.entry_id IS DISTINCT FROM (p.stash->>'entry')::uuid OR e.character_id<>p.character_id OR e.world_id<>r.world_id
    OR e.definition_key<>p.stash->>'key' OR e.definition_revision<>p.stash->>'revision'
    OR e.amount<>(p.stash->>'amount')::bigint OR e.quantity_before<>(p.stash->>'before')::bigint
    OR e.quantity_after<>(p.stash->>'after')::bigint OR e.quantity_after<>e.quantity_before-e.amount
    OR e.previous_entry_id IS DISTINCT FROM (p.stash->>'previous')::uuid OR e.created_xact_id<>pg_current_xact_id()
    OR NOT EXISTS(SELECT 1 FROM game_character_stash_balances b WHERE b.character_id=e.character_id AND b.world_id=e.world_id
      AND b.definition_key=e.definition_key AND b.definition_revision=e.definition_revision AND b.last_entry_id=e.entry_id AND b.quantity=e.quantity_after)
    OR NOT EXISTS(SELECT 1 FROM game_character_stash_operations s WHERE s.transaction_id=p_transaction
      AND s.action=4 AND s.outcome=2 AND s.game_session_id=p.game_session_id AND s.command_id=p.command_id
      AND s.character_id=p.character_id AND s.world_id=r.world_id AND s.event_id=p.event_id
      AND s.envelope_sha256=p.envelope_sha256 AND s.intent=sha256(r.intent) AND s.created_xact_id=pg_current_xact_id())
    OR EXISTS(SELECT 1 FROM game_character_stash_item_lines WHERE transaction_id=p_transaction)
    OR EXISTS(SELECT 1 FROM game_character_stash_audit_outbox WHERE transaction_id=p_transaction)
  THEN RAISE EXCEPTION 'Forge stash debit needs its exact native entry and common event' USING ERRCODE='23514'; END IF;
 END IF;
END $$;

CREATE OR REPLACE FUNCTION game_forge_transition_proven() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE p game_forge_operations%ROWTYPE; r game_forge_reservations%ROWTYPE;
 a JSONB; b JSONB; pa RECORD; pb RECORD; price RECORD; target INT; source INT; result_tier INT;
 source_tier INT:=0; source_quantity INT:=0; nominal_dust INT; nominal_cores INT:=0;
 dust INT; cores INT; gold BIGINT:=0; succeeded BOOLEAN:=true; bonus INT:=0;
 boost BOOLEAN; protect BOOLEAN; expected_reply JSONB; expected_proof JSONB; active BYTEA;
 first_handle NUMERIC; second_handle NUMERIC; retired BIGINT; tier_changes BIGINT;
BEGIN
 SELECT * INTO STRICT p FROM game_forge_operations WHERE character_id=NEW.character_id AND occurrence_id=NEW.occurrence_id;
 IF NOT p.applied OR p.operation=7 THEN RETURN NULL; END IF;
 SELECT * INTO STRICT r FROM game_forge_reservations WHERE character_id=p.character_id AND occurrence_id=p.occurrence_id;
 PERFORM pg_advisory_xact_lock_shared(hashtextextended('oteryn:content-activation:'||r.world_id::text||':'||r.channel_id::text,0));
 SELECT server_artifact_digest INTO active FROM game_content_activations
  WHERE world_id=r.world_id AND channel_id=r.channel_id ORDER BY activation_sequence DESC LIMIT 1;
 IF active IS DISTINCT FROM r.content_digest OR r.ruleset_revision<>'crystal-201189f-forge-v1'
    OR r.simulation_revision<>1 OR r.exhausted OR p.created_xact_id<>pg_current_xact_id()
    OR ARRAY[r.success_roll,r.protection_roll,r.bonus_roll] IS DISTINCT FROM game_forge_expected_rolls(r.character_id,r.occurrence_id) THEN
  RAISE EXCEPTION 'Forge applied proof has stale revisions or substituted draws' USING ERRCODE='23514'; END IF;
 IF EXISTS(SELECT 1 FROM jsonb_array_elements(p.inputs) WITH ORDINALITY AS e(l,n)
     WHERE (p.operation>=5 OR n>2) AND NOT game_forge_admitted_material(l->'before'->>'definition_production_key',l->'before'->>'definition_revision_ref'))
    OR EXISTS(SELECT 1 FROM jsonb_array_elements(p.outputs) l
        WHERE NOT game_forge_admitted_material(l->>'key',l->>'revision')
          AND NOT (p.operation BETWEEN 1 AND 4 AND p.reply->'succeeded'='true'::jsonb
            AND l->>'key'='oteryn:item.tibia.i37561' AND l->>'revision'='definition-r1'))
    OR (p.stash IS NOT NULL AND NOT game_forge_admitted_material(p.stash->>'key',p.stash->>'revision')) THEN
  RAISE EXCEPTION 'Forge material has no admitted native definition revision' USING ERRCODE='23514'; END IF;
 IF p.operation BETWEEN 1 AND 4 THEN
  a:=p.inputs->0; b:=p.inputs->1;
  SELECT * INTO pa FROM game_forge_admitted_profile(a->'before'->>'definition_production_key',a->'before'->>'definition_revision_ref');
  IF NOT FOUND THEN RAISE EXCEPTION 'Forge receiver has no admitted SQL policy projection' USING ERRCODE='23514'; END IF;
  SELECT * INTO pb FROM game_forge_admitted_profile(b->'before'->>'definition_production_key',b->'before'->>'definition_revision_ref');
  IF NOT FOUND THEN RAISE EXCEPTION 'Forge source has no admitted SQL policy projection' USING ERRCODE='23514'; END IF;
  target:=(a->>'tier_before')::int; source:=(b->>'tier_before')::int;
  IF pa.classification<>pb.classification OR target>pa.max_tier OR source>pb.max_tier
     OR (a->>'item')::uuid IS DISTINCT FROM r.first_item_instance_id
     OR (b->>'item')::uuid IS DISTINCT FROM r.second_item_instance_id THEN
   RAISE EXCEPTION 'Forge pair violates admitted classification, cap or identity' USING ERRCODE='23514'; END IF;
  boost:=(get_byte(r.intent,17)&4)<>0; protect:=(get_byte(r.intent,17)&8)<>0;
  IF p.operation IN(1,2) THEN
   IF target<>source OR target>=pa.max_tier
      OR (p.operation=1 AND (a->'before'->>'definition_production_key',a->'before'->>'definition_revision_ref')
           IS DISTINCT FROM (b->'before'->>'definition_production_key',b->'before'->>'definition_revision_ref'))
      OR (p.operation=2 AND (pa.classification<>4 OR pa.equipment_slot<>pb.equipment_slot)) THEN
    RAISE EXCEPTION 'Forge fusion pair violates its admitted shape' USING ERRCODE='23514'; END IF;
   result_tier:=target+1;
   IF p.operation=1 THEN
    nominal_cores:=boost::int+protect::int; succeeded:=r.success_roll<50+15*boost::int;
    IF succeeded THEN
     bonus:=CASE WHEN r.bonus_roll<7400 THEN 0 WHEN r.bonus_roll<9000 THEN 1 WHEN r.bonus_roll<9500 THEN 2
       WHEN r.bonus_roll<9525 THEN 3 WHEN r.bonus_roll<9550 THEN 4 WHEN r.bonus_roll<9950 THEN 5
       WHEN r.bonus_roll<9975 THEN 6 ELSE 7 END;
     IF bonus IN(4,5,6) THEN source_quantity:=1;
      source_tier:=CASE bonus WHEN 4 THEN greatest(target-1,0) WHEN 5 THEN target ELSE target+1 END; END IF;
    ELSIF protect AND r.protection_roll>=50 THEN bonus:=8; source_quantity:=1; source_tier:=target;
    ELSIF target>0 THEN source_quantity:=1; source_tier:=target-1;
    END IF;
   END IF;
  ELSE
   IF target<>0 OR source<(CASE WHEN p.operation=3 THEN 2 ELSE 1 END)
      OR (p.operation=3 AND pa.equipment_slot<>pb.equipment_slot)
      OR (p.operation=4 AND pa.classification<>4) THEN
    RAISE EXCEPTION 'Forge transfer pair violates its admitted shape' USING ERRCODE='23514'; END IF;
   result_tier:=source-CASE WHEN p.operation=3 THEN 1 ELSE 0 END;
   IF result_tier>pa.max_tier THEN RAISE EXCEPTION 'Forge transfer exceeds receiver cap' USING ERRCODE='23514'; END IF;
  END IF;
  SELECT * INTO price FROM game_forge_admitted_price(pa.classification,result_tier);
  IF NOT FOUND THEN RAISE EXCEPTION 'Forge result tier has no admitted price' USING ERRCODE='23514'; END IF;
  nominal_dust:=CASE p.operation WHEN 2 THEN 130 WHEN 4 THEN 160 ELSE 100 END;
  IF p.operation IN(3,4) THEN nominal_cores:=price.cores; END IF;
  gold:=CASE p.operation WHEN 2 THEN price.convergence_fusion WHEN 4 THEN price.convergence_transfer ELSE price.gold END;
  IF bonus=3 THEN gold:=0; END IF;
  dust:=CASE WHEN bonus=1 THEN 0 ELSE nominal_dust END;
  cores:=CASE WHEN bonus=2 THEN 0 ELSE nominal_cores END;
  IF NOT succeeded THEN result_tier:=target;
  ELSIF bonus=7 AND target+2<=pa.classification AND target+2<=pa.max_tier THEN result_tier:=target+2;
  END IF;
  IF (a->>'quantity_after')::int<>1 OR (a->>'tier_after')::int<>result_tier
     OR (b->>'quantity_after')::int<>source_quantity OR (b->>'tier_after')::int<>source_tier
     OR p.dust_before<nominal_dust
     OR game_forge_available_cores(p.character_id,r.world_id)+cores<nominal_cores THEN
   RAISE EXCEPTION 'Forge transition or nominal resources do not match its frozen outcome' USING ERRCODE='23514'; END IF;
  first_handle:=game_forge_wire_handle(r.intent,18); second_handle:=game_forge_wire_handle(r.intent,34);
 ELSE
  nominal_dust:=CASE WHEN p.operation=5 THEN 60 ELSE 0 END; dust:=nominal_dust; cores:=0;
 END IF;
 IF p.gold_spent<>gold OR p.dust_spent<>dust
    OR game_forge_resource_delta(p,'oteryn:item.tibia.i37110')<>(CASE WHEN p.operation=6 THEN 1 ELSE -cores END)
    OR game_forge_resource_delta(p,'oteryn:item.tibia.i37109')<>(CASE p.operation WHEN 5 THEN 3 WHEN 6 THEN -50 ELSE 0 END) THEN
  RAISE EXCEPTION 'Forge mandatory costs do not match its frozen outcome' USING ERRCODE='23514'; END IF;
 expected_reply:=jsonb_build_object('operation',CASE p.operation WHEN 1 THEN 'fusion' WHEN 2 THEN 'convergence_fusion'
   WHEN 3 THEN 'transfer' WHEN 4 THEN 'convergence_transfer' WHEN 5 THEN 'dust_to_slivers' ELSE 'slivers_to_cores' END,
  'succeeded',succeeded,'applied',true,'reason',NULL,'bonus',bonus,
  'result',jsonb_build_object('first_item',first_handle,'first_tier',result_tier,'second_item',second_handle,
    'second_tier',CASE WHEN source_quantity=1 THEN source_tier ELSE NULL END,'dust',p.dust_before-dust,
    'dust_limit',p.dust_limit,'gold_spent',gold,'cores_spent',cores,'slivers_spent',CASE WHEN p.operation=6 THEN 50 ELSE 0 END,
    'slivers_created',CASE WHEN p.operation=5 THEN 3 ELSE 0 END,'cores_created',CASE WHEN p.operation=6 THEN 1 ELSE 0 END));
 IF p.reply IS DISTINCT FROM expected_reply THEN RAISE EXCEPTION 'Forge reply differs from its proven native result' USING ERRCODE='23514'; END IF;
 SELECT count(*) FILTER(WHERE (l->>'quantity_after')::bigint=0),
        count(*) FILTER(WHERE (l->>'tier_before')::int<>(l->>'tier_after')::int)
  INTO retired,tier_changes FROM jsonb_array_elements(p.inputs) l;
 expected_proof:=jsonb_build_object('occurrence',game_forge_json_bytes(uuid_send(p.occurrence_id)),
  'character',game_forge_json_bytes(uuid_send(p.character_id)),'world',game_forge_json_bytes(uuid_send(r.world_id)),
  'operation',p.operation,'content',game_forge_json_bytes(r.content_digest),'ruleset',r.ruleset_revision,
  'simulation_revision',r.simulation_revision,'rolls',jsonb_build_array(r.success_roll,r.protection_roll,r.bonus_roll),'reply',p.reply,
  'inputs_hash',game_forge_json_bytes(sha256(convert_to(game_forge_canonical_json(p.inputs),'UTF8'))),
  'outputs_hash',game_forge_json_bytes(sha256(convert_to(game_forge_canonical_json(p.outputs),'UTF8'))),
  'stash_hash',game_forge_json_bytes(sha256(convert_to(game_forge_canonical_json(coalesce(p.stash,'null'::jsonb)),'UTF8'))),
  'inputs',jsonb_array_length(p.inputs),'outputs',jsonb_array_length(p.outputs),'retired',retired,'tier_changes',tier_changes,
  'dust_spent',p.dust_spent,'gold_spent',p.gold_spent,'stash_spent',p.stash IS NOT NULL);
 IF convert_from(p.audit_proof,'UTF8')::jsonb IS DISTINCT FROM expected_proof THEN
  RAISE EXCEPTION 'Forge audit proof does not bind its exact native effects and reservation' USING ERRCODE='23514'; END IF;
 RETURN NULL;
END $$;


-- The reverse custody guards admit only receipt-bound moves of the first two
-- inputs. The deferred Forge proof checks the exact destination and contents.
DO $migration$
DECLARE n TEXT; body TEXT; arm TEXT; begin_at INT;
BEGIN
 FOREACH n IN ARRAY ARRAY['game_item_container_entry_removal_proven','game_item_nested_entry_proven'] LOOP
  body:=pg_get_functiondef((n||'()')::regprocedure);
  IF position('forge_exaltation_chest_3326' IN body)>0 THEN CONTINUE; END IF;
  begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
  IF begin_at=0 THEN RAISE EXCEPTION 'unexpected existing proof body: %',n; END IF;
  arm:=$arm$
   -- forge_exaltation_chest_3326
   IF TG_OP='DELETE' AND EXISTS(
      SELECT 1 FROM game_item_instances i JOIN game_forge_operations p ON p.transaction_id=i.last_transaction_id
      CROSS JOIN LATERAL jsonb_array_elements(p.inputs) WITH ORDINALITY e(l,ord)
      WHERE i.item_instance_id=OLD.item_instance_id AND i.world_id=OLD.world_id
        AND p.created_xact_id=pg_current_xact_id() AND p.applied AND p.operation BETWEEN 1 AND 4
        AND p.reply->'succeeded'='true'::jsonb AND ord<=2
        AND (l->>'item')::uuid=OLD.item_instance_id AND (l->>'quantity_after')::bigint=1
        AND (l->'location'->>'kind')::int=$kind$
        AND (l->'location'->>'parent')::uuid=OLD.parent_item_instance_id
        AND (l->'location'->>'ordinal')::numeric=OLD.placement_ordinal$character$
   ) THEN RETURN NULL; END IF;
$arm$;
  IF n='game_item_container_entry_removal_proven' THEN
   arm:=replace(replace(arm,'$kind$','1'),'$character$',' AND p.character_id=OLD.character_id');
  ELSE
   arm:=replace(replace(arm,'$kind$','2'),'$character$','') || $arm$
   IF TG_OP='INSERT' AND EXISTS(
      SELECT 1 FROM game_forge_operations p
      CROSS JOIN LATERAL jsonb_array_elements(p.inputs) WITH ORDINALITY e(l,ord)
      WHERE p.transaction_id=NEW.placed_transaction_id AND p.created_xact_id=pg_current_xact_id()
        AND p.applied AND p.operation BETWEEN 1 AND 4 AND p.reply->'succeeded'='true'::jsonb
        AND ord<=2 AND (l->>'quantity_after')::bigint=1 AND (l->>'item')::uuid=NEW.item_instance_id
        AND (p.outputs->0->>'item')::uuid=NEW.parent_item_instance_id AND NEW.placement_ordinal=ord
   ) THEN PERFORM game_forge_complete(NEW.placed_transaction_id); RETURN NULL; END IF;
$arm$;
  END IF;
  EXECUTE overlay(body placing E'BEGIN\n'||arm from begin_at for 5);
 END LOOP;
 FOREACH n IN ARRAY ARRAY['game_forge_operation_admit()','game_forge_complete(uuid)',
                          'game_forge_transition_proven()'] LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',n,current_schema());
 END LOOP;
END $migration$;
