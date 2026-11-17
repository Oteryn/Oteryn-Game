-- NPC-TRADE-3: fluid subtype belongs to the Item owner (0560), not quantity.
ALTER TABLE game_npc_trades ADD COLUMN fluid_subtype SMALLINT
 CHECK (fluid_subtype BETWEEN 0 AND 20);

-- Extend the existing loot/player-transition proof without removing either arm.
DO $$
DECLARE body TEXT; anchor TEXT := '    IF NOT EXISTS (';
BEGIN
 SELECT pg_get_functiondef('game_item_fluid_state_proven()'::regprocedure) INTO STRICT body;
 IF strpos(body,anchor)=0 THEN RAISE EXCEPTION 'missing fluid proof extension anchor'; END IF;
 body:=replace(body,anchor,$extension$
    IF TG_OP = 'INSERT' AND EXISTS (
      SELECT 1 FROM game_npc_trades t
      JOIN game_npc_trade_lines l USING(transaction_id)
      JOIN game_item_instances i ON i.item_instance_id=l.item_instance_id
      WHERE t.transaction_id=NEW.last_transaction_id AND t.side=1
        AND t.fluid_subtype=NEW.fluid_subtype AND t.world_id=NEW.world_id
        AND l.item_instance_id=NEW.item_instance_id AND l.line_kind=4
        AND i.minted_transaction_id=t.transaction_id AND i.quantity=1
        AND t.created_xact_id=pg_current_xact_id()
    ) THEN RETURN NEW; END IF;
    IF NOT EXISTS ($extension$);
 EXECUTE body;
END $$;

CREATE FUNCTION game_npc_trade_fluid_guard() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 IF NEW.fluid_subtype IS NOT NULL AND (
   NEW.item_definition_family<>'Item' OR NEW.item_definition_production_key NOT IN
    ('oteryn:item.tibia.i2874','oteryn:item.tibia.i2875','oteryn:item.tibia.i2877',
     'oteryn:item.tibia.i2880','oteryn:item.tibia.i2901','oteryn:item.tibia.i5552','oteryn:item.tibia.i9232')
   OR (NOT NEW.sell_all AND (NEW.quantity<>1 OR NEW.item_units<>1))) THEN
  RAISE EXCEPTION 'NPC fluid must be a single container' USING ERRCODE='23514';
 END IF;
 IF EXISTS(SELECT 1 FROM game_npc_trade_lines l
   LEFT JOIN game_item_fluid_states f USING(item_instance_id)
   WHERE l.transaction_id=NEW.transaction_id AND l.line_kind IN (3,4)
     AND COALESCE(f.fluid_subtype,0)<>COALESCE(NEW.fluid_subtype,0)) THEN
  RAISE EXCEPTION 'NPC fluid subtype mismatch' USING ERRCODE='23514';
 END IF;
 RETURN NULL;
END $$;
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_npc_trade_fluid_guard() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
CREATE CONSTRAINT TRIGGER game_npc_trade_fluid_guard AFTER INSERT ON game_npc_trades
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_npc_trade_fluid_guard();
REVOKE ALL ON FUNCTION game_npc_trade_fluid_guard() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_npc_trade_fluid_guard() TO oteryn_game_runtime;

-- SELL_ALL completeness is relative to the selected Item subtype. Preserve every
-- existing ledger, placement, timed-state and Hireling proof in the native guard.
DO $$
DECLARE body TEXT; anchor TEXT := $anchor$       -- A sell-all leaves no live default stack of the item in this backpack.
       OR (NEW.sell_all AND EXISTS (
        SELECT 1 FROM game_item_container_entries e
          JOIN game_item_instances i ON i.item_instance_id = e.item_instance_id
         WHERE e.parent_item_instance_id = NEW.backpack_item_instance_id
           AND i.lifecycle = 1$anchor$;
BEGIN
 SELECT pg_get_functiondef('game_npc_trade_consistency_guard()'::regprocedure) INTO STRICT body;
 IF strpos(body,anchor)=0 THEN RAISE EXCEPTION 'missing NPC sell-all subtype proof anchor'; END IF;
 body:=replace(body,anchor,anchor || $extension$
           AND COALESCE((SELECT f.fluid_subtype FROM game_item_fluid_states f
                         WHERE f.item_instance_id=i.item_instance_id),0)
               = COALESCE(NEW.fluid_subtype,0)$extension$);
 EXECUTE body;
END $$;
