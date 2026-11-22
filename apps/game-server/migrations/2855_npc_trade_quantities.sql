-- Extended NPC quantities are a bounded multi-stack form of the existing trade.
-- Each individual item and coin line retains its 1..100 stack ceiling.
ALTER TABLE game_npc_trades
 DROP CONSTRAINT game_npc_trades_quantity_check,
 ADD CONSTRAINT game_npc_trades_quantity_check CHECK(quantity BETWEEN 1 AND 10000),
 DROP CONSTRAINT game_npc_trades_item_units_check,
 ADD CONSTRAINT game_npc_trades_item_units_check CHECK(item_units BETWEEN 1 AND 10000),
 DROP CONSTRAINT game_npc_trades_sell_all_shape,
 ADD CONSTRAINT game_npc_trades_sell_all_shape CHECK(
  CASE WHEN sell_all THEN side=2 AND quantity=item_units AND item_units<=2000
       ELSE quantity<=10000 AND item_units<=10000 AND item_units%quantity=0 END),
 DROP CONSTRAINT game_npc_trades_line_count_check,
 ADD CONSTRAINT game_npc_trades_line_count_check CHECK(line_count BETWEEN 1 AND 42);
ALTER TABLE game_npc_trade_lines DROP CONSTRAINT game_npc_trade_lines_line_ordinal_check,
 ADD CONSTRAINT game_npc_trade_lines_line_ordinal_check CHECK(line_ordinal BETWEEN 1 AND 42);

-- Retain every current exchange, package, subtype, account and conservation arm.
DO $migration$
DECLARE body TEXT; needle TEXT;
BEGIN
 SELECT pg_get_constraintdef(oid) INTO STRICT body FROM pg_constraint
  WHERE conrelid='game_npc_trades'::regclass AND conname='game_npc_trades_conservation';
 body:=replace(body,'line_count <= 23','line_count <= CASE WHEN side=1 THEN 42 ELSE 23 END');
 body:=replace(body,'WHEN sell_all THEN 23','WHEN sell_all OR side=2 THEN 23');
 EXECUTE 'ALTER TABLE game_npc_trades DROP CONSTRAINT game_npc_trades_conservation';
 EXECUTE 'ALTER TABLE game_npc_trades ADD CONSTRAINT game_npc_trades_conservation '||body;
 body:=pg_get_functiondef('game_npc_trade_consistency_guard()'::regprocedure);
 needle:='OR (NOT NEW.sell_all AND (item_lines <> 1 OR item_kind_lines <> 1))';
 IF position(needle IN body)=0 THEN RAISE EXCEPTION 'missing NPC single-stack shape guard'; END IF;
 body:=replace(body,needle,$guard$
 OR (NOT NEW.sell_all AND CASE WHEN item_kind_lines=1 THEN (item_lines<>1 OR item_kind_lines<>1)
 ELSE (
  NEW.side NOT IN(1,2) OR item_kind_lines NOT BETWEEN 1 AND 20
  OR NEW.item_units<>(SELECT coalesce(sum(abs(quantity_after-quantity_before)),0)
   FROM game_npc_trade_lines WHERE transaction_id=NEW.transaction_id
    AND line_kind=CASE NEW.side WHEN 1 THEN 4 ELSE 3 END)
  OR NOT EXISTS(SELECT 1 FROM game_npc_trade_lines WHERE transaction_id=NEW.transaction_id
    AND item_instance_id=NEW.item_instance_id AND line_kind=CASE NEW.side WHEN 1 THEN 4 ELSE 3 END
    AND line_ordinal=(SELECT min(line_ordinal) FROM game_npc_trade_lines
      WHERE transaction_id=NEW.transaction_id AND line_kind=CASE NEW.side WHEN 1 THEN 4 ELSE 3 END))
  OR (NEW.side=2 AND EXISTS(SELECT 1 FROM game_npc_trade_lines p JOIN game_npc_trade_lines l USING(transaction_id)
   WHERE l.transaction_id=NEW.transaction_id AND p.line_kind=3 AND l.line_kind=3
    AND p.line_ordinal<l.line_ordinal AND (p.placement_ordinal<=l.placement_ordinal OR p.quantity_after<>0)))
 ) END)
$guard$);
 EXECUTE body;
END $migration$;
