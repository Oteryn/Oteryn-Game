-- Explicit additive bulk shape. Every old CHECK and guard remains unchanged for purchase_v2=false.
ALTER TABLE game_npc_trades ADD COLUMN purchase_v2 BOOLEAN NOT NULL DEFAULT false;
DO $$ DECLARE c RECORD; extension TEXT; BEGIN
 FOR c IN SELECT conname,pg_get_constraintdef(oid) AS definition FROM pg_constraint
  WHERE conrelid='game_npc_trades'::regclass AND contype='c' AND
   (conname IN('game_npc_trades_quantity_check','game_npc_trades_item_units_check','game_npc_trades_sell_all_shape','game_npc_trades_conservation','game_npc_trades_line_count_check','game_npc_trade_subtype_exclusive') OR pg_get_constraintdef(oid) LIKE '%total_gold_units = (unit_price * quantity)%') LOOP
  extension:=CASE c.conname
   WHEN 'game_npc_trade_subtype_exclusive' THEN 'full_charges IS NULL OR fluid_subtype IS NULL'
   WHEN 'game_npc_trades_quantity_check' THEN 'quantity BETWEEN 1 AND 10000'
   WHEN 'game_npc_trades_item_units_check' THEN 'item_units BETWEEN 1 AND 10000'
   WHEN 'game_npc_trades_line_count_check' THEN 'line_count BETWEEN 1 AND 150'
   ELSE 'side=1 AND NOT sell_all AND bank_debit_gold_units=0 AND burned_gold_units-minted_gold_units=total_gold_units AND line_count BETWEEN 1 AND 150' END;
  EXECUTE format('ALTER TABLE game_npc_trades DROP CONSTRAINT %I',c.conname);
  EXECUTE format('ALTER TABLE game_npc_trades ADD CONSTRAINT %I CHECK ((NOT purchase_v2 AND %s) OR (purchase_v2 AND %s))',c.conname,substr(c.definition,7),extension);
 END LOOP;
END $$;
ALTER TABLE game_npc_trade_lines DROP CONSTRAINT game_npc_trade_lines_line_ordinal_check,
 ADD CONSTRAINT game_npc_trade_lines_line_ordinal_check CHECK(line_ordinal BETWEEN 1 AND 150);

CREATE TABLE game_npc_purchase_details (
 transaction_id UUID PRIMARY KEY REFERENCES game_npc_trades(transaction_id),
 in_backpacks BOOLEAN NOT NULL, ignore_cap BOOLEAN NOT NULL,
 stack_size INTEGER NOT NULL CHECK(stack_size BETWEEN 1 AND 100),
 bag_count INTEGER NOT NULL CHECK(bag_count BETWEEN 0 AND 128),
 response_text TEXT NOT NULL CHECK(octet_length(response_text) BETWEEN 1 AND 1024),
 created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id()
);
CREATE TABLE game_npc_purchase_outputs (
 transaction_id UUID NOT NULL REFERENCES game_npc_purchase_details(transaction_id),
 purchase_index INTEGER NOT NULL CHECK(purchase_index BETWEEN 0 AND 127),
 item_instance_id UUID NOT NULL UNIQUE REFERENCES game_item_instances(item_instance_id) DEFERRABLE INITIALLY DEFERRED,
 parent_id UUID REFERENCES game_item_instances(item_instance_id) DEFERRABLE INITIALLY DEFERRED,
 placement_ordinal NUMERIC(20,0) NOT NULL CHECK(placement_ordinal BETWEEN 1 AND 18446744073709551615),
 quantity BIGINT NOT NULL CHECK(quantity BETWEEN 1 AND 100),
 definition_key TEXT NOT NULL, definition_revision TEXT NOT NULL,
 is_bag BOOLEAN NOT NULL,
 ground_position BYTEA, corpse_ref BYTEA, map_revision TEXT, content_revision TEXT, placement_context BYTEA,
 created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id(),
 PRIMARY KEY(transaction_id,purchase_index),
 CHECK((parent_id IS NOT NULL AND ground_position IS NULL AND corpse_ref IS NULL AND map_revision IS NULL AND content_revision IS NULL AND placement_context IS NULL)
    OR (parent_id IS NULL AND octet_length(ground_position)=10 AND corpse_ref IS NOT NULL AND map_revision IS NOT NULL AND content_revision IS NOT NULL AND placement_context IS NOT NULL))
);
CREATE INDEX game_npc_purchase_outputs_parent ON game_npc_purchase_outputs(parent_id);

-- Re-prove all new mints, their exact locations and the native coin fee at commit time.
CREATE FUNCTION game_npc_purchase_current(p_transaction UUID) RETURNS BOOLEAN
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE t game_npc_trades%ROWTYPE; p game_npc_purchase_details%ROWTYPE;
 stacks BIGINT; bags BIGINT; n BIGINT; burned BIGINT; minted BIGINT; last_worth BIGINT;
 o game_npc_purchase_outputs%ROWTYPE; expected_quantity BIGINT; bag_id UUID;
BEGIN
 SELECT * INTO t FROM game_npc_trades WHERE transaction_id=p_transaction;
 IF NOT FOUND OR NOT t.purchase_v2 OR t.side<>1 OR t.sell_all OR t.house_key IS NOT NULL OR t.bank_debit_gold_units<>0 OR t.created_xact_id<>pg_current_xact_id() THEN RETURN FALSE; END IF;
 SELECT * INTO p FROM game_npc_purchase_details WHERE transaction_id=p_transaction;
 IF NOT FOUND OR p.created_xact_id<>pg_current_xact_id() THEN RETURN FALSE; END IF;
 stacks:=(t.quantity+p.stack_size-1)/p.stack_size;
 bags:=CASE WHEN p.in_backpacks THEN (stacks+19)/20 ELSE 0 END;
 IF p.bag_count<>bags OR t.item_units<>t.quantity OR t.total_gold_units<>t.unit_price*t.quantity+20*bags
  OR NOT EXISTS(SELECT 1 FROM game_item_container_slots WHERE character_id=t.character_id AND item_instance_id=t.backpack_item_instance_id AND world_id=t.world_id)
  OR (SELECT count(*) FROM game_npc_purchase_outputs WHERE transaction_id=p_transaction)<>stacks+bags
  OR (SELECT max(purchase_index) FROM game_npc_purchase_outputs WHERE transaction_id=p_transaction)<>stacks+bags-1
  OR (SELECT count(*) FROM game_npc_trade_lines WHERE transaction_id=p_transaction)<>t.line_count
  OR (SELECT max(line_ordinal) FROM game_npc_trade_lines WHERE transaction_id=p_transaction)<>t.line_count
  OR (SELECT count(*) FROM game_npc_trade_lines WHERE transaction_id=p_transaction AND line_kind=4)<>stacks+bags
  OR EXISTS(SELECT 1 FROM game_npc_trade_lines WHERE transaction_id=p_transaction AND (line_kind=3 OR created_xact_id<>pg_current_xact_id()))
  OR (SELECT count(*) FROM game_item_instances WHERE minted_transaction_id=p_transaction)<>(SELECT count(*) FROM game_npc_trade_lines WHERE transaction_id=p_transaction AND line_kind IN(2,4))
 THEN RETURN FALSE; END IF;
 SELECT coalesce(sum((quantity_before-quantity_after)*coin_worth) FILTER(WHERE line_kind=1),0),coalesce(sum(quantity_after*coin_worth) FILTER(WHERE line_kind=2),0)
 INTO burned,minted FROM game_npc_trade_lines WHERE transaction_id=p_transaction;
 SELECT coin_worth INTO last_worth FROM game_npc_trade_lines WHERE transaction_id=p_transaction AND line_kind=1 ORDER BY line_ordinal DESC LIMIT 1;
 IF burned<>t.burned_gold_units OR minted<>t.minted_gold_units OR burned-minted<>t.total_gold_units
  OR (last_worth IS NULL AND t.total_gold_units<>0) OR minted>=coalesce(last_worth,1)
  OR EXISTS(SELECT 1 FROM game_npc_trade_lines a JOIN game_npc_trade_lines b USING(transaction_id) WHERE a.transaction_id=p_transaction AND a.line_ordinal<b.line_ordinal AND ((a.line_kind<>1 AND b.line_kind=1) OR (a.line_kind=4 AND b.line_kind<>4) OR (a.line_kind=2 AND b.line_kind=2 AND a.coin_worth<=b.coin_worth)))
  OR EXISTS(SELECT 1 FROM game_npc_trade_lines l JOIN game_item_instances i USING(item_instance_id)
   WHERE l.transaction_id=p_transaction AND (i.world_id<>t.world_id OR i.quantity<>l.quantity_after OR i.lifecycle<>CASE WHEN l.quantity_after=0 THEN 2 ELSE 1 END
    OR (l.line_kind IN(1,2) AND game_item_coin_worth(i.definition_production_key) IS DISTINCT FROM l.coin_worth)
    OR (l.line_kind=1 AND i.last_transaction_id IS DISTINCT FROM p_transaction)
    OR (l.line_kind IN(2,4) AND (i.minted_transaction_id<>p_transaction OR i.last_transaction_id IS NOT NULL))
    OR (l.line_kind=2 AND (l.coin_worth=10000 OR l.quantity_after>99))))
 THEN RETURN FALSE; END IF;
 IF EXISTS(SELECT 1 FROM game_npc_trade_lines l WHERE l.transaction_id=p_transaction AND l.line_kind=2
  AND NOT EXISTS(SELECT 1 FROM game_item_container_entries e WHERE e.item_instance_id=l.item_instance_id AND e.parent_item_instance_id=t.backpack_item_instance_id AND e.character_id=t.character_id AND e.world_id=t.world_id AND e.placement_ordinal=l.placement_ordinal AND e.placed_transaction_id=p_transaction)) THEN RETURN FALSE; END IF;
 n:=0;
 FOR o IN SELECT * FROM game_npc_purchase_outputs WHERE transaction_id=p_transaction ORDER BY purchase_index LOOP
  IF o.created_xact_id<>pg_current_xact_id() OR o.purchase_index<>n THEN RETURN FALSE; END IF;
  IF p.in_backpacks AND n%21=0 THEN
   IF NOT o.is_bag OR o.definition_key<>'oteryn:item.tibia.i21411' OR o.quantity<>1 THEN RETURN FALSE; END IF;
   bag_id:=o.item_instance_id;
  ELSE
   expected_quantity:=least(p.stack_size,t.quantity-(CASE WHEN p.in_backpacks THEN n-(n/21+1) ELSE n END)*p.stack_size);
   IF o.is_bag OR o.definition_key<>t.item_definition_production_key OR o.definition_revision<>t.item_definition_revision_ref OR o.quantity<>expected_quantity THEN RETURN FALSE; END IF;
   IF p.in_backpacks AND (o.parent_id IS DISTINCT FROM bag_id OR o.placement_ordinal<>(n%21)) THEN RETURN FALSE; END IF;
  END IF;
  IF coalesce((SELECT fluid_subtype FROM game_item_fluid_states WHERE item_instance_id=o.item_instance_id),0)<>(CASE WHEN o.is_bag THEN 0 ELSE coalesce(t.fluid_subtype,0) END) THEN RETURN FALSE; END IF;
  IF NOT EXISTS(SELECT 1 FROM game_npc_trade_lines l JOIN game_item_instances i USING(item_instance_id)
    WHERE l.transaction_id=p_transaction AND l.item_instance_id=o.item_instance_id AND l.line_kind=4 AND l.quantity_after=o.quantity AND l.placement_ordinal=o.placement_ordinal
     AND i.definition_family='Item' AND i.definition_production_key=o.definition_key AND i.definition_revision_ref=o.definition_revision AND i.state_revision=1 AND i.world_id=t.world_id) THEN RETURN FALSE; END IF;
  IF o.parent_id IS NOT NULL THEN
   IF o.parent_id=t.backpack_item_instance_id THEN
    IF NOT EXISTS(SELECT 1 FROM game_item_container_entries e WHERE e.item_instance_id=o.item_instance_id AND e.parent_item_instance_id=o.parent_id AND e.world_id=t.world_id AND e.character_id=t.character_id AND e.placement_ordinal=o.placement_ordinal AND e.placed_transaction_id=p_transaction) THEN RETURN FALSE; END IF;
   ELSE
    IF NOT EXISTS(SELECT 1 FROM game_item_nested_entries e WHERE e.item_instance_id=o.item_instance_id AND e.parent_item_instance_id=o.parent_id AND e.world_id=t.world_id AND e.placement_ordinal=o.placement_ordinal AND e.placed_transaction_id=p_transaction) THEN RETURN FALSE; END IF;
   END IF;
  ELSE
   IF NOT p.ignore_cap OR NOT EXISTS(SELECT 1 FROM game_item_ground_locations g WHERE g.item_instance_id=o.item_instance_id AND g.world_id=t.world_id AND g.channel_id=t.channel_id AND g.runtime_scope_ownership_generation=t.runtime_scope_ownership_generation AND g.spatial_position=o.ground_position AND g.corpse_ref=o.corpse_ref AND g.map_revision=o.map_revision AND g.content_revision=o.content_revision AND g.native_room_placement_context=o.placement_context) THEN RETURN FALSE; END IF;
   IF (SELECT count(*) FROM game_item_ground_locations WHERE world_id=t.world_id AND channel_id=t.channel_id AND spatial_position=o.ground_position)>30 THEN RETURN FALSE; END IF;
  END IF;
  n:=n+1;
 END LOOP;
 IF NOT EXISTS(SELECT 1 FROM game_item_audit_outbox a WHERE a.transaction_id=p_transaction AND a.event_id=t.event_id AND a.item_instance_id=t.item_instance_id AND a.envelope_sha256=t.envelope_sha256 AND a.created_xact_id=pg_current_xact_id()) THEN RETURN FALSE; END IF;
 RETURN TRUE;
END $$;
CREATE FUNCTION game_npc_purchase_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NOT game_npc_purchase_current(NEW.transaction_id) THEN RAISE EXCEPTION 'NPC bulk purchase requires exact same-transaction payment, mints and custody' USING ERRCODE='23514'; END IF;
 RETURN NULL;
END $$;
DO $$ DECLARE name TEXT; BEGIN
 FOREACH name IN ARRAY ARRAY['game_npc_purchase_details','game_npc_purchase_outputs'] LOOP
  EXECUTE format('CREATE TRIGGER %I BEFORE UPDATE OR DELETE ON %I FOR EACH ROW EXECUTE FUNCTION game_item_immutable()',name||'_immutable',name);
  EXECUTE format('CREATE TRIGGER %I BEFORE TRUNCATE ON %I FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate()',name||'_no_truncate',name);
  EXECUTE format('CREATE TRIGGER %I BEFORE INSERT ON %I FOR EACH ROW EXECUTE FUNCTION game_item_stamp_created_xact_id()',name||'_stamp',name);
  EXECUTE format('CREATE CONSTRAINT TRIGGER %I AFTER INSERT ON %I DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_npc_purchase_guard()',name||'_proven',name);
 END LOOP;
END $$;
-- New shape is admitted only with its own complete proof; original guard bodies are retained.
DO $$ DECLARE body TEXT; name TEXT; extension TEXT; BEGIN
 FOREACH name IN ARRAY ARRAY['game_npc_trade_consistency_guard','game_item_mint_consistency_guard','game_item_placement_proven','game_item_nested_entry_proven','game_item_ground_insertion_guard','game_npc_trade_fluid_guard'] LOOP
  SELECT pg_get_functiondef((name||'()')::regprocedure) INTO STRICT body;
  extension:=CASE name
   WHEN 'game_npc_trade_consistency_guard' THEN 'IF NEW.purchase_v2 THEN IF NOT game_npc_purchase_current(NEW.transaction_id) THEN RAISE EXCEPTION ''invalid bulk purchase'' USING ERRCODE=''23514''; END IF; RETURN NULL; END IF;'
   WHEN 'game_npc_trade_fluid_guard' THEN 'IF NEW.purchase_v2 THEN IF NOT game_npc_purchase_current(NEW.transaction_id) THEN RAISE EXCEPTION ''invalid bulk purchase'' USING ERRCODE=''23514''; END IF; RETURN NULL; END IF;'
   WHEN 'game_item_mint_consistency_guard' THEN 'IF EXISTS(SELECT 1 FROM game_npc_purchase_outputs o WHERE o.item_instance_id=NEW.item_instance_id AND o.transaction_id=NEW.minted_transaction_id AND game_npc_purchase_current(o.transaction_id)) THEN RETURN NULL; END IF;'
   WHEN 'game_item_ground_insertion_guard' THEN 'IF EXISTS(SELECT 1 FROM game_npc_purchase_outputs o JOIN game_npc_trades t USING(transaction_id) JOIN game_item_instances i ON i.item_instance_id=o.item_instance_id WHERE o.item_instance_id=NEW.item_instance_id AND t.purchase_v2 AND o.created_xact_id=pg_current_xact_id() AND t.created_xact_id=pg_current_xact_id() AND i.minted_transaction_id=t.transaction_id AND i.world_id=NEW.world_id AND i.lifecycle=1 AND o.ground_position=NEW.spatial_position AND o.corpse_ref=NEW.corpse_ref AND o.map_revision=NEW.map_revision AND o.content_revision=NEW.content_revision AND o.placement_context=NEW.native_room_placement_context) THEN RETURN NULL; END IF;'
   ELSE 'IF TG_OP=''INSERT'' AND EXISTS(SELECT 1 FROM game_npc_purchase_outputs o WHERE o.item_instance_id=NEW.item_instance_id AND o.transaction_id=NEW.placed_transaction_id AND game_npc_purchase_current(o.transaction_id)) THEN RETURN NULL; END IF;' END;
  IF strpos(body,'BEGIN')=0 THEN RAISE EXCEPTION 'missing bulk purchase guard extension anchor: %',name; END IF;
  EXECUTE regexp_replace(body,'BEGIN','BEGIN '||extension);
 END LOOP;
END $$;
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_npc_purchase_current(UUID) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON game_npc_purchase_details,game_npc_purchase_outputs FROM PUBLIC;
GRANT SELECT,INSERT ON game_npc_purchase_details,game_npc_purchase_outputs TO oteryn_game_runtime;
GRANT SELECT ON game_npc_purchase_details,game_npc_purchase_outputs TO oteryn_game_control;
REVOKE ALL ON FUNCTION game_npc_purchase_current(UUID),game_npc_purchase_guard() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_npc_purchase_current(UUID),game_npc_purchase_guard() TO oteryn_game_runtime;
