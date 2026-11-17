-- Admit only Wyda's two Bored exchange outcomes in the existing trade ledger.
ALTER TABLE game_npc_trades DROP CONSTRAINT game_npc_trades_quest_item_exchange_check;
ALTER TABLE game_npc_trades ADD CONSTRAINT game_npc_trades_quest_item_exchange_check CHECK (
    side<>3 OR (catalogue_revision>0 AND offer_index=0 AND quantity=1 AND unit_price=0
        AND total_gold_units=0 AND item_units=1 AND item_definition_family='Item'
        AND item_definition_revision_ref='definition-r1' AND burned_gold_units=0
        AND minted_gold_units=0 AND bank_debit_gold_units=0 AND NOT sell_all AND house_key IS NULL
        AND ((npc_key='oteryn:npc.al_dee' AND item_definition_production_key='oteryn:item.tibia.i3456' AND line_count=2)
          OR (npc_key='oteryn:npc.uzgod' AND item_definition_production_key='oteryn:item.tibia.i5908' AND line_count=3)
          OR (npc_key='oteryn:npc.seymour' AND item_definition_production_key='oteryn:item.tibia.i3374' AND line_count=2)
          OR (npc_key='oteryn:npc.wyda' AND item_definition_production_key IN ('oteryn:item.tibia.i3211','oteryn:item.tibia.i12617') AND line_count=2))));

DO $migration$
DECLARE body TEXT; source_case TEXT; begin_at INT;
BEGIN
    body:=pg_get_functiondef('game_npc_trade_consistency_guard()'::regprocedure);
    source_case:='WHEN NEW.npc_key=''oteryn:npc.seymour'' THEN ''oteryn:item.tibia.i2856'' ELSE ''oteryn:item.tibia.i3462'' END';
    IF strpos(body,source_case)=0 THEN RAISE EXCEPTION 'missing NPC exchange input guard'; END IF;
    body:=replace(body,source_case,
        'WHEN NEW.npc_key=''oteryn:npc.wyda'' THEN ''oteryn:item.tibia.i3734'' '||source_case);
    begin_at:=regexp_instr(body,'\mBEGIN\M',1,1,0,'i');
    IF begin_at=0 THEN RAISE EXCEPTION 'unexpected NPC trade consistency function'; END IF;
    body:=overlay(body placing $guard$BEGIN
    IF NEW.side=3 AND NEW.npc_key='oteryn:npc.wyda' AND NOT EXISTS (
        SELECT 1 FROM game_npc_trade_lines l JOIN game_item_instances i USING(item_instance_id)
        WHERE l.transaction_id=NEW.transaction_id AND l.line_ordinal=1 AND l.line_kind=3
          AND l.quantity_before-l.quantity_after=1 AND l.coin_worth IS NULL
          AND i.definition_family='Item' AND i.definition_production_key='oteryn:item.tibia.i3734'
          AND i.definition_revision_ref='definition-r1'
          AND NOT EXISTS(SELECT 1 FROM game_item_keys k WHERE k.item_instance_id=i.item_instance_id)
          AND NOT EXISTS(SELECT 1 FROM game_item_texts t WHERE t.item_instance_id=i.item_instance_id)
          AND NOT EXISTS(SELECT 1 FROM game_item_timed_states t WHERE t.item_instance_id=i.item_instance_id)
          AND NOT EXISTS(SELECT 1 FROM game_item_imbuements b WHERE b.item_instance_id=i.item_instance_id)
    ) THEN RAISE EXCEPTION 'Wyda requires one unchanged blood herb' USING ERRCODE='23514'; END IF;
    $guard$ from begin_at for 5);
    EXECUTE body;
END $migration$;

REVOKE ALL ON FUNCTION game_npc_trade_consistency_guard() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_npc_trade_consistency_guard() TO oteryn_game_runtime;
GRANT EXECUTE ON FUNCTION game_npc_trade_consistency_guard() TO oteryn_game_control;
