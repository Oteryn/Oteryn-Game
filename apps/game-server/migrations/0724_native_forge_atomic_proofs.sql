-- P18 exact native effects and reverse proofs. Every admission remains tied to
-- one current-transaction operation; no general item/tier write exemption.
CREATE FUNCTION game_forge_resource_delta(p game_forge_operations,p_key TEXT)
RETURNS BIGINT LANGUAGE sql IMMUTABLE AS $$
 SELECT coalesce((SELECT sum((l->>'quantity_after')::bigint-(l->'before'->>'quantity')::bigint)
    FROM jsonb_array_elements(p.inputs) l WHERE l->'before'->>'definition_production_key'=p_key),0)
  +coalesce((SELECT sum((l->>'count')::bigint) FROM jsonb_array_elements(p.outputs) l WHERE l->>'key'=p_key),0)
  -CASE WHEN p.stash->>'key'=p_key THEN (p.stash->>'amount')::bigint ELSE 0 END
$$;

CREATE FUNCTION game_forge_complete(p_transaction UUID) RETURNS void LANGUAGE plpgsql AS $$
DECLARE p game_forge_operations%ROWTYPE; r game_forge_reservations%ROWTYPE; i game_item_instances%ROWTYPE;
 l JSONB; f game_item_fee_burns%ROWTYPE; e game_character_stash_entries%ROWTYPE; expected_mints INT;
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
 FOR l IN SELECT value FROM jsonb_array_elements(p.inputs) LOOP
  SELECT * INTO i FROM game_item_instances WHERE item_instance_id=(l->>'item')::uuid;
  IF i.item_instance_id IS NULL OR i.quantity<>(l->>'quantity_after')::bigint
    OR i.lifecycle<>(CASE WHEN i.quantity=0 THEN 2 ELSE 1 END) OR i.last_transaction_id IS DISTINCT FROM p_transaction
    OR (to_jsonb(i)-ARRAY['quantity','lifecycle','last_transaction_id']) IS DISTINCT FROM ((l->'before')-ARRAY['quantity','lifecycle','last_transaction_id'])
    OR coalesce((SELECT tier FROM game_item_tiers WHERE item_instance_id=i.item_instance_id),0)<>(l->>'tier_after')::int
    OR (i.quantity=0 AND (EXISTS(SELECT 1 FROM game_item_container_entries WHERE item_instance_id=i.item_instance_id)
      OR EXISTS(SELECT 1 FROM game_item_nested_entries WHERE item_instance_id=i.item_instance_id)))
    OR (i.quantity>0 AND NOT game_forge_carried(p.character_id,r.world_id,i.item_instance_id,l->'location'))
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
  -- A retired sliver may not supply the fresh core's slot: Q3 preflights room
  -- before consumption. Add those proven material leaves back to parent count.
  IF ((l->'location'->>'kind')::int=1 AND (SELECT count(*) FROM game_item_container_entries
        WHERE parent_item_instance_id=(l->'location'->>'parent')::uuid)
        +(SELECT count(*) FROM jsonb_array_elements(p.inputs) c WHERE (c->>'quantity_after')::bigint=0
          AND (c->'location'->>'parent')::uuid=(l->'location'->>'parent')::uuid)>20)
     OR ((l->'location'->>'kind')::int=2 AND (SELECT count(*) FROM game_item_nested_entries
        WHERE parent_item_instance_id=(l->'location'->>'parent')::uuid)
        +(SELECT count(*) FROM jsonb_array_elements(p.inputs) c WHERE (c->>'quantity_after')::bigint=0
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
CREATE FUNCTION game_forge_consistent() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN PERFORM game_forge_complete(NEW.transaction_id); RETURN NULL; END $$;
CREATE CONSTRAINT TRIGGER game_forge_consistent AFTER INSERT OR UPDATE ON game_forge_operations
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_forge_consistent();

-- Preserve the migration owner's existing audited tier-fixture/recovery facility
-- (0156), while granting the runtime only the exact native Forge cause.
CREATE FUNCTION game_forge_tier_guard() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE item UUID:=coalesce(NEW.item_instance_id,OLD.item_instance_id); p_transaction UUID;
BEGIN
 IF current_user=(SELECT pg_get_userbyid(relowner) FROM pg_class WHERE oid=TG_RELID) THEN RETURN coalesce(NEW,OLD); END IF;
 SELECT p.transaction_id INTO p_transaction FROM game_forge_operations p,jsonb_array_elements(p.inputs) l
  WHERE p.applied AND p.created_xact_id=pg_current_xact_id() AND (l->>'item')::uuid=item
    AND (l->>'tier_before')::int=CASE WHEN TG_OP='INSERT' THEN 0 ELSE OLD.tier END
    AND (l->>'tier_after')::int=CASE WHEN TG_OP='DELETE' THEN 0 ELSE NEW.tier END;
 IF p_transaction IS NULL THEN RAISE EXCEPTION 'runtime tier mutation requires its own Forge action' USING ERRCODE='23514'; END IF;
 RETURN coalesce(NEW,OLD);
END $$;
CREATE TRIGGER game_forge_tier_guard BEFORE INSERT OR UPDATE OR DELETE ON game_item_tiers
 FOR EACH ROW EXECUTE FUNCTION game_forge_tier_guard();
GRANT INSERT,UPDATE,DELETE ON game_item_tiers TO oteryn_game_runtime;

-- Add only Forge proof arms; keep every existing owner's body unchanged.
DO $migration$ DECLARE n TEXT; body TEXT; arm TEXT; begin_at INT; BEGIN
 FOREACH n IN ARRAY ARRAY['game_item_instance_guard','game_item_instance_change_proven',
  'game_item_container_entry_removal_proven','game_item_nested_entry_proven',
  'game_item_placement_proven','game_item_mint_consistency_guard'] LOOP
  body:=pg_get_functiondef((n||'()')::regprocedure);
  begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
  IF begin_at=0 THEN RAISE EXCEPTION 'unexpected existing proof body: %',n; END IF;
  IF n='game_item_instance_guard' THEN arm:=$arm$
   IF TG_OP='UPDATE' AND EXISTS(SELECT 1 FROM game_forge_operations p,jsonb_array_elements(p.inputs) l
      WHERE p.transaction_id=NEW.last_transaction_id AND p.applied AND p.created_xact_id=pg_current_xact_id()
        AND (l->>'item')::uuid=OLD.item_instance_id AND l->'before'=to_jsonb(OLD)
        AND (l->>'quantity_after')::bigint=NEW.quantity) THEN
    IF NEW.lifecycle<>(CASE WHEN NEW.quantity=0 THEN 2 ELSE 1 END)
       OR (to_jsonb(NEW)-ARRAY['quantity','lifecycle','last_transaction_id']) IS DISTINCT FROM
          (to_jsonb(OLD)-ARRAY['quantity','lifecycle','last_transaction_id'])
    THEN RAISE EXCEPTION 'Forge may change only its proven quantity and tier' USING ERRCODE='23514'; END IF;
    RETURN NEW;
   END IF;
$arm$;
  ELSIF n='game_item_instance_change_proven' THEN arm:=$arm$
   IF EXISTS(SELECT 1 FROM game_forge_operations p,jsonb_array_elements(p.inputs) l
      WHERE p.transaction_id=NEW.last_transaction_id AND p.applied AND p.created_xact_id=pg_current_xact_id()
        AND (l->>'item')::uuid=NEW.item_instance_id) THEN
    PERFORM game_forge_complete(NEW.last_transaction_id); RETURN NULL;
   END IF;
$arm$;
  ELSIF n='game_item_container_entry_removal_proven' THEN arm:=$arm$
   IF TG_OP='DELETE' AND EXISTS(SELECT 1 FROM game_forge_operations p,jsonb_array_elements(p.inputs) l
     JOIN game_item_instances i ON i.item_instance_id=(l->>'item')::uuid
     WHERE p.transaction_id=i.last_transaction_id AND p.created_xact_id=pg_current_xact_id() AND p.applied
       AND i.item_instance_id=OLD.item_instance_id AND i.quantity=0 AND i.lifecycle=2
       AND p.character_id=OLD.character_id AND (l->'location'->>'kind')::int=1
       AND (l->'location'->>'parent')::uuid=OLD.parent_item_instance_id AND (l->'location'->>'ordinal')::numeric=OLD.placement_ordinal)
   THEN RETURN NULL; END IF;
$arm$;
  ELSIF n='game_item_nested_entry_proven' THEN arm:=$arm$
   IF TG_OP='DELETE' AND EXISTS(SELECT 1 FROM game_forge_operations p,jsonb_array_elements(p.inputs) l
     JOIN game_item_instances i ON i.item_instance_id=(l->>'item')::uuid
     WHERE p.transaction_id=i.last_transaction_id AND p.created_xact_id=pg_current_xact_id() AND p.applied
       AND i.item_instance_id=OLD.item_instance_id AND i.quantity=0 AND i.lifecycle=2
       AND (l->'location'->>'kind')::int=2 AND (l->'location'->>'parent')::uuid=OLD.parent_item_instance_id
       AND (l->'location'->>'ordinal')::numeric=OLD.placement_ordinal) THEN RETURN NULL; END IF;
   IF TG_OP='INSERT' AND EXISTS(SELECT 1 FROM game_forge_operations p,jsonb_array_elements(p.outputs) l
     WHERE p.transaction_id=NEW.placed_transaction_id AND p.created_xact_id=pg_current_xact_id() AND p.applied
       AND (l->>'item')::uuid=NEW.item_instance_id AND (l->'location'->>'kind')::int=2
       AND (l->'location'->>'parent')::uuid=NEW.parent_item_instance_id AND (l->'location'->>'ordinal')::numeric=NEW.placement_ordinal)
   THEN PERFORM game_forge_complete(NEW.placed_transaction_id); RETURN NULL; END IF;
$arm$;
  ELSIF n='game_item_placement_proven' THEN arm:=$arm$
   IF TG_TABLE_NAME='game_item_container_entries' THEN
    IF EXISTS(SELECT 1 FROM game_forge_operations p,jsonb_array_elements(p.outputs) l
      WHERE p.transaction_id=NEW.placed_transaction_id AND p.created_xact_id=pg_current_xact_id() AND p.applied
        AND p.character_id=NEW.character_id AND (l->>'item')::uuid=NEW.item_instance_id
        AND (l->'location'->>'kind')::int=1 AND (l->'location'->>'parent')::uuid=NEW.parent_item_instance_id
        AND (l->'location'->>'ordinal')::numeric=NEW.placement_ordinal)
    THEN PERFORM game_forge_complete(NEW.placed_transaction_id); RETURN NULL; END IF;
   END IF;
$arm$;
  ELSE arm:=$arm$
   IF EXISTS(SELECT 1 FROM game_forge_operations p,jsonb_array_elements(p.outputs) l
     WHERE p.transaction_id=NEW.minted_transaction_id AND p.created_xact_id=pg_current_xact_id() AND p.applied
       AND (l->>'item')::uuid=NEW.item_instance_id) THEN
    PERFORM game_forge_complete(NEW.minted_transaction_id); RETURN NULL;
   END IF;
$arm$;
  END IF;
  EXECUTE overlay(body placing E'BEGIN\n'||arm from begin_at for 5);
 END LOOP;
END $migration$;

DO $$ DECLARE n TEXT; BEGIN
 FOREACH n IN ARRAY ARRAY['game_forge_resource_delta(game_forge_operations,text)','game_forge_complete(uuid)',
  'game_forge_consistent()','game_forge_tier_guard()'] LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',n,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC',n);
 END LOOP;
END $$;
GRANT EXECUTE ON FUNCTION game_forge_complete(uuid),game_forge_resource_delta(game_forge_operations,text) TO oteryn_game_runtime;
