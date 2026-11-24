-- An ordinary BUY can pay for and mint one 20-slot shopping bag in its existing transaction.
ALTER TABLE game_npc_trades ADD COLUMN shopping_bag_item_id UUID UNIQUE
  CHECK (shopping_bag_item_id IS NULL OR
    (side=1 AND NOT sell_all AND house_key IS NULL
     AND game_character_is_uuid_v7(shopping_bag_item_id)));

DO $migration$
DECLARE price_check TEXT;
BEGIN
 SELECT conname INTO STRICT price_check FROM pg_constraint
  WHERE conrelid='game_npc_trades'::regclass AND contype='c'
    AND pg_get_constraintdef(oid) LIKE '%total_gold_units = (unit_price * quantity)%';
 EXECUTE format('ALTER TABLE game_npc_trades DROP CONSTRAINT %I',price_check);
END $migration$;
ALTER TABLE game_npc_trades ADD CONSTRAINT game_npc_trade_price_with_packaging
  CHECK (total_gold_units = unit_price * quantity + CASE WHEN shopping_bag_item_id IS NULL THEN 0 ELSE 20 END);

CREATE FUNCTION game_npc_packed_purchase_current(p_transaction UUID) RETURNS BOOLEAN
LANGUAGE SQL VOLATILE AS $proof$
 SELECT EXISTS (
  SELECT 1 FROM game_npc_trades t
  JOIN game_item_instances bag ON bag.item_instance_id=t.shopping_bag_item_id
  JOIN game_item_container_entries e ON e.item_instance_id=bag.item_instance_id
  JOIN LATERAL (
    SELECT count(*) AS stacks,min(l.placement_ordinal) AS first_ordinal
    FROM game_npc_trade_lines l WHERE l.transaction_id=t.transaction_id AND l.line_kind=4
  ) a ON TRUE
  WHERE t.transaction_id=p_transaction AND t.created_xact_id=pg_current_xact_id()
    AND t.side=1 AND t.house_key IS NULL
    AND bag.minted_transaction_id=t.transaction_id AND bag.world_id=t.world_id
    AND bag.definition_family='Item' AND bag.definition_production_key='oteryn:item.tibia.i21411'
    AND bag.quantity=1 AND bag.lifecycle=1 AND bag.last_transaction_id IS NULL
    AND e.character_id=t.character_id AND e.parent_item_instance_id=t.backpack_item_instance_id
    AND e.world_id=t.world_id AND e.placed_transaction_id=t.transaction_id
    AND e.placement_ordinal=a.first_ordinal AND a.stacks BETWEEN 1 AND 20
    AND (SELECT count(*) FROM game_item_container_entries b
          WHERE b.parent_item_instance_id=t.backpack_item_instance_id)<=20
    AND game_item_loop_tree_size(t.character_id)<=500
    AND (SELECT count(*) FROM game_item_nested_entries n WHERE n.parent_item_instance_id=bag.item_instance_id)=a.stacks
    AND NOT EXISTS (
      SELECT 1 FROM game_npc_trade_lines l
      WHERE l.transaction_id=t.transaction_id AND l.line_kind=4
        AND NOT EXISTS (
          SELECT 1 FROM game_item_nested_entries n
          WHERE n.item_instance_id=l.item_instance_id AND n.world_id=t.world_id
            AND n.parent_item_instance_id=bag.item_instance_id AND n.placed_transaction_id=t.transaction_id
            AND n.placement_ordinal=l.placement_ordinal-a.first_ordinal+1
        )
    )
 )
$proof$;

DO $migration$
DECLARE body TEXT; name TEXT; arm TEXT; needle TEXT;
BEGIN
 EXECUTE format('ALTER FUNCTION game_npc_packed_purchase_current(uuid) SET search_path=pg_catalog,%I,pg_temp',current_schema());
 FOREACH name IN ARRAY ARRAY['game_item_mint_consistency_guard','game_item_nested_entry_proven',
                              'game_item_placement_proven','game_npc_trade_consistency_guard'] LOOP
  SELECT pg_get_functiondef(format('%I()',name)::regprocedure) INTO STRICT body;
  IF position('BEGIN' IN body)=0 THEN RAISE EXCEPTION 'missing NPC packed purchase guard entry %',name; END IF;
  CASE name
    WHEN 'game_item_mint_consistency_guard' THEN
      arm:=$arm$IF game_npc_packed_purchase_current(NEW.minted_transaction_id) AND EXISTS(
        SELECT 1 FROM game_npc_trades t WHERE t.transaction_id=NEW.minted_transaction_id
        AND (t.shopping_bag_item_id=NEW.item_instance_id OR EXISTS(
          SELECT 1 FROM game_npc_trade_lines l WHERE l.transaction_id=t.transaction_id
          AND l.item_instance_id=NEW.item_instance_id AND l.line_kind=4))) THEN RETURN NULL; END IF;$arm$;
    WHEN 'game_item_nested_entry_proven' THEN
      arm:=$arm$IF TG_OP='INSERT' THEN
        IF game_npc_packed_purchase_current(NEW.placed_transaction_id) AND EXISTS(
          SELECT 1 FROM game_npc_trade_lines l JOIN game_npc_trades t USING(transaction_id)
          WHERE l.transaction_id=NEW.placed_transaction_id AND l.item_instance_id=NEW.item_instance_id
          AND l.line_kind=4 AND t.shopping_bag_item_id=NEW.parent_item_instance_id)
        THEN RETURN NULL; END IF;
      END IF;$arm$;
    WHEN 'game_item_placement_proven' THEN
      arm:=$arm$IF TG_TABLE_NAME='game_item_container_entries' THEN
        IF game_npc_packed_purchase_current(NEW.placed_transaction_id) AND EXISTS(
          SELECT 1 FROM game_npc_trades t WHERE t.transaction_id=NEW.placed_transaction_id
          AND t.shopping_bag_item_id=NEW.item_instance_id) THEN RETURN NULL; END IF;
      END IF;$arm$;
    WHEN 'game_npc_trade_consistency_guard' THEN
      arm:=$arm$IF NEW.shopping_bag_item_id IS NOT NULL AND NOT game_npc_packed_purchase_current(NEW.transaction_id)
      THEN RAISE EXCEPTION 'packed NPC purchase lacks exact bag custody' USING ERRCODE='23514'; END IF;$arm$;
      needle:='WHERE i.minted_transaction_id = NEW.transaction_id';
      IF position(needle IN body)=0 THEN RAISE EXCEPTION 'missing NPC mint count'; END IF;
      body:=replace(body,needle,needle||' AND i.item_instance_id IS DISTINCT FROM NEW.shopping_bag_item_id');
      needle:='OR (l.quantity_after > 0 AND NOT EXISTS (';
      IF position(needle IN body)=0 THEN RAISE EXCEPTION 'missing NPC positive custody guard'; END IF;
      body:=replace(body,needle,'OR (l.quantity_after > 0 AND (NEW.shopping_bag_item_id IS NULL OR l.line_kind<>4) AND NOT EXISTS (');
  END CASE;
  EXECUTE regexp_replace(body,'BEGIN','BEGIN '||arm);
 END LOOP;
END $migration$;

REVOKE ALL ON FUNCTION game_npc_packed_purchase_current(UUID) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_npc_packed_purchase_current(UUID) TO oteryn_game_runtime;
