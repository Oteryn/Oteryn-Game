-- P07: carried coin sources for Atelier only (Crystal 201189f game.cpp:2862-2969).
-- Native carried_inventory.rs:169-256 admits 500 tree nodes including the main bag and nine
-- equipment roots separately. Equipped-container descendants consume the same tree budget;
-- at most 508 leaves can be coins. The 510-node sentinel refuses excess without truncation.
-- Historical fee rows and every other fee cause retain the prior guard path and limits.
-- Immutable same-transaction evidence captures all eligible coins before any coin mutation.
CREATE TABLE IF NOT EXISTS game_atelier_fee_coin_records (
 transaction_id UUID PRIMARY KEY REFERENCES game_item_fee_burns(transaction_id),
 character_id UUID NOT NULL REFERENCES game_character_roots,
 world_id UUID NOT NULL,
 backpack_item_instance_id UUID REFERENCES game_item_instances,
 backpack_capacity SMALLINT NOT NULL CHECK(backpack_capacity BETWEEN 0 AND 20),
 backpack_ordinals JSONB NOT NULL CHECK(jsonb_typeof(backpack_ordinals)='array' AND jsonb_array_length(backpack_ordinals)<=20),
 inputs JSONB NOT NULL CHECK(jsonb_typeof(inputs)='array' AND jsonb_array_length(inputs)<=508),
 equipment_before JSONB CHECK(equipment_before IS NULL OR jsonb_typeof(equipment_before)='object'),
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
 CHECK((backpack_item_instance_id IS NULL)=(backpack_capacity=0))
);
-- FK lookups and deferred current-transaction evidence must not scan historical payments.
CREATE INDEX IF NOT EXISTS game_atelier_fee_coin_character_xact
 ON game_atelier_fee_coin_records(character_id,created_xact_id);
CREATE INDEX IF NOT EXISTS game_atelier_fee_coin_current_world
 ON game_atelier_fee_coin_records(created_xact_id,world_id);
CREATE INDEX IF NOT EXISTS game_atelier_fee_coin_backpack
 ON game_atelier_fee_coin_records(backpack_item_instance_id) WHERE backpack_item_instance_id IS NOT NULL;

CREATE OR REPLACE FUNCTION game_atelier_carried_coin_sources(p_character UUID,p_world UUID)
RETURNS TABLE(item_instance_id UUID,parent_item_instance_id UUID,placement_ordinal NUMERIC,surface SMALLINT,slot SMALLINT,selection_order BIGINT)
LANGUAGE plpgsql STABLE SECURITY DEFINER AS $$
DECLARE nodes JSONB;
BEGIN
 WITH RECURSIVE roots AS (
  SELECT s.item_instance_id,3 AS slot_order,0::smallint AS slot,0::smallint AS surface FROM game_item_container_slots s WHERE s.character_id=p_character AND s.world_id=p_world
  UNION ALL
  SELECT s.item_instance_id,CASE s.slot WHEN 1 THEN 1 WHEN 7 THEN 2 WHEN 2 THEN 4 WHEN 5 THEN 5 WHEN 6 THEN 6 WHEN 3 THEN 7 WHEN 4 THEN 8 WHEN 8 THEN 9 ELSE 10 END,s.slot,3::smallint
   FROM game_character_equipment_slots s WHERE s.character_id=p_character AND s.world_id=p_world
 ), carried AS (
  SELECT r.item_instance_id,NULL::uuid AS parent_item_instance_id,r.slot::numeric AS placement_ordinal,r.surface,r.slot,r.slot_order,0 AS depth,ARRAY[]::numeric[] AS path,ARRAY[r.item_instance_id] AS seen
   FROM roots r JOIN game_item_instances i USING(item_instance_id) WHERE i.world_id=p_world AND i.lifecycle=1
  UNION ALL
  SELECT e.item_instance_id,e.parent_item_instance_id,e.placement_ordinal,e.surface,0::smallint,c.slot_order,c.depth+1,c.path||e.placement_ordinal,c.seen||e.item_instance_id
   FROM carried c JOIN (
    SELECT e.item_instance_id,e.parent_item_instance_id,e.placement_ordinal,e.world_id,1::smallint AS surface FROM game_item_container_entries e WHERE e.character_id=p_character
    UNION ALL SELECT e.item_instance_id,e.parent_item_instance_id,e.placement_ordinal,e.world_id,2::smallint FROM game_item_nested_entries e
   ) e ON e.parent_item_instance_id=c.item_instance_id AND e.world_id=p_world
   JOIN game_item_instances i ON i.item_instance_id=e.item_instance_id AND i.world_id=p_world AND i.lifecycle=1
   WHERE c.depth<8 AND NOT e.item_instance_id=ANY(c.seen)
 ), bounded AS (SELECT c.* FROM carried c LIMIT 510)
 SELECT coalesce(jsonb_agg(to_jsonb(b) ORDER BY b.depth,b.slot_order,b.path DESC),'[]'::jsonb) INTO nodes FROM bounded b;
 IF jsonb_array_length(nodes)>509
  OR (SELECT count(*) FROM jsonb_array_elements(nodes) n WHERE (n->>'surface')::int<>3)>500
  OR (SELECT count(*) FROM jsonb_array_elements(nodes) n WHERE (n->>'surface')::int=3)>9
  OR EXISTS(SELECT 1 FROM jsonb_array_elements(nodes) n WHERE (n->>'depth')::int=8
   AND (EXISTS(SELECT 1 FROM game_item_nested_entries e WHERE e.parent_item_instance_id=(n->>'item_instance_id')::uuid)
     OR EXISTS(SELECT 1 FROM game_item_container_entries e WHERE e.parent_item_instance_id=(n->>'item_instance_id')::uuid))) THEN
  RAISE EXCEPTION 'Atelier carried inventory exceeds its existing node/depth bound' USING ERRCODE='23514';
 END IF;
 RETURN QUERY SELECT i.item_instance_id,(n->>'parent_item_instance_id')::uuid,(n->>'placement_ordinal')::numeric,
   (n->>'surface')::smallint,(n->>'slot')::smallint,ordinality
  FROM jsonb_array_elements(nodes) WITH ORDINALITY AS q(n,ordinality)
  JOIN game_item_instances i ON i.item_instance_id=(n->>'item_instance_id')::uuid
  WHERE (n->>'surface')::int<>0 AND i.definition_family='Item' AND game_item_coin_worth(i.definition_production_key) IS NOT NULL
   AND NOT EXISTS(SELECT 1 FROM game_item_container_entries e WHERE e.parent_item_instance_id=i.item_instance_id)
   AND NOT EXISTS(SELECT 1 FROM game_item_nested_entries e WHERE e.parent_item_instance_id=i.item_instance_id)
  ORDER BY ordinality;
END $$;

CREATE OR REPLACE FUNCTION game_atelier_coin_source_present(p_character UUID,p_world UUID,p_line JSONB) RETURNS BOOLEAN
LANGUAGE sql STABLE SECURITY DEFINER AS $$
 SELECT CASE (p_line->>'surface')::int
  WHEN 1 THEN EXISTS(SELECT 1 FROM game_item_container_entries e JOIN game_item_container_slots s ON s.item_instance_id=e.parent_item_instance_id
   WHERE e.item_instance_id=(p_line->>'item')::uuid AND e.parent_item_instance_id=(p_line->>'parent')::uuid AND e.placement_ordinal=(p_line->>'ordinal')::numeric
    AND e.character_id=p_character AND e.world_id=p_world AND s.character_id=p_character AND s.world_id=p_world AND (p_line->>'slot')::int=0)
  WHEN 2 THEN EXISTS(SELECT 1 FROM game_item_nested_entries e WHERE e.item_instance_id=(p_line->>'item')::uuid
   AND e.parent_item_instance_id=(p_line->>'parent')::uuid AND e.placement_ordinal=(p_line->>'ordinal')::numeric AND e.world_id=p_world AND (p_line->>'slot')::int=0
   AND game_atelier_carried_parent(p_character,p_world,e.parent_item_instance_id))
  WHEN 3 THEN EXISTS(SELECT 1 FROM game_character_equipment_slots s WHERE s.item_instance_id=(p_line->>'item')::uuid
   AND s.character_id=p_character AND s.world_id=p_world AND s.slot=(p_line->>'slot')::smallint AND (p_line->>'ordinal')::numeric=s.slot AND p_line->>'parent' IS NULL)
  ELSE false END
$$;

CREATE OR REPLACE FUNCTION game_atelier_coin_admit() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE line JSONB; item game_item_instances%ROWTYPE; current_equipment JSONB; material_equipment JSONB; current_sources JSONB;
BEGIN
 SELECT coalesce(jsonb_agg(to_jsonb(c)),'[]'::jsonb) INTO current_sources FROM game_atelier_carried_coin_sources(NEW.character_id,NEW.world_id) c;
 IF NOT EXISTS(SELECT 1 FROM game_item_fee_burns f WHERE f.transaction_id=NEW.transaction_id AND f.created_xact_id=pg_current_xact_id()
   AND f.cause_kind IN (9,10,11) AND f.character_id=NEW.character_id AND f.world_id=NEW.world_id
   AND f.backpack_item_instance_id IS NOT DISTINCT FROM NEW.backpack_item_instance_id)
  OR NEW.backpack_item_instance_id IS DISTINCT FROM (SELECT s.item_instance_id FROM game_item_container_slots s WHERE s.character_id=NEW.character_id AND s.world_id=NEW.world_id)
  OR NEW.backpack_ordinals IS DISTINCT FROM coalesce((SELECT jsonb_agg(e.placement_ordinal ORDER BY e.placement_ordinal DESC) FROM game_item_container_entries e WHERE e.parent_item_instance_id=NEW.backpack_item_instance_id),'[]'::jsonb)
  OR jsonb_array_length(NEW.inputs)<>jsonb_array_length(current_sources)
  OR (SELECT count(DISTINCT l->>'item') FROM jsonb_array_elements(NEW.inputs) l)<>jsonb_array_length(NEW.inputs) THEN
  RAISE EXCEPTION 'Atelier coin proof requires the current complete carried sources' USING ERRCODE='23514';
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
   RAISE EXCEPTION 'Atelier coin input is not its unchanged carried leaf' USING ERRCODE='23514';
  END IF;
 END LOOP;
 IF (NEW.equipment_before IS NOT NULL) IS DISTINCT FROM EXISTS(SELECT 1 FROM jsonb_array_elements(NEW.inputs) l WHERE (l->>'surface')::int=3 AND (l->>'burned')::int>0) THEN
  RAISE EXCEPTION 'Atelier coin equipment proof shape mismatch' USING ERRCODE='23514';
 END IF;
 IF NEW.equipment_before IS NOT NULL THEN
  SELECT to_jsonb(s) INTO current_equipment FROM game_character_equipment_state s WHERE s.character_id=NEW.character_id FOR UPDATE;
  SELECT a.equipment_before INTO material_equipment FROM game_atelier_item_records a WHERE a.transaction_id=NEW.transaction_id AND a.created_xact_id=pg_current_xact_id();
  IF (material_equipment IS NULL AND NEW.equipment_before IS DISTINCT FROM current_equipment)
    OR (material_equipment IS NOT NULL AND (NEW.equipment_before IS DISTINCT FROM material_equipment
      OR (current_equipment->>'revision')::numeric<>(material_equipment->>'revision')::numeric+1
      OR (current_equipment->>'last_transaction_id')::uuid IS DISTINCT FROM NEW.transaction_id)) THEN
   RAISE EXCEPTION 'Atelier coin equipment requires the original shared epoch' USING ERRCODE='23514';
  END IF;
 END IF;
 NEW.created_xact_id:=pg_current_xact_id(); RETURN NEW;
END $$;

CREATE OR REPLACE FUNCTION game_atelier_coin_complete(p_transaction UUID) RETURNS void LANGUAGE plpgsql AS $$
DECLARE r game_atelier_fee_coin_records%ROWTYPE; f game_item_fee_burns%ROWTYPE; line JSONB; item game_item_instances%ROWTYPE;
 burn game_item_fee_burn_lines%ROWTYPE; position INT:=0; remaining BIGINT; worth BIGINT; before_quantity BIGINT; amount BIGINT;
 first_item UUID; outputs INT; payer UUID; equipment JSONB; last_worth BIGINT;
BEGIN
 SELECT * INTO r FROM game_atelier_fee_coin_records WHERE transaction_id=p_transaction;
 SELECT * INTO f FROM game_item_fee_burns WHERE transaction_id=p_transaction;
 IF r.transaction_id IS NULL OR f.transaction_id IS NULL OR r.created_xact_id<>pg_current_xact_id() OR f.created_xact_id<>pg_current_xact_id()
  OR f.cause_kind NOT IN (9,10,11) OR f.character_id<>r.character_id OR f.world_id<>r.world_id OR f.backpack_item_instance_id IS DISTINCT FROM r.backpack_item_instance_id
  OR f.backpack_item_instance_id IS DISTINCT FROM (SELECT s.item_instance_id FROM game_item_container_slots s WHERE s.character_id=f.character_id AND s.world_id=f.world_id)
  OR NOT EXISTS(SELECT 1 FROM game_character_roots cr WHERE cr.character_id=f.character_id AND cr.world_id=f.world_id AND cr.character_revision=f.committed_character_revision AND cr.xmin=pg_current_xact_id()::xid)
  OR NOT EXISTS(SELECT 1 FROM game_character_atelier_receipts a WHERE a.character_id=f.character_id AND a.occurrence=f.cause_occurrence_id
   AND a.committed_character_revision=f.committed_character_revision AND a.fee_transaction_id=f.transaction_id AND a.created_xact_id=pg_current_xact_id()) THEN
  RAISE EXCEPTION 'Atelier coin payment requires its fresh Character receipt' USING ERRCODE='23514';
 END IF;
 remaining:=f.fee_gold_units-f.bank_debit_gold_units;
 FOR line IN SELECT l FROM jsonb_array_elements(r.inputs) l ORDER BY game_item_coin_worth(l->'before'->>'definition_production_key'),(l->'before'->>'quantity')::bigint,(l->>'selection_order')::bigint LOOP
  worth:=game_item_coin_worth(line->'before'->>'definition_production_key'); before_quantity:=(line->'before'->>'quantity')::bigint;
  amount:=least(before_quantity,(remaining+worth-1)/worth);
  IF amount IS DISTINCT FROM (line->>'burned')::bigint THEN RAISE EXCEPTION 'Atelier coins must follow donor denomination and stack-worth order' USING ERRCODE='23514'; END IF;
  SELECT * INTO STRICT item FROM game_item_instances WHERE item_instance_id=(line->>'item')::uuid;
  IF amount=0 THEN
   IF to_jsonb(item) IS DISTINCT FROM line->'before' OR NOT game_atelier_coin_source_present(r.character_id,r.world_id,line) THEN
    RAISE EXCEPTION 'unselected Atelier coins must remain unchanged' USING ERRCODE='23514';
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
   RAISE EXCEPTION 'Atelier coin burn lacks exact before/after custody' USING ERRCODE='23514';
  END IF;
  remaining:=greatest(remaining-amount*worth,0);
 END LOOP;
 IF remaining<>0 OR position<>f.line_count OR (SELECT count(*) FROM game_item_fee_burn_lines WHERE transaction_id=p_transaction)<>position
  OR (SELECT coalesce(sum((quantity_before-quantity_after)*coin_worth),0) FROM game_item_fee_burn_lines WHERE transaction_id=p_transaction)<>f.burned_gold_units
  OR (position>0 AND f.change_gold_units>=last_worth)
  OR (f.bank_debit_gold_units>0 AND EXISTS(SELECT 1 FROM jsonb_array_elements(r.inputs) l WHERE (l->>'burned')::bigint<>(l->'before'->>'quantity')::bigint)) THEN
  RAISE EXCEPTION 'Atelier coin payment is not the exact coins-first plan' USING ERRCODE='23514';
 END IF;
 SELECT account_id INTO STRICT payer FROM game_character_roots WHERE character_id=f.character_id;
 IF (SELECT count(*) FROM game_account_bank_entries WHERE fee_transaction_id=p_transaction)<>(f.bank_debit_gold_units>0)::int
  OR (f.bank_debit_gold_units>0 AND NOT EXISTS(SELECT 1 FROM game_account_bank_entries e WHERE e.fee_transaction_id=p_transaction AND e.kind=5
   AND e.amount=f.bank_debit_gold_units AND e.account_id=payer AND e.world_id=f.world_id AND e.acting_character_id=f.character_id AND e.created_xact_id=pg_current_xact_id())) THEN
  RAISE EXCEPTION 'Atelier bank residual requires its one exact ledger debit' USING ERRCODE='23514';
 END IF;
 outputs:=(f.change_gold_units/100>0)::int+(f.change_gold_units%100>0)::int;
 IF (SELECT count(*) FROM game_item_instances WHERE minted_transaction_id=p_transaction)<>outputs
  OR (SELECT count(*) FROM game_item_instances WHERE last_transaction_id=p_transaction)<>position+coalesce((SELECT jsonb_array_length(inputs) FROM game_atelier_item_records WHERE transaction_id=p_transaction),0)
  OR (SELECT count(*) FROM game_item_container_entries WHERE parent_item_instance_id=r.backpack_item_instance_id)>r.backpack_capacity
  OR (outputs>0 AND (r.backpack_item_instance_id IS NULL OR f.change_placement_ordinal<>1+coalesce((SELECT max(value::text::numeric) FROM jsonb_array_elements(r.backpack_ordinals)),0)))
  OR EXISTS(SELECT 1 FROM (VALUES (f.change_platinum_item_instance_id,'oteryn:item.tibia.i3035',f.change_gold_units/100,f.change_placement_ordinal),
    (f.change_gold_item_instance_id,'oteryn:item.tibia.i3031',f.change_gold_units%100,f.change_placement_ordinal+(f.change_gold_units/100>0)::int)) o(id,key,quantity,ordinal)
   WHERE o.quantity>0 AND NOT EXISTS(SELECT 1 FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id,world_id)
    WHERE i.item_instance_id=o.id AND i.world_id=f.world_id AND i.definition_family='Item' AND i.definition_production_key=o.key AND i.quantity=o.quantity AND i.lifecycle=1
     AND i.minted_transaction_id=p_transaction AND i.last_transaction_id IS NULL AND e.character_id=f.character_id AND e.parent_item_instance_id=r.backpack_item_instance_id
     AND e.placement_ordinal=o.ordinal AND e.placed_transaction_id=p_transaction)) THEN
  RAISE EXCEPTION 'Atelier change requires its bounded exact fresh main-backpack outputs' USING ERRCODE='23514';
 END IF;
 IF NOT EXISTS(SELECT 1 FROM game_item_audit_outbox a WHERE a.transaction_id=p_transaction AND a.event_id=f.event_id AND a.item_instance_id IS NOT DISTINCT FROM first_item
   AND a.occurred_at=f.occurred_at AND a.envelope_sha256=f.envelope_sha256 AND a.created_xact_id=pg_current_xact_id() AND a.publication_state=1 AND a.published_at IS NULL)
  OR (SELECT count(*) FROM game_item_audit_outbox WHERE transaction_id=p_transaction)<>1 THEN
  RAISE EXCEPTION 'Atelier coin payment requires one atomic audit event' USING ERRCODE='23514';
 END IF;
 IF r.equipment_before IS NOT NULL THEN
  SELECT to_jsonb(s) INTO equipment FROM game_character_equipment_state s WHERE s.character_id=r.character_id;
  IF equipment IS NULL OR (equipment->>'revision')::numeric<>(r.equipment_before->>'revision')::numeric+1 OR (equipment->>'last_transaction_id')::uuid IS DISTINCT FROM p_transaction
   OR (equipment-ARRAY['revision','last_transaction_id']) IS DISTINCT FROM (r.equipment_before-ARRAY['revision','last_transaction_id']) THEN
   RAISE EXCEPTION 'Atelier coin equipment requires one shared successor' USING ERRCODE='23514';
  END IF;
 END IF;
END $$;

CREATE OR REPLACE FUNCTION game_atelier_coin_consistent() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN PERFORM game_atelier_coin_complete(NEW.transaction_id); RETURN NULL; END $$;
DROP TRIGGER IF EXISTS game_atelier_coin_admit ON game_atelier_fee_coin_records;
CREATE TRIGGER game_atelier_coin_admit BEFORE INSERT ON game_atelier_fee_coin_records FOR EACH ROW EXECUTE FUNCTION game_atelier_coin_admit();
DROP TRIGGER IF EXISTS game_atelier_coin_consistent ON game_atelier_fee_coin_records;
CREATE CONSTRAINT TRIGGER game_atelier_coin_consistent AFTER INSERT ON game_atelier_fee_coin_records DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_atelier_coin_consistent();
DROP TRIGGER IF EXISTS game_atelier_coin_immutable ON game_atelier_fee_coin_records;
CREATE TRIGGER game_atelier_coin_immutable BEFORE UPDATE OR DELETE ON game_atelier_fee_coin_records FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
DROP TRIGGER IF EXISTS game_atelier_coin_no_truncate ON game_atelier_fee_coin_records;
CREATE TRIGGER game_atelier_coin_no_truncate BEFORE TRUNCATE ON game_atelier_fee_coin_records EXECUTE FUNCTION game_item_reject_truncate();

ALTER TABLE game_item_fee_burns DROP CONSTRAINT game_item_fee_burns_burned_gold_units_check;
ALTER TABLE game_item_fee_burns ADD CONSTRAINT game_item_fee_burns_burned_gold_units_check CHECK(burned_gold_units BETWEEN 0 AND CASE WHEN cause_kind IN (9,10,11) THEN 508000000 ELSE 20000000 END);
ALTER TABLE game_item_fee_burns DROP CONSTRAINT game_item_fee_burns_line_count_check;
ALTER TABLE game_item_fee_burns ADD CONSTRAINT game_item_fee_burns_line_count_check CHECK(line_count BETWEEN 0 AND CASE WHEN cause_kind IN (9,10,11) THEN 508 ELSE 20 END);
ALTER TABLE game_item_fee_burns DROP CONSTRAINT game_item_fee_burns_bank_only;
ALTER TABLE game_item_fee_burns ADD CONSTRAINT game_item_fee_burns_bank_only CHECK((line_count=0)=(burned_gold_units=0) AND (line_count>0 OR bank_debit_gold_units=fee_gold_units) AND (backpack_item_instance_id IS NOT NULL OR line_count=0 OR cause_kind IN (9,10,11)));
ALTER TABLE game_item_fee_burn_lines DROP CONSTRAINT game_item_fee_burn_lines_line_ordinal_check;
ALTER TABLE game_item_fee_burn_lines ADD CONSTRAINT game_item_fee_burn_lines_line_ordinal_check CHECK(line_ordinal BETWEEN 1 AND 508);

-- Extend existing proof functions without replacing any other owner's guard chain.
DO $migration$
DECLARE n TEXT; body TEXT; arm TEXT; begin_at INT;
BEGIN
 FOREACH n IN ARRAY ARRAY['game_item_fee_burn_consistency_guard','game_item_nested_entry_proven','game_equipment_slot_proven','game_equipment_state_proven'] LOOP
  body:=pg_get_functiondef((n||'()')::regprocedure);
  IF position('game_atelier_fee_coin_records' IN body)>0 THEN CONTINUE; END IF;
  IF n='game_item_fee_burn_consistency_guard' THEN
   arm:=$arm$
 IF NEW.cause_kind IN (9,10,11) AND EXISTS(SELECT 1 FROM game_atelier_fee_coin_records r WHERE r.transaction_id=NEW.transaction_id AND r.created_xact_id=pg_current_xact_id()) THEN
  PERFORM game_atelier_coin_complete(NEW.transaction_id); RETURN NULL;
 END IF;
$arm$;
  ELSIF n='game_item_nested_entry_proven' THEN
   arm:=$arm$
 IF TG_OP='DELETE' AND EXISTS(SELECT 1 FROM game_atelier_fee_coin_records r,LATERAL jsonb_array_elements(r.inputs) l
  WHERE r.created_xact_id=pg_current_xact_id() AND r.world_id=OLD.world_id AND (l->>'surface')::int=2 AND (l->>'item')::uuid=OLD.item_instance_id
   AND (l->>'parent')::uuid=OLD.parent_item_instance_id AND (l->>'ordinal')::numeric=OLD.placement_ordinal AND (l->>'burned')::bigint=(l->'before'->>'quantity')::bigint) THEN
  PERFORM game_atelier_coin_complete(r.transaction_id) FROM game_atelier_fee_coin_records r,LATERAL jsonb_array_elements(r.inputs) l
   WHERE r.created_xact_id=pg_current_xact_id() AND (l->>'item')::uuid=OLD.item_instance_id; RETURN NULL;
 END IF;
$arm$;
  ELSIF n='game_equipment_slot_proven' THEN
   arm:=$arm$
 IF TG_OP='DELETE' AND EXISTS(SELECT 1 FROM game_atelier_fee_coin_records r,LATERAL jsonb_array_elements(r.inputs) l
  WHERE r.created_xact_id=pg_current_xact_id() AND r.character_id=OLD.character_id AND r.world_id=OLD.world_id AND r.equipment_before IS NOT NULL
   AND (l->>'surface')::int=3 AND (l->>'item')::uuid=OLD.item_instance_id AND (l->>'slot')::smallint=OLD.slot AND (l->>'burned')::bigint=(l->'before'->>'quantity')::bigint) THEN
  PERFORM game_atelier_coin_complete(r.transaction_id) FROM game_atelier_fee_coin_records r WHERE r.created_xact_id=pg_current_xact_id() AND r.character_id=OLD.character_id; RETURN NULL;
 END IF;
$arm$;
  ELSE
   arm:=$arm$
 IF EXISTS(SELECT 1 FROM game_atelier_fee_coin_records r WHERE r.transaction_id=NEW.last_transaction_id AND r.character_id=NEW.character_id
  AND r.created_xact_id=pg_current_xact_id() AND r.equipment_before=to_jsonb(OLD)) THEN
  PERFORM game_atelier_coin_complete(NEW.last_transaction_id); RETURN NULL;
 END IF;
$arm$;
  END IF;
  begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
  IF begin_at=0 THEN RAISE EXCEPTION 'missing carried coin guard anchor: %',n; END IF;
  EXECUTE overlay(body placing E'BEGIN\n'||arm from begin_at for 5);
 END LOOP;
END $migration$;

REVOKE ALL ON game_atelier_fee_coin_records FROM PUBLIC;
GRANT SELECT,INSERT ON game_atelier_fee_coin_records TO oteryn_game_runtime;
DO $$ DECLARE n TEXT; BEGIN
 FOREACH n IN ARRAY ARRAY['game_atelier_carried_coin_sources(uuid,uuid)','game_atelier_coin_source_present(uuid,uuid,jsonb)','game_atelier_coin_admit()',
  'game_atelier_coin_complete(uuid)','game_atelier_coin_consistent()','game_item_fee_burn_consistency_guard()','game_item_nested_entry_proven()','game_equipment_slot_proven()','game_equipment_state_proven()'] LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',n,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC',n);
 END LOOP;
END $$;
GRANT EXECUTE ON FUNCTION game_atelier_carried_coin_sources(UUID,UUID),game_atelier_coin_source_present(UUID,UUID,JSONB),game_atelier_coin_complete(UUID) TO oteryn_game_runtime;
