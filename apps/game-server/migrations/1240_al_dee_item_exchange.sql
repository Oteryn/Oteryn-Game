-- CRAFT-1 C8: the closed Al Dee exchange uses the existing NPC trade owner.
-- Command 7 confirms it; no new wire trade side or capability is exposed.
ALTER TABLE game_npc_trades DROP CONSTRAINT game_npc_trades_side_check;
ALTER TABLE game_npc_trades ADD CONSTRAINT game_npc_trades_side_check CHECK (side IN (1,2,3));
ALTER TABLE game_npc_trades ADD CONSTRAINT game_npc_trades_al_dee_exchange_check CHECK (
    side<>3 OR (npc_key='oteryn:npc.al_dee' AND catalogue_revision>0 AND offer_index=0
        AND quantity=1 AND unit_price=0 AND total_gold_units=0 AND item_units=1
        AND item_definition_family='Item' AND item_definition_production_key='oteryn:item.tibia.i3456'
        AND item_definition_revision_ref='definition-r1' AND line_count=2
        AND burned_gold_units=0 AND minted_gold_units=0 AND bank_debit_gold_units=0
        AND NOT sell_all AND house_key IS NULL));

-- Extend the current guard; retain every prior proof, including later mechanics.
DO $migration$
DECLARE body TEXT; before TEXT; after TEXT;
BEGIN
    body:=pg_get_functiondef('game_npc_trade_consistency_guard()'::regprocedure);
    before:='CASE NEW.side WHEN 1 THEN 4 ELSE 3 END';
    IF strpos(body,before)=0 THEN RAISE EXCEPTION 'unexpected NPC trade item-line guard'; END IF;
    body:=replace(body,before,'CASE NEW.side WHEN 1 THEN 4 WHEN 3 THEN 4 ELSE 3 END');
    -- Keep main's original opposite-side and RHS typed-definition substrings
    -- intact so later Yeo/fluid migrations can compose their exact guards.
    before:=E'AND l.line_kind IN (CASE NEW.side WHEN 1 THEN 3 ELSE 1 END,\n                                         CASE NEW.side WHEN 1 THEN 3 ELSE 4 END)';
    IF strpos(body,before)=0 THEN RAISE EXCEPTION 'unexpected NPC trade mint-kind guard'; END IF;
    body:=replace(body,before,before||' AND NOT (NEW.side=3 AND l.line_kind=4)');
    before:='(l.line_kind IN (3, 4) AND (i.definition_family, i.definition_production_key,';
    after:='(l.line_kind IN (3, 4) AND (i.definition_family, CASE WHEN NEW.side=3 AND l.line_kind=3 THEN CASE WHEN i.definition_production_key=''oteryn:item.tibia.i3462'' THEN NEW.item_definition_production_key ELSE NULL END ELSE i.definition_production_key END,';
    IF strpos(body,before)=0 THEN RAISE EXCEPTION 'unexpected NPC trade definition guard'; END IF;
    -- Only the first occurrence belongs to the line query (alias l). The
    -- later Sell All inventory query has different aliases and stays intact.
    body:=overlay(body placing after from strpos(body,before) for length(before));
    before:='BEGIN';
    after:=$guard$BEGIN
    IF NEW.side=3 AND EXISTS (SELECT 1 FROM game_npc_trade_lines l
        WHERE l.transaction_id=NEW.transaction_id AND l.line_kind NOT IN (3,4)) THEN
        RAISE EXCEPTION 'Quest item exchange allows only exact item burns and mint' USING ERRCODE='23514';
    END IF;
    IF NEW.side=3 AND NOT EXISTS (
        SELECT 1 FROM game_npc_trade_lines l JOIN game_item_instances i USING(item_instance_id)
        WHERE l.transaction_id=NEW.transaction_id AND l.line_ordinal=1 AND l.line_kind=3
          AND l.quantity_before=1 AND l.quantity_after=0 AND l.coin_worth IS NULL
          AND i.definition_family='Item' AND i.definition_production_key='oteryn:item.tibia.i3462'
          AND i.definition_revision_ref='definition-r1'
          AND NOT EXISTS(SELECT 1 FROM game_item_keys k WHERE k.item_instance_id=i.item_instance_id)
          AND NOT EXISTS(SELECT 1 FROM game_item_texts t WHERE t.item_instance_id=i.item_instance_id)
          AND NOT EXISTS(SELECT 1 FROM game_item_imbuements b WHERE b.item_instance_id=i.item_instance_id)
    ) THEN RAISE EXCEPTION 'Al Dee exchange requires one unchanged plain small axe' USING ERRCODE='23514'; END IF;
    $guard$;
    body:=overlay(body placing after from regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i') for 5);
    EXECUTE body;
END $migration$;
